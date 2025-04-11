use std::{
    collections::VecDeque,
    io::{self, IoSliceMut},
    mem::MaybeUninit,
    os::fd::{AsFd, BorrowedFd, OwnedFd},
    sync::atomic::{AtomicU64, Ordering},
};

use derive_more::Display;
use log::warn;
use rustix::net::{self, RecvAncillaryBuffer, RecvFlags, ReturnFlags, Shutdown};

use crate::{Message, message::MessageParser, socket::SocketId};

/// A unique id that represents a single wayland [`ClientStream`]
#[repr(transparent)]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(u64);

/// A client stream connection to a wayland socket.
///
/// This provides functions to send and receive [`Message`] data to and from the client.
#[derive(Debug)]
pub struct ClientStream {
    stream_fd: OwnedFd,
    socket_id: SocketId,
    client_id: ClientId,
    parser: MessageParser,
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
            parser: MessageParser::new(),
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

    /// Reads bytes from the client stream into the `buffer`.
    ///
    /// Returns `true` if any data was received, otherwise returns false.
    pub fn receive(&mut self, buffer: &mut RecvBuffer) -> io::Result<bool> {
        // receive data from the socket stream
        let data_buffer = &mut [IoSliceMut::new(&mut buffer.data_space)];
        let fd_buffer = &mut RecvAncillaryBuffer::new(&mut buffer.control_space);
        let flags = RecvFlags::CMSG_CLOEXEC | RecvFlags::DONTWAIT;
        let recv_msg = match net::recvmsg(&self.stream_fd, data_buffer, fd_buffer, flags) {
            Ok(recv_msg) => recv_msg,
            Err(e) => {
                return match e.kind() {
                    io::ErrorKind::WouldBlock => Ok(false),
                    _ => Err(e.into()),
                };
            }
        };

        // immediately return an error if any control data was truncated
        if recv_msg.flags.contains(ReturnFlags::CTRUNC) {
            return Err(io::Error::other("truncated file descriptors"));
        }

        // drain all file descriptors
        for message in fd_buffer.drain() {
            match message {
                net::RecvAncillaryMessage::ScmRights(fds) => buffer.fds.extend(fds),
                net::RecvAncillaryMessage::ScmCredentials(_) => {
                    warn!("Received ScmCredentials from ancillary buffer");
                }
                _ => unreachable!(),
            }
        }

        // parse all available messages
        let bytes = &buffer.data_space[0..recv_msg.bytes];
        buffer.messages.extend(self.parser.parse(bytes));

        // otherwise return true
        Ok(true)
    }
}

/// A buffer that can be used to receive message data from a [`ClientStream`]
#[derive(Debug)]
pub struct RecvBuffer {
    data_space: Box<[u8]>,
    control_space: Box<[MaybeUninit<u8>]>,
    messages: VecDeque<Message>,
    fds: VecDeque<OwnedFd>,
}

impl Default for RecvBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl RecvBuffer {
    /// Default byte space used for incoming message data.
    const DEFAULT_DATA_SPACE: usize = 64;

    /// Default byte space used for incoming file descriptors.
    ///
    /// The space must be sufficiently large.
    /// Any missed file descriptors are a hard error for the protocol.
    const DEFAULT_FD_SPACE: usize = 4096;

    /// Returns a new buffer with default space allocated.
    pub fn new() -> Self {
        Self::with_space(Self::DEFAULT_DATA_SPACE, Self::DEFAULT_FD_SPACE)
    }

    /// Returns a new buffer with a custom amount of space allocated.
    ///
    /// ## Warning
    /// Be sure to set `fd_space` to a reasonable size for incoming data.
    /// Any missed file descriptors are a hard error for the protocol.
    pub fn with_space(data_space: usize, fd_space: usize) -> Self {
        Self {
            data_space: vec![0; data_space].into_boxed_slice(),
            control_space: Box::new_uninit_slice(fd_space),
            messages: VecDeque::new(),
            fds: VecDeque::new(),
        }
    }

    /// Pop and return the next file descriptor in the queue
    pub fn pop_fd(&mut self) -> Option<OwnedFd> {
        self.fds.pop_front()
    }

    /// Pop and return the next message in the queue
    pub fn pop_message(&mut self) -> Option<Message> {
        self.messages.pop_front()
    }

    /// Peek the next file descriptor in the queue
    pub fn peek_fd(&self) -> Option<BorrowedFd> {
        self.fds.front().map(AsFd::as_fd)
    }

    /// Peek the next message in the queue
    pub fn peek_message(&self) -> Option<&Message> {
        self.messages.front()
    }
}
