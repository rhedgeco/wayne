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

use crate::{Message, message::MessageBuilder, socket::SocketId};

const DATA_SPACE: usize = 128;
const FD_SPACE: usize = cmsg_space!(ScmRights(8));

#[repr(transparent)]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(u64);

#[derive(Debug)]
pub struct WaylandClient {
    stream_fd: OwnedFd,
    socket_id: SocketId,
    client_id: ClientId,
    message_space: [u8; DATA_SPACE],
    message_builder: MessageBuilder,
    message_buffer: VecDeque<Message>,
    fd_space: [MaybeUninit<u8>; FD_SPACE],
    fd_buffer: VecDeque<OwnedFd>,
}

impl Drop for WaylandClient {
    fn drop(&mut self) {
        let _ = net::shutdown(&self.stream_fd, Shutdown::Both);
    }
}

impl WaylandClient {
    pub(crate) fn new(stream_fd: OwnedFd, socket_id: SocketId) -> Self {
        Self {
            stream_fd,
            socket_id,
            client_id: ClientId({
                static GENERATOR: AtomicU64 = AtomicU64::new(0);
                GENERATOR.fetch_add(1, Ordering::Relaxed)
            }),
            message_space: [0; DATA_SPACE],
            message_builder: MessageBuilder::new(),
            message_buffer: VecDeque::new(),
            fd_space: [MaybeUninit::uninit(); FD_SPACE],
            fd_buffer: VecDeque::new(),
        }
    }

    pub fn id(&self) -> ClientId {
        self.client_id
    }

    pub fn socket_id(&self) -> SocketId {
        self.socket_id
    }

    pub fn take_fd(&mut self) -> Option<OwnedFd> {
        self.fd_buffer.pop_front()
    }

    pub fn take_message(&mut self) -> Option<Message> {
        self.message_buffer.pop_front()
    }

    pub fn fill_buffers(&mut self) -> io::Result<()> {
        let mut fd_buffer = RecvAncillaryBuffer::new(&mut self.fd_space);
        let result = net::recvmsg(
            &self.stream_fd,
            &mut [IoSliceMut::new(&mut self.message_space)],
            &mut fd_buffer,
            RecvFlags::CMSG_CLOEXEC | RecvFlags::DONTWAIT,
        );

        for message in fd_buffer.drain() {
            match message {
                net::RecvAncillaryMessage::ScmRights(fds) => self.fd_buffer.extend(fds),
                net::RecvAncillaryMessage::ScmCredentials(_) => {
                    warn!("Received ScmCredentials from ancillary buffer");
                }
                _ => unreachable!(),
            }
        }

        match result {
            Err(e) => match e.kind() {
                io::ErrorKind::WouldBlock => Ok(()),
                _ => Err(e.into()),
            },
            Ok(msg) => {
                let bytes = &self.message_space[0..msg.bytes];
                let parser = self.message_builder.parse(bytes);
                self.message_buffer.extend(parser);
                Ok(())
            }
        }
    }
}
