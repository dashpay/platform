//! SDK [`ContextProvider`] backed by the local SPV runtime.
//!
//! Currently unused: the harness wires
//! [`rs_sdk_trusted_context_provider::TrustedHttpContextProvider`]
//! instead. Kept compilable for re-enablement (Task #15).
//!
//! Bridges the synchronous `ContextProvider::get_quorum_public_key`
//! to the async SPV API via [`dash_async::block_on`], which handles
//! the no-runtime / current-thread / multi-thread flavors.
//! Data-contract and token-configuration lookups return `Ok(None)`
//! so the SDK falls back to a network fetch — quorum keys are the
//! only thing local SPV state can answer authoritatively.

use std::sync::Arc;

use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::DataContract;
use dpp::prelude::{CoreBlockHeight, Identifier};
use dpp::version::PlatformVersion;
use platform_wallet::SpvRuntime;

use dash_sdk::error::ContextProviderError;
use dash_sdk::platform::ContextProvider;

/// Platform activation height returned by
/// [`SpvContextProvider::get_platform_activation_height`].
///
/// Hard-coded to `0` for the testnet-only e2e scope: mn_rr
/// activation on testnet sits well past any height this flow
/// compares against, so a conservative `0` is safe-by-position.
/// Mainnet / activation-height-sensitive flows must surface the
/// real value via [`SpvRuntime`] after `QRInfo`.
const PLATFORM_ACTIVATION_HEIGHT_TESTNET_SAFE: CoreBlockHeight = 0;

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
    /// Bridge SDK proof verification to the SPV masternode-list state
    /// via [`dash_async::block_on`].
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        // `block_on` requires `Future: Send + 'static`; outer Result
        // is the bridge error, inner is the SPV's own — both fold
        // into `InvalidQuorum` for the SDK.
        let spv = Arc::clone(&self.spv_runtime);
        let inner = dash_async::block_on(async move {
            spv.get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
                .await
        })
        .map_err(|e| {
            ContextProviderError::InvalidQuorum(format!(
                "SPV quorum lookup bridge failed (type={quorum_type}, \
                 height={core_chain_locked_height}): {e}"
            ))
        })?;
        inner.map_err(|e| {
            ContextProviderError::InvalidQuorum(format!(
                "SPV quorum lookup failed (type={quorum_type}, \
                 height={core_chain_locked_height}): {e}"
            ))
        })
    }

    /// Defer to the SDK's network fetch (`None` == "not cached").
    fn get_data_contract(
        &self,
        _id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok(None)
    }

    /// Defer to the SDK's network fetch (see `get_data_contract`).
    fn get_token_configuration(
        &self,
        _id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(PLATFORM_ACTIVATION_HEIGHT_TESTNET_SAFE)
    }
}
