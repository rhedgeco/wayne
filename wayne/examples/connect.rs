use std::process::Command;

use wayne::{protocol::protocols::wayland::wl_display::WlDisplayRequest, server::WaylandSocket};
use wayne_stream::StreamBuilder;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    // bind the wayland socket to an available port
    let socket = WaylandSocket::bind(32)?;

    // spawn a terminal connected to the socket
    Command::new("weston-terminal")
        .env("WAYLAND_DISPLAY", socket.name())
        .spawn()?;

    // create a loop to process events
    let mut clients = Vec::new();
    loop {
        // accept all pending clients
        if let Some(stream) = socket.accept()? {
            log::debug!(
                "New client connected to Wayland Socket: '{}'",
                socket.name()
            );

            clients.push(
                StreamBuilder::from_unix(stream)
                    .with_data_buffer([0; 64])
                    .with_ctrl_buffer([0; 64])
                    .build(),
            );
        }

        for stream in &mut clients {
            // read from the socket to get fresh messages
            if !stream.read_socket()? {
                continue;
            }

            // read all pending messages
            while let Some(message) = stream.parse_message() {
                // build the message parser
                log::debug!("parsing message: {message:?}");
                let Some(_) = WlDisplayRequest::parser(message.opcode) else {
                    log::error!("invalid opcode");
                    continue;
                };
            }

            // // build and store all new messages
            // messages.extend(message.parse(recv.data()));

            // // parse all pending messages
            // for message in messages.drain(..) {
            //     // build the message parser
            //     log::debug!("parsing message: {message:?}");
            //     let Some(mut parser) = WlDisplayRequest::parser(message.opcode) else {
            //         log::error!("invalid opcode");
            //         continue;
            //     };

            //     // try to parse the message
            //     match parser.parse(message.body.iter().map(|b| *b).buffer(), &mut *fds) {
            //         Some(request) => log::info!("{request:?}"),
            //         None => log::error!("Failed to parse message"),
            //     }
            // }
        }
    }
}
