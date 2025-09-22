// TODO: remove and fix all warnings
#![allow(unused_attributes)]
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
pub mod sync;

// Re-export main error types for convenience
pub use error::{DAPIResult, DapiError};
