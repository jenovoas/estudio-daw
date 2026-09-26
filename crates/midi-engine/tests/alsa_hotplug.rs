//! Prueba de integración del ciclo de vida de puertos MIDI ALSA.
//!
//! No requiere el KeyLab ni otro controlador físico: crea clientes y puertos
//! virtuales, elimina el emisor y vuelve a suscribir un emisor nuevo. En hosts
//! sin el secuenciador ALSA (`/dev/snd/seq`) se omite con un diagnóstico claro.
use std::{
    ffi::CString,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use alsa::seq::{Addr, EvNote, Event, EventType, PortCap, PortSubscribe, PortType, Seq};

fn open_seq_client(name: &str) -> Option<Seq> {
    let seq = match Seq::open(None, None, true) {
        Ok(seq) => seq,
        Err(error) if !Path::new("/dev/snd/seq").exists() => {
            eprintln!("omitido: ALSA sequencer no disponible ({error})");
            return None;
        }
        Err(error) => panic!("no se pudo abrir el cliente ALSA de prueba: {error}"),
    };
    let client_name = CString::new(name).expect("el nombre de prueba no contiene NUL");
    seq.set_client_name(&client_name)
        .expect("debe poder nombrar el cliente virtual");
    Some(seq)
}

fn create_port(seq: &Seq, name: &str, caps: PortCap) -> i32 {
    let port_name = CString::new(name).expect("el nombre de puerto no contiene NUL");
    seq.create_simple_port(
        &port_name,
        caps,
        PortType::MIDI_GENERIC | PortType::APPLICATION,
    )
    .expect("debe poder crear el puerto virtual")
}

fn subscribe(seq: &Seq, source: Addr, destination: Addr) {
    let subscription = PortSubscribe::empty().expect("debe crear la suscripción");
    subscription.set_sender(source);
    subscription.set_dest(destination);
    seq.subscribe_port(&subscription)
        .expect("debe suscribirse al emisor MIDI virtual");
}

fn send_note(seq: &Seq, source_port: i32, destination: Addr, note: u8) {
    let mut event = Event::new(
        EventType::Noteon,
        &EvNote {
            channel: 0,
            note,
            velocity: 96,
            off_velocity: 0,
            duration: 0,
        },
    );
    event.set_source(source_port);
    event.set_dest(destination);
    event.set_direct();
    seq.event_output_direct(&mut event)
        .expect("debe enviar el evento por el secuenciador ALSA");
}

fn receive_note(input: &mut alsa::seq::Input<'_>, expected_note: u8) -> bool {
    let deadline = Instant::now() + Duration::from_millis(500);
    while Instant::now() < deadline {
        if input
            .event_input_pending(true)
            .expect("la consulta de eventos debe funcionar")
            > 0
        {
            let event = input
                .event_input()
                .expect("debe leerse el evento MIDI recibido");
            if event.get_type() == EventType::Noteon
                && event
                    .get_data::<EvNote>()
                    .is_some_and(|note| note.note == expected_note)
            {
                return true;
            }
        } else {
            thread::sleep(Duration::from_millis(1));
        }
    }
    false
}

#[test]
fn receiver_can_resume_after_source_port_disconnect_and_reconnect() {
    let Some(receiver) =
        open_seq_client(&format!("Estudio DAW test receiver {}", std::process::id()))
    else {
        return;
    };
    let receiver_port = create_port(
        &receiver,
        "test-input",
        PortCap::WRITE | PortCap::SUBS_WRITE,
    );
    let destination = Addr {
        client: receiver.client_id().expect("receiver client id"),
        port: receiver_port,
    };

    let Some(first_source) =
        open_seq_client(&format!("Estudio DAW source A {}", std::process::id()))
    else {
        return;
    };
    let first_port = create_port(
        &first_source,
        "test-output",
        PortCap::READ | PortCap::SUBS_READ,
    );
    let mut input = receiver.input();
    subscribe(
        &receiver,
        Addr {
            client: first_source.client_id().expect("source A client id"),
            port: first_port,
        },
        destination,
    );
    send_note(&first_source, first_port, destination, 60);
    assert!(
        receive_note(&mut input, 60),
        "la nota debe llegar antes del hot-plug"
    );

    // Quitar el puerto emisor simula la desconexión física; drop elimina también
    // el cliente ALSA y cualquier suscripción restante de esa fuente.
    first_source
        .delete_port(first_port)
        .expect("debe poder eliminar el puerto fuente");
    drop(first_source);

    let Some(second_source) =
        open_seq_client(&format!("Estudio DAW source B {}", std::process::id()))
    else {
        return;
    };
    let second_port = create_port(
        &second_source,
        "test-output",
        PortCap::READ | PortCap::SUBS_READ,
    );
    subscribe(
        &receiver,
        Addr {
            client: second_source.client_id().expect("source B client id"),
            port: second_port,
        },
        destination,
    );
    send_note(&second_source, second_port, destination, 72);
    assert!(
        receive_note(&mut input, 72),
        "tras crear y suscribir el nuevo puerto, MIDI debe volver a recibirse"
    );
}
