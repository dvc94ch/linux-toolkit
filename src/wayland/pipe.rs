//! Pipe abstraction for dnd and clipboard handling
use std::fs::File;
use std::io::{Read, Result, Write};
pub use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

/// A file descriptor that can only be written to
pub struct ReadPipe {
    file: File,
}

impl Read for ReadPipe {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.file.read(buf)
    }
}

impl FromRawFd for ReadPipe {
    unsafe fn from_raw_fd(fd: RawFd) -> ReadPipe {
        ReadPipe {
            file: FromRawFd::from_raw_fd(fd),
        }
    }
}

impl AsRawFd for ReadPipe {
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

impl IntoRawFd for ReadPipe {
    fn into_raw_fd(self) -> RawFd {
        self.file.into_raw_fd()
    }
}

/// A file descriptor that can only be written to
pub struct WritePipe {
    file: File,
}

impl Write for WritePipe {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.file.write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        self.file.flush()
    }
}

impl FromRawFd for WritePipe {
    unsafe fn from_raw_fd(fd: RawFd) -> WritePipe {
        WritePipe {
            file: FromRawFd::from_raw_fd(fd),
        }
    }
}

impl AsRawFd for WritePipe {
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

impl IntoRawFd for WritePipe {
    fn into_raw_fd(self) -> RawFd {
        self.file.into_raw_fd()
    }
}
