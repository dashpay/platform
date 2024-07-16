//! Mocking support for Dash SDK.
//!
//! This module provides a way to mock SDK operations. It is used in tests and examples.
//!
//! In order to mock SDK operations, you need to create a mock SDK instance using
//! [Sdk::new_mock()](crate::Sdk::new_mock()).
//! Next step is to create mock query expectations on [MockDashPlatformSdk] object returned by
//! [Sdk::mock()](crate::Sdk::mock()), using [MockDashPlatformSdk::expect_fetch()]
//! and [MockDashPlatformSdk::expect_fetch_many()].
//!
//!
//! ## Example
//!
//! ```no_run
//! let mut sdk = dash_sdk::Sdk::new_mock();
//! let query = dash_sdk::platform::Identifier::random();
//! sdk.mock().expect_fetch(query, None as Option<dash_sdk::platform::Identity>);
//! ```
//!
//! See tests/mock_*.rs for more detailed examples.

#[cfg(not(feature = "mocks"))]
mod noop;
#[cfg(feature = "mocks")]
pub mod provider;
#[cfg(feature = "mocks")]
mod requests;
#[cfg(feature = "mocks")]
pub mod sdk;

// Mockable reexport is needed even if mocks feature is disabled - it just does nothing.
// Otherwise  dapi_grpc_macros::Mockable fails.
// TODO: move Mockable to some crate that can be shared between dapi-grpc, rs-dapi-client, and dash-sdk
pub use dapi_grpc::mock::Mockable;

// MockResponse is needed even if mocks feature is disabled - it just does nothing.
#[cfg(not(feature = "mocks"))]
pub use noop::MockResponse;
#[cfg(feature = "mocks")]
pub use requests::MockResponse;
#[cfg(feature = "mocks")]
pub use sdk::MockDashPlatformSdk;
