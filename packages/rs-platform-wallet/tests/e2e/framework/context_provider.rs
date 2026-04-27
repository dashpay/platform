//! SDK [`ContextProvider`] backed by the local SPV runtime.
//!
//! [`SpvContextProvider`] satisfies the synchronous `ContextProvider`
//! trait by bridging to [`SpvRuntime::get_quorum_public_key`]
//! (`async fn`) via [`tokio::task::block_in_place`] +
//! [`tokio::runtime::Handle::block_on`]. The harness therefore MUST
//! run on a multi-threaded tokio runtime — the
//! `#[tokio_shared_rt::test(shared)]` attribute used by the e2e test
//! cases provides that by default.
//!
//! Calling [`SpvContextProvider::get_quorum_public_key`] from a
//! single-threaded runtime panics inside `block_in_place`. If the
//! suite ever needs single-threaded execution, replace this provider
//! with a channel-based bridge (push the request onto a sync channel
//! polled by an async helper task).
//!
//! Data-contract and token-configuration lookups deliberately return
//! `Ok(None)` — the SDK falls back to a network fetch. We surface
//! quorum keys (the only lookup proof verification truly needs from
//! the wallet's local SPV state) and let the SDK handle the rest.

use std::sync::Arc;

use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::DataContract;
use dpp::prelude::{CoreBlockHeight, Identifier};
use dpp::version::PlatformVersion;
use platform_wallet::SpvRuntime;

use dash_sdk::error::ContextProviderError;
use dash_sdk::platform::ContextProvider;

/// Placeholder activation height returned by
/// [`SpvContextProvider::get_platform_activation_height`] until we
/// surface the real value from the SPV's mn-list state.
///
/// The SDK consumes this when verifying proofs against historic core
/// chain locked heights; on testnet the mn_rr (masternode reward
/// reallocation) activation height is well past the heights we care
/// about for the platform-address transfer flow, so a conservative
/// `0` is correct enough to unblock that test path.
//
// TODO(Wave5): pull from SPV mn-list once we surface that info — the
// SPV client knows the activation height after its first QRInfo
// round-trip, but `SpvRuntime` doesn't expose an accessor today.
const PLACEHOLDER_ACTIVATION_HEIGHT: CoreBlockHeight = 0;

/// SDK [`ContextProvider`] that resolves quorum public keys from the
/// local SPV runtime.
#[derive(Debug, Clone)]
pub struct SpvContextProvider {
    spv_runtime: Arc<SpvRuntime>,
}

impl SpvContextProvider {
    /// Wrap an [`Arc<SpvRuntime>`] in a fresh provider.
    pub fn new(spv_runtime: Arc<SpvRuntime>) -> Self {
        Self { spv_runtime }
    }

    /// Borrow the underlying SPV runtime.
    pub fn spv(&self) -> &Arc<SpvRuntime> {
        &self.spv_runtime
    }
}

impl ContextProvider for SpvContextProvider {
    /// Bridge SDK proof verification to the SPV's masternode-list
    /// state.
    ///
    /// Uses `block_in_place` + `Handle::block_on` to call the async
    /// SPV API from the synchronous trait method. **Multi-threaded
    /// tokio runtime required** — see the module docs.
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        let spv = Arc::clone(&self.spv_runtime);
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                spv.get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
                    .await
            })
        });
        result.map_err(|e| {
            ContextProviderError::InvalidQuorum(format!(
                "SPV quorum lookup failed (type={quorum_type}, height={core_chain_locked_height}): {e}"
            ))
        })
    }

    /// Defer to the SDK's network fetch path. Returning `None` is
    /// the documented "I don't have it cached, please fetch it"
    /// signal in the `ContextProvider` contract.
    fn get_data_contract(
        &self,
        _id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok(None)
    }

    /// Defer to the SDK's network fetch path (see `get_data_contract`).
    fn get_token_configuration(
        &self,
        _id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(PLACEHOLDER_ACTIVATION_HEIGHT)
    }
}
