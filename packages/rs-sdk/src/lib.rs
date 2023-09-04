//! Dash SDK.

#[deny(missing_docs)]
pub mod platform;

pub mod error;

pub mod core;

pub mod crud;

pub mod dapi;

pub use crud::*;
pub use dapi::{Api, DashAPI};
