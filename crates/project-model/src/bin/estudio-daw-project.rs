use std::{env, fs, path::PathBuf, process::ExitCode};

use estudio_daw_midi_engine::{record_alsa_midi, MidiTake};
use estudio_daw_project_model::{
    attach_midi_take, export_dawproject, import_dawproject, quantize_midi_clip, Project,
};
use estudio_daw_runtime_diagnostics::{audio_devices, midi_devices, monitor_alsa_midi, DeviceInfo};
use std::time::Duration;

fn usage() {
    eprintln!(concat!(
        "Uso:\n  estudio-daw-project devices\n  estudio-daw-project midi-monitor [nombre]\n  estudio-daw-project midi-record <segundos> <salida.json> [nombre]\n  estudio-daw-project audio-test\n  estudio-daw-project import <entrada.dawproject> <salida.json>\n  estudio-daw-project export <entrada.json> <salida.dawproject>",
            "\n  estudio-daw-project attach-take <toma.json> <proyecto.json> <salida.json> [nombre]",
        "\n  estudio-daw-project quantize <proyecto.json> <clip-id> <rejilla-ticks> <salida.json>",
        "\n  estudio-daw-project midi-summary <proyecto.json>"
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

fn audio_test_command() -> Result<(), Box<dyn std::error::Error>> {
    println!("Dispositivos de audio PipeWire:");
    print_devices(&audio_devices()?);
    println!(
        "Prueba de stream: pendiente del backend de audio RT; no se abre ningún stream todavía."
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

fn print_warnings(warnings: &[String]) {
    for warning in warnings {
        eprintln!("warning: {warning}");
    }
}
