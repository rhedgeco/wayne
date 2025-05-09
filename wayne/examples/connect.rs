use std::{collections::VecDeque, mem::MaybeUninit, process::Command};

use wayne::{
    core::{StreamExt, WaylandSocket, message::MessageParser},
    protocol::{Parser, buffer::IterExt, protocols::wayland::wl_display::WlDisplayRequest},
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
    let mut data_buffer = [MaybeUninit::uninit(); 64];
    let mut ctrl_buffer = [MaybeUninit::uninit(); 4096];
    loop {
        // accept all pending clients
        if let Some(stream) = socket.accept()? {
            log::debug!(
                "New client connected to Wayland Socket: '{}'",
                socket.name()
            );
            clients.push((
                stream,
                MessageParser::new(),
                VecDeque::new(),
                VecDeque::new(),
            ));
        }

        for (stream, message, messages, fds) in &mut clients {
            // get fresh bytes from the stream
            let recv = stream.read(&mut data_buffer, &mut ctrl_buffer)?;

            // store the fds
            fds.extend(recv.fds());

            // build and store all new messages
            messages.extend(message.parse(recv.data()));

            // parse all pending messages
            for message in messages.drain(..) {
                // build the message parser
                log::debug!("parsing message: {message:?}");
                let Some(mut parser) = WlDisplayRequest::parser(message.opcode) else {
                    log::error!("invalid opcode");
                    continue;
                };

                // try to parse the message
                match parser.parse(message.body.iter().map(|b| *b).buffer(), &mut *fds) {
                    Some(request) => log::info!("{request:?}"),
                    None => log::error!("Failed to parse message"),
                }
            }
        }
    }
}
