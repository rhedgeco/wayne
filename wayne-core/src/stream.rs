use std::{
    io,
    marker::PhantomData,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::net::UnixStream,
    },
};

use crate::Buffer;

mod private {
    pub trait Sealed {}
}

pub trait StreamExt: private::Sealed {
    fn read<'data, 'ctrl>(
        &mut self,
        data_buffer: &'data mut impl Buffer,
        ctrl_buffer: &'ctrl mut impl Buffer,
    ) -> io::Result<ReadData<'data, 'ctrl>>;
}

impl private::Sealed for UnixStream {}
impl StreamExt for UnixStream {
    fn read<'data, 'ctrl>(
        &mut self,
        data_buffer: &'data mut impl Buffer,
        ctrl_buffer: &'ctrl mut impl Buffer,
    ) -> io::Result<ReadData<'data, 'ctrl>> {
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

        // ensure the control buffer is zeroed
        for offset in 0..ctrl_buffer.len() {
            unsafe { ctrl_buffer.as_mut_ptr().byte_add(offset).write(0) };
        }

        // call recvmsg to get data from the client
        let data_len = match unsafe {
            libc::recvmsg(
                self.as_raw_fd(),
                (&mut msghdr) as *mut _,
                libc::MSG_CMSG_CLOEXEC | libc::MSG_DONTWAIT,
            )
        } {
            ..0 => match io::Error::last_os_error() {
                e if e.kind() == io::ErrorKind::WouldBlock => 0,
                e => return Err(e),
            },
            data_len => data_len as usize,
        };

        // ensure no control data was truncated
        if msghdr.msg_flags & libc::MSG_CTRUNC > 0 {
            return Err(io::Error::other(
                "control buffer too small. lost potential file descriptors",
            ));
        }

        // get the data from the buffer
        // SAFETY: the data buffer is garunteed to be initialized for exactly data_len
        let data = unsafe { core::slice::from_raw_parts(data_buffer.as_ptr(), data_len) };

        // build and return the received data
        Ok(ReadData {
            _ctrl: PhantomData,
            msghdr,
            data,
        })
    }
}

pub struct ReadData<'data, 'ctrl> {
    _ctrl: PhantomData<&'ctrl [u8]>,
    msghdr: libc::msghdr,
    data: &'data [u8],
}

impl<'data, 'ctrl> ReadData<'data, 'ctrl> {
    pub fn data(&self) -> &'data [u8] {
        self.data
    }

    pub fn fds(&self) -> Fds {
        Fds {
            msghdr: &self.msghdr,
            last: None,
        }
    }
}

pub struct Fds<'a> {
    msghdr: &'a libc::msghdr,
    last: Option<*const libc::cmsghdr>,
}

impl Iterator for Fds<'_> {
    type Item = OwnedFd;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let cmsg_ptr = match self.last {
                Some(last) => unsafe {
                    // SAFETY: msghdr and last cmsg pointers are directly derived from references
                    libc::CMSG_NXTHDR(self.msghdr as *const _, last)
                },
                None => unsafe {
                    // SAFETY: msghdr pointer is directly derived from a reference
                    libc::CMSG_FIRSTHDR(self.msghdr as *const _)
                },
            };

            // ensure the ptr is non null
            if cmsg_ptr.is_null() {
                return None;
            }

            // store the cmsg for the next iteration
            self.last = Some(cmsg_ptr);

            // SAFETY: cmsg_ptr is garunteed to be not null
            let cmsg = unsafe { core::ptr::read_unaligned(cmsg_ptr) };

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

            // then return the valid fd
            // SAFETY: since raw_fd is valid, it can be built into an owned fd here
            return Some(unsafe { OwnedFd::from_raw_fd(raw_fd) });
        }
    }
}
