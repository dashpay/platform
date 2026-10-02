//! SDK [`ContextProvider`]s backed by the local SPV runtime.
//!
//! Two providers live here:
//!
//! - [`SpvContextProvider`] — resolves quorum public keys from the SPV
//!   masternode list. Data-contract and token-configuration lookups
//!   return `Ok(None)` so the SDK falls back to a network fetch.
//! - [`CompositeContextProvider`] — routes quorum lookups to
//!   [`SpvContextProvider`] but serves data-contract / token-config
//!   lookups (and the platform-activation height) from a
//!   [`TrustedHttpContextProvider`]'s known-contracts cache, so the
//!   harness keeps its `add_known_contract` / `add_known_token_configuration`
//!   surface (QA-900) while needing no hosted quorums HTTP host. This is
//!   the SPV-mode replacement for the trusted provider on devnets like
//!   porter that publish no quorums endpoint (QA-001).
//!
//! Both bridge the synchronous `ContextProvider::get_quorum_public_key`
//! to the async SPV API via [`dash_async::block_on`], which handles
//! the no-runtime / current-thread / multi-thread flavors.

use std::sync::Arc;

use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::DataContract;
use dpp::prelude::{CoreBlockHeight, Identifier};
use dpp::version::PlatformVersion;
use platform_wallet::SpvRuntime;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;

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

/// SPV quorum lookups + the trusted provider's contract cache.
///
/// Quorum public keys come from [`SpvContextProvider`] (no hosted
/// quorums service needed); data contracts, token configurations, and
/// the platform-activation height come from a shared
/// [`TrustedHttpContextProvider`] whose `get_data_contract` /
/// `get_token_configuration` paths are cache-only (they never hit the
/// network — only `get_quorum_public_key` would, and we never call the
/// trusted provider's). The harness keeps handing tests the
/// `Arc<TrustedHttpContextProvider>` for `add_known_contract` /
/// `add_known_token_configuration`; mutations through that handle are
/// visible here because the inner caches are `Arc<Mutex<…>>`. (QA-900)
#[derive(Clone)]
pub struct CompositeContextProvider {
    quorums: SpvContextProvider,
    contracts: Arc<TrustedHttpContextProvider>,
}

impl CompositeContextProvider {
    /// Build a composite from a live SPV runtime and the shared trusted
    /// provider used as the known-contracts cache.
    pub fn new(spv_runtime: Arc<SpvRuntime>, contracts: Arc<TrustedHttpContextProvider>) -> Self {
        Self {
            quorums: SpvContextProvider::new(spv_runtime),
            contracts,
        }
    }
}

impl ContextProvider for CompositeContextProvider {
    /// SPV-backed (see [`SpvContextProvider::get_quorum_public_key`]).
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        self.quorums
            .get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
    }

    /// Served from the trusted provider's known-contracts cache (no
    /// network — its contract path is cache + system-contract only).
    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        self.contracts.get_data_contract(id, platform_version)
    }

    /// Served from the trusted provider's known-token-config cache.
    fn get_token_configuration(
        &self,
        id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        self.contracts.get_token_configuration(id)
    }

    /// Delegated to the trusted provider's per-network activation height.
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        self.contracts.get_platform_activation_height()
    }
}
