use std::{
    collections::VecDeque,
    io::{self, IoSliceMut},
    mem::MaybeUninit,
    os::fd::OwnedFd,
    sync::atomic::{AtomicU64, Ordering},
};

use derive_more::Display;
use log::warn;
use rustix::{
    cmsg_space,
    net::{self, RecvAncillaryBuffer, RecvFlags, Shutdown},
};

use crate::{Message, message::MessageParser, socket::SocketId};

/// Arbitrary constant for the receive size of the data buffer
const DATA_SPACE: usize = 128;

/// The theoretical maximum number of file descriptors that could appear in the data stream
const MAX_FDS: usize = (DATA_SPACE / 8) + 1;
const FD_SPACE: usize = cmsg_space!(ScmRights(MAX_FDS));

#[repr(transparent)]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(u64);

#[derive(Debug)]
pub struct ClientStream {
    stream_fd: OwnedFd,
    socket_id: SocketId,
    client_id: ClientId,
    message_space: [u8; DATA_SPACE],
    message_builder: MessageParser,
    message_buffer: VecDeque<Message>,
    fd_space: [MaybeUninit<u8>; FD_SPACE],
    fd_buffer: VecDeque<OwnedFd>,
}

impl Drop for ClientStream {
    fn drop(&mut self) {
        let _ = net::shutdown(&self.stream_fd, Shutdown::Both);
    }
}

impl ClientStream {
    pub(crate) fn new(stream_fd: OwnedFd, socket_id: SocketId) -> Self {
        Self {
            stream_fd,
            socket_id,
            client_id: ClientId({
                static GENERATOR: AtomicU64 = AtomicU64::new(0);
                GENERATOR.fetch_add(1, Ordering::Relaxed)
            }),
            message_space: [0; DATA_SPACE],
            message_builder: MessageParser::new(),
            message_buffer: VecDeque::new(),
            fd_space: [MaybeUninit::uninit(); FD_SPACE],
            fd_buffer: VecDeque::new(),
        }
    }

    /// Returns the unique id for this client.
    pub fn id(&self) -> ClientId {
        self.client_id
    }

    /// Returns the unique id of the socket associated with this stream.
    pub fn socket_id(&self) -> SocketId {
        self.socket_id
    }

    /// Pops the next file descriptor from the receive queue.
    pub fn pop_fd(&mut self) -> Option<OwnedFd> {
        self.fd_buffer.pop_front()
    }

    /// Pops the next message from the receive queue.
    pub fn pop_message(&mut self) -> Option<Message> {
        self.message_buffer.pop_front()
    }

    /// Reads bytes from the client stream an updates internal buffers.
    ///
    /// Returns `true` if data was received, otherwise returns false.
    pub fn receive_data(&mut self) -> io::Result<bool> {
        let mut fd_buffer = RecvAncillaryBuffer::new(&mut self.fd_space);
        let received = match net::recvmsg(
            &self.stream_fd,
            &mut [IoSliceMut::new(&mut self.message_space)],
            &mut fd_buffer,
            RecvFlags::CMSG_CLOEXEC | RecvFlags::DONTWAIT,
        ) {
            Ok(msg) => msg.bytes,
            Err(e) => {
                return match e.kind() {
                    io::ErrorKind::WouldBlock => Ok(false),
                    _ => Err(e.into()),
                };
            }
        };

        for message in fd_buffer.drain() {
            match message {
                net::RecvAncillaryMessage::ScmRights(fds) => self.fd_buffer.extend(fds),
                net::RecvAncillaryMessage::ScmCredentials(_) => {
                    warn!("Received ScmCredentials from ancillary buffer");
                }
                _ => unreachable!(),
            }
        }

        let bytes = &self.message_space[0..received];
        let parser = self.message_builder.parse(bytes);
        self.message_buffer.extend(parser);
        Ok(true)
    }
}
