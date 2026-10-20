//! Mocking support.
//!
//! The `dash_platform_macros::Mockable` derive expands to an impl of
//! `crate::mock::Mockable`, so every crate that derives it must expose the
//! trait at this path. The trait itself lives in `dapi-grpc` and is defined
//! even when mocks are disabled — serialization then just returns `None`.
pub use dapi_grpc::mock::Mockable;
