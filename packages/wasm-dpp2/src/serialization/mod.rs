//! Serialization utilities for WASM bindings.
//!
//! This module contains:
//! - `bytes_b64`: Serde helpers for bytes that serialize as Base64 in human-readable formats
//! - `conversions`: Format-aware conversion helpers between Rust/JS/JSON representations

pub mod bytes_b64;
pub mod conversions;

// Re-export commonly used items from conversions
pub use conversions::*;
