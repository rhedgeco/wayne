use std::process::Command;

use wayne::{
    message::MessageBuffer, protocol::protocols::wayland::wl_display::WlDisplayRequest,
    server::WaylandSocket,
};

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

            clients.push((stream, MessageBuffer::new([0; 64], [0; 64])));
        }

        for (stream, buffer) in &mut clients {
            // read from the socket to get fresh messages
            if !buffer.read_from_stream(stream)? {
                continue;
            }

            // read all pending messages
            while let Some(message) = buffer.parse_message() {
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
