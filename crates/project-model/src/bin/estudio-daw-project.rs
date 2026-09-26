use std::{env, fs, path::PathBuf, process::ExitCode};

use estudio_daw_midi_engine::record_alsa_midi;
use estudio_daw_project_model::{export_dawproject, import_dawproject, Project};
use estudio_daw_runtime_diagnostics::{audio_devices, midi_devices, monitor_alsa_midi, DeviceInfo};
use std::time::Duration;

fn usage() {
    eprintln!(
        "Uso:\n  estudio-daw-project devices\n  estudio-daw-project midi-monitor [nombre]\n  estudio-daw-project midi-record <segundos> <salida.json> [nombre]\n  estudio-daw-project audio-test\n  estudio-daw-project import <entrada.dawproject> <salida.json>\n  estudio-daw-project export <entrada.json> <salida.dawproject>"
    );
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

fn print_warnings(warnings: &[String]) {
    for warning in warnings {
        eprintln!("warning: {warning}");
    }
}
