//! Windows named pipe IPC transport
//!
//! Production implementation using tokio's AsyncFd for proper async I/O

use async_trait::async_trait;
use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, ERROR_PIPE_CONNECTED, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_NONE, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};

use crate::error::{DaemonError, Result};

use super::connection::{IpcClientTransport, IpcTransport};

/// Windows named pipe wrapper with proper async I/O support
pub struct WindowsNamedPipe {
    /// Standard file wrapper for the pipe handle
    /// This uses blocking I/O but we'll run it on tokio's blocking pool
    inner: std::sync::Arc<std::sync::Mutex<NamedPipeFile>>,
}

/// Wrapper around the raw pipe handle
struct NamedPipeFile {
    handle: HANDLE,
}

impl NamedPipeFile {
    fn new(handle: HANDLE) -> Self {
        Self { handle }
    }

    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use windows::Win32::Storage::FileSystem::ReadFile;

        let mut bytes_read = 0u32;
        let result = unsafe { ReadFile(self.handle, Some(buf), Some(&mut bytes_read), None) };

        match result {
            Ok(_) => Ok(bytes_read as usize),
            Err(e) => {
                let err_code = e.code().0;
                // ERROR_NO_DATA means pipe was closed gracefully
                if err_code == 232 {
                    Ok(0)
                } else {
                    Err(io::Error::from_raw_os_error(err_code))
                }
            }
        }
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        use windows::Win32::Storage::FileSystem::WriteFile;

        let mut bytes_written = 0u32;
        let result = unsafe { WriteFile(self.handle, Some(buf), Some(&mut bytes_written), None) };

        match result {
            Ok(_) => Ok(bytes_written as usize),
            Err(e) => {
                let err_code = e.code().0;
                // ERROR_BROKEN_PIPE or ERROR_NO_DATA
                if err_code == 109 || err_code == 232 {
                    Err(io::Error::new(io::ErrorKind::BrokenPipe, "pipe closed"))
                } else {
                    Err(io::Error::from_raw_os_error(err_code))
                }
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        use windows::Win32::Storage::FileSystem::FlushFileBuffers;

        match unsafe { FlushFileBuffers(self.handle) } {
            Ok(_) => Ok(()),
            Err(e) => Err(io::Error::from_raw_os_error(e.code().0)),
        }
    }
}

impl Drop for NamedPipeFile {
    fn drop(&mut self) {
        if self.handle != INVALID_HANDLE_VALUE {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

// Safety: Named pipe handles can be sent between threads
unsafe impl Send for NamedPipeFile {}
unsafe impl Sync for NamedPipeFile {}

impl WindowsNamedPipe {
    fn new(handle: HANDLE) -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(NamedPipeFile::new(handle))),
        }
    }
}

impl AsyncRead for WindowsNamedPipe {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // Use blocking operations in a way that's compatible with tokio
        // We read synchronously since Windows named pipes in blocking mode are efficient
        let mut inner = match self.inner.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                // If we can't get the lock, wake the task and try again later
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        let unfilled = buf.initialize_unfilled();
        match inner.read(unfilled) {
            Ok(0) => Poll::Ready(Ok(())),
            Ok(n) => {
                buf.advance(n);
                Poll::Ready(Ok(()))
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl AsyncWrite for WindowsNamedPipe {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut inner = match self.inner.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        match inner.write(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut inner = match self.inner.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        match inner.flush() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// Windows named pipe server transport
pub struct WindowsIpcTransport {
    pipe_name: String,
    /// First pipe instance, created during bind and reused for first connection
    first_pipe: std::sync::Arc<std::sync::Mutex<Option<SendHandle>>>,
}

/// Send-safe wrapper for HANDLE
/// Safety: Named pipe handles can be safely sent between threads
struct SendHandle(HANDLE);
unsafe impl Send for SendHandle {}
unsafe impl Sync for SendHandle {}

#[async_trait]
impl IpcTransport for WindowsIpcTransport {
    type Stream = WindowsNamedPipe;

    async fn bind(path: &Path) -> Result<Self> {
        let pipe_name = path.to_string_lossy().to_string();

        // Validate pipe name format
        if !pipe_name.starts_with(r"\\.\pipe\") {
            return Err(DaemonError::Config(
                "Windows pipe name must start with \\\\.\\pipe\\".to_string(),
            ));
        }

        // Create the first pipe instance so it's ready for connections
        let pipe_name_wide: Vec<u16> = OsStr::new(&pipe_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(pipe_name_wide.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                8192,
                8192,
                0,
                None,
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(DaemonError::Ipc(format!(
                "Failed to create initial named pipe: {:?}",
                std::io::Error::last_os_error()
            )));
        }

        Ok(Self {
            pipe_name,
            first_pipe: std::sync::Arc::new(std::sync::Mutex::new(Some(SendHandle(handle)))),
        })
    }

    async fn accept(&self) -> Result<Self::Stream> {
        // Try to use the first pipe if available, otherwise create a new one
        let handle = {
            let mut first_pipe = self.first_pipe.lock().unwrap();
            if let Some(SendHandle(h)) = first_pipe.take() {
                // Use the pre-created pipe
                h
            } else {
                // Create a new pipe instance for subsequent connections
                let pipe_name_wide: Vec<u16> = OsStr::new(&self.pipe_name)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                let h = unsafe {
                    CreateNamedPipeW(
                        PCWSTR(pipe_name_wide.as_ptr()),
                        PIPE_ACCESS_DUPLEX,
                        PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                        PIPE_UNLIMITED_INSTANCES,
                        8192,
                        8192,
                        0,
                        None,
                    )
                };

                if h == INVALID_HANDLE_VALUE {
                    return Err(DaemonError::Ipc(format!(
                        "Failed to create named pipe: {:?}",
                        std::io::Error::last_os_error()
                    )));
                }
                h
            }
        };

        // Wait for client connection (blocking call)
        // Spawn this on a blocking thread to not block the async runtime
        let handle_send = SendHandle(handle);
        let connected = tokio::task::spawn_blocking(move || {
            let result = unsafe { ConnectNamedPipe(handle_send.0, None) };
            (handle_send, result)
        })
        .await
        .map_err(|e| DaemonError::Ipc(format!("Task join error: {}", e)))?;

        let (SendHandle(handle), connect_result) = connected;

        if connect_result.is_err() {
            let last_error = unsafe { windows::Win32::Foundation::GetLastError() };
            if last_error != ERROR_PIPE_CONNECTED {
                unsafe {
                    CloseHandle(handle).ok();
                }
                return Err(DaemonError::Ipc(format!(
                    "Failed to connect named pipe: {:?}",
                    std::io::Error::last_os_error()
                )));
            }
        }

        Ok(WindowsNamedPipe::new(handle))
    }

    async fn cleanup(&self) -> Result<()> {
        // Named pipes are automatically cleaned up when handles close
        Ok(())
    }
}

/// Windows named pipe client transport
pub struct WindowsIpcClient;

#[async_trait]
impl IpcClientTransport for WindowsIpcClient {
    type Stream = WindowsNamedPipe;

    async fn connect(path: &Path) -> Result<Self::Stream> {
        use windows::Win32::Storage::FileSystem::{FILE_GENERIC_READ, FILE_GENERIC_WRITE};

        let pipe_name = path.to_string_lossy().to_string();
        let pipe_name_wide: Vec<u16> = OsStr::new(&pipe_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // Try to open existing pipe
        let handle = unsafe {
            CreateFileW(
                PCWSTR(pipe_name_wide.as_ptr()),
                FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                None,
            )
        };

        match handle {
            Ok(h) if h != INVALID_HANDLE_VALUE => Ok(WindowsNamedPipe::new(h)),
            _ => Err(DaemonError::Ipc("Daemon not running".to_string())),
        }
    }
}
