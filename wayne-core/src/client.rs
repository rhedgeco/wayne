use std::{
    io::{self, Read},
    os::unix::net::UnixStream,
    sync::atomic::{AtomicU64, Ordering},
};

use derive_more::Display;

use crate::Message;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(u64);

pub struct Client {
    client_id: ClientId,
    stream: UnixStream,
}

impl Client {
    pub fn new(stream: UnixStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        Ok(Self {
            client_id: ClientId({
                static GENERATOR: AtomicU64 = AtomicU64::new(0);
                GENERATOR.fetch_add(1, Ordering::Relaxed)
            }),
            stream,
        })
    }

    pub fn id(&self) -> ClientId {
        self.client_id
    }

    pub fn read_message(&mut self) -> io::Result<Option<Message>> {
        // try to read the next message header
        let mut header = [0; 8];
        match self.stream.read_exact(&mut header) {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(None),
            Err(e) => return Err(e),
            _ => (),
        }

        // parse the header parts
        let object_id = u32::from_ne_bytes([header[0], header[1], header[2], header[3]]);
        let opcode = u16::from_ne_bytes([header[4], header[5]]);
        let mut size = u16::from_ne_bytes([header[6], header[7]]).max(8) as usize;
        size = (size + 3) & !3; // round the size to the nearest 32 bit value

        // parse the message body
        let remaining = size - 8;
        let mut body = vec![0; remaining];
        self.stream.read_exact(&mut body)?;

        // build and return the message
        Ok(Some(Message {
            object_id,
            opcode,
            body: body.into_boxed_slice(),
        }))
    }
}
