//! Dash Core SDK implementation.
//!
//! TODO: This is work in progress.
#[cfg(feature = "mocks")]
mod dash_core_client;
mod transaction;
#[cfg(feature = "mocks")]
pub use dash_core_client::LowLevelDashCoreClient;
mod error;
pub use error::DashCoreError;
