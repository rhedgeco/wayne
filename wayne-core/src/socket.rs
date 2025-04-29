use std::{
    io,
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
};

use crate::AdvisoryLock;

pub struct WaylandSocket {
    listener: UnixListener,
    #[allow(dead_code)]
    lock: AdvisoryLock,
}

impl WaylandSocket {
    pub fn bind(path: impl AsRef<Path>) -> io::Result<Self> {
        let sock_path = path.as_ref();
        let lock_path = sock_path.with_extension("lock");
        let lock = AdvisoryLock::lock(lock_path)?;
        let listener = UnixListener::bind(sock_path)?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener, lock })
    }

    pub fn accept(&self) -> io::Result<Option<UnixStream>> {
        match self.listener.accept() {
            Ok((stream, _)) => Ok(Some(stream)),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }
}
