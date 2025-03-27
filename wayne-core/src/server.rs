use std::{
    env::{self, VarError},
    ffi::{OsStr, OsString},
    fs::{self, File},
    io,
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt},
        net::UnixListener,
    },
    path::{Path, PathBuf},
};

use fs2::FileExt;
use thiserror::Error;

use crate::Client;

#[derive(Debug, Error)]
pub enum TryBindError {
    #[error("Failed to bind socket: {_0}")]
    IOError(#[from] io::Error),
    #[error("Failed to get `XDG_RUNTIME_DIR`: {_0}")]
    VarError(#[from] VarError),
    #[error("Failed to bind sockets ({start}-{end}): Already in use.")]
    AlreadyInUse { start: usize, end: usize },
}

pub struct Server {
    listener: UnixListener,
    socket_path: PathBuf,
    socket_name: Option<OsString>,
    #[allow(dead_code)]
    lock_file: File, // keep lock file alive
    lock_path: PathBuf,
}

#[bon::bon]
impl Server {
    #[builder(finish_fn = bind)]
    pub fn try_bind(
        #[builder(start_fn)] start: usize,
        #[builder(name = until)] end: Option<usize>,
    ) -> Result<Self, TryBindError> {
        // get the xdg runtime directory from the environment variable
        let runtime_dir: PathBuf = env::var("XDG_RUNTIME_DIR")?.into();

        // try binding the socket to a range of values
        let end = end.unwrap_or(start);
        for address in start..=end {
            let name = format!("wayland-{address}");
            let path = runtime_dir.join(format!("wayland-{address}"));
            match Self::bind_path(path) {
                Ok(mut socket) => {
                    socket.socket_name = Some(name.into());
                    return Ok(socket);
                }
                Err(err) => match err.kind() {
                    // keep trying to bind to sockets if the error was one of the following
                    // AddrInUse: A bind was attempted, but the socket was already bound elsewhere
                    // WouldBlock: The lockfile could not be aquired, and would have blocked
                    io::ErrorKind::AddrInUse | io::ErrorKind::WouldBlock => continue,
                    // any other error is unexpected and should be returned
                    _ => return Err(TryBindError::IOError(err)),
                },
            }
        }

        // if no bind was successful, return an already in use error
        Err(TryBindError::AlreadyInUse { start, end })
    }
}

impl Server {
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub fn socket_name(&self) -> Option<&OsStr> {
        self.socket_name.as_ref().map(|s| s.as_os_str())
    }

    pub fn accept_client(&self) -> io::Result<Option<Client>> {
        match self.listener.accept() {
            Ok((stream, _)) => Ok(Some(Client::new(stream)?)),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn bind_path(path: PathBuf) -> io::Result<Self> {
        let lock_path = path.with_extension("lock");

        // USEFUL LOCKING CONDITION FROM:
        // https://github.com/Smithay/wayland-rs/blob/a1cc0482b98595ec26b4744c7bfd9e525f81acb1/wayland-server/src/socket.rs#L72
        // The locking code uses a loop to avoid an open()-flock() race condition, described in more
        // detail in the comment below. The implementation roughtly follows the one from libbsd:
        //
        // https://gitlab.freedesktop.org/libbsd/libbsd/-/blob/73b25a8f871b3a20f6ff76679358540f95d7dbfd/src/flopen.c#L71
        let lock_file = loop {
            // open and lock file
            let lock_file = File::options()
                .create(true)
                .read(true)
                .write(true)
                .mode(0o660)
                .open(&lock_path)?;
            lock_file.try_lock_exclusive()?;

            // Verify that the file we locked is the same as the file on disk. An unlucky unlink()
            // from a different thread which happens right between our open() and flock() may
            // result in us successfully locking a now-nonexistent file, with another thread locking
            // the same-named but newly created lock file, then both threads will think they have
            // exclusive access to the same socket. To prevent this, check that we locked the actual
            // currently existing file.
            let fd_meta = lock_file.metadata()?;
            let disk_meta = match fs::metadata(&lock_path) {
                Ok(disk_meta) => disk_meta,
                Err(err) if err.kind() == io::ErrorKind::NotFound => {
                    // This can happen during the aforementioned race condition.
                    continue;
                }
                Err(err) => return Err(err),
            };

            if fd_meta.dev() == disk_meta.dev() && fd_meta.ino() == disk_meta.ino() {
                break lock_file;
            }
        };

        // check if an old socket exists, and cleanup if relevant
        if path.try_exists()? {
            fs::remove_file(&path)?;
        }

        // create and build the socket
        let listener = UnixListener::bind(&path)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            socket_path: path,
            socket_name: None,
            lock_file,
            lock_path,
        })
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // remove socket created at target path
        let _ = fs::remove_file(&self.socket_path);
        let _ = fs::remove_file(&self.lock_path);
    }
}
