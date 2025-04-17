use std::{env, io, os::unix::prelude::OwnedFd, path::PathBuf, process::Command};

use wayne_core::{Message, WaylandListener, stream::MessageRecv};

struct Receiver;
impl MessageRecv for Receiver {
    fn recv_fd(&mut self, _fd: OwnedFd) {
        println!("Received File Descriptor");
    }

    fn revc_message(&mut self, message: Message) {
        println!("{message:?}");
    }
}

fn main() -> anyhow::Result<()> {
    // get the XDG_RUNTIME_DIR path
    let xdg_dir: PathBuf = env::var("XDG_RUNTIME_DIR")?.into();

    // build a socket on the first available socket name
    let mut listener = None;
    let mut sock_name = String::new();
    for index in 0..=32 {
        sock_name = format!("wayland-{index}");
        println!("Trying to bind to socket '{sock_name}'...");
        match WaylandListener::bind(xdg_dir.join(&sock_name)) {
            Ok(new_sock) => {
                println!("Success!");
                listener = Some(new_sock);
                break;
            }
            Err(e) => match e.kind() {
                io::ErrorKind::AddrInUse | io::ErrorKind::WouldBlock => continue,
                _ => return Err(e.into()),
            },
        };
    }

    // try to get the bound socket
    let Some(listener) = listener else {
        eprintln!("Failed to bind socket. No sockets in range 0-32");
        return Ok(());
    };

    // spawn a terminal connected to the socket
    Command::new("weston-terminal")
        .env("WAYLAND_DISPLAY", sock_name)
        .spawn()?;

    // create a loop to process events
    let mut clients = Vec::new();
    let mut data_buffer = vec![0u8; 64];
    let mut control_buffer = vec![0u8; 4096];
    loop {
        // accept all pending clients
        if let Some(stream) = listener.accept()? {
            println!("Client Connected");
            clients.push(stream);
        };

        // parse messages from all clients
        for stream in &mut clients {
            stream
                .transfer(data_buffer.as_mut_slice(), control_buffer.as_mut_slice())
                .recv(Receiver)?;
        }
    }
}
