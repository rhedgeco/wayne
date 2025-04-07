use std::process::Command;

use wayne_core::WaylandSocket;

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

    let mut clients = Vec::new();
    loop {
        if let Some(client) = socket.accept()? {
            println!("Client Connected");
            clients.push(client);
        };

        for client in &mut clients {
            client.receive_data()?;
            while let Some(message) = client.pop_message() {
                println!("{message:?}");
            }
        }
    }
}
