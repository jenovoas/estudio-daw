use std::{env, fs, path::PathBuf, process::ExitCode};

use estudio_daw_audio_engine::RenderPlanBuilder;
use estudio_daw_audio_platform::{
    run_pipewire_duplex_for_targets, PipeWireStreamConfig, PipeWireTargets,
};
use estudio_daw_midi_engine::{
    play_midi_take, play_midi_take_interactive, play_midi_take_live, record_alsa_midi,
    record_alsa_midi_live, run_alsa_midi_control, MidiControlMap, MidiTake,
};
use estudio_daw_project_model::{
    attach_midi_take, export_dawproject, import_dawproject, quantize_midi_clip, Project,
};
use estudio_daw_runtime_diagnostics::{
    audio_devices, enumerate_alsa_midi_output_ports, midi_devices, monitor_alsa_midi, DeviceInfo,
};
use estudio_daw_session::{CommandBus, Session};
use std::time::Duration;

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
    let bus = CommandBus::bounded(64);
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
    let audiobox = |device: &&DeviceInfo| {
        let text = format!("{} {}", device.name, device.description).to_ascii_lowercase();
        text.contains("audiobox")
    };
    let capture_node = devices
        .iter()
        .find(|device| {
            device.media_class.to_ascii_lowercase().contains("source") && audiobox(device)
        })
        .map(|device| device.id);
    let playback_node = devices
        .iter()
        .find(|device| device.media_class.to_ascii_lowercase().contains("sink") && audiobox(device))
        .map(|device| device.id);
    println!("AudioBox targets: captura={capture_node:?}, reproducción={playback_node:?}");
    println!("Abriendo smoke test duplex PipeWire durante 3 segundos...");
    let render_plan = RenderPlanBuilder::new().build()?;
    let report = run_pipewire_duplex_for_targets(
        PipeWireStreamConfig::default(),
        render_plan,
        Duration::from_secs(3),
        PipeWireTargets {
            capture_node,
            playback_node,
        },
    )?;
    println!(
        "Callbacks: captura={} salida={}; muestras descartadas={} silencio de salida={}",
        report.capture_callbacks,
        report.output_callbacks,
        report.capture_dropped_samples,
        report.output_silence_samples
    );
    Ok(())
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
    let result = import_dawproject(input)?;
    fs::write(&output, serde_json::to_string_pretty(&result.project)?)?;
    println!("Importado a {}", output.display());
    print_warnings(&result.warnings);
    Ok(())
}

fn export_command(input: PathBuf, output: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let project: Project = serde_json::from_slice(&fs::read(&input)?)?;
    let result = export_dawproject(&project, &output)?;
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
    let mut project: Project = serde_json::from_slice(&fs::read(&project_path)?)?;
    let clip_id = attach_midi_take(
        &mut project,
        take,
        name.unwrap_or_else(|| "MIDI Take".into()),
    )?;
    fs::write(&output_path, serde_json::to_string_pretty(&project)?)?;
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
    let mut project: Project = serde_json::from_slice(&fs::read(&project_path)?)?;
    let changed = quantize_midi_clip(&mut project, &clip_id, grid)?;
    fs::write(&output_path, serde_json::to_string_pretty(&project)?)?;
    println!(
        "{} eventos cuantizados en {clip_id}; salida: {}",
        changed,
        output_path.display()
    );
    Ok(())
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
