//! Windows named pipe IPC transport

use async_trait::async_trait;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, ERROR_PIPE_CONNECTED, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX;
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_NONE, OPEN_EXISTING,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};

use crate::error::{DaemonError, Result};

use super::connection::{IpcClientTransport, IpcTransport};

/// Windows named pipe wrapper with async I/O support
pub struct WindowsNamedPipe {
    handle: SendHandle,
    // Note: We keep the raw handle and use blocking I/O for simplicity
    // A production implementation would use OVERLAPPED I/O with tokio's reactor
}

/// Send-safe wrapper for HANDLE
/// Safety: Named pipe handles can be safely sent between threads
struct SendHandle(HANDLE);
unsafe impl Send for SendHandle {}
unsafe impl Sync for SendHandle {}

impl WindowsNamedPipe {
    fn new(handle: HANDLE) -> Self {
        Self {
            handle: SendHandle(handle),
        }
    }
}

impl AsyncRead for WindowsNamedPipe {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        use windows::Win32::Storage::FileSystem::ReadFile;

        let mut bytes_read = 0u32;
        let result = unsafe {
            ReadFile(
                self.handle.0,
                Some(buf.initialize_unfilled()),
                Some(&mut bytes_read),
                None,
            )
        };

        match result {
            Ok(_) => {
                buf.advance(bytes_read as usize);
                Poll::Ready(Ok(()))
            }
            Err(e) => Poll::Ready(Err(std::io::Error::from_raw_os_error(e.code().0))),
        }
    }
}

impl AsyncWrite for WindowsNamedPipe {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        use windows::Win32::Storage::FileSystem::WriteFile;

        let mut bytes_written = 0u32;
        let result = unsafe { WriteFile(self.handle.0, Some(buf), Some(&mut bytes_written), None) };

        match result {
            Ok(_) => Poll::Ready(Ok(bytes_written as usize)),
            Err(e) => Poll::Ready(Err(std::io::Error::from_raw_os_error(e.code().0))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        use windows::Win32::Storage::FileSystem::FlushFileBuffers;

        match unsafe { FlushFileBuffers(self.handle.0) } {
            Ok(_) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(std::io::Error::from_raw_os_error(e.code().0))),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl Drop for WindowsNamedPipe {
    fn drop(&mut self) {
        if self.handle.0 != INVALID_HANDLE_VALUE {
            unsafe {
                let _ = CloseHandle(self.handle.0);
            }
        }
    }
}

/// Windows named pipe server transport
pub struct WindowsIpcTransport {
    pipe_name: String,
}

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

        Ok(Self { pipe_name })
    }

    async fn accept(&self) -> Result<Self::Stream> {
        // Convert pipe name to wide string
        let pipe_name_wide: Vec<u16> = OsStr::new(&self.pipe_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // Create named pipe instance
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(pipe_name_wide.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                4096, // out buffer size
                4096, // in buffer size
                0,    // default timeout
                None, // default security
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(DaemonError::Ipc(format!(
                "Failed to create named pipe: {:?}",
                std::io::Error::last_os_error()
            )));
        }

        // Wait for client connection
        let connected = unsafe { ConnectNamedPipe(handle, None) };
        if connected.is_err() {
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
