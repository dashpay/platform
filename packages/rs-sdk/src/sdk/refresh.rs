//! Protocol-version refresh for [`Sdk`].
//!
//! Houses [`Sdk::refresh_protocol_version`], a thin eager wrapper around the
//! SDK's ordinary proven-query machinery. The shared
//! [`Sdk::maybe_update_protocol_version`] ratchet stays in the parent `sdk`
//! module — this child module reaches it through `self`, and clamps to the
//! [`super::DEFAULT_INITIAL_PROTOCOL_VERSION`] floor.

use super::Sdk;
use crate::platform::fetch_current_no_parameters::FetchCurrent;
use crate::Error;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use std::sync::atomic::Ordering;

impl Sdk {
    /// Eagerly teach this SDK the network's current protocol version and ratchet
    /// up to it.
    ///
    /// ## Why this exists (bootstrap problem)
    ///
    /// An auto-detect SDK (one built without [`SdkBuilder::with_version()`]) is
    /// seeded at [`DEFAULT_INITIAL_PROTOCOL_VERSION`] (or a caller-supplied initial
    /// version) and only learns the network's *actual* protocol version after the
    /// first metadata-bearing platform response is parsed (see
    /// [`Self::verify_response_metadata`]). Fee-sensitive flows — shielded pool
    /// shield/unshield/transfer/withdraw — compute their reserve from
    /// `self.version()`, so an SDK that hasn't yet observed network metadata can
    /// under-reserve against a network running a newer protocol version. Calling
    /// this method on app start / network switch closes that window before any such
    /// flow runs.
    ///
    /// ## How it works — one trust path, not two
    ///
    /// This issues an ordinary **proven** `getEpochsInfo` query
    /// ([`ExtendedEpochInfo::fetch_current`]) and discards the epoch payload. The
    /// protocol version that query carries in its response metadata is ratcheted
    /// into this SDK by the *same* [`Self::maybe_update_protocol_version`] path
    /// every other query uses, and **only after** proof + quorum-signature
    /// verification succeeds (the version is bound to the Tenderdash
    /// `StateId.app_version`; see the security invariant in
    /// [`Self::parse_proof_with_metadata_and_proof`]). So refresh inherits exactly
    /// the same cryptographic trust as ordinary traffic — it adds **no** second,
    /// weaker source of truth, it merely runs one proven query eagerly instead of
    /// waiting for the next one.
    ///
    /// If the proven query fails (e.g. no [`ContextProvider`] is set, a transport
    /// error, or `UNIMPLEMENTED` on a stale evonode) the failure is **non-fatal**:
    /// we deliberately do *not* fall back to an unverified version. The stored
    /// version is left untouched and then clamped to
    /// [`DEFAULT_INITIAL_PROTOCOL_VERSION`], so it can never sit below that floor
    /// even when the refresh round-trip fails.
    ///
    /// ## Pinned SDKs (version updating disabled)
    ///
    /// An SDK pinned via [`SdkBuilder::with_version()`] has explicitly opted out
    /// of version tracking, so there is nothing to refresh. This method
    /// short-circuits for a pinned SDK: it issues **no** network request and
    /// returns the pinned version unchanged. (Construction already raised any
    /// sub-floor pin up to [`DEFAULT_INITIAL_PROTOCOL_VERSION`], so the floor clamp
    /// would be a no-op anyway.)
    ///
    /// For an auto-detect SDK the usual ratchet guards still apply: version `0`
    /// and unknown/future versions are ignored, and the stored version only ever
    /// ratchets upward via `fetch_max`.
    ///
    /// ## Returns
    ///
    /// The SDK's protocol version number after the (possible) ratchet and the
    /// [`DEFAULT_INITIAL_PROTOCOL_VERSION`] floor clamp.
    ///
    /// [`SdkBuilder::with_version()`]: super::SdkBuilder::with_version
    /// [`ContextProvider`]: crate::platform::ContextProvider
    /// [`DEFAULT_INITIAL_PROTOCOL_VERSION`]: super::DEFAULT_INITIAL_PROTOCOL_VERSION
    pub async fn refresh_protocol_version(&self) -> Result<u32, Error> {
        // A pinned SDK (built via `SdkBuilder::with_version`) has opted out of
        // version tracking: `maybe_update_protocol_version` is a no-op for it, so
        // the proven query below could never change anything. Skip the round-trip
        // and return the pinned version. (Construction already raised any sub-floor
        // pin up to the cross-network floor, so there is nothing left to clamp.)
        if !self.auto_detect_protocol_version {
            return Ok(self.protocol_version_number());
        }

        // A proven query whose response metadata flows through the verified
        // `maybe_update_protocol_version` ratchet (see this method's docs). We only
        // care about the side effect on the protocol version, not the epoch payload.
        if let Err(error) = ExtendedEpochInfo::fetch_current(self).await {
            tracing::warn!(
                target: "dash_sdk::protocol_version",
                %error,
                "proven protocol-version refresh failed; keeping current version \
                 (never falling back to an unverified one)"
            );
        }

        // Refresh-time floor (clamp site 2 of 2; the other is `SdkBuilder::build`).
        // Independently of whether the proven query ran or ratcheted the version,
        // the stored version must never end up below the cross-network
        // `DEFAULT_INITIAL_PROTOCOL_VERSION`. `fetch_max` keeps this monotonic and
        // concurrency-safe alongside the auto-detect ratchet.
        self.protocol_version
            .fetch_max(super::DEFAULT_INITIAL_PROTOCOL_VERSION, Ordering::Relaxed);

        Ok(self.protocol_version_number())
    }
}
