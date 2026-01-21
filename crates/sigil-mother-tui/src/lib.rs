//! Sigil Mother TUI - Terminal User Interface
//!
//! A secure terminal interface for the air-gapped Sigil Mother device.
//! Provides functionality for:
//! - Master key management
//! - Child disk creation and management
//! - Agent registration and nullification
//! - Reconciliation workflows
//! - QR code display for data transfer

pub mod app;
pub mod ui;

pub use app::{App, AppResult};
