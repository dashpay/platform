// lib.rs - rs-dapi library

pub mod clients;
pub mod config;
pub mod error;
pub mod errors;
pub mod protocol;
pub mod server;
pub mod services;

// Re-export main error types for convenience
pub use error::{DAPIResult, DapiError};
