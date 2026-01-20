//! Unix domain socket IPC transport

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::net::{UnixListener, UnixStream};

use crate::error::{DaemonError, Result};

use super::connection::{IpcClientTransport, IpcTransport};

/// Unix domain socket server transport
pub struct UnixIpcTransport {
    listener: UnixListener,
    socket_path: PathBuf,
}

#[async_trait]
impl IpcTransport for UnixIpcTransport {
    type Stream = UnixStream;

    async fn bind(path: &Path) -> Result<Self> {
        // Remove existing socket if present
        if path.exists() {
            std::fs::remove_file(path)?;
        }

        let listener = UnixListener::bind(path)
            .map_err(|e| DaemonError::Ipc(format!("Failed to bind socket: {}", e)))?;

        Ok(Self {
            listener,
            socket_path: path.to_path_buf(),
        })
    }

    async fn accept(&self) -> Result<Self::Stream> {
        let (stream, _) = self
            .listener
            .accept()
            .await
            .map_err(|e| DaemonError::Ipc(format!("Accept failed: {}", e)))?;
        Ok(stream)
    }

    async fn cleanup(&self) -> Result<()> {
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }
        Ok(())
    }
}

/// Unix domain socket client transport
pub struct UnixIpcClient;

#[async_trait]
impl IpcClientTransport for UnixIpcClient {
    type Stream = UnixStream;

    async fn connect(path: &Path) -> Result<Self::Stream> {
        UnixStream::connect(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound
                || e.kind() == std::io::ErrorKind::ConnectionRefused
            {
                DaemonError::Ipc("Daemon not running".to_string())
            } else {
                DaemonError::Ipc(format!("Failed to connect: {}", e))
            }
        })
    }
}
