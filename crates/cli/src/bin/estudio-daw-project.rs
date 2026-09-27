//! Adaptador CLI: conecta argumentos, archivos y hardware con los crates del DAW.
//! Las mutaciones del proyecto pasan por `estudio-daw-command-bus`; aquí no se
//! replica la lógica de edición que también necesitarán la UI y el scripting.

use std::{
    env, fs,
    path::PathBuf,
    process::ExitCode,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};

use estudio_daw_application::{
    load_audio_runtime_settings, AudioProfileSettings, CommandEnvelope, CommandMetadata,
    DomainCommand, ProjectApplication, ProjectCommand,
};
use estudio_daw_audio_engine::{EqBandConfig, EqualizerNode, GainNode, RenderPlanBuilder};
use estudio_daw_audio_platform::{
    run_pipewire_duplex_for_targets, run_pipewire_duplex_for_targets_with_capture,
    run_pipewire_output_for_targets, run_pipewire_output_for_targets_with_monitor_capture,
    run_pipewire_output_for_targets_with_source_capture, PipeWireStreamConfig, PipeWireTargets,
    WavCaptureRecorder,
};
use estudio_daw_media_adapter::{
    generate_audio_proxy_ffmpeg, inspect_media_source, AudioProxyProfile, ProxyCacheManager,
};
use estudio_daw_midi_engine::{
    play_midi_take, play_midi_take_interactive, play_midi_take_live, record_alsa_midi,
    record_alsa_midi_live, run_alsa_midi_control, run_alsa_midi_input_until, MidiControlMap,
    MidiRecorder, MidiTake, RecordedMidiMessage, DEFAULT_PPQ,
};
use estudio_daw_project_model::{export_dawproject, import_dawproject, Project};
use estudio_daw_runtime_diagnostics::{
    audio_devices, enumerate_alsa_midi_output_ports, midi_devices, monitor_alsa_midi, DeviceInfo,
    NormalizedMidiEvent,
};
use estudio_daw_session::{CommandBus as SessionCommandBus, Session};
use estudio_daw_synth::{
    midi_event_queue, SineSynthNode, SoundFontEventSender, SoundFontInstrumentWorker,
    SynthEventSender, SynthMidiEvent,
};

fn usage() {
    eprintln!(concat!(
        "Uso:\n  estudio-daw-project devices\n  estudio-daw-project midi-monitor [nombre]\n  estudio-daw-project midi-record <segundos> <salida.json> [nombre]\n  estudio-daw-project midi-record-live <salida.json> [entrada] [control]\n  estudio-daw-project audio-test\n  estudio-daw-project import <entrada.dawproject> <salida.json>\n  estudio-daw-project export <entrada.json> <salida.dawproject>",
            "\n  estudio-daw-project attach-take <toma.json> <proyecto.json> <salida.json> [nombre]",
        "\n  estudio-daw-project quantize <proyecto.json> <clip-id> <rejilla-ticks> <salida.json>",
            "\n  estudio-daw-project midi-summary <proyecto.json>",
            "\n  estudio-daw-project midi-play <toma.json> [destino]",
            "\n  estudio-daw-project midi-play-live <toma.json> [destino] [control]",
            "\n  estudio-daw-project midi-control-monitor [nombre]",
            "\n  estudio-daw-project project-play <proyecto.json> [clip-id] [destino]",
            "\n  estudio-daw-project project-play-live <proyecto.json> [clip-id] [destino] [control]",
            "\n  estudio-daw-project midi-outputs"
            ,
            "\n  estudio-daw-project audio-record <segundos> <salida.wav>"
            ,
            "\n  estudio-daw-project midi-synth-live <segundos> <salida-toma.json> [entrada] [--soundfont archivo.sf2] [--bank N] [--program N] [--capture salida.wav] [--capture-monitor]"
            ,
            "\n  estudio-daw-project midi-synth-play <toma.json> [--soundfont archivo.sf2] [--bank N] [--program N] [--capture salida.wav] [--capture-monitor]"
            ,
            "\n  estudio-daw-project soundfont-presets <archivo.sf2>"
            ,
            "\n  estudio-daw-project proxy-audio <entrada> <salida>"
            ,
            "\n  estudio-daw-project proxy-track <proyecto.json> <pista> <cache> <salida.json>"
            ,
            "\n  estudio-daw-project attach-media <proyecto.json> <pista> <audio> <salida.json>"
            ,
            "\n  estudio-daw-project add-audio-clip <proyecto.json> <pista> <inicio-tick> <inicio-sample> <duracion-samples> <salida.json>"
   ));
}

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let Some(command) = args.next() else {
        usage();
        return ExitCode::from(2);
    };

    let result = match command.to_string_lossy().as_ref() {
        "devices" => devices_command(),
        "midi-monitor" => midi_monitor_command(
            args.next()
                .map(|value| value.to_string_lossy().into_owned()),
        ),
        "audio-test" => audio_test_command(),
        "audio-record" => {
            let Some(seconds) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(output) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            audio_record_command(seconds.to_string_lossy().as_ref(), output.into())
        }
        "midi-synth-live" => {
            let Some(seconds) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(take_output) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let trailing: Vec<String> = args
                .map(|value| value.to_string_lossy().into_owned())
                .collect();
            let mut capture_path = None;
            let mut capture_monitor = false;
            let mut synth_options = Vec::new();
            let mut index = 0;
            while index < trailing.len() {
                if trailing[index] == "--capture" {
                    index += 1;
                    let Some(path) = trailing.get(index) else {
                        eprintln!("--capture requiere una ruta WAV");
                        usage();
                        return ExitCode::from(2);
                    };
                    capture_path = Some(PathBuf::from(path));
                } else if trailing[index] == "--capture-monitor" {
                    capture_monitor = true;
                } else {
                    synth_options.push(trailing[index].clone());
                }
                index += 1;
            }
            if capture_monitor && capture_path.is_none() {
                eprintln!("--capture-monitor requiere --capture salida.wav");
                usage();
                return ExitCode::from(2);
            }
            let (midi_query, instrument) =
                match parse_synth_options(&synth_options, Some("KeyLab Essential 49 MID".into())) {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        eprintln!("{error}");
                        usage();
                        return ExitCode::from(2);
                    }
                };
            midi_synth_live_command(
                seconds.to_string_lossy().as_ref(),
                take_output.into(),
                midi_query.expect("entrada MIDI por defecto"),
                instrument,
                capture_path,
                capture_monitor,
            )
        }
        "midi-synth-play" => {
            let Some(take) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let trailing: Vec<String> = args
                .map(|value| value.to_string_lossy().into_owned())
                .collect();
            let mut capture_path = None;
            let mut capture_monitor = false;
            let mut synth_options = Vec::new();
            let mut index = 0;
            while index < trailing.len() {
                if trailing[index] == "--capture" {
                    index += 1;
                    let Some(path) = trailing.get(index) else {
                        eprintln!("--capture requiere una ruta WAV");
                        usage();
                        return ExitCode::from(2);
                    };
                    capture_path = Some(PathBuf::from(path));
                } else if trailing[index] == "--capture-monitor" {
                    capture_monitor = true;
                } else {
                    synth_options.push(trailing[index].clone());
                }
                index += 1;
            }
            if capture_monitor && capture_path.is_none() {
                eprintln!("--capture-monitor requiere --capture salida.wav");
                usage();
                return ExitCode::from(2);
            }
            let (unexpected_input, instrument) = match parse_synth_options(&synth_options, None) {
                Ok(parsed) => parsed,
                Err(error) => {
                    eprintln!("{error}");
                    usage();
                    return ExitCode::from(2);
                }
            };
            if unexpected_input.is_some() {
                eprintln!(
                    "midi-synth-play sólo acepta las opciones --soundfont, --bank y --program"
                );
                usage();
                return ExitCode::from(2);
            }
            midi_synth_play_command(take.into(), instrument, capture_path, capture_monitor)
        }
        "soundfont-presets" => {
            let Some(soundfont) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            soundfont_presets_command(soundfont.into())
        }
        "proxy-audio" => {
            let Some(input) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(output) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            proxy_audio_command(input.into(), output.into())
        }
        "proxy-track" => {
            let Some(project) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(track_id) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(cache) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(output) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            proxy_track_command(
                project.into(),
                track_id.to_string_lossy().into_owned(),
                cache.into(),
                output.into(),
            )
        }
        "attach-media" => {
            let Some(project) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(track_id) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(audio) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(output) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            attach_media_command(
                project.into(),
                track_id.to_string_lossy().into_owned(),
                audio.into(),
                output.into(),
            )
        }
        "add-audio-clip" => {
            let Some(project) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(track_id) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(start_tick) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(source_start) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(duration) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(output) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            add_audio_clip_command(
                project.into(),
                track_id.to_string_lossy().into_owned(),
                start_tick.to_string_lossy().as_ref(),
                source_start.to_string_lossy().as_ref(),
                duration.to_string_lossy().as_ref(),
                output.into(),
            )
        }
        "midi-record" => {
            let Some(seconds) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(output) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            midi_record_command(
                seconds.to_string_lossy().as_ref(),
                output.into(),
                args.next()
                    .map(|value| value.to_string_lossy().into_owned()),
            )
        }
        "midi-record-live" => {
            let Some(output) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            midi_record_live_command(
                output.into(),
                args.next()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "KeyLab Essential 49 MID".into()),
                args.next()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "KeyLab Essential 49 DAW".into()),
            )
        }
        "import" | "export" => {
            let Some(input) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(output) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            match command.to_string_lossy().as_ref() {
                "import" => import_command(input.into(), output.into()),
                "export" => export_command(input.into(), output.into()),
                _ => unreachable!(),
            }
        }
        "attach-take" => {
            let Some(take) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(project) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(output) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            attach_take_command(
                take.into(),
                project.into(),
                output.into(),
                args.next()
                    .map(|value| value.to_string_lossy().into_owned()),
            )
        }
        "quantize" => {
            let Some(project) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(clip_id) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(grid) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            let Some(output) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            quantize_command(
                project.into(),
                clip_id.to_string_lossy().into_owned(),
                grid.to_string_lossy().into_owned(),
                output.into(),
            )
        }
        "midi-summary" => {
            let Some(project) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            midi_summary_command(project.into())
        }
        "midi-play" => {
            let Some(take) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            midi_play_command(
                take.into(),
                args.next()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "FluidSynth".into()),
            )
        }
        "midi-play-live" => {
            let Some(take) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            midi_play_live_command(
                take.into(),
                args.next()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "FluidSynth".into()),
                args.next()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "KeyLab".into()),
            )
        }
        "midi-output" | "midi-outputs" => midi_outputs_command(),
        "midi-control-monitor" => midi_control_monitor_command(
            args.next()
                .map(|value| value.to_string_lossy().into_owned()),
        ),
        "project-play" => {
            let Some(project) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            project_play_command(
                project.into(),
                args.next()
                    .map(|value| value.to_string_lossy().into_owned()),
                args.next()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "FluidSynth".into()),
            )
        }
        "project-play-live" => {
            let Some(project) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            project_play_live_command(
                project.into(),
                args.next()
                    .map(|value| value.to_string_lossy().into_owned()),
                args.next()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "FluidSynth".into()),
                args.next()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "KeyLab Essential 49 DAW".into()),
            )
        }
        _ => {
            usage();
            Err("comando desconocido".into())
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

fn devices_command() -> Result<(), Box<dyn std::error::Error>> {
    println!("Dispositivos PipeWire:");
    print_devices(&estudio_daw_runtime_diagnostics::enumerate_pipewire()?);
    Ok(())
}

fn midi_monitor_command(query: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Puertos MIDI detectados:");
    print_devices(&midi_devices()?);
    monitor_alsa_midi(query.as_deref().unwrap_or("KeyLab"))?;
    Ok(())
}

fn midi_control_monitor_command(query: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let query = query.unwrap_or_else(|| "KeyLab".into());
    let map = MidiControlMap::for_port_query(&query);
    println!("Mapa de control seleccionado para '{query}'.");
    let bus = SessionCommandBus::bounded(64);
    let mut session = Session::default();
    run_alsa_midi_control(&query, &map, |command| {
        let domain_command = command.to_session_command();
        if let Err(error) = bus.dispatch(domain_command) {
            eprintln!("Comando MIDI descartado: {error}");
            return;
        }
        bus.drain_into(&mut session);
        let snapshot = session.snapshot();
        println!(
            "Sesión: transporte={:?} rec={} loop={} escena={} master={:.3}",
            snapshot.state,
            snapshot.recording,
            snapshot.loop_enabled,
            snapshot.scene_index,
            snapshot.master_volume
        );
        println!("  <- {:?}, valor {:.3}", command.action, command.value);
    })?;
    Ok(())
}

fn midi_play_live_command(
    take_path: PathBuf,
    output_query: String,
    control_query: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let take: MidiTake = serde_json::from_slice(&fs::read(take_path)?)?;
    println!("Reproducción live: salida MIDI='{output_query}', control MIDI='{control_query}'.");
    let sent = play_midi_take_live(&take, &output_query, &control_query)?;
    println!("Eventos enviados: {sent}");
    Ok(())
}

fn audio_test_command() -> Result<(), Box<dyn std::error::Error>> {
    println!("Dispositivos de audio PipeWire:");
    let devices = match audio_devices() {
        Ok(devices) => {
            print_devices(&devices);
            devices
        }
        Err(error) => {
            eprintln!("Diagnóstico PipeWire no disponible: {error}");
            Vec::new()
        }
    };
    let targets = audiobox_targets(&devices);
    println!(
        "AudioBox targets: captura={:?}, reproducción={:?}",
        targets.capture_node, targets.playback_node
    );
    println!("Abriendo smoke test duplex PipeWire durante 3 segundos...");
    let mut equalizer = EqualizerNode::new(48_000.0, 2)?;
    // Primer nodo DSP real de la ruta AudioBox → salida: elimina DC y
    // subgraves no musicales sin alterar de forma audible la prueba.
    equalizer.add_band(EqBandConfig::high_pass(20.0, 0.707))?;
    let mut render_builder = RenderPlanBuilder::new();
    render_builder.add_node(equalizer);
    let render_plan = render_builder.build();
    let report = run_pipewire_duplex_for_targets(
        PipeWireStreamConfig::default(),
        render_plan,
        Duration::from_secs(3),
        targets,
    )?;
    println!(
        "Callbacks: captura={} salida={}; muestras/callback: captura={} salida={}",
        report.capture_callbacks,
        report.output_callbacks,
        report.capture_last_samples,
        report.output_last_samples
    );
    println!(
        "Muestras totales: captura={} salida={}; descartadas={} silencio de salida={}",
        report.capture_total_samples,
        report.output_total_samples,
        report.capture_dropped_samples,
        report.output_silence_samples
    );
    Ok(())
}

/// Lista presets locales con los índices MIDI 0-based que aceptan los comandos.
fn soundfont_presets_command(soundfont_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let config = PipeWireStreamConfig::default();
    let mut synth = estudio_daw_synth::FluidSynthEngine::open(&soundfont_path, config.sample_rate)?;
    println!(
        "FluidSynth {}; banco local: {}",
        synth.version(),
        soundfont_path.display()
    );
    for preset in synth.presets()? {
        println!(
            "bank={} program={}  {}",
            preset.bank, preset.program, preset.name
        );
    }
    Ok(())
}

#[derive(Clone)]
enum SynthInstrument {
    Sine,
    SoundFont {
        path: String,
        bank: u16,
        program: u8,
    },
}

enum SynthEventSink {
    Sine(SynthEventSender),
    SoundFont(SoundFontEventSender),
}

enum SynthAudioSource {
    Sine(SineSynthNode),
    SoundFont(estudio_daw_synth::FluidSynthPcmNode),
}

impl estudio_daw_audio_engine::AudioNode for SynthAudioSource {
    fn process(
        &mut self,
        interleaved: &mut [f32],
    ) -> Result<(), estudio_daw_audio_engine::AudioNodeError> {
        match self {
            Self::Sine(node) => estudio_daw_audio_engine::AudioNode::process(node, interleaved),
            Self::SoundFont(node) => {
                estudio_daw_audio_engine::AudioNode::process(node, interleaved)
            }
        }
    }
}

impl SynthEventSink {
    fn try_send(&mut self, event: SynthMidiEvent) -> bool {
        match self {
            Self::Sine(sender) => sender.try_send(event),
            Self::SoundFont(sender) => sender.try_send(event),
        }
    }

    fn dropped_events(&self) -> usize {
        match self {
            Self::Sine(sender) => sender.dropped_events(),
            Self::SoundFont(sender) => sender.dropped_events(),
        }
    }
}

/// Lee opciones compartidas por reproducción y captura sintetizada.
fn parse_synth_options(
    arguments: &[String],
    default_input: Option<String>,
) -> Result<(Option<String>, SynthInstrument), String> {
    let mut input = default_input;
    let mut path = None;
    let mut bank = 0_u16;
    let mut program = 0_u8;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--soundfont" => {
                index += 1;
                path = Some(
                    arguments
                        .get(index)
                        .ok_or("--soundfont requiere una ruta SF2")?
                        .clone(),
                );
            }
            "--bank" => {
                index += 1;
                bank = arguments
                    .get(index)
                    .ok_or("--bank requiere un entero")?
                    .parse()
                    .map_err(|_| "banco SF2 inválido")?;
            }
            "--program" => {
                index += 1;
                program = arguments
                    .get(index)
                    .ok_or("--program requiere un entero")?
                    .parse()
                    .map_err(|_| "programa MIDI inválido (0-127)")?;
            }
            value if !value.starts_with('-') && input.is_none() => input = Some(value.to_owned()),
            value => return Err(format!("opción o argumento no reconocido: {value}")),
        }
        index += 1;
    }
    let instrument = match path {
        Some(path) => SynthInstrument::SoundFont {
            path,
            bank,
            program,
        },
        None if bank == 0 && program == 0 => SynthInstrument::Sine,
        None => return Err("--bank y --program requieren --soundfont".into()),
    };
    Ok((input, instrument))
}

/// Compila la cadena de instrumento, EQ y master antes de iniciar PipeWire.
/// La fuente seleccionada no cambia el contrato del callback de audio.
fn build_synth_render_plan(
    config: PipeWireStreamConfig,
    instrument: SynthInstrument,
    playback_safety_frames: usize,
) -> Result<
    (
        estudio_daw_audio_engine::RenderPlan,
        SynthEventSink,
        Option<Arc<SoundFontInstrumentWorker>>,
    ),
    Box<dyn std::error::Error>,
> {
    let (source, sink, worker) = match instrument {
        SynthInstrument::Sine => {
            let (sender, receiver) = midi_event_queue();
            (
                SynthAudioSource::Sine(SineSynthNode::new(
                    config.sample_rate,
                    config.channels as usize,
                    receiver,
                )?),
                SynthEventSink::Sine(sender),
                None,
            )
        }
        SynthInstrument::SoundFont {
            path,
            bank,
            program,
        } => {
            let (worker, node) = SoundFontInstrumentWorker::start_with_queue_target_frames(
                path,
                config.sample_rate,
                bank,
                program,
                playback_safety_frames,
            )?;
            let worker = Arc::new(worker);
            let sender = worker.event_sender();
            (
                SynthAudioSource::SoundFont(node),
                SynthEventSink::SoundFont(sender),
                Some(worker),
            )
        }
    };
    let mut equalizer = EqualizerNode::new(config.sample_rate as f32, config.channels as usize)?;
    equalizer.add_band(EqBandConfig::high_pass(20.0, 0.707))?;

    let mut builder = RenderPlanBuilder::new();
    builder.add_node(source);
    builder.add_node(equalizer);
    builder.add_node(GainNode::new(0.8));
    let mut plan = builder.build();
    if let Some(worker) = worker.as_ref() {
        // The plan owns the worker lifetime: a replacement plan can be
        // prepared while the active instrument keeps rendering, and the old
        // worker stops only after control reclaims its retired plan.
        plan.retain_resource(Arc::clone(worker));
    }
    Ok((plan, sink, worker))
}

/// Escucha y graba MIDI desde el KeyLab mientras el instrumento nativo toca por
/// PipeWire. El thread MIDI sólo publica eventos a la cola SPSC; nunca entra al
/// callback ni comparte estado mutable del sinte.
fn midi_synth_live_command(
    seconds: &str,
    take_output: PathBuf,
    midi_query: String,
    instrument: SynthInstrument,
    capture_output: Option<PathBuf>,
    capture_monitor: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let seconds: f64 = seconds.parse()?;
    if !(seconds.is_finite() && seconds > 0.0) {
        return Err("los segundos deben ser un número positivo".into());
    }

    let settings = load_audio_runtime_settings()?;
    let profile = settings.live_record;
    let config = capture_config(capture_monitor, profile.device_period_frames as usize);
    print_audio_profile("Live / Grabar", profile, config.sample_rate);
    let devices = audio_devices()?;
    let targets = synth_capture_targets(&devices, capture_monitor);
    let (render_plan, mut event_sink, instrument_worker) =
        build_synth_render_plan(config, instrument, profile.playback_safety_frames as usize)?;
    let stop = Arc::new(AtomicBool::new(false));
    let dropped_events = Arc::new(AtomicUsize::new(0));
    let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<String, String>>(1);
    let midi_origin = Instant::now();
    let worker_stop = Arc::clone(&stop);
    let worker_dropped = Arc::clone(&dropped_events);
    let worker_query = midi_query.clone();
    let listener = thread::Builder::new()
        .name("estudio-midi-instrument".into())
        .spawn(move || {
            let mut recorder = MidiRecorder::new(120, DEFAULT_PPQ);
            let ready_error_tx = ready_tx.clone();
            if let Err(error) = recorder.start_at(midi_origin) {
                let message = error.to_string();
                let _ = ready_tx.send(Err(message.clone()));
                return Err(message);
            }
            let mut recorder_error = None;
            let listen_result = run_alsa_midi_input_until(
                &worker_query,
                &worker_stop,
                move |label, _local_port| {
                    let _ = ready_tx.send(Ok(label));
                },
                |event| {
                    if let Some(synth_event) = synth_event_from_input(event) {
                        if !event_sink.try_send(synth_event) {
                            worker_dropped.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    // El recorder corre en este thread fuera del camino RT y
                    // mantiene los eventos completos, además de las notas.
                    if let Err(error) = recorder.record(event) {
                        recorder_error = Some(error.to_string());
                    }
                },
            );
            if let Err(error) = listen_result {
                let message = error.to_string();
                let _ = ready_error_tx.send(Err(message.clone()));
                return Err(message);
            }
            let take = recorder.stop().map_err(|error| error.to_string())?;
            if let Some(error) = recorder_error {
                return Err(error);
            }
            Ok(take)
        })?;

    let label = match ready_rx.recv()? {
        Ok(label) => label,
        Err(error) => {
            stop.store(true, Ordering::Release);
            let _ = listener.join();
            return Err(error.into());
        }
    };
    println!(
        "Instrumento nativo conectado a {label}; grabando {} segundos.",
        seconds
    );
    if capture_monitor {
        println!("Presiona el KeyLab; captura del monitor digital de AudioBox con quantum temporal de 256 frames.");
    } else {
        println!("Presiona el KeyLab; audio de salida dirigido a AudioBox si está disponible.");
    }

    let pipewire_origin = Instant::now();
    let stream_result: Result<_, Box<dyn std::error::Error>> = (|| {
        if let Some(path) = capture_output.as_ref() {
            let frames = (seconds.ceil() as usize)
                .saturating_add(2)
                .saturating_mul(config.sample_rate as usize);
            let recorder = WavCaptureRecorder::new(
                path.clone(),
                config.sample_rate,
                config.channels as u16,
                frames,
            )?;
            let duration = Duration::from_secs_f64(seconds);
            let (report, capture) = if capture_monitor {
                let capture_node = targets
                    .capture_node
                    .clone()
                    .ok_or("no se encontró el sink AudioBox para capturar su monitor")?;
                run_pipewire_output_for_targets_with_monitor_capture(
                    config,
                    render_plan,
                    duration,
                    targets.playback_node.clone(),
                    capture_node,
                    recorder,
                )?
            } else {
                let capture_node = targets
                    .capture_node
                    .clone()
                    .ok_or("no se encontró la entrada física de AudioBox")?;
                run_pipewire_output_for_targets_with_source_capture(
                    config,
                    render_plan,
                    duration,
                    targets.playback_node.clone(),
                    capture_node,
                    recorder,
                )?
            };
            Ok((report, Some(capture)))
        } else {
            Ok((
                run_pipewire_output_for_targets(
                    config,
                    render_plan,
                    Duration::from_secs_f64(seconds),
                    targets.playback_node.clone(),
                )?,
                None,
            ))
        }
    })();
    stop.store(true, Ordering::Release);
    let take_result = listener
        .join()
        .map_err(|_| "el thread de entrada MIDI terminó inesperadamente".to_string())?;
    let take = take_result.map_err(|error| format!("falló la entrada MIDI: {error}"))?;
    let (report, capture_report) = stream_result?;
    fs::write(&take_output, serde_json::to_vec_pretty(&take)?)?;

    println!(
        "Take guardada: {} eventos en {}. Cola MIDI descartó {} eventos; salida PipeWire {} callbacks.",
        take.events.len(),
        take_output.display(),
        dropped_events.load(Ordering::Relaxed),
        report.output_callbacks
    );
    if let Some(worker) = instrument_worker.as_ref() {
        print_soundfont_metrics(worker, config.sample_rate, config.period_frames);
    }
    if let Some(capture) = capture_report {
        let origin_to_capture_us = midi_origin
            .elapsed()
            .saturating_sub(pipewire_origin.elapsed())
            .as_micros() as u64
            + report.capture_start_delay_micros;
        println!(
            "AudioBox WAV: {} muestras, descartadas={}; primer callback de captura {} us desde el origen MIDI compartido.",
            capture.captured_samples,
            capture.dropped_samples,
            origin_to_capture_us
        );
    }
    Ok(())
}

/// Reproduce una toma MIDI en el mismo instrumento nativo y motor PipeWire.
fn midi_synth_play_command(
    take_path: PathBuf,
    instrument: SynthInstrument,
    capture_output: Option<PathBuf>,
    capture_monitor: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let take: MidiTake = serde_json::from_slice(&fs::read(&take_path)?)?;
    let settings = load_audio_runtime_settings()?;
    let profile = settings.multitrack_playback;
    let config = capture_config(capture_monitor, profile.device_period_frames as usize);
    print_audio_profile("Reproducción multipista", profile, config.sample_rate);
    let devices = audio_devices()?;
    let targets = synth_capture_targets(&devices, capture_monitor);
    let (render_plan, mut event_sink, instrument_worker) =
        build_synth_render_plan(config, instrument, profile.playback_safety_frames as usize)?;
    let scheduled_end = take
        .events
        .iter()
        .map(|event| event.micros_since_start)
        .max()
        .unwrap_or(0)
        .max(take.duration_micros);
    let duration = Duration::from_micros(scheduled_end).saturating_add(Duration::from_millis(400));
    let midi_origin = Instant::now();
    let scheduler_origin = midi_origin;
    let scheduler = thread::Builder::new()
        .name("estudio-midi-scheduler".into())
        .spawn(move || {
            for event in take.events {
                let target = Duration::from_micros(event.micros_since_start);
                loop {
                    let elapsed = scheduler_origin.elapsed();
                    if elapsed >= target {
                        break;
                    }
                    thread::sleep((target - elapsed).min(Duration::from_millis(2)));
                }
                if let Some(synth_event) = synth_event_from_recorded(&event.message) {
                    let _ = event_sink.try_send(synth_event);
                }
            }
            event_sink.dropped_events()
        })?;

    println!(
        "Reproduciendo {} por el instrumento nativo...",
        take_path.display()
    );
    let pipewire_origin = Instant::now();
    let stream_result: Result<_, Box<dyn std::error::Error>> = (|| {
        if let Some(path) = capture_output.as_ref() {
            let frames = (duration.as_secs() as usize)
                .saturating_add(2)
                .saturating_mul(config.sample_rate as usize);
            let recorder = WavCaptureRecorder::new(
                path.clone(),
                config.sample_rate,
                config.channels as u16,
                frames,
            )?;
            let (report, capture) = if capture_monitor {
                let capture_node = targets
                    .capture_node
                    .clone()
                    .ok_or("no se encontró el sink AudioBox para capturar su monitor")?;
                run_pipewire_output_for_targets_with_monitor_capture(
                    config,
                    render_plan,
                    duration,
                    targets.playback_node.clone(),
                    capture_node,
                    recorder,
                )?
            } else {
                let capture_node = targets
                    .capture_node
                    .clone()
                    .ok_or("no se encontró la entrada física de AudioBox")?;
                run_pipewire_output_for_targets_with_source_capture(
                    config,
                    render_plan,
                    duration,
                    targets.playback_node.clone(),
                    capture_node,
                    recorder,
                )?
            };
            Ok((report, Some(capture)))
        } else {
            Ok((
                run_pipewire_output_for_targets(
                    config,
                    render_plan,
                    duration,
                    targets.playback_node.clone(),
                )?,
                None,
            ))
        }
    })();
    let dropped_events = scheduler
        .join()
        .map_err(|_| "el scheduler MIDI terminó inesperadamente")?;
    let (report, capture_report) = stream_result?;
    println!(
        "Reproducción sintetizada finalizada: {} callbacks PipeWire, {} frames de salida, último bloque={} muestras, última solicitud={} frames; eventos MIDI descartados={dropped_events}.",
        report.output_callbacks,
        report.output_total_samples / u64::from(config.channels),
        report.output_last_samples,
        report.output_last_requested_frames,
    );
    if let Some(worker) = instrument_worker.as_ref() {
        print_soundfont_metrics(worker, config.sample_rate, config.period_frames);
    }
    if let Some(capture) = capture_report {
        let origin_to_capture_us = midi_origin
            .elapsed()
            .saturating_sub(pipewire_origin.elapsed())
            .as_micros() as u64
            + report.capture_start_delay_micros;
        println!(
            "AudioBox WAV: {} muestras, descartadas={}; primer callback de captura {} us desde el origen MIDI compartido.",
            capture.captured_samples,
            capture.dropped_samples,
            origin_to_capture_us
        );
    }
    Ok(())
}

/// Reporta salud y profundidad observada del ring sin confundirla con
/// latencia acústica total: faltan la latencia del dispositivo y el recorrido
/// físico de entrada/salida para medir ese extremo a extremo.
fn print_soundfont_metrics(
    worker: &SoundFontInstrumentWorker,
    sample_rate: u32,
    period_frames: usize,
) {
    let queue = worker.pcm_queue_metrics();
    let peak_buffer_ms = queue.peak_frames as f64 * 1_000.0 / f64::from(sample_rate);
    let period_ms = period_frames as f64 * 1_000.0 / f64::from(sample_rate);
    println!(
        "SoundFont: underruns={}, worker_errors={}, PCM ring actual={} frames, pico={} / {} frames (pico equivalente={peak_buffer_ms:.2} ms); PipeWire periodo solicitado={period_frames} frames ({period_ms:.2} ms). La latencia total del hardware no se mide aquí.",
        worker.underrun_samples(),
        worker.worker_errors(),
        queue.current_frames,
        queue.peak_frames,
        queue.capacity_frames,
    );
}

fn print_audio_profile(label: &str, profile: AudioProfileSettings, sample_rate: u32) {
    print_audio_period(label, profile.device_period_frames, sample_rate);
    let safety_ms = f64::from(profile.playback_safety_frames) * 1_000.0 / f64::from(sample_rate);
    println!(
        "Objetivo de cola de reproducción={} frames ({safety_ms:.2} ms) a {sample_rate} Hz.",
        profile.playback_safety_frames,
    );
}

fn print_audio_period(label: &str, period_frames: u32, sample_rate: u32) {
    let period_ms = f64::from(period_frames) * 1_000.0 / f64::from(sample_rate);
    println!(
        "Perfil {label}: periodo solicitado={period_frames} frames ({period_ms:.2} ms) a {sample_rate} Hz. PipeWire puede negociar otro quantum; el periodo efectivo no está disponible en esta interfaz."
    );
}

fn synth_event_from_input(event: &NormalizedMidiEvent) -> Option<SynthMidiEvent> {
    match event {
        NormalizedMidiEvent::NoteOn {
            channel,
            note,
            velocity,
            ..
        } if *velocity > 0 => Some(SynthMidiEvent::NoteOn {
            channel: *channel,
            note: *note,
            velocity: *velocity,
        }),
        NormalizedMidiEvent::NoteOn { channel, note, .. }
        | NormalizedMidiEvent::NoteOff { channel, note, .. } => Some(SynthMidiEvent::NoteOff {
            channel: *channel,
            note: *note,
        }),
        NormalizedMidiEvent::ControlChange {
            channel,
            controller,
            value,
            ..
        } => synth_control_change(*channel, *controller, *value),
        _ => None,
    }
}

fn synth_event_from_recorded(message: &RecordedMidiMessage) -> Option<SynthMidiEvent> {
    match message {
        RecordedMidiMessage::NoteOn {
            channel,
            note,
            velocity,
        } if *velocity > 0 => Some(SynthMidiEvent::NoteOn {
            channel: *channel,
            note: *note,
            velocity: *velocity,
        }),
        RecordedMidiMessage::NoteOn { channel, note, .. }
        | RecordedMidiMessage::NoteOff { channel, note, .. } => Some(SynthMidiEvent::NoteOff {
            channel: *channel,
            note: *note,
        }),
        RecordedMidiMessage::ControlChange {
            channel,
            controller,
            value,
        } => synth_control_change(*channel, *controller, *value),
        _ => None,
    }
}

fn synth_control_change(channel: u8, controller: u32, value: i32) -> Option<SynthMidiEvent> {
    (controller == 64).then_some(SynthMidiEvent::ControlChange {
        channel,
        controller: 64,
        value: value.clamp(0, 127) as u8,
    })
}

fn audio_record_command(seconds: &str, output: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let seconds: f64 = seconds.parse()?;
    if !(seconds.is_finite() && seconds > 0.0) {
        return Err("los segundos deben ser un número positivo".into());
    }
    let settings = load_audio_runtime_settings()?;
    let profile = settings.live_record;
    let devices = audio_devices()?;
    let targets = audiobox_targets(&devices);
    let config = capture_config(false, profile.device_period_frames as usize);
    print_audio_period(
        "Live / Grabar",
        profile.device_period_frames,
        config.sample_rate,
    );
    println!(
        "Grabación WAV: objetivo de cola PCM SoundFont no aplica; PipeWire muestra su quantum efectivo en el grafo mientras el stream está activo."
    );
    let mut equalizer = EqualizerNode::new(config.sample_rate as f32, config.channels as usize)?;
    equalizer.add_band(EqBandConfig::high_pass(20.0, 0.707))?;
    let mut render_builder = RenderPlanBuilder::new();
    render_builder.add_node(equalizer);
    let render_plan = render_builder.build();
    // Diez segundos de margen amortiguan ráfagas de disco sin convertir el
    // callback en un productor bloqueante. El writer sigue siendo el dueño de
    // la persistencia y reporta overflow si el sistema no alcanza.
    let recorder = WavCaptureRecorder::new(
        output.clone(),
        config.sample_rate,
        config.channels as u16,
        config.sample_rate as usize * 10,
    )?;
    println!(
        "Grabando AudioBox durante {seconds:.2}s en {}...",
        output.display()
    );
    let (report, capture) = run_pipewire_duplex_for_targets_with_capture(
        config,
        render_plan,
        Duration::from_secs_f64(seconds),
        targets,
        recorder,
    )?;
    println!(
        "Captura finalizada: {} muestras, descartadas={}; callbacks captura={} salida={}",
        capture.captured_samples,
        capture.dropped_samples,
        report.capture_callbacks,
        report.output_callbacks
    );
    Ok(())
}

fn proxy_audio_command(input: PathBuf, output: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !input.is_file() {
        return Err(format!("no existe el archivo de audio fuente: {}", input.display()).into());
    }
    let source = inspect_media_source(&input)?;
    let profile = AudioProxyProfile::opus_preview();
    let asset = generate_audio_proxy_ffmpeg(&source, &output, &profile)?;
    println!(
        "Proxy generado: {} (perfil={}, sha256={})",
        asset.path.display(),
        asset.profile,
        asset.source_hash
    );
    Ok(())
}

fn proxy_track_command(
    project_path: PathBuf,
    track_id: String,
    cache_path: PathBuf,
    output_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let project: Project = serde_json::from_slice(&fs::read(&project_path)?)?;
    let track = project
        .tracks
        .iter()
        .find(|track| track.id == track_id)
        .ok_or_else(|| format!("no existe la pista '{track_id}'"))?;
    if track.kind != estudio_daw_project_model::TrackKind::Audio {
        return Err(format!("la pista '{track_id}' no es de audio").into());
    }
    let source = project
        .audio_sources
        .iter()
        .find(|source| source.owner_track_id == track_id)
        .ok_or_else(|| format!("la pista '{track_id}' no tiene una fuente de audio asociada"))?;
    let source_id = source.id.clone();
    let mut media = source.media.clone();
    let manager = ProxyCacheManager::new(cache_path);
    let profile = AudioProxyProfile::opus_preview();
    let state = manager.ensure_audio_proxy(&mut media, &profile)?;
    let _project = apply_project_command_file(
        project_path,
        output_path.clone(),
        "cli-set-audio-source-proxy",
        ProjectCommand::SetAudioSourceProxy {
            source_id,
            proxy: media.proxy,
        },
    )?;
    println!(
        "Proxy de pista listo: pista={} estado={state:?} proyecto={}",
        track_id,
        output_path.display()
    );
    Ok(())
}

fn attach_media_command(
    project_path: PathBuf,
    track_id: String,
    audio_path: PathBuf,
    output_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = inspect_media_source(&audio_path)?;
    let _project = apply_project_command_file(
        project_path,
        output_path.clone(),
        "cli-attach-media",
        ProjectCommand::AttachMediaSource {
            track_id: track_id.clone(),
            source,
        },
    )?;
    println!(
        "Fuente asociada: pista={} audio={} proyecto={}",
        track_id,
        audio_path.display(),
        output_path.display()
    );
    Ok(())
}

fn add_audio_clip_command(
    project_path: PathBuf,
    track_id: String,
    start_tick: &str,
    source_start: &str,
    duration: &str,
    output_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let project = apply_project_command_file(
        project_path,
        output_path.clone(),
        "cli-add-audio-clip",
        ProjectCommand::AddAudioClip {
            track_id: track_id.clone(),
            name: "Audio region".into(),
            start_tick: start_tick.parse()?,
            source_start_samples: source_start.parse()?,
            duration_samples: duration.parse()?,
            sample_rate: 48_000,
            channels: 2,
        },
    )?;
    let id = project
        .audio_clips
        .last()
        .map(|clip| clip.id.as_str())
        .unwrap_or("(sin id)");
    println!(
        "Clip de audio creado: {} en pista={} proyecto={}",
        id,
        track_id,
        output_path.display()
    );
    Ok(())
}

fn audiobox_targets(devices: &[DeviceInfo]) -> PipeWireTargets {
    let is_audiobox = |device: &&DeviceInfo| {
        let text = format!("{} {}", device.name, device.description).to_ascii_lowercase();
        text.contains("audiobox")
    };
    PipeWireTargets {
        capture_node: devices
            .iter()
            .find(|device| {
                device.media_class.to_ascii_lowercase().contains("source") && is_audiobox(device)
            })
            .map(|device| device.name.clone()),
        playback_node: devices
            .iter()
            .find(|device| {
                device.media_class.to_ascii_lowercase().contains("sink") && is_audiobox(device)
            })
            .map(|device| device.name.clone()),
    }
}

fn capture_config(capture_monitor: bool, period_frames: usize) -> PipeWireStreamConfig {
    let mut config = PipeWireStreamConfig::default();
    config.period_frames = period_frames;
    if capture_monitor {
        // El monitor del sink puede renegociar el graph a callbacks minúsculos.
        // Este quantum se fuerza sólo mientras los streams de captura vivan.
        config.force_graph_quantum = Some(period_frames);
        config.capture_sink_monitor = true;
    }
    config
}

fn synth_capture_targets(devices: &[DeviceInfo], capture_monitor: bool) -> PipeWireTargets {
    let mut targets = audiobox_targets(devices);
    if capture_monitor {
        // El source monitor es una salida del sink; no es una entrada física.
        // Input 1 (micrófono) e Input 2 (guitarra) quedan intactas.
        targets.capture_node = targets.playback_node.clone();
    }
    targets
}

fn midi_record_command(
    seconds: &str,
    output: PathBuf,
    query: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let seconds: f64 = seconds.parse()?;
    if !(seconds.is_finite() && seconds > 0.0) {
        return Err("los segundos deben ser un número positivo".into());
    }
    let take = record_alsa_midi(
        query.as_deref().unwrap_or("KeyLab"),
        Duration::from_secs_f64(seconds),
        120,
    )?;
    fs::write(&output, serde_json::to_string_pretty(&take)?)?;
    println!("Toma MIDI guardada en {}", output.display());
    Ok(())
}

fn print_devices(devices: &[DeviceInfo]) {
    if devices.is_empty() {
        println!("  (ninguno)");
    }
    for device in devices {
        println!(
            "  [{}] {} — {} ({})",
            device.id, device.name, device.description, device.media_class
        );
    }
}

fn import_command(input: PathBuf, output: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(&input)?;
    let result = import_dawproject(&bytes)?;
    fs::write(&output, serde_json::to_string_pretty(&result.project)?)?;
    println!("Importado a {}", output.display());
    print_warnings(&result.warnings);
    Ok(())
}

fn export_command(input: PathBuf, output: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let project: Project = serde_json::from_slice(&fs::read(&input)?)?;
    let (bytes, result) = export_dawproject(&project)?;
    fs::write(&output, bytes)?;
    println!("Exportado a {}", output.display());
    print_warnings(&result.warnings);
    Ok(())
}

fn attach_take_command(
    take_path: PathBuf,
    project_path: PathBuf,
    output_path: PathBuf,
    name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let take: MidiTake = serde_json::from_slice(&fs::read(&take_path)?)?;
    let project = apply_project_command_file(
        project_path,
        output_path.clone(),
        "cli-attach-midi-take",
        ProjectCommand::AttachMidiTake {
            take,
            name: name.unwrap_or_else(|| "MIDI Take".into()),
        },
    )?;
    let clip_id = project
        .midi_clips
        .last()
        .map(|clip| clip.id.as_str())
        .unwrap_or("(sin id)");
    println!("Clip MIDI {clip_id} adjuntado a {}", output_path.display());
    Ok(())
}

fn quantize_command(
    project_path: PathBuf,
    clip_id: String,
    grid: String,
    output_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let grid: u64 = grid.parse()?;
    let _project = apply_project_command_file(
        project_path,
        output_path.clone(),
        "cli-quantize-midi-clip",
        ProjectCommand::QuantizeMidiClip {
            clip_id: clip_id.clone(),
            grid_ticks: grid,
        },
    )?;
    println!(
        "Cuantización aplicada a {clip_id}; salida: {}",
        output_path.display()
    );
    Ok(())
}

/// Lee un proyecto, ejecuta una mutación mediante el bus de dominio y persiste
/// el snapshot resultante. La CLI y una futura UI comparten así el mismo contrato.
fn apply_project_command_file(
    input_path: PathBuf,
    output_path: PathBuf,
    command_id: &str,
    command: ProjectCommand,
) -> Result<Project, Box<dyn std::error::Error>> {
    let mut application = ProjectApplication::open(input_path)?;
    application.dispatch(CommandEnvelope {
        metadata: CommandMetadata::user(command_id),
        command: DomainCommand::Project(command),
    })?;
    let project = application.snapshot().project.project;
    application.save_to(output_path)?;
    Ok(project)
}

fn midi_summary_command(project_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let project: Project = serde_json::from_slice(&fs::read(&project_path)?)?;
    if project.midi_clips.is_empty() {
        println!("El proyecto no contiene clips MIDI.");
        return Ok(());
    }
    for clip in &project.midi_clips {
        println!(
            "{} | {} | pista={} | eventos={} | duración={} ticks | tempo={} BPM",
            clip.id,
            clip.name,
            clip.track_id,
            clip.take.events.len(),
            clip.duration_ticks,
            clip.take.tempo_bpm
        );
    }
    Ok(())
}

fn midi_play_command(
    take_path: PathBuf,
    destination: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let take: MidiTake = serde_json::from_slice(&fs::read(&take_path)?)?;
    play_midi_take(&take, &destination)?;
    Ok(())
}

fn midi_record_live_command(
    output_path: PathBuf,
    input_query: String,
    control_query: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let take = record_alsa_midi_live(&input_query, &control_query, 120)?;
    fs::write(&output_path, serde_json::to_string_pretty(&take)?)?;
    println!("Toma live guardada en {}", output_path.display());
    Ok(())
}

/// Reproduce un clip que ya pertenece a la sesión.
///
/// Mantener esta resolución en la CLI nos permite reutilizar el mismo motor
/// `play_midi_take` mientras todavía no existe el transporte global del DAW.
/// Más adelante esta función será reemplazada por una orden del motor y no
/// tendrá que cargar el JSON completo en cada reproducción.
fn project_play_command(
    project_path: PathBuf,
    clip_id: Option<String>,
    destination: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let project: Project = serde_json::from_slice(&fs::read(&project_path)?)?;
    let clip = match clip_id.filter(|id| !id.is_empty()) {
        Some(ref id) => project
            .midi_clips
            .iter()
            .find(|clip| clip.id == *id)
            .ok_or_else(|| format!("no existe el clip MIDI '{id}'"))?,
        None => project
            .midi_clips
            .first()
            .ok_or("el proyecto no contiene clips MIDI")?,
    };
    println!("Clip seleccionado: {} ({})", clip.id, clip.name);
    play_midi_take_interactive(&clip.take, &destination)?;
    Ok(())
}

/// Reproduce el primer clip del proyecto y conecta el transporte al puerto
/// DAW del controlador MIDI. La resolución de proyecto y la reproducción
/// siguen siendo las mismas; sólo cambia la fuente de comandos de transporte.
fn project_play_live_command(
    project_path: PathBuf,
    clip_id: Option<String>,
    destination: String,
    control: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let project: Project = serde_json::from_slice(&fs::read(&project_path)?)?;
    let clip = match clip_id.filter(|id| !id.is_empty()) {
        Some(ref id) => project
            .midi_clips
            .iter()
            .find(|clip| clip.id == *id)
            .ok_or_else(|| format!("no existe el clip MIDI '{id}'"))?,
        None => project
            .midi_clips
            .first()
            .ok_or("el proyecto no contiene clips MIDI")?,
    };
    println!(
        "Clip live seleccionado: {} ({}), control={control}",
        clip.id, clip.name
    );
    play_midi_take_live(&clip.take, &destination, &control)?;
    Ok(())
}

fn midi_outputs_command() -> Result<(), Box<dyn std::error::Error>> {
    let outputs = enumerate_alsa_midi_output_ports()?;
    if outputs.is_empty() {
        println!("No hay destinos MIDI ALSA disponibles.");
    } else {
        for (address, label) in outputs {
            println!("[{}:{}] {}", address.client, address.port, label);
        }
    }
    Ok(())
}

fn print_warnings(warnings: &[String]) {
    for warning in warnings {
        eprintln!("warning: {warning}");
    }
}

#[cfg(test)]
mod synth_option_tests {
    use super::*;

    #[test]
    fn soundfont_options_choose_a_local_preset_and_keep_input_name() {
        let args = vec![
            "KeyLab Essential 49 MID".into(),
            "--soundfont".into(),
            "/banks/piano.sf2".into(),
            "--bank".into(),
            "2".into(),
            "--program".into(),
            "11".into(),
        ];
        let (input, instrument) = parse_synth_options(&args, None).unwrap();
        assert_eq!(input.as_deref(), Some("KeyLab Essential 49 MID"));
        assert!(matches!(
            instrument,
            SynthInstrument::SoundFont {
                bank: 2,
                program: 11,
                ..
            }
        ));
    }

    #[test]
    fn defaults_to_sine_and_requires_soundfont_for_nondefault_preset() {
        let (input, instrument) = parse_synth_options(&[], Some("KeyLab".into())).unwrap();
        assert_eq!(input.as_deref(), Some("KeyLab"));
        assert!(matches!(instrument, SynthInstrument::Sine));
        assert!(parse_synth_options(&["--program".into(), "5".into()], None).is_err());
    }

    #[test]
    fn capture_config_uses_the_selected_period_for_normal_and_monitor_streams() {
        let regular = capture_config(false, 128);
        assert_eq!(regular.period_frames, 128);
        assert_eq!(regular.force_graph_quantum, None);

        let monitor = capture_config(true, 512);
        assert_eq!(monitor.period_frames, 512);
        assert_eq!(monitor.force_graph_quantum, Some(512));
        assert!(monitor.capture_sink_monitor);
    }

    #[test]
    fn synth_translation_preserves_sustain_controller_for_live_and_take_playback() {
        let live = synth_control_change(2, 64, 127);
        assert_eq!(
            live,
            Some(SynthMidiEvent::ControlChange {
                channel: 2,
                controller: 64,
                value: 127,
            })
        );

        let recorded = synth_control_change(2, 64, 0);
        assert_eq!(
            recorded,
            Some(SynthMidiEvent::ControlChange {
                channel: 2,
                controller: 64,
                value: 0,
            })
        );
    }
}
