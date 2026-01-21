//! Authentication module for PIN-based security

mod lockout;
mod pin;
mod session;

pub use lockout::LockoutPolicy;
pub use pin::PinManager;
pub use session::Session;

use std::time::Instant;

/// Authentication state
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthState {
    /// PIN needs to be set up (first run)
    SetupRequired,
    /// PIN required for authentication
    RequiresPin,
    /// Successfully authenticated
    Authenticated,
    /// Account is locked out until the specified time
    LockedOut(Instant),
}

impl Default for AuthState {
    fn default() -> Self {
        AuthState::RequiresPin
    }
}
