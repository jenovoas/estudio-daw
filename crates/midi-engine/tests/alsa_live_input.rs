//! Prueba la ruta del lector ALSA que alimenta el instrumento nativo.
//!
//! Usa un cliente virtual con nombre único; no requiere conectar el KeyLab.
//! En entornos sin secuenciador ALSA, se omite con un mensaje explícito.
use std::{
    ffi::CString,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};

use alsa::seq::{EvNote, Event, EventType, PortCap, PortType, Seq};
use estudio_daw_midi_engine::run_alsa_midi_input_until;
use estudio_daw_runtime_diagnostics::NormalizedMidiEvent;

#[test]
fn live_input_subscribes_receives_a_note_and_stops() {
    if !Path::new("/dev/snd/seq").exists() {
        eprintln!("omitido: ALSA sequencer no disponible en este entorno");
        return;
    }

    let source_name = format!("Estudio DAW synth test source {}", std::process::id());
    let source = Seq::open(None, None, false).expect("abrir ALSA source client");
    source
        .set_client_name(&CString::new(source_name.clone()).unwrap())
        .unwrap();
    let source_port = source
        .create_simple_port(
            &CString::new("test-midi-out").unwrap(),
            PortCap::READ | PortCap::SUBS_READ,
            PortType::MIDI_GENERIC | PortType::APPLICATION,
        )
        .unwrap();

    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (event_tx, event_rx) = mpsc::sync_channel(1);
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let worker_name = source_name.clone();
    let listener = thread::spawn(move || {
        run_alsa_midi_input_until(
            &worker_name,
            &worker_stop,
            move |_, local_port| ready_tx.send(local_port).unwrap(),
            move |event| event_tx.send(event.clone()).unwrap(),
        )
    });

    let destination = match ready_rx.recv_timeout(Duration::from_secs(2)) {
        Ok(destination) => destination,
        Err(error) => {
            stop.store(true, Ordering::Release);
            let result = listener.join().expect("el lector debe terminar");
            panic!("el lector no anunció su suscripción ({error}): {result:?}");
        }
    };
    let mut note = Event::new(
        EventType::Noteon,
        &EvNote {
            channel: 0,
            note: 69,
            velocity: 91,
            off_velocity: 0,
            duration: 0,
        },
    );
    note.set_source(source_port);
    note.set_dest(destination);
    note.set_direct();
    source.event_output_direct(&mut note).unwrap();

    let received = event_rx.recv_timeout(Duration::from_secs(2));
    stop.store(true, Ordering::Release);
    let listener_result = listener.join().expect("el lector no debe entrar en panic");
    assert!(
        listener_result.is_ok(),
        "el lector debe finalizar limpiamente"
    );
    assert!(matches!(
        received.expect("debe recibirse la nota virtual"),
        NormalizedMidiEvent::NoteOn {
            note: 69,
            velocity: 91,
            ..
        }
    ));
}
