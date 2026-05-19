//! Compile-safe JSON serialization for large integers (u64/i64).
//!
//! JavaScript's `Number.MAX_SAFE_INTEGER` is `2^53 - 1`. Values above this lose
//! precision when parsed by JS clients. This module provides serde `with` helpers
//! that stringify large values in JSON while keeping native integers in non-JSON
//! formats (platform_value, bincode) via `serializer.is_human_readable()`.
//!
//! ## Modules
//!
//! - [`safe_fields`] — `JsonSafeFields` marker trait for compile-time enforcement
//! - [`safe_integer`] — `serde(with)` modules for bare u64/i64 and Option variants
//! - [`safe_integer_map`] — `serde(with)` modules for BTreeMap fields containing u64
//!
//! ## Usage
//!
//! These modules are used automatically by the `#[json_safe_fields]` attribute macro
//! from `dpp-json-convertible-derive`. See that crate's README for the full system.

pub mod safe_fields;
pub mod safe_integer;
pub mod safe_integer_map;

pub use safe_fields::JsonSafeFields;
