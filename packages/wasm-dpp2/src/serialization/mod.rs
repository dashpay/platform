//! Serialization utilities for WASM bindings.
//!
//! This module contains:
//! - `conversions`: Format-aware conversion helpers between Rust/JS/JSON representations
//!
//! For bytes serde helpers (base64 in human-readable, raw bytes in binary),
//! use the canonical helpers from rs-dpp:
//! - `dpp::serialization::serde_bytes` — for `[u8; N]` (and `Option<[u8; N]>`
//!   via `dpp::serialization::serde_bytes::option`)
//! - `dpp::serialization::serde_bytes_var` — for `Vec<u8>`

pub mod conversions;

// Re-export commonly used items from conversions
pub use conversions::*;
