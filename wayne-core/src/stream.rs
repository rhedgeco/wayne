use std::{
    io,
    marker::PhantomData,
    os::fd::{AsFd, FromRawFd, OwnedFd},
};

use crate::{Buffer, sys};

pub struct WaylandStream {
    stream_fd: OwnedFd,
}

impl WaylandStream {
    pub(crate) fn new(stream_fd: OwnedFd) -> Self {
        Self { stream_fd }
    }

    pub fn receive<'data, 'ctrl>(
        &self,
        data_buffer: &'data mut impl Buffer,
        ctrl_buffer: &'ctrl mut impl Buffer,
    ) -> io::Result<Received<'data, 'ctrl>> {
        // build scatter/gather array with single buffer
        let msg_iov = &mut [libc::iovec {
            iov_base: data_buffer.as_mut_ptr() as *mut _,
            iov_len: data_buffer.len(),
        }];

        // build msghdr for the recv call
        let mut msghdr = libc::msghdr {
            msg_name: core::ptr::null_mut(),
            msg_namelen: 0,
            msg_iov: msg_iov.as_mut_ptr(),
            msg_iovlen: 1,
            msg_control: ctrl_buffer.as_mut_ptr() as *mut _,
            msg_controllen: ctrl_buffer.len(),
            msg_flags: 0,
        };

        // call recvmsg to get data from the client
        let data_len = sys::recvmsg(self.stream_fd.as_fd(), &mut msghdr)?;

        // ensure no control data was truncated
        if msghdr.msg_flags & libc::MSG_CTRUNC > 0 {
            return Err(io::Error::other(
                "control buffer too small. lost potential file descriptors",
            ));
        }

        // get the data from the buffer
        // SAFETY: the data buffer is garunteed to be initialized for exactly data_len
        let bytes = unsafe { core::slice::from_raw_parts(data_buffer.as_ptr(), data_len) };

        // build and return the received data
        Ok(Received {
            _ctrl: PhantomData,
            msghdr,
            bytes,
        })
    }
}

pub struct Received<'data, 'ctrl> {
    _ctrl: PhantomData<&'ctrl [u8]>,
    msghdr: libc::msghdr,
    bytes: &'data [u8],
}

impl<'data, 'ctrl> Received<'data, 'ctrl> {
    pub fn bytes(&self) -> &'data [u8] {
        self.bytes
    }

    pub fn fd_iter<'a>(&'a self) -> FdIter<'a, 'ctrl> {
        FdIter {
            msghdr: &self.msghdr,
            state: Some(CmsgState::Start),
        }
    }
}

enum CmsgState<'ctrl> {
    Start,
    Next(&'ctrl libc::cmsghdr),
}

pub struct FdIter<'a, 'ctrl> {
    msghdr: &'a libc::msghdr,
    state: Option<CmsgState<'ctrl>>,
}

impl Iterator for FdIter<'_, '_> {
    type Item = OwnedFd;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // get the next cmsg
            let cmsg_ptr = match self.state.take()? {
                CmsgState::Start => {
                    // SAFETY: msghdr pointer is directly derived from a reference
                    unsafe { libc::CMSG_FIRSTHDR(self.msghdr as *const _) }
                }
                CmsgState::Next(last) => {
                    // SAFETY: msghdr and last cmsg pointers are directly derived from references
                    unsafe { libc::CMSG_NXTHDR(self.msghdr as *const _, last as *const _) }
                }
            };

            // ensure the ptr is valid
            if cmsg_ptr.is_null() {
                return None;
            }

            // SAFETY: when the cmsg_ptr is not null, it is garunteed to be valid here
            let cmsg = unsafe { &*cmsg_ptr };

            // store the cmsg for the next iteration
            self.state = Some(CmsgState::Next(cmsg));

            // ensure the cmsg is a file descriptor
            // if it is not, just continue and try again
            if cmsg.cmsg_type != libc::SCM_RIGHTS {
                continue;
            }

            // load the fd pointer from the cmsg data
            // SAFETY: cmsg_ptr is garunteed to be valid at this point
            let fd_ptr = unsafe { libc::CMSG_DATA(cmsg_ptr) as *mut i32 };
            // SAFETY: fd_ptr is valid for reads and is properly initialized by cmsg
            let raw_fd = unsafe { core::ptr::read_unaligned(fd_ptr) };
            // SAFETY: since raw_fd is valid, it can be built into an owned fd here
            let fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };

            // then return the valid fd
            return Some(fd);
        }
    }
}
