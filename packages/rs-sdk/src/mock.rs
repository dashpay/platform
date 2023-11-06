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
//! let mut sdk = rs_sdk::Sdk::new_mock();
//! let query = rs_sdk::platform::Identifier::random();
//! sdk.mock().expect_fetch(query, None as Option<rs_sdk::platform::Identity>);
//! ```
//!
//! See tests/mock_*.rs for more detailed examples.
#[cfg(feature = "mocks")]
pub mod config;
#[cfg(feature = "mocks")]
mod requests;
#[cfg(feature = "mocks")]
pub mod sdk;

#[cfg(not(feature = "mocks"))]
mod noop;

#[cfg(feature = "mocks")]
// TODO: move Mockable to some crate that can be shared between dapi-grpc, rs-dapi-client, and rs-sdk
pub use dapi_grpc::mock::Mockable;
#[cfg(feature = "mocks")]
pub use requests::*;
#[cfg(feature = "mocks")]
pub use sdk::MockDashPlatformSdk;

#[cfg(not(feature = "mocks"))]
pub use noop::*;
