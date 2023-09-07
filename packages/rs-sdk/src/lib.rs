//! Dash SDK.

#[deny(missing_docs)]
pub mod platform;

pub mod error;

pub mod core;

pub mod sdk;

#[cfg(feature = "mocks")]
pub use sdk::MockDashPlatformSdk;
pub use sdk::{DashPlatformSdk, Sdk};
