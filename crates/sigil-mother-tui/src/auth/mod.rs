//! Authentication module for PIN-based security
//!
//! This module re-exports authentication primitives from sigil-mother.
//! ALL authentication logic lives in sigil-mother to ensure that the
//! library itself is protected by PIN, not just the TUI.

// Re-export everything from sigil-mother's auth module
pub use sigil_mother::auth::{AuthError, AuthState, PinManager, Session, SessionConfig};
