use std::process::Command;

use wayne_core::{WaylandSocket, client::RecvBuffer};

fn main() -> anyhow::Result<()> {
    let socket = WaylandSocket::build(0).try_until(32).bind()?;

    Command::new("weston-terminal")
        .env(
            "WAYLAND_DISPLAY",
            socket
                .socket_name()
                .unwrap_or_else(|| socket.path().as_os_str()),
        )
        .spawn()?;

    let mut buffer = RecvBuffer::new();
    let mut clients = Vec::new();
    loop {
        if let Some(client) = socket.accept()? {
            println!("Client Connected");
            clients.push(client);
        };

        for client in &mut clients {
            while client.receive(&mut buffer)? {}
            while let Some(message) = buffer.pop_message() {
                println!("{message:?}");
            }
        }
    }
}
