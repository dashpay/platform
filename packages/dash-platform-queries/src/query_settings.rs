//! Query encoding settings.
//!
//! [`QuerySettings`] is a small, borrow-style bundle handed to the SDK's
//! `Query::query` implementations so they can encode a user-facing query into
//! a wire `TransportRequest` without taking a full `&Sdk` dependency. This
//! keeps the encoder layer free of `Sdk`-shaped transitive deps (transport,
//! mock cache, nonce cache, context provider, …) and lets unit tests
//! construct settings directly without spinning up `Sdk::new_mock()`.
//!
//! The fields are the minimum surface a wire encoder needs today: protocol
//! version (to pick V0 vs V1 wire shapes) and the `prove` flag (proof-mode
//! requests vs unproved queries).

use dpp::version::PlatformVersion;

/// Settings passed to the SDK's `Query::query` for encoding a user-facing
/// query into a wire `TransportRequest`.
///
/// Construct via `Sdk::query_settings` for normal use, or directly in unit
/// tests that want to exercise the encoder without an `Sdk`.
#[derive(Debug, Clone, Copy)]
pub struct QuerySettings<'a> {
    /// Platform protocol version, used to pick wire encoding (V0 vs V1, etc).
    pub protocol_version: &'a PlatformVersion,

    /// Whether to request and verify cryptographic proofs.
    pub prove: bool,
}

impl QuerySettings<'_> {
    /// Cheap derivative with proofs forced off — used by `FetchUnproved`.
    pub fn without_proofs(&self) -> Self {
        Self {
            prove: false,
            ..*self
        }
    }
}
