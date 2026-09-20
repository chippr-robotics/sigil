//! Unix domain socket IPC transport

use async_trait::async_trait;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tokio::net::{UnixListener, UnixStream};

use crate::error::{DaemonError, Result};

use super::connection::{BindOptions, IpcClientTransport, IpcTransport};

/// Unix domain socket server transport
pub struct UnixIpcTransport {
    listener: UnixListener,
    #[allow(dead_code)] // Used in cleanup(), which may not be called yet
    socket_path: PathBuf,
}

#[async_trait]
impl IpcTransport for UnixIpcTransport {
    type Stream = UnixStream;

    async fn bind(path: &Path, options: BindOptions) -> Result<Self> {
        // The IPC socket is a direct, unauthenticated path to the signer: a
        // process that can connect can ask the daemon to spend presignatures
        // from an inserted disk. Restrict it at the filesystem layer.
        //
        // Two steps, in this order:
        //
        // 1. Ensure the parent directory exists and is 0700. This is what
        //    actually closes the window between `bind` and `set_permissions`
        //    below — nothing can traverse into the directory to reach a
        //    briefly-permissive socket. It also prevents another local user
        //    from squatting the socket path before the daemon starts, which
        //    a world-writable parent (the old `/tmp` default) allowed.
        // 2. Apply explicit permissions to the socket itself, rather than
        //    inheriting whatever the process umask happens to be.
        //
        // See Constitution Principle VII.
        prepare_socket_dir(path)?;

        // Remove existing socket if present
        if path.exists() {
            std::fs::remove_file(path)?;
        }

        let listener = UnixListener::bind(path)
            .map_err(|e| DaemonError::Ipc(format!("Failed to bind socket: {}", e)))?;

        std::fs::set_permissions(path, std::fs::Permissions::from_mode(options.socket_mode))
            .map_err(|e| {
                DaemonError::Ipc(format!(
                    "Failed to restrict permissions on {}: {}",
                    path.display(),
                    e
                ))
            })?;

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

/// Permission bits for the directory holding the IPC socket: owner only.
const SOCKET_DIR_MODE: u32 = 0o700;

/// Ensure the socket's parent directory exists and is not traversable by other
/// users.
///
/// A pre-existing directory's permissions are left alone: operators who point
/// the daemon at a shared, deliberately-configured location (for example a
/// group-owned `/run/sigil`) keep their configuration. Only directories this
/// function creates are forced to `0700`.
fn prepare_socket_dir(socket_path: &Path) -> Result<()> {
    let Some(parent) = socket_path.parent() else {
        return Ok(());
    };

    if parent.as_os_str().is_empty() || parent.exists() {
        return Ok(());
    }

    std::fs::create_dir_all(parent).map_err(|e| {
        DaemonError::Ipc(format!(
            "Failed to create socket directory {}: {}",
            parent.display(),
            e
        ))
    })?;

    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(SOCKET_DIR_MODE)).map_err(
        |e| {
            DaemonError::Ipc(format!(
                "Failed to restrict permissions on {}: {}",
                parent.display(),
                e
            ))
        },
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::connection::DEFAULT_SOCKET_MODE;
    use super::*;

    fn scratch_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sigil-ipc-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[tokio::test]
    async fn socket_is_not_accessible_to_other_users() {
        let dir = scratch_dir("mode");
        let path = dir.join("sigil.sock");

        let transport = UnixIpcTransport::bind(&path, BindOptions::default())
            .await
            .expect("bind must succeed");

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o007,
            0,
            "IPC socket must grant nothing to `other`, got {:o}",
            mode & 0o777
        );
        assert_eq!(mode & 0o777, DEFAULT_SOCKET_MODE);

        transport.cleanup().await.unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn custom_socket_mode_is_applied() {
        let dir = scratch_dir("custom-mode");
        let path = dir.join("sigil.sock");

        let transport = UnixIpcTransport::bind(&path, BindOptions { socket_mode: 0o600 })
            .await
            .expect("bind must succeed");

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);

        transport.cleanup().await.unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn absent_parent_directory_is_created_owner_only() {
        let dir = scratch_dir("parent");
        let path = dir.join("nested").join("sigil.sock");

        let transport = UnixIpcTransport::bind(&path, BindOptions::default())
            .await
            .expect("bind must create the parent directory");

        let parent_mode = std::fs::metadata(path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(
            parent_mode & 0o777,
            SOCKET_DIR_MODE,
            "socket directory must not be traversable by others, got {:o}",
            parent_mode & 0o777
        );

        transport.cleanup().await.unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn stale_socket_is_replaced() {
        let dir = scratch_dir("stale");
        let path = dir.join("sigil.sock");

        let first = UnixIpcTransport::bind(&path, BindOptions::default())
            .await
            .unwrap();
        drop(first);

        // Socket file still on disk; bind must reclaim it rather than fail.
        let second = UnixIpcTransport::bind(&path, BindOptions::default())
            .await
            .expect("bind must replace a stale socket");

        second.cleanup().await.unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn existing_directory_permissions_are_left_alone() {
        let dir = scratch_dir("existing");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o750)).unwrap();

        prepare_socket_dir(&dir.join("sigil.sock")).unwrap();

        let mode = std::fs::metadata(&dir).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o750);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
