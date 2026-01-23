//! Sigil Mother TUI Library
//!
//! This library provides the terminal user interface components for the
//! Sigil Mother air-gapped MPC guardian system.

// Many utility functions and components are prepared for future use
#![allow(dead_code)]

pub mod app;
pub mod auth;
pub mod operations;
pub mod reports;
pub mod ui;
pub mod utils;

pub use app::App;
