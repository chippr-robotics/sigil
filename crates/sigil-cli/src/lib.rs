//! Sigil CLI - Claude CLI tools for MPC signing
//!
//! This crate provides CLI tools that integrate with Claude Code for
//! blockchain transaction signing using MPC presignatures.

pub mod client;
pub mod commands;
pub mod tools;

pub use client::SigilClient;
pub use commands::*;
