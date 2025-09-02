// lib.rs - rs-dapi library

pub mod cache;
pub mod clients;
pub mod config;
pub mod error;
pub mod logging;
pub mod metrics;
pub mod protocol;
pub mod server;
pub mod services;

// Re-export main error types for convenience
pub use error::{DAPIResult, DapiError};
