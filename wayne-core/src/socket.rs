use std::{
    env::{self, VarError},
    ffi::{OsStr, OsString},
    io,
    os::fd::OwnedFd,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use derive_more::Display;
use log::error;
use rustix::{
    fs::{self, FlockOperation, Mode, OFlags},
    net::{self, AddressFamily, Shutdown, SocketAddrUnix, SocketFlags, SocketType},
};
use thiserror::Error;

use crate::ClientStream;

#[derive(Debug, Error)]
pub enum BindError {
    #[error("Failed to bind socket: {_0}")]
    IOError(#[from] io::Error),
    #[error("Failed to get `XDG_RUNTIME_DIR`: {_0}")]
    VarError(#[from] VarError),
    #[error("Failed to bind socket: Addr(s) already in use.")]
    AlreadyInUse,
}

#[repr(transparent)]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SocketId(u64);

#[derive(Debug)]
pub struct WaylandSocket {
    socket_name: Option<OsString>,
    socket_path: PathBuf,
    socket_fd: OwnedFd,
    lock_path: PathBuf,
    #[allow(dead_code)]
    lock_fd: OwnedFd,
    id: SocketId,
}

impl Drop for WaylandSocket {
    fn drop(&mut self) {
        // try to shutdown and remove socket items on drop
        let _ = net::shutdown(&self.socket_fd, Shutdown::Both);
        let _ = std::fs::remove_file(&self.socket_path);
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

#[bon::bon]
impl WaylandSocket {
    #[builder(finish_fn = bind)]
    pub fn build(
        #[builder(start_fn)] value: usize,
        #[builder(name = try_until)] end: Option<usize>,
    ) -> Result<Self, BindError> {
        // get the xdg runtime directory from the environment variable
        let runtime_dir: PathBuf = env::var("XDG_RUNTIME_DIR")?.into();

        // try binding the socket on the range of values
        let end = end.unwrap_or(value);
        for address in value..=end {
            let name = format!("wayland-{address}");
            let path = runtime_dir.join(format!("wayland-{address}"));
            match Self::bind_path(path) {
                Ok(mut socket) => {
                    socket.socket_name = Some(name.into());
                    return Ok(socket);
                }
                Err(err) => {
                    match err.kind() {
                        // keep trying to bind to sockets if the error was one of the following
                        // AddrInUse: A bind was attempted, but the socket was already bound elsewhere
                        // WouldBlock: The lockfile could not be aquired, and would have blocked
                        io::ErrorKind::AddrInUse | io::ErrorKind::WouldBlock => continue,
                        // any other error is unexpected and should be returned
                        _ => return Err(BindError::IOError(err)),
                    }
                }
            }
        }

        Err(BindError::AlreadyInUse)
    }
}

impl WaylandSocket {
    pub fn id(&self) -> SocketId {
        self.id
    }

    pub fn path(&self) -> &Path {
        &self.socket_path
    }

    pub fn socket_name(&self) -> Option<&OsStr> {
        self.socket_name.as_ref().map(|s| s.as_os_str())
    }

    pub fn accept(&self) -> io::Result<Option<ClientStream>> {
        match net::accept(&self.socket_fd) {
            Ok(stream_fd) => Ok(Some(ClientStream::new(stream_fd, self.id))),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn bind_path(path: impl Into<PathBuf>) -> io::Result<Self> {
        // create the path for the socket lockfile
        let socket_path = path.into();
        let lock_path = socket_path.with_extension("lock");

        // aquire the lockfile
        // https://gitlab.freedesktop.org/libbsd/libbsd/-/blob/73b25a8f871b3a20f6ff76679358540f95d7dbfd/src/flopen.c#L71
        let lock_fd = loop {
            // open the lockfile
            let lock_fd = fs::open(
                &lock_path,
                OFlags::CREATE | OFlags::RDWR,
                Mode::RUSR | Mode::WUSR | Mode::RGRP,
            )?;

            // lock the file in a non-blocking manner
            fs::flock(&lock_fd, FlockOperation::NonBlockingLockExclusive)?;

            // In rare cases, a race condition may occur.
            // this can happen when the lockfile is removed/replaced after `open`, but before `flock`.
            // In this instance, the file descriptor we have is different than the file on disk.
            // To ensure the lock was aquired successfully, we need to check the metadata.
            // If the descriptor and file metadata math, we can be sure that the lock was successful.

            // get the metadata for the lockfile on disk
            let Ok(fs_meta) = fs::stat(&lock_path) else {
                // "disappeared from under our feet"
                // https://gitlab.freedesktop.org/libbsd/libbsd/-/blob/73b25a8f871b3a20f6ff76679358540f95d7dbfd/src/flopen.c#L101
                // when we cant get the meta data from the disk, the file must have been yanked.
                // so just try to create or lock the file again.
                continue;
            };

            // get the metadata for the file descriptor we have
            let fd_meta = fs::fstat(&lock_fd)?;

            // ensure both significant metadata sections match
            if fs_meta.st_dev != fd_meta.st_dev || fs_meta.st_ino != fd_meta.st_ino {
                // if they dont, then the file on disk was replaced before the lock happened
                continue;
            }

            // if all the above succeeded, then we have successfully locked the file
            break lock_fd;
        };

        // create the socket path
        if socket_path.try_exists()? {
            // delete any old leftover paths
            // this expects the lockfile to be respected
            fs::unlink(&socket_path)?;
        }

        // build a unix socket to listen on
        let socket_addr = SocketAddrUnix::new(&socket_path)?;
        let socket_fd = net::socket_with(
            AddressFamily::UNIX,
            SocketType::STREAM,
            SocketFlags::all(),
            None,
        )?;

        // bind the socket to the path and listen for connections
        net::bind(&socket_fd, &socket_addr)?;
        net::listen(&socket_fd, 20)?;

        // generate a unique socket id
        let id = SocketId({
            static GENERATOR: AtomicU64 = AtomicU64::new(0);
            GENERATOR.fetch_add(1, Ordering::Relaxed)
        });

        // finally build the server and return
        Ok(Self {
            socket_name: None,
            socket_path,
            socket_fd,
            lock_path,
            lock_fd,
            id,
        })
    }
}
