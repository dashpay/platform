//! Wallet functionality for the WASM SDK
//!
//! This module provides wallet-related operations including:
//! - Key generation and management
//! - Address derivation
//! - Message signing
//! - Key derivation paths (BIP44/DIP9)
//! - Extended key derivation (DIP14/DIP15)

pub mod key_derivation;
pub mod key_generation;
pub mod extended_derivation;
pub mod dip14;

pub use extended_derivation::*;
pub use key_derivation::*;
pub use key_generation::*;
