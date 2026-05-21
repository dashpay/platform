//! Query encoding settings.
//!
//! [`QuerySettings`] is a small, borrow-style bundle handed to
//! [`crate::platform::query::Query::query`] implementations so they can encode
//! a user-facing query into a wire `TransportRequest` without taking a full
//! `&Sdk` dependency. This keeps the encoder layer free of `Sdk`-shaped
//! transitive deps (transport, mock cache, nonce cache, context provider, …)
//! and lets unit tests construct settings directly without spinning up
//! `Sdk::new_mock()`.
//!
//! The fields are the minimum surface a wire encoder needs today:
//! protocol version (to pick V0 vs V1 wire shapes), the `prove` flag
//! (proof-mode requests vs unproved queries), and a borrowed
//! [`RequestSettings`] for any future encoder that needs to consult
//! transport-layer hints (timeouts, ban policy, …) — none do today, but
//! it costs nothing to thread through and avoids another trait churn
//! when the first encoder needs it.

use dpp::version::PlatformVersion;
use rs_dapi_client::RequestSettings;

/// Settings passed to [`crate::platform::query::Query::query`] for encoding a
/// user-facing query into a wire `TransportRequest`.
///
/// Construct via [`crate::Sdk::query_settings`] for normal use, or directly in
/// unit tests that want to exercise the encoder without an `Sdk`.
#[derive(Debug, Clone, Copy)]
pub struct QuerySettings<'a> {
    /// Transport-layer settings (timeouts, retries, TLS, ban behaviour).
    /// Not consulted by any current encoder; threaded for forward compatibility.
    pub request_settings: &'a RequestSettings,

    /// Platform protocol version, used to pick wire encoding (V0 vs V1, etc).
    pub protocol_version: &'a PlatformVersion,

    /// Whether to request and verify cryptographic proofs.
    pub prove: bool,
}

impl<'a> QuerySettings<'a> {
    /// Cheap derivative with proofs forced off — used by `FetchUnproved`.
    pub fn without_proofs(&self) -> Self {
        Self {
            prove: false,
            ..*self
        }
    }
}
