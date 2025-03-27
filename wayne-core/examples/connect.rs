use std::process::Command;

use wayne_core::WaylandSocket;

fn main() -> anyhow::Result<()> {
    let socket = WaylandSocket::try_bind(0).until(32).build()?;

    Command::new("weston-terminal")
        .env(
            "WAYLAND_DISPLAY",
            socket
                .socket_name()
                .unwrap_or_else(|| socket.socket_path().as_os_str()),
        )
        .spawn()?;

    let mut clients = Vec::new();
    loop {
        while let Some(client) = socket.accept_client()? {
            println!("Connected Client");
            clients.push(client);
        }

        for client in clients.iter_mut() {
            while let Some(message) = client.read()? {
                println!("{message:?}");
            }
        }
    }
}
