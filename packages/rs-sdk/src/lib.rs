//! Dash SDK.

#[deny(missing_docs)]
pub mod platform;

pub mod error;

pub mod core;

pub mod crud;

pub mod sdk;

pub use crud::*;
pub use sdk::{DashPlatformSdk, Sdk};
