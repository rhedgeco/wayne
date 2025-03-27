use std::process::Command;

use wayne_core::Server;

fn main() -> anyhow::Result<()> {
    let server = Server::try_bind(0).until(32).bind()?;

    Command::new("weston-terminal")
        .env(
            "WAYLAND_DISPLAY",
            server
                .socket_name()
                .unwrap_or_else(|| server.socket_path().as_os_str()),
        )
        .spawn()?;

    let mut clients = Vec::new();
    loop {
        while let Some(client) = server.accept_client()? {
            println!("Connected Client");
            clients.push(client);
        }

        for client in clients.iter_mut() {
            while let Some(message) = client.read_message()? {
                println!("{message:?}");
            }
        }
    }
}
