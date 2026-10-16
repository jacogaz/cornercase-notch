use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::log;

const RETRY: Duration = Duration::from_secs(2);

pub(crate) fn path() -> PathBuf {
    std::env::var_os("TMPDIR")
        .filter(|dir| !dir.is_empty())
        .map_or_else(|| PathBuf::from("/tmp"), PathBuf::from)
        .join("cornercase-notch.lock")
}

pub(crate) fn acquire(path: &Path) -> io::Result<File> {
    let mut waited = false;
    loop {
        if let Some(file) = try_acquire(path)? {
            return Ok(file);
        }
        if !waited {
            log::line("another notch is showing; waiting for it to close");
            waited = true;
        }
        thread::sleep(RETRY);
    }
}

pub(crate) fn try_acquire(path: &Path) -> io::Result<Option<File>> {
    let file = OpenOptions::new().create(true).write(true).truncate(false).open(path)?;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        return Ok(Some(file));
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::WouldBlock { Ok(None) } else { Err(error) }
}

#[cfg(test)]
mod tests;
