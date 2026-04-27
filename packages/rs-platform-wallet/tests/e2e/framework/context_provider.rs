//! SDK [`ContextProvider`] backed by the local SPV runtime.
//!
//! **NOTE: currently disabled in favor of
//! `rs_sdk_trusted_context_provider::TrustedHttpContextProvider`
//! — see `harness.rs` for the commented-out wiring. Re-enable
//! when SPV cold-start is stable (Task #15). The module remains
//! compilable so re-enablement is a single-block uncomment.**
//!
//! [`SpvContextProvider`] satisfies the synchronous `ContextProvider`
//! trait by bridging to [`SpvRuntime::get_quorum_public_key`]
//! (`async fn`) via [`dash_async::block_on`], which transparently
//! handles all three tokio runtime scenarios:
//!
//! - No active runtime: spins up a temporary current-thread runtime
//!   for the call.
//! - Current-thread runtime (the `tokio_shared_rt::test` default):
//!   spawns a dedicated OS thread with its own runtime so the call
//!   doesn't deadlock and `block_in_place` doesn't panic.
//! - Multi-thread runtime: uses the optimal `block_in_place + spawn`
//!   path via the workspace helper.
//!
//! As a result the e2e harness works on every runtime flavor —
//! tests can use `#[tokio_shared_rt::test(shared)]` directly — but
//! [`cases::transfer`](crate::cases::transfer) still spells out
//! `flavor = "multi_thread", worker_threads = 12` for parity with
//! `dash-evo-tool/tests/backend-e2e/` and to take the optimal
//! bridge path when the test is run live.
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

/// Platform activation height returned by
/// [`SpvContextProvider::get_platform_activation_height`].
///
/// **Hard-coded to `0` — intentional for the e2e framework's
/// testnet-only scope.** The SDK consumes this when verifying
/// proofs against historic core-chain-locked heights; on Dash
/// testnet the mn_rr (masternode reward reallocation) activation
/// height is well past any height the platform-address transfer
/// flow exercises, so the verification path that consumes this
/// value never compares against an unactivated quorum and
/// returning a conservative `0` is safe-by-position.
///
/// If a future test exercises activation-height-sensitive
/// verification (Core-feature flows, identity verification against
/// older quorums, mainnet runs), surface the real value via
/// [`SpvRuntime`] (the SPV client knows the activation height
/// after its first `QRInfo` round-trip) and wire it through here.
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
    /// Bridge SDK proof verification to the SPV's masternode-list
    /// state.
    ///
    /// Uses [`dash_async::block_on`] to call the async SPV API from
    /// the synchronous trait method. The helper picks the right
    /// strategy for whichever tokio runtime is in scope — see the
    /// module docs for the per-flavor breakdown.
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        // `dash_async::block_on` requires `Future: Send + 'static`,
        // so capture an owned `Arc<SpvRuntime>` clone and the small
        // `Copy` arguments by value. Outer `Result` carries a
        // bridge-level `AsyncError` (runtime panic, channel hangup,
        // …); inner `Result` carries the SPV's own quorum-lookup
        // error. Both fold into `InvalidQuorum` for the SDK.
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
        Ok(PLATFORM_ACTIVATION_HEIGHT_TESTNET_SAFE)
    }
}
