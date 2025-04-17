use std::{
    io,
    os::fd::{AsRawFd, OwnedFd},
};

use crate::{Buffer, Message, message::MessageParser};

pub trait MessageRecv {
    fn recv_fd(&mut self, fd: OwnedFd);
    fn revc_message(&mut self, message: Message);
}

impl<T: MessageRecv> MessageRecv for &mut T {
    fn recv_fd(&mut self, fd: OwnedFd) {
        T::recv_fd(self, fd);
    }

    fn revc_message(&mut self, message: Message) {
        T::revc_message(self, message);
    }
}

pub struct WaylandStream {
    stream_fd: OwnedFd,
    parser: MessageParser,
}

impl WaylandStream {
    pub(crate) fn new(stream_fd: OwnedFd) -> Self {
        Self {
            stream_fd,
            parser: MessageParser::new(),
        }
    }

    pub fn transfer<Data: Buffer, Control: Buffer>(
        &mut self,
        data_buffer: Data,
        control_buffer: Control,
    ) -> TransferBuilder<Data, Control> {
        TransferBuilder {
            stream: self,
            data_buffer,
            control_buffer,
        }
    }
}

pub struct TransferBuilder<'a, Data, Control> {
    stream: &'a mut WaylandStream,
    data_buffer: Data,
    control_buffer: Control,
}

impl<'a, Data: Buffer, Control: Buffer> TransferBuilder<'a, Data, Control> {
    pub fn recv(mut self, mut receiver: impl MessageRecv) -> io::Result<()> {
        // build scatter/gather array with single buffer
        let msg_iov = &mut [libc::iovec {
            iov_base: self.data_buffer.buffer_ptr() as *mut _,
            iov_len: self.data_buffer.buffer_len(),
        }];

        // build msghdr for the recv call
        let mut msghdr = libc::msghdr {
            msg_name: core::ptr::null_mut(),
            msg_namelen: 0,
            msg_iov: msg_iov.as_mut_ptr(),
            msg_iovlen: 1,
            msg_control: self.control_buffer.buffer_ptr() as *mut _,
            msg_controllen: self.control_buffer.buffer_len(),
            msg_flags: 0,
        };

        // call recv_msg to get data from the client
        let data_bytes = match unsafe {
            libc::recvmsg(
                self.stream.stream_fd.as_raw_fd(),
                (&mut msghdr) as *mut _,
                libc::MSG_CMSG_CLOEXEC | libc::MSG_DONTWAIT,
            )
        } {
            -1 => match io::Error::last_os_error() {
                // if we got a would block error, just return immediately
                e if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                e => return Err(e),
            },
            len => unsafe { self.data_buffer.assume_init(len as usize) },
        };

        // ensure no control data was truncated
        if msghdr.msg_flags & libc::MSG_CTRUNC > 0 {
            return Err(io::Error::other("control buffer was too small"));
        }

        // iterate over all cmsg data
        for cmsghdr in CmsgIter::new(&msghdr) {
            println!("got cmsg type: {}", cmsghdr.cmsg_type);
        }

        // parse all messages in the data_bytes
        for message in self.stream.parser.parse(data_bytes) {
            receiver.revc_message(message);
        }

        Ok(())
    }
}

struct CmsgIter<'a> {
    header: &'a libc::msghdr,
    current: Option<&'a libc::cmsghdr>,
}

impl<'a> CmsgIter<'a> {
    pub fn new(header: &'a libc::msghdr) -> Self {
        Self {
            header,
            current: {
                let ptr = unsafe { libc::CMSG_FIRSTHDR(header as *const _) };
                match ptr.is_null() {
                    false => Some(unsafe { &*ptr }),
                    true => None,
                }
            },
        }
    }
}

impl<'a> Iterator for CmsgIter<'a> {
    type Item = &'a libc::cmsghdr;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current.take()?;
        let next = unsafe { libc::CMSG_NXTHDR(self.header as *const _, current as *const _) };
        if !next.is_null() {
            self.current = Some(unsafe { &*next });
        }
        Some(current)
    }
}
