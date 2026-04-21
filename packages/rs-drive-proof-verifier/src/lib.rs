//! Proof verification library for Dash Drive
#![warn(missing_docs)]
#![allow(clippy::result_large_err)]

/// Errors that can occur during proof verification
pub mod error;
/// Implementation of proof verification
mod proof;
pub mod types;
mod verify;
pub use error::Error;
pub use proof::{FromProof, Length};

// Re-export context provider types from dash-context-provider
#[cfg(feature = "mocks")]
pub use dash_context_provider::MockContextProvider;
pub use dash_context_provider::{ContextProvider, ContextProviderError, DataContractProvider};

/// From Request
pub mod from_request;
/// Implementation of unproved verification
pub mod unproved;

/// Default query limit applied by Platform when proved paginated requests omit `count` or `limit`.
///
/// Proof verification must mirror this behavior; otherwise a valid proof for a default-bounded
/// server query is reconstructed locally as an unbounded query and looks truncated.
pub(crate) const DEFAULT_QUERY_LIMIT: u16 = 100;

/// Parse a proved request's optional `count`/`limit`, applying Platform's default when omitted.
pub(crate) fn proved_request_limit(limit: Option<u32>) -> Result<u16, Error> {
    limit.map_or(Ok(DEFAULT_QUERY_LIMIT), |limit| {
        u16::try_from(limit).map_err(|_| Error::RequestError {
            error: "query limit exceeds u16::MAX".to_string(),
        })
    })
}

// Needed for #[derive(PlatformSerialize, PlatformDeserialize)]
#[cfg(feature = "mocks")]
use dpp::serialization;

#[cfg(test)]
mod tests {
    use super::{proved_request_limit, DEFAULT_QUERY_LIMIT};

    #[test]
    fn proved_request_limit_defaults_when_omitted() {
        assert_eq!(proved_request_limit(None).unwrap(), DEFAULT_QUERY_LIMIT);
    }

    #[test]
    fn proved_request_limit_preserves_explicit_value() {
        assert_eq!(proved_request_limit(Some(7)).unwrap(), 7);
    }

    #[test]
    fn proved_request_limit_rejects_u32_overflow() {
        assert!(proved_request_limit(Some(u16::MAX as u32 + 1)).is_err());
    }
}
