use std::{
    fs::{self, File},
    io,
    ops::Deref,
    os::{linux::fs::MetadataExt, unix::fs::OpenOptionsExt},
    path::Path,
};

use fs2::FileExt;

pub struct AdvisoryLock(File);

impl Deref for AdvisoryLock {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AdvisoryLock {
    pub fn into_file(self) -> File {
        self.0
    }

    pub fn aquire(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();

        // aquire the lockfile
        // https://gitlab.freedesktop.org/libbsd/libbsd/-/blob/73b25a8f871b3a20f6ff76679358540f95d7dbfd/src/flopen.c#L71
        loop {
            // open the lockfile
            let file = File::options()
                .create(true)
                .truncate(true)
                .read(true)
                .write(true)
                .mode(0o640)
                .open(path)?;

            // aquire the advisory lock
            file.try_lock_exclusive()?;

            // get the metadata from the lockfile path
            let Ok(path_meta) = fs::metadata(path) else {
                // "disappeared from under our feet"
                // https://gitlab.freedesktop.org/libbsd/libbsd/-/blob/73b25a8f871b3a20f6ff76679358540f95d7dbfd/src/flopen.c#L101
                // when we cant get the meta data from the disk, the file must have been yanked/changed.
                // we need to continue here to try to open/create or lock_fd again.
                continue;
            };

            // get the metadata from the lockfile we currently have
            let file_meta = file.metadata()?;

            // ensure both significant metadata sections match
            if path_meta.st_dev() != file_meta.st_dev() || path_meta.st_ino() != file_meta.st_ino()
            {
                // if they dont, then the file on disk was replaced before the lock was aquired
                // this means we need to try opening and locking the file again
                continue;
            }

            // if all the above succeeded, then we have successfully locked the file
            return Ok(Self(file));
        }
    }
}
