//! SPV-based Context Provider
//!
//! Thin [`ContextProvider`] that resolves Platform proof quorum public keys
//! from the SPV runtime owned by the [`PlatformWalletManager`].
//!
//! # Architecture
//!
//! [`SpvContextProvider`] holds a shared [`Arc<SpvRuntime>`] — a live reference
//! to the same runtime the SPV client writes to during sync — and delegates
//! every lookup to [`SpvRuntime::get_quorum_public_key`], which reads the
//! in-memory masternode list engine. No quorum data is stored here.
//!
//! The [`ContextProvider`] trait method is synchronous, but the runtime lookup
//! is async. Proof verification runs inside the SDK's multi-threaded Tokio
//! runtime (`rs-sdk-ffi`'s `BigStackRuntime::block_on`), so the bridge uses
//! [`tokio::task::block_in_place`] (avoids the nested-runtime panic) plus the
//! ambient [`Handle::try_current`](tokio::runtime::Handle::try_current) of that
//! verify runtime. The provider is constructed at the FFI SDK-create call,
//! which runs off any runtime, so the handle is resolved at call time (and a
//! call from outside a runtime returns an error rather than panicking).
//!
//! [`PlatformWalletManager`]: crate::manager::PlatformWalletManager
//! [`SpvRuntime::get_quorum_public_key`]: crate::spv::SpvRuntime::get_quorum_public_key

use std::sync::Arc;

use dash_context_provider::ContextProvider;
use dash_context_provider::ContextProviderError;
use dashcore::Network;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;

use crate::spv::SpvRuntime;

/// Context provider backed by an SPV client's synced masternode data.
///
/// Delegates quorum-key lookups to the shared [`SpvRuntime`]; the same runtime
/// the SPV client populates during sync is read live for each proof.
pub struct SpvContextProvider {
    spv: Arc<SpvRuntime>,
    network: Network,
}

impl SpvContextProvider {
    /// Create a new SPV context provider.
    ///
    /// # Arguments
    ///
    /// * `spv` - Shared reference to the SPV runtime, obtained from
    ///   [`PlatformWalletManager::spv_arc`](crate::manager::PlatformWalletManager::spv_arc).
    /// * `network` - The Dash network (mainnet, testnet, devnet, etc.).
    pub fn new(spv: Arc<SpvRuntime>, network: Network) -> Self {
        Self { spv, network }
    }
}

impl ContextProvider for SpvContextProvider {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        // Bridge the sync trait method to the async runtime lookup. Proof
        // verification always runs inside the SDK's multi-threaded runtime, so
        // the ambient handle is present; a call from outside a runtime returns
        // an error rather than panicking. The lookup is pure in-memory (two
        // brief RwLock reads, no network I/O); on write contention with SPV
        // sync it waits (fail-slow) rather than erroring.
        let handle = tokio::runtime::Handle::try_current().map_err(|_| {
            ContextProviderError::Generic(
                "SPV quorum lookup called outside a Tokio runtime".to_string(),
            )
        })?;
        tokio::task::block_in_place(|| {
            handle.block_on(self.spv.get_quorum_public_key(
                quorum_type,
                quorum_hash,
                core_chain_locked_height,
            ))
        })
        .map_err(|e| ContextProviderError::InvalidQuorum(e.to_string()))
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        // Match the values the trusted HTTP provider ships (the L1 locked
        // height per network) so proof verification behaves identically
        // whether quorum keys come from SPV or the trusted service. See
        // `rs-sdk-trusted-context-provider`'s `get_platform_activation_height`.
        match self.network {
            Network::Mainnet => Ok(2_132_092),
            Network::Testnet => Ok(1_090_319),
            Network::Devnet | Network::Regtest => Ok(1),
        }
    }

    fn get_data_contract(
        &self,
        _data_contract_id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        // Data contract lookup is handled by the SDK's contract cache,
        // not the SPV layer.
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        // Token configuration lookup is handled by the SDK's contract cache,
        // not the SPV layer.
        Ok(None)
    }
}
