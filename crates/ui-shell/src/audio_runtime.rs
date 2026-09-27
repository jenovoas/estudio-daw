//! Host de reproducción usado por el transporte de escritorio.
//!
//! La preparación de instrumentos y el scheduler corren fuera del callback. El
//! callback PipeWire sólo procesa el plan ya compilado y memoria preasignada.

use estudio_daw_application::AudioProfileSettings;
use estudio_daw_audio_engine::{render_plan_exchange, RenderPlanBuilder};
use estudio_daw_audio_platform::{run_pipewire_output_until, PipeWireStreamConfig};
use estudio_daw_midi_engine::RecordedMidiMessage;
use estudio_daw_project_model::{InstrumentConfig, Project, TrackKind};
use estudio_daw_runtime_diagnostics::{audio_devices, DeviceInfo};
use estudio_daw_synth::{
    midi_event_queue, InstrumentMixerNode, SineSynthNode, SoundFontEventSender,
    SoundFontInstrumentWorker, SynthEventSender, SynthMidiEvent,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

enum EventSender {
    Sine(SynthEventSender),
    SoundFont(SoundFontEventSender),
}

impl EventSender {
    fn send(&mut self, event: SynthMidiEvent) {
        match self {
            Self::Sine(sender) => {
                let _ = sender.try_send(event);
            }
            Self::SoundFont(sender) => {
                let _ = sender.try_send(event);
            }
        }
    }
}

struct ScheduledEvent {
    at: Duration,
    sequence: u64,
    sender: usize,
    midi: SynthMidiEvent,
}

struct PlaybackSession {
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    thread: JoinHandle<Result<(), String>>,
}

#[derive(Default)]
pub struct AudioRuntimeHost {
    playback: Option<PlaybackSession>,
}

impl AudioRuntimeHost {
    pub fn is_connected(&self) -> bool {
        self.playback
            .as_ref()
            .is_some_and(|playback| playback.connected.load(Ordering::Acquire))
    }

    pub fn play(&mut self, project: &Project, profile: AudioProfileSettings) -> Result<(), String> {
        if let Some(playback) = &self.playback {
            if playback.connected.load(Ordering::Acquire) {
                playback.paused.store(false, Ordering::Release);
                return Ok(());
            }
        }
        self.stop()?;
        let config = PipeWireStreamConfig {
            period_frames: profile.device_period_frames as usize,
            ..PipeWireStreamConfig::default()
        };
        let max_samples = config
            .period_frames
            .saturating_mul(config.channels as usize);
        let paused = Arc::new(AtomicBool::new(false));
        let (plan, senders, schedule) = build_project_playback(
            project,
            config.sample_rate,
            max_samples,
            profile.playback_safety_frames as usize,
            Arc::clone(&paused),
        )?;
        let (control, processor) = render_plan_exchange(plan);
        drop(control);

        let stop = Arc::new(AtomicBool::new(false));
        let connected = Arc::new(AtomicBool::new(false));
        let playback_node = preferred_playback_node();
        let worker_stop = Arc::clone(&stop);
        let worker_paused = Arc::clone(&paused);
        let worker_connected = Arc::clone(&connected);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("estudio-daw-playback".into())
            .spawn(move || {
                let scheduler_stop = Arc::clone(&worker_stop);
                let scheduler_paused = Arc::clone(&worker_paused);
                let scheduler = thread::Builder::new()
                    .name("estudio-daw-midi-scheduler".into())
                    .spawn(move || {
                        schedule_events(schedule, senders, scheduler_stop, scheduler_paused)
                    })
                    .map_err(|error| error.to_string())?;

                let ready_error = ready_tx.clone();
                let result = run_pipewire_output_until(
                    config,
                    processor,
                    playback_node,
                    Arc::clone(&worker_stop),
                    Arc::clone(&worker_paused),
                    ready_tx,
                )
                .map_err(|error| error.to_string());
                worker_stop.store(true, Ordering::Release);
                worker_connected.store(false, Ordering::Release);
                if let Err(error) = &result {
                    let _ = ready_error.send(Err(error.clone()));
                }
                let scheduler_result = scheduler
                    .join()
                    .map_err(|_| "el scheduler MIDI terminó inesperadamente".to_owned());
                result.and(scheduler_result)
            })
            .map_err(|error| error.to_string())?;

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {
                if stop.load(Ordering::Acquire) {
                    let _ = thread.join();
                    return Err("el stream PipeWire terminó durante el inicio".to_owned());
                }
                connected.store(true, Ordering::Release);
                self.playback = Some(PlaybackSession {
                    stop,
                    paused,
                    connected,
                    thread,
                });
                Ok(())
            }
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(error)
            }
            Err(error) => {
                stop.store(true, Ordering::Release);
                let _ = thread.join();
                Err(format!("PipeWire no confirmó el stream: {error}"))
            }
        }
    }

    pub fn pause(&mut self, paused: bool) {
        if let Some(playback) = &self.playback {
            playback.paused.store(paused, Ordering::Release);
        }
    }

    pub fn stop(&mut self) -> Result<(), String> {
        let Some(playback) = self.playback.take() else {
            return Ok(());
        };
        playback.stop.store(true, Ordering::Release);
        playback.connected.store(false, Ordering::Release);
        playback
            .thread
            .join()
            .map_err(|_| "el hilo de reproducción terminó inesperadamente".to_owned())?
    }
}

fn preferred_playback_node() -> Option<String> {
    audio_devices()
        .ok()?
        .into_iter()
        .find(is_audiobox_sink)
        .map(|device| device.name)
}

fn is_audiobox_sink(device: &DeviceInfo) -> bool {
    let descriptor = format!("{} {}", device.name, device.description).to_ascii_lowercase();
    device.media_class.to_ascii_lowercase().contains("sink")
        && (descriptor.contains("audiobox") || descriptor.contains("pre sonus"))
}

impl Drop for AudioRuntimeHost {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn build_project_playback(
    project: &Project,
    sample_rate: u32,
    max_samples: usize,
    queue_target_frames: usize,
    paused: Arc<AtomicBool>,
) -> Result<
    (
        estudio_daw_audio_engine::RenderPlan,
        Vec<EventSender>,
        Vec<ScheduledEvent>,
    ),
    String,
> {
    let channels = 2;
    let bpm = if project.transport.tempo_bpm.is_finite() {
        project.transport.tempo_bpm.clamp(20.0, 999.0)
    } else {
        120.0
    };
    let mut sources: Vec<Box<dyn estudio_daw_audio_engine::AudioNode>> = Vec::new();
    let mut workers = Vec::new();
    let mut senders = Vec::new();
    let mut schedule = Vec::new();
    let mut sequence = 0_u64;

    for track in &project.tracks {
        if !matches!(&track.kind, TrackKind::Midi) {
            continue;
        }
        let sender_index = senders.len();
        let mut has_events = false;
        for clip in project
            .midi_clips
            .iter()
            .filter(|clip| clip.track_id == track.id)
        {
            for event in &clip.take.events {
                // Include events exactly at the clip boundary (notably a
                // NoteOff at the final tick), but never schedule beyond it.
                if event.tick > clip.duration_ticks {
                    continue;
                }
                let Some(midi) = synth_event(&event.message) else {
                    continue;
                };
                let absolute_tick = clip.start_tick.saturating_add(event.tick);
                let micros = ticks_to_micros(absolute_tick, clip.take.ppq, bpm);
                schedule.push(ScheduledEvent {
                    at: Duration::from_micros(micros),
                    sequence,
                    sender: sender_index,
                    midi,
                });
                sequence = sequence.saturating_add(1);
                has_events = true;
            }
        }
        if !has_events {
            schedule.retain(|event| event.sender != sender_index);
            continue;
        }

        match track.instrument.clone().unwrap_or(InstrumentConfig::Sine) {
            InstrumentConfig::Sine => {
                let (sender, receiver) = midi_event_queue();
                let node = SineSynthNode::new(sample_rate, channels, receiver)
                    .map_err(|error| error.to_string())?;
                sources.push(Box::new(node));
                senders.push(EventSender::Sine(sender));
            }
            InstrumentConfig::FluidSynth {
                soundfont,
                bank,
                program,
            } => {
                let (worker, node) =
                    SoundFontInstrumentWorker::start_with_queue_target_frames_paused(
                        soundfont.path,
                        sample_rate,
                        bank,
                        program,
                        queue_target_frames,
                        Arc::clone(&paused),
                    )
                    .map_err(|error| error.to_string())?;
                senders.push(EventSender::SoundFont(worker.event_sender()));
                sources.push(Box::new(node));
                workers.push(worker);
            }
        }
    }

    schedule.sort_by_key(|event| (event.at, event.sequence));
    let mut builder = RenderPlanBuilder::new();
    builder.add_node(InstrumentMixerNode::new(sources, max_samples));
    let mut plan = builder.build();
    for worker in workers {
        plan.retain_resource(worker);
    }
    Ok((plan, senders, schedule))
}

fn synth_event(message: &RecordedMidiMessage) -> Option<SynthMidiEvent> {
    match message {
        RecordedMidiMessage::NoteOn {
            channel,
            note,
            velocity,
        } => Some(SynthMidiEvent::NoteOn {
            channel: *channel,
            note: *note,
            velocity: *velocity,
        }),
        RecordedMidiMessage::NoteOff { channel, note, .. } => Some(SynthMidiEvent::NoteOff {
            channel: *channel,
            note: *note,
        }),
        RecordedMidiMessage::ControlChange {
            channel,
            controller,
            value,
        } if *controller == 64 || *controller == 123 => Some(SynthMidiEvent::ControlChange {
            channel: *channel,
            controller: *controller as u8,
            value: (*value).clamp(0, 127) as u8,
        }),
        _ => None,
    }
}

fn ticks_to_micros(ticks: u64, ppq: u32, bpm: f64) -> u64 {
    let ppq = u128::from(ppq.max(1));
    let milli_bpm = (bpm * 1_000.0).round().max(1.0) as u128;
    (u128::from(ticks)
        .saturating_mul(60_000_000_000)
        .checked_div(ppq.saturating_mul(milli_bpm))
        .unwrap_or(u128::MAX))
    .min(u128::from(u64::MAX)) as u64
}

fn schedule_events(
    schedule: Vec<ScheduledEvent>,
    mut senders: Vec<EventSender>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
) {
    let origin = Instant::now();
    let mut paused_total = Duration::ZERO;
    let mut pause_started = None;
    for event in schedule {
        loop {
            if stop.load(Ordering::Acquire) {
                return;
            }
            if paused.load(Ordering::Acquire) {
                pause_started.get_or_insert_with(Instant::now);
                thread::sleep(Duration::from_millis(2));
                continue;
            }
            if let Some(started) = pause_started.take() {
                paused_total = paused_total.saturating_add(started.elapsed());
            }
            let target = event.at.saturating_add(paused_total);
            let elapsed = origin.elapsed();
            if elapsed >= target {
                break;
            }
            thread::sleep((target - elapsed).min(Duration::from_millis(2)));
        }
        if let Some(sender) = senders.get_mut(event.sender) {
            sender.send(event.midi);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use estudio_daw_midi_engine::{MidiSource, MidiTake, RecordedMidiEvent};
    use estudio_daw_project_model::{
        ImportProvenance, MidiClip, TimeSignature, Track, TrackChannelConfig, TrackMixerState,
        TrackRole, Transport,
    };

    fn midi_track(id: &str) -> Track {
        Track {
            id: id.into(),
            name: id.into(),
            kind: TrackKind::Midi,
            role: TrackRole::Instrument,
            channel_config: TrackChannelConfig::default(),
            color: "#58a6b8".into(),
            mixer: TrackMixerState::default(),
            notes: Vec::new(),
            audio_channels: None,
            media_source: None,
            instrument: Some(InstrumentConfig::Sine),
        }
    }

    fn project_with_two_clips() -> Project {
        let source = MidiSource { client: 1, port: 0 };
        let take = MidiTake {
            ppq: 960,
            tempo_bpm: 120,
            duration_micros: 500_000,
            events: vec![
                RecordedMidiEvent {
                    tick: 0,
                    micros_since_start: 0,
                    source: source.clone(),
                    message: RecordedMidiMessage::NoteOn {
                        channel: 0,
                        note: 60,
                        velocity: 100,
                    },
                },
                RecordedMidiEvent {
                    tick: 960,
                    micros_since_start: 500_000,
                    source,
                    message: RecordedMidiMessage::NoteOff {
                        channel: 0,
                        note: 60,
                        release_velocity: 0,
                    },
                },
            ],
        };
        Project {
            schema_version: "estudio-daw.project.v4".into(),
            project_id: "playback-test".into(),
            transport: Transport {
                tempo_bpm: 120.0,
                time_signature: TimeSignature {
                    numerator: 4,
                    denominator: 4,
                },
            },
            tracks: vec![midi_track("track-1"), midi_track("track-2")],
            midi_clips: vec![
                MidiClip {
                    id: "clip-1".into(),
                    name: "first".into(),
                    track_id: "track-1".into(),
                    start_tick: 960,
                    duration_ticks: 1920,
                    take: take.clone(),
                },
                MidiClip {
                    id: "clip-2".into(),
                    name: "second".into(),
                    track_id: "track-2".into(),
                    start_tick: 0,
                    duration_ticks: 1920,
                    take,
                },
            ],
            audio_clips: Vec::new(),
            import_provenance: ImportProvenance {
                format: "internal".into(),
                format_version: "1".into(),
                source_file: String::new(),
                warnings: Vec::new(),
            },
        }
    }

    #[test]
    fn compiles_midi_clips_from_multiple_tracks_at_project_tempo_and_clip_offsets() {
        let project = project_with_two_clips();
        let (plan, senders, schedule) = build_project_playback(
            &project,
            48_000,
            512,
            1024,
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();

        assert_eq!(plan.node_count(), 1);
        assert_eq!(senders.len(), 2);
        assert_eq!(schedule.len(), 4);
        assert_eq!(schedule[0].at, Duration::ZERO);
        assert_eq!(schedule[1].at, Duration::from_millis(500));
        assert_eq!(schedule[2].at, Duration::from_millis(500));
        assert_eq!(schedule[3].at, Duration::from_secs(1));
        assert_ne!(schedule[1].sender, schedule[2].sender);
    }

    #[test]
    fn excludes_events_after_clip_duration_and_keeps_boundary_note_off() {
        let mut project = project_with_two_clips();
        for clip in &mut project.midi_clips {
            clip.duration_ticks = 960;
            clip.take.events.push(RecordedMidiEvent {
                tick: 961,
                micros_since_start: 500_521,
                source: MidiSource { client: 1, port: 0 },
                message: RecordedMidiMessage::NoteOn {
                    channel: 0,
                    note: 72,
                    velocity: 100,
                },
            });
        }
        let (_, _, schedule) = build_project_playback(
            &project,
            48_000,
            512,
            1024,
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
        assert_eq!(schedule.len(), 4);
        assert!(schedule.iter().any(|event| {
            event.at == Duration::from_millis(500)
                && matches!(event.midi, SynthMidiEvent::NoteOff { note: 60, .. })
        }));
    }

    #[test]
    fn maps_sustain_and_all_notes_off_controllers_for_clip_playback() {
        assert_eq!(
            synth_event(&RecordedMidiMessage::ControlChange {
                channel: 2,
                controller: 64,
                value: 127,
            }),
            Some(SynthMidiEvent::ControlChange {
                channel: 2,
                controller: 64,
                value: 127,
            })
        );
        assert_eq!(
            synth_event(&RecordedMidiMessage::ControlChange {
                channel: 2,
                controller: 123,
                value: 0,
            }),
            Some(SynthMidiEvent::ControlChange {
                channel: 2,
                controller: 123,
                value: 0,
            })
        );
        assert_eq!(ticks_to_micros(960, 960, 120.0), 500_000);
    }

    #[test]
    fn selects_audiobox_sink_without_confusing_monitor_source_or_other_devices() {
        let sink = DeviceInfo {
            id: 17,
            media_class: "Audio/Sink".into(),
            name: "alsa_output.usb-PreSonus_AudioBox_USB_96.analog-stereo".into(),
            description: "AudioBox USB 96".into(),
        };
        let monitor = DeviceInfo {
            id: 18,
            media_class: "Audio/Source".into(),
            name: "alsa_output.usb-PreSonus_AudioBox_USB_96.monitor".into(),
            description: "AudioBox USB 96 Monitor".into(),
        };
        assert!(is_audiobox_sink(&sink));
        assert!(!is_audiobox_sink(&monitor));
    }
}
