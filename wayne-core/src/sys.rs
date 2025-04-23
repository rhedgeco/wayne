use std::{
    ffi::{CStr, CString},
    io,
    mem::MaybeUninit,
    os::{
        fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
        unix::ffi::OsStrExt,
    },
    path::Path,
};

pub type Stat = libc::stat;

pub fn cpath(path: impl AsRef<Path>) -> io::Result<CString> {
    match CString::new(path.as_ref().as_os_str().as_bytes()) {
        Ok(string) => Ok(string),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "path contains invalid nul bytes",
        )),
    }
}

pub fn unlink(path: &CStr) -> io::Result<()> {
    // SAFETY: path ptr comes directly from a valid &Cstr
    match unsafe { libc::unlink(path.as_ptr()) } {
        -1 => Err(io::Error::last_os_error()),
        _ => Ok(()),
    }
}

pub fn open_lockfile(path: &CStr) -> io::Result<OwnedFd> {
    // SAFETY: path ptr comes directly from a valid &Cstr
    match unsafe {
        libc::open(
            path.as_ptr(),
            // O_CREAT - create the file if it doesnt exist
            // O_RDWR - aquire with read/write permissions
            libc::O_CREAT | libc::O_RDWR,
            // S_IRUSR - user read permission
            // S_IWUSR - user write permission
            // S_IRGRP - group read permission
            libc::S_IRUSR | libc::S_IWUSR | libc::S_IRGRP,
        )
    } {
        -1 => Err(io::Error::last_os_error()),
        // SAFETY: raw file descriptor is garunteed to be owned and valid here
        raw => Ok(unsafe { OwnedFd::from_raw_fd(raw) }),
    }
}

pub fn flock(fd: BorrowedFd) -> io::Result<()> {
    // lock the file in a non-blocking manner
    // - LOCK_EX - aquire exclusive lock
    // - LOCK_NB - use non-blocking operation
    let operation = libc::LOCK_EX | libc::LOCK_NB;
    // SAFETY: raw file descriptor comes from a valid BorrowedFd
    match unsafe { libc::flock(fd.as_raw_fd(), operation) } {
        -1 => Err(io::Error::last_os_error()),
        _ => Ok(()),
    }
}

pub fn path_stat(path: &CStr) -> io::Result<Stat> {
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    // SAFETY: path ptr comes directly from a valid &Cstr
    match unsafe { libc::stat(path.as_ptr(), stat.as_mut_ptr()) } {
        -1 => Err(io::Error::last_os_error()),
        // SAFETY:
        // if the stat function did not return an error,
        // then the stat data can be assumed to be initialized
        _ => Ok(unsafe { stat.assume_init() }),
    }
}

pub fn fd_stat(fd: BorrowedFd) -> io::Result<Stat> {
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    // SAFETY: raw file descriptor comes from a valid BorrowedFd
    match unsafe { libc::fstat(fd.as_raw_fd(), stat.as_mut_ptr()) } {
        -1 => Err(io::Error::last_os_error()),
        // SAFETY:
        // if the fstat function did not return an error,
        // then the stat data can be assumed to be initialized
        _ => Ok(unsafe { stat.assume_init() }),
    }
}

pub fn bind_socket(path: &CStr) -> io::Result<OwnedFd> {
    // build the socket address
    let mut socket_addr = libc::sockaddr_un {
        sun_family: libc::AF_UNIX as _,
        sun_path: [0; 108],
    };

    // insert the bytes into the socketaddr
    let mut path_bytes = path.to_bytes_with_nul();
    if path_bytes.len() > socket_addr.sun_path.len() {
        path_bytes = path.to_bytes(); // without NUL
        if path_bytes.len() > socket_addr.sun_path.len() {
            // if path is too long return ENAMETOOLONG
            return Err(io::Error::from_raw_os_error(36));
        }
    }

    // copy the bytes from the path_name into the socket_addr
    // SAFETY: transmuting valid [u8] into [i8] has no invalid memory state
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

    Ok(sock_fd)
}

pub fn listen(fd: BorrowedFd) -> io::Result<()> {
    match unsafe { libc::listen(fd.as_raw_fd(), 20) } {
        -1 => Err(io::Error::last_os_error()),
        _ => Ok(()),
    }
}

pub fn accept(fd: BorrowedFd) -> io::Result<Option<OwnedFd>> {
    match unsafe { libc::accept(fd.as_raw_fd(), core::ptr::null_mut(), core::ptr::null_mut()) } {
        -1 => match io::Error::last_os_error() {
            e if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
            e => Err(e),
        },
        raw => Ok(Some(unsafe { OwnedFd::from_raw_fd(raw) })),
    }
}

pub fn recvmsg(fd: BorrowedFd, msghdr: &mut libc::msghdr) -> io::Result<usize> {
    match unsafe {
        libc::recvmsg(
            fd.as_raw_fd(),
            msghdr as *mut _,
            libc::MSG_CMSG_CLOEXEC | libc::MSG_DONTWAIT,
        )
    } {
        -1 => match io::Error::last_os_error() {
            // if we got a would block error, just use length 0
            e if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
            e => Err(e),
        },
        len => {
            debug_assert!(len > 0);
            Ok(len as usize)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_cpath() {
        let valid = cpath("/valid/path");
        assert!(valid.is_ok());

        let invalid = cpath("/invalid\0/path");
        assert!(invalid.is_err());
    }
}
