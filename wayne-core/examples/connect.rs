use std::{env, io, mem::MaybeUninit, path::PathBuf, process::Command};

use wayne_core::{StreamExt, WaylandSocket, message::MessageParser};

fn main() -> anyhow::Result<()> {
    // get the XDG_RUNTIME_DIR path
    let xdg_dir: PathBuf = env::var("XDG_RUNTIME_DIR")?.into();

    // try to bind a wayland socket
    let mut index = 0;
    let (socket, sock_name) = loop {
        let sock_name = format!("wayland-{index}");
        println!("Trying to bind to socket '{sock_name}'...");
        match WaylandSocket::bind(xdg_dir.join(&sock_name)) {
            Ok(listener) => {
                println!("Success!");
                break (listener, sock_name);
            }
            Err(e) => match e.kind() {
                io::ErrorKind::AddrInUse | io::ErrorKind::WouldBlock => {}
                _ => return Err(e.into()),
            },
        };

        index += 1;
        if index > 32 {
            eprintln!("Failed to bind socket. No sockets in range 0-32");
            return Ok(());
        }
    };

    // spawn a terminal connected to the socket
    Command::new("weston-terminal")
        .env("WAYLAND_DISPLAY", sock_name)
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
