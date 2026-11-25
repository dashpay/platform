//! Mock error type, compiled without the `mocks` feature.
//!
//! `dash_sdk::Error::DapiMocksError` wraps it. Keeping the type, and so the
//! variant, in every build stops a consumer's exhaustive match from depending
//! on whether some other crate in the build turned `mocks` on.

#[derive(Debug, thiserror::Error, Clone)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
/// Mock errors
pub enum MockError {
    #[error("mock expectation not found for request: {0}")]
    /// Expectation not found
    MockExpectationNotFound(String),

    #[error("expectation already defined for request: {0}")]
    /// Expectation already defined for request
    MockExpectationConflict(String),
}
