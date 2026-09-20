//! Bearer-token authentication for the out-of-TCB HTTP bridge.
//!
//! This is a lock on a door that is marked "not part of the trusted computing
//! base". It exists so that reaching the daemon over HTTP requires a secret the
//! operator holds, which removes the confused-deputy class of attack (any
//! browser or host that can route to the bridge driving `/api/sign` on the
//! operator's behalf). It is explicitly **not** the boundary the product's
//! security claim rests on — that boundary is the physically inserted disk.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use rand::RngCore;
use subtle::ConstantTimeEq;
use tracing::{info, warn};

/// Environment variable an operator can use to supply the token directly.
pub const TOKEN_ENV: &str = "SIGIL_BRIDGE_TOKEN";

/// Minimum accepted length for an operator-supplied token, in characters.
const MIN_TOKEN_LEN: usize = 16;

/// Number of random bytes in a generated token (rendered as 64 hex chars).
const GENERATED_TOKEN_BYTES: usize = 32;

/// The shared secret guarding every `/api/*` route.
#[derive(Clone)]
pub struct BridgeToken {
    value: String,
}

impl BridgeToken {
    /// Resolve the token, in order of precedence:
    ///
    /// 1. `--token-file <path>` — read the first line of the file.
    /// 2. `SIGIL_BRIDGE_TOKEN` — read from the environment.
    /// 3. Generate a fresh token and persist it with owner-only permissions.
    ///
    /// There is deliberately no "no token" outcome. A bridge that starts
    /// without authentication is the bug this module exists to prevent.
    pub fn resolve(token_file: Option<&Path>, runtime_dir: Option<&Path>) -> Result<Self> {
        if let Some(path) = token_file {
            let raw = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read token file {}", path.display()))?;
            return Self::from_supplied(raw.trim(), &format!("token file {}", path.display()));
        }

        if let Ok(raw) = std::env::var(TOKEN_ENV) {
            return Self::from_supplied(raw.trim(), TOKEN_ENV);
        }

        let value = generate_token();
        let path = default_token_path(runtime_dir);
        persist_token(&path, &value)?;
        warn!(
            "No bridge token supplied; generated one and wrote it to {}",
            path.display()
        );
        info!(
            "Clients must send: Authorization: Bearer $(cat {})",
            path.display()
        );
        Ok(Self { value })
    }

    fn from_supplied(raw: &str, source: &str) -> Result<Self> {
        if raw.is_empty() {
            bail!("bridge token from {source} is empty");
        }
        if raw.chars().count() < MIN_TOKEN_LEN {
            bail!("bridge token from {source} is shorter than {MIN_TOKEN_LEN} characters");
        }
        info!("Using bridge token from {}", source);
        Ok(Self {
            value: raw.to_string(),
        })
    }

    /// Construct a token directly from a known value. Test-only: production
    /// callers must go through `resolve`, which guarantees a token exists.
    #[cfg(test)]
    pub fn from_value(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    /// Constant-time comparison against a presented credential.
    ///
    /// Length is compared first and unequal lengths short-circuit; that leaks
    /// only the length of the configured token, which is not secret. Equal
    /// lengths are compared without data-dependent branching.
    pub fn matches(&self, presented: &str) -> bool {
        let expected = self.value.as_bytes();
        let actual = presented.as_bytes();
        if expected.len() != actual.len() {
            return false;
        }
        expected.ct_eq(actual).into()
    }
}

fn generate_token() -> String {
    let mut bytes = [0u8; GENERATED_TOKEN_BYTES];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Where a generated token is written when the operator supplied none.
pub fn default_token_path(runtime_dir: Option<&Path>) -> PathBuf {
    let dir = runtime_dir
        .map(Path::to_path_buf)
        .or_else(|| std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    dir.join("sigil-bridge.token")
}

fn persist_token(path: &Path, value: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    std::fs::write(path, format!("{value}\n"))
        .with_context(|| format!("failed to write token to {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("failed to restrict permissions on {}", path.display()))?;
    }

    Ok(())
}

/// Extract a bearer credential from an `Authorization` header value.
fn bearer_credential(header_value: &str) -> Option<&str> {
    let (scheme, credential) = header_value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("Bearer") {
        return None;
    }
    let credential = credential.trim();
    if credential.is_empty() {
        None
    } else {
        Some(credential)
    }
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "type": "Error",
            "message": "missing or invalid bearer token"
        })),
    )
        .into_response()
}

/// Axum middleware rejecting any request without a valid bearer token.
///
/// Rejection happens before the handler runs, so an unauthenticated request
/// never reaches the daemon IPC socket.
pub async fn require_token(
    axum::extract::State(token): axum::extract::State<BridgeToken>,
    request: Request,
    next: Next,
) -> Response {
    let presented = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(bearer_credential);

    match presented {
        Some(credential) if token.matches(credential) => next.run(request).await,
        _ => {
            warn!("Rejected unauthenticated request to {}", request.uri());
            unauthorized()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_token_is_accepted() {
        let token = BridgeToken::from_value("correct-horse-battery-staple");
        assert!(token.matches("correct-horse-battery-staple"));
    }

    #[test]
    fn equal_length_mismatch_is_rejected() {
        let token = BridgeToken::from_value("correct-horse-battery-staple");
        // Same length, differs in one byte: exercises the constant-time path.
        assert!(!token.matches("correct-horse-battery-stapXe"));
    }

    #[test]
    fn different_length_is_rejected() {
        let token = BridgeToken::from_value("correct-horse-battery-staple");
        assert!(!token.matches("correct-horse"));
        assert!(!token.matches(""));
    }

    #[test]
    fn bearer_credential_is_parsed_case_insensitively() {
        assert_eq!(bearer_credential("Bearer abc"), Some("abc"));
        assert_eq!(bearer_credential("bearer abc"), Some("abc"));
        assert_eq!(bearer_credential("BEARER abc"), Some("abc"));
    }

    #[test]
    fn non_bearer_schemes_are_rejected() {
        assert_eq!(bearer_credential("Basic abc"), None);
        assert_eq!(bearer_credential("abc"), None);
        assert_eq!(bearer_credential("Bearer "), None);
        assert_eq!(bearer_credential("Bearer   "), None);
    }

    #[test]
    fn short_supplied_tokens_are_refused() {
        assert!(BridgeToken::from_supplied("short", "test").is_err());
        assert!(BridgeToken::from_supplied("", "test").is_err());
        assert!(BridgeToken::from_supplied("0123456789abcdef", "test").is_ok());
    }

    #[test]
    fn generated_tokens_are_unique_and_long() {
        let a = generate_token();
        let b = generate_token();
        assert_eq!(a.len(), GENERATED_TOKEN_BYTES * 2);
        assert_ne!(a, b);
    }

    #[test]
    fn token_file_is_read_and_trimmed() {
        let dir = std::env::temp_dir().join(format!("sigil-bridge-auth-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("token");
        std::fs::write(&path, "0123456789abcdef0123\n").unwrap();

        let token = BridgeToken::resolve(Some(&path), None).unwrap();
        assert!(token.matches("0123456789abcdef0123"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn generated_token_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("sigil-bridge-perm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sigil-bridge.token");
        persist_token(&path, "deadbeef").unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "token file must not be readable by others"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
