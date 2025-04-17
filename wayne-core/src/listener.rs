use std::{
    ffi::CString,
    io::{self},
    mem::MaybeUninit,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::ffi::OsStrExt,
    },
    path::Path,
};

use crate::WaylandStream;

pub struct WaylandListener {
    sock_fd: OwnedFd,
    #[allow(dead_code)]
    lock_fd: OwnedFd,
}

impl WaylandListener {
    pub fn accept(&self) -> io::Result<Option<WaylandStream>> {
        // accept a new stream
        let stream_fd = match unsafe {
            libc::accept(
                self.sock_fd.as_raw_fd(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            )
        } {
            -1 => {
                return match io::Error::last_os_error() {
                    // if the error is `WouldBlock` just return `None`
                    e if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
                    e => Err(e),
                };
            }
            raw => unsafe { OwnedFd::from_raw_fd(raw) },
        };

        // build and return the client stream
        Ok(Some(WaylandStream::new(stream_fd)))
    }

    pub fn bind(path: impl AsRef<Path>) -> io::Result<Self> {
        // create sock and lock paths
        let sock_path = path.as_ref();
        let lock_path = sock_path.with_extension("lock");

        // build cstring for sock path
        let Ok(sock_path) = CString::new(sock_path.as_os_str().as_bytes()) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "sock path has invalid bytes",
            ));
        };

        // build cstring for lock path
        let Ok(lock_path) = CString::new(lock_path.as_os_str().as_bytes()) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "lock path has invalid bytes",
            ));
        };

        // ensure the file doesnt already exist
        let mut sock_stat = MaybeUninit::uninit();
        if unsafe { libc::stat(sock_path.as_ptr(), sock_stat.as_mut_ptr()) } == 0 {
            // if the file was found, then unlink it
            if unsafe { libc::unlink(sock_path.as_ptr()) } < 0 {
                return Err(io::Error::last_os_error());
            }
        } else {
            match io::Error::last_os_error() {
                // if the file is not found, then do nothing
                e if e.kind() == io::ErrorKind::NotFound => {}
                // otherwise return any error
                e => return Err(e),
            }
        }

        // aquire the lockfile
        // https://gitlab.freedesktop.org/libbsd/libbsd/-/blob/73b25a8f871b3a20f6ff76679358540f95d7dbfd/src/flopen.c#L71
        let lock_fd = loop {
            // open the lockfile
            let lock_fd = match unsafe {
                libc::open(
                    lock_path.as_ptr(),
                    // O_CREAT - create the file if it doesnt exist
                    // O_RDWR - aquire with read/write permissions
                    libc::O_CREAT | libc::O_RDWR,
                    // S_IRUSR - user read permission
                    // S_IWUSR - user write permission
                    // S_IRGRP - group read permission
                    libc::S_IRUSR | libc::S_IWUSR | libc::S_IRGRP,
                )
            } {
                -1 => return Err(io::Error::last_os_error()),
                raw => unsafe { OwnedFd::from_raw_fd(raw) },
            };

            // lock the file in a non-blocking manner
            // - LOCK_EX - aquire exclusive lock
            // - LOCK_NB - use non-blocking operation
            let operation = libc::LOCK_EX | libc::LOCK_NB;
            if unsafe { libc::flock(lock_fd.as_raw_fd(), operation) } < 0 {
                return Err(io::Error::last_os_error());
            }

            // get the metadata for the lockfile on disk
            let mut lock_path_stat = MaybeUninit::<libc::stat>::uninit();
            if unsafe { libc::stat(lock_path.as_ptr(), lock_path_stat.as_mut_ptr()) } < 0 {
                // "disappeared from under our feet"
                // https://gitlab.freedesktop.org/libbsd/libbsd/-/blob/73b25a8f871b3a20f6ff76679358540f95d7dbfd/src/flopen.c#L101
                // when we cant get the meta data from the disk, the file must have been yanked/changed.
                // we need to continue here to try to open/create or lock_fd again.
                continue;
            }
            let fs_stat = unsafe { lock_path_stat.assume_init() };

            // get the metadata for the file descriptor we currently have
            let mut lock_fd_stat = MaybeUninit::<libc::stat>::uninit();
            if unsafe { libc::fstat(lock_fd.as_raw_fd(), lock_fd_stat.as_mut_ptr()) } < 0 {
                return Err(io::Error::last_os_error());
            }
            let fd_stat = unsafe { lock_fd_stat.assume_init() };

            // ensure both significant metadata sections match
            if fs_stat.st_dev != fd_stat.st_dev || fs_stat.st_ino != fd_stat.st_ino {
                // if they dont, then the file on disk was replaced before the lock happened
                continue;
            }

            // if all the above succeeded, then we have successfully locked the file
            break lock_fd;
        };

        // build the socket address
        let mut socket_addr = libc::sockaddr_un {
            sun_family: libc::AF_UNIX as _,
            sun_path: [0; 108],
        };

        // insert the bytes into the socketaddr
        let mut path_bytes = sock_path.as_bytes_with_nul();
        if path_bytes.len() > socket_addr.sun_path.len() {
            path_bytes = sock_path.to_bytes(); // without NUL
            if path_bytes.len() > socket_addr.sun_path.len() {
                // if path is too long return ENAMETOOLONG
                return Err(io::Error::from_raw_os_error(36));
            }
        }

        // copy the bytes from the path_name into the socket_addr
        let path_bytes: &[i8] = unsafe { core::mem::transmute(path_bytes) };
        socket_addr.sun_path[0..path_bytes.len()].copy_from_slice(path_bytes);

        // build a new socket
        let sock_fd = match unsafe {
            libc::socket(
                libc::AF_UNIX,
                libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                0,
            )
        } {
            -1 => return Err(io::Error::last_os_error()),
            raw => unsafe { OwnedFd::from_raw_fd(raw) },
        };

        // get the size of the socket_addr usize ptr math
        let socket_addr_ptr = (&socket_addr) as *const _;
        let sun_path_ptr = (&socket_addr.sun_path) as *const _;
        let path_offset = sun_path_ptr as usize - socket_addr_ptr as usize;
        let sock_addr_len = (path_offset + path_bytes.len()) as libc::socklen_t;

        // bind the socket to the address
        if unsafe {
            libc::bind(
                sock_fd.as_raw_fd(),
                socket_addr_ptr as *const _,
                sock_addr_len,
            )
        } < 0
        {
            return Err(io::Error::last_os_error());
        }

        // start listening on the socket
        if unsafe { libc::listen(sock_fd.as_raw_fd(), 20) } < 0 {
            return Err(io::Error::last_os_error());
        }

        // build and return socket
        Ok(Self { sock_fd, lock_fd })
    }
}
