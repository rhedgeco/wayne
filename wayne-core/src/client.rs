use std::{
    collections::VecDeque,
    io::{self, IoSliceMut},
    mem::MaybeUninit,
    os::fd::OwnedFd,
    sync::atomic::{AtomicU64, Ordering},
};

use derive_more::Display;
use log::error;
use rustix::{
    cmsg_space,
    net::{self, RecvAncillaryBuffer, RecvFlags, ReturnFlags, Shutdown},
};
use thiserror::Error;

use crate::{Message, message::MessageDecoder, socket::SocketId};

#[repr(transparent)]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(u64);

#[derive(Debug)]
pub struct ClientStream {
    stream_fd: OwnedFd,
    socket_id: SocketId,
    client_id: ClientId,
    message_builder: MessageDecoder,
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
            message_builder: MessageDecoder::new(),
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
    pub fn receive(&mut self, buffer: &mut RecvBuffer) -> Result<bool, RecvError> {
        // receive data from the socket stream
        let data_buffer = &mut [IoSliceMut::new(&mut buffer.data_space)];
        let fd_buffer = &mut RecvAncillaryBuffer::new(&mut buffer.control_space);
        let flags = RecvFlags::CMSG_CLOEXEC | RecvFlags::DONTWAIT;
        let recv_msg = match net::recvmsg(&self.stream_fd, data_buffer, fd_buffer, flags) {
            Ok(recv_msg) => recv_msg,
            Err(e) => {
                return match e.kind() {
                    io::ErrorKind::WouldBlock => Ok(false),
                    _ => Err(RecvError::IoError(e.into())),
                };
            }
        };

        // drain all file descriptors
        for message in fd_buffer.drain() {
            match message {
                net::RecvAncillaryMessage::ScmRights(fds) => buffer.fds.extend(fds),
                net::RecvAncillaryMessage::ScmCredentials(_) => {
                    error!("Received ScmCredentials from ancillary buffer");
                    return Err(RecvError::InvalidControl);
                }
                _ => unreachable!(),
            }
        }

        // parse all available messages
        let bytes = &buffer.data_space[0..recv_msg.bytes];
        let messages = self.message_builder.decode(bytes);
        buffer.messages.extend(messages);

        // return an error if any control data was truncated
        if recv_msg.flags.contains(ReturnFlags::CTRUNC) {
            return Err(RecvError::TruncatedControl);
        }

        // otherwise return true
        Ok(true)
    }
}

#[derive(Debug, Error)]
pub enum RecvError {
    #[error("An invalid control message came over the wire")]
    InvalidControl,
    #[error("A control message was truncated. Potential file descriptors were lost")]
    TruncatedControl,
    #[error("Failed to receive data: {_0}")]
    IoError(#[from] io::Error),
}

pub struct RecvBuffer {
    data_space: Box<[u8]>,
    control_space: Box<[MaybeUninit<u8>]>,
    messages: VecDeque<Message>,
    fds: VecDeque<OwnedFd>,
}

impl RecvBuffer {
    pub fn with_space(data_space: usize, fd_space: usize) -> Self {
        Self {
            data_space: vec![0; data_space].into_boxed_slice(),
            control_space: Box::new_uninit_slice(cmsg_space!(ScmRights(fd_space))),
            messages: VecDeque::new(),
            fds: VecDeque::new(),
        }
    }

    pub fn pop_fd(&mut self) -> Option<OwnedFd> {
        self.fds.pop_front()
    }

    pub fn pop_message(&mut self) -> Option<Message> {
        self.messages.pop_front()
    }
}
