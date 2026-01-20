//! Platform-agnostic IPC transport abstraction

use async_trait::async_trait;
use std::path::Path;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::Result;

/// Server-side IPC transport trait
#[async_trait]
pub trait IpcTransport: Send + Sync {
    /// The stream type for this transport
    type Stream: AsyncRead + AsyncWrite + Send + Unpin + 'static;

    /// Bind to the configured address and start listening
    async fn bind(path: &Path) -> Result<Self>
    where
        Self: Sized;

    /// Accept an incoming connection
    async fn accept(&self) -> Result<Self::Stream>;

    /// Clean up resources (e.g., delete socket file on Unix)
    #[allow(dead_code)]
    async fn cleanup(&self) -> Result<()>;
}

/// Client-side IPC transport trait
#[async_trait]
pub trait IpcClientTransport: Send + Sync {
    /// The stream type for this transport
    type Stream: AsyncRead + AsyncWrite + Send + Unpin + 'static;

    /// Connect to the daemon at the given path
    async fn connect(path: &Path) -> Result<Self::Stream>;
}

// Platform-specific type aliases
#[cfg(unix)]
pub use super::unix::{UnixIpcClient as PlatformClient, UnixIpcTransport as PlatformTransport};

#[cfg(windows)]
pub use super::windows::{
    WindowsIpcClient as PlatformClient, WindowsIpcTransport as PlatformTransport,
};
