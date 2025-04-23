use std::{
    io,
    os::fd::{AsFd, OwnedFd},
    path::Path,
};

use crate::{WaylandStream, sys};

pub struct WaylandListener {
    sock_fd: OwnedFd,
    #[allow(dead_code)]
    lock_fd: OwnedFd,
}

impl WaylandListener {
    pub fn accept(&self) -> io::Result<Option<WaylandStream>> {
        Ok(sys::accept(self.sock_fd.as_fd())?.map(WaylandStream::new))
    }

    pub fn bind(path: impl AsRef<Path>) -> io::Result<Self> {
        // create lock path
        let lock_path = sys::cpath(path.as_ref().with_extension("lock"))?;

        // aquire the lockfile
        // https://gitlab.freedesktop.org/libbsd/libbsd/-/blob/73b25a8f871b3a20f6ff76679358540f95d7dbfd/src/flopen.c#L71
        let lock_fd = loop {
            // open the lockfile
            let lock_fd = sys::open_lockfile(&lock_path)?;

            // lock the file
            sys::flock(lock_fd.as_fd())?;

            // get the metadata for the lockfile on disk
            let Ok(fs_stat) = sys::path_stat(&lock_path) else {
                // "disappeared from under our feet"
                // https://gitlab.freedesktop.org/libbsd/libbsd/-/blob/73b25a8f871b3a20f6ff76679358540f95d7dbfd/src/flopen.c#L101
                // when we cant get the meta data from the disk, the file must have been yanked/changed.
                // we need to continue here to try to open/create or lock_fd again.
                continue;
            };

            // get the metadata for the file descriptor we currently have
            let fd_stat = sys::fd_stat(lock_fd.as_fd())?;

            // ensure both significant metadata sections match
            if fs_stat.st_dev != fd_stat.st_dev || fs_stat.st_ino != fd_stat.st_ino {
                // if they dont, then the file on disk was replaced before the lock happened
                continue;
            }

            // if all the above succeeded, then we have successfully locked the file
            break lock_fd;
        };

        // create sock path
        let sock_path = sys::cpath(path.as_ref())?;

        // ensure the socket file doesnt already exist
        match sys::path_stat(&sock_path) {
            Ok(_) => sys::unlink(&sock_path)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }

        // bind a new socket
        let sock_fd = sys::bind_socket(&sock_path)?;

        // start listening on the socket
        sys::listen(sock_fd.as_fd())?;

        // build and return socket
        Ok(Self { sock_fd, lock_fd })
    }
}
