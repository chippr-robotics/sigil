//! Platform-agnostic IPC transport abstraction

use async_trait::async_trait;
use std::path::Path;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::Result;

/// Default Unix permission bits for the IPC socket.
///
/// `0o660` — owner and group only, nothing for `other`. The systemd unit runs
/// the daemon as `root:sigil`, so members of the `sigil` group can reach the
/// signer and no one else can. Before this, the socket inherited the process
/// umask in a world-writable directory: any local user could connect and spend
/// presignatures from an inserted disk. See Constitution Principle VII.
pub const DEFAULT_SOCKET_MODE: u32 = 0o660;

/// Security options applied when binding the IPC endpoint.
#[derive(Debug, Clone, Copy)]
pub struct BindOptions {
    /// Unix permission bits applied to the socket immediately after bind.
    /// Ignored on Windows, where the named pipe's ACL governs access.
    pub socket_mode: u32,
}

impl Default for BindOptions {
    fn default() -> Self {
        Self {
            socket_mode: DEFAULT_SOCKET_MODE,
        }
    }
}

/// Server-side IPC transport trait
#[async_trait]
pub trait IpcTransport: Send + Sync {
    /// The stream type for this transport
    type Stream: AsyncRead + AsyncWrite + Send + Unpin + 'static;

    /// Bind to the configured address and start listening.
    ///
    /// Implementations MUST restrict access to the endpoint before returning:
    /// the local IPC socket is a direct path to the signer.
    async fn bind(path: &Path, options: BindOptions) -> Result<Self>
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
