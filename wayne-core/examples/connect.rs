use std::{mem::MaybeUninit, process::Command};

use wayne_core::{StreamExt, WaylandSocket, message::MessageParser};

fn main() -> anyhow::Result<()> {
    // bind the wayland socket to an available port
    let socket = WaylandSocket::bind(32)?;

    // spawn a terminal connected to the socket
    Command::new("weston-terminal")
        .env("WAYLAND_DISPLAY", socket.name())
        .spawn()?;

    // create a loop to process events
    let mut clients = Vec::new();
    let mut data_buffer = [MaybeUninit::uninit(); 64];
    let mut ctrl_buffer = [MaybeUninit::uninit(); 4096];
    loop {
        // accept all pending clients
        if let Some(stream) = socket.accept()? {
            println!("Client Connected");
            clients.push((stream, MessageParser::new()));
        }

        for (stream, parser) in &mut clients {
            // receive messages from all clients
            let recv = stream.read(&mut data_buffer, &mut ctrl_buffer)?;

            // print the messages
            for message in parser.parse(recv.data()) {
                println!("{message:?}");
            }

            // print the fds
            for fd in recv.fds() {
                println!("FD: {fd:?}");
            }
        }
    }
}
