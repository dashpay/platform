//! Protocol-version refresh for [`Sdk`].
//!
//! Houses [`Sdk::refresh_protocol_version`] and its private helper
//! [`extract_network_protocol_version`]. The shared
//! [`super::min_protocol_version`] / [`Sdk::maybe_update_protocol_version`]
//! helpers stay in the parent `sdk` module — this child module reaches them
//! through `super::` / `self`.

use super::Sdk;
use crate::error::Error;
use rs_dapi_client::{DapiRequestExecutor, IntoInner};
use std::sync::atomic::Ordering;

impl Sdk {
    /// Query the connected network for its current protocol version and ratchet
    /// this SDK's auto-detected protocol version up to it.
    ///
    /// ## Why this exists (bootstrap problem)
    ///
    /// An auto-detect SDK (one built without [`SdkBuilder::with_version()`]) is
    /// seeded with [`PlatformVersion::latest()`] (or a caller-supplied initial
    /// version) and only learns the network's *actual* protocol version after the
    /// first metadata-bearing platform response is parsed (see
    /// [`Self::verify_response_metadata`]). Fee-sensitive flows — shielded pool
    /// shield/unshield/transfer/withdraw — compute their reserve from
    /// `self.version()`, so an SDK that hasn't yet observed network metadata can
    /// under-reserve against a network running a newer protocol version. Calling
    /// this method on app start / network switch teaches the SDK the network
    /// version eagerly, before any such flow runs.
    ///
    /// ## How it works
    ///
    /// Issues an **unproved** `getStatus` request (no proof parsing), which keeps
    /// working even when proofed queries fail (e.g. UNIMPLEMENTED on stale
    /// evonodes) and is immune to proof-interpretation version skew. The
    /// network's current Drive protocol version is read from the response and fed
    /// into [`Self::maybe_update_protocol_version`], which applies the usual
    /// guards: pinned (non-auto-detect) SDKs are left untouched, version `0` and
    /// unknown versions are ignored, and the stored version only ever ratchets
    /// upward via `fetch_max`.
    ///
    /// ## Returns
    ///
    /// The SDK's protocol version number after the (possible) ratchet. A response
    /// that omits the protocol-version field is treated as a non-fatal no-op: a
    /// warning is logged and the current version number is returned unchanged.
    pub async fn refresh_protocol_version(&self) -> Result<u32, Error> {
        use dapi_grpc::platform::v0::{get_status_request, GetStatusRequest};

        let request = GetStatusRequest {
            version: Some(get_status_request::Version::V0(
                get_status_request::GetStatusRequestV0 {},
            )),
        };

        let response = self
            .execute(request, self.dapi_client_settings)
            .await
            .into_inner()?;

        match extract_network_protocol_version(&response) {
            Some(network_version) => {
                self.maybe_update_protocol_version(network_version);
            }
            None => {
                tracing::warn!(
                    target: "dash_sdk::protocol_version",
                    "getStatus response did not contain a Drive protocol version; \
                     keeping current protocol version"
                );
            }
        }

        // Refresh-time floor (clamp site 2 of 2; the other is `SdkBuilder::build`).
        // Independently of what the network reported — a too-low value the ratchet
        // ignored, an unknown/zero version, or a missing version block — the stored
        // version must never end up below the per-network minimum. `fetch_max` keeps
        // this monotonic and concurrency-safe alongside the auto-detect ratchet.
        self.protocol_version
            .fetch_max(super::min_protocol_version(self.network), Ordering::Relaxed);

        Ok(self.protocol_version_number())
    }
}

/// Extract the network's current Drive protocol version from a `getStatus`
/// response.
///
/// Walks `version → V0(v0) → v0.version → protocol → drive → current`, returning
/// `None` if any link in that chain is absent (e.g. a node that did not populate
/// the version block). Mirrors the field path used by
/// `drive_proof_verifier::types::evonode_status::Version::try_from`.
pub(super) fn extract_network_protocol_version(
    response: &dapi_grpc::platform::v0::GetStatusResponse,
) -> Option<u32> {
    use dapi_grpc::platform::v0::get_status_response;

    match &response.version {
        Some(get_status_response::Version::V0(v0)) => v0
            .version
            .as_ref()
            .and_then(|v| v.protocol)
            .and_then(|p| p.drive)
            .map(|d| d.current),
        None => None,
    }
}
