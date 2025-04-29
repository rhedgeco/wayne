use std::{
    env::{self, VarError},
    io,
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
};

use thiserror::Error;

use crate::AdvisoryLock;

#[derive(Debug, Error)]
pub enum BindError {
    #[error("Failed to bind socket: {_0}")]
    Io(#[from] io::Error),
    #[error("Failed to get 'XDG_RUNTIME_DIR': {_0}")]
    VarError(#[from] VarError),
    #[error("Failed to bind wayland socket in range 0-{_0}")]
    InUse(usize),
}

pub struct WaylandSocket {
    listener: UnixListener,
    #[allow(dead_code)]
    lock: AdvisoryLock,
    name: String,
}

impl WaylandSocket {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn accept(&self) -> io::Result<Option<UnixStream>> {
        match self.listener.accept() {
            Ok((stream, _)) => Ok(Some(stream)),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn bind(max: usize) -> Result<Self, BindError> {
        // get the xdg environment variable
        let xdg_dir: PathBuf = env::var("XDG_RUNTIME_DIR")?.into();

        // create a function to bind a socket path
        let bind_name = |name: String| -> io::Result<WaylandSocket> {
            let sock_path = xdg_dir.join(&name);
            let lock_path = sock_path.with_extension("lock");
            let lock = AdvisoryLock::aquire(lock_path)?;
            let listener = UnixListener::bind(sock_path)?;
            listener.set_nonblocking(true)?;
            Ok(WaylandSocket {
                listener,
                lock,
                name,
            })
        };

        // try binding a range of wayland socket locations
        for index in 0..max {
            return match bind_name(format!("wayland-{index}")) {
                Ok(socket) => Ok(socket),
                Err(e) => match e.kind() {
                    // if an address is in use or a blocking procedure was reached,
                    // continue and to try the next socket address
                    io::ErrorKind::AddrInUse | io::ErrorKind::WouldBlock => continue,
                    // otherwise the error is unexpected and we should return immediately
                    _ => Err(e.into()),
                },
            };
        }

        // if no socket was bound, return an error
        Err(BindError::InUse(max))
    }
}
