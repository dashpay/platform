//! SPV-based Context Provider
//!
//! Thin quorum source that resolves Platform proof quorum public keys
//! from the SPV runtime owned by the [`PlatformWalletManager`].
//!
//! # Architecture
//!
//! [`SpvQuorumSource`] holds a shared [`Arc<SpvRuntime>`] — a live reference
//! to the same runtime the SPV client writes to during sync — and delegates
//! every lookup to [`SpvRuntime::get_quorum_public_key`], which reads the
//! in-memory masternode list engine. No quorum data is stored here.
//!
//! The [`ContextProvider`] trait method is synchronous, but the runtime lookup
//! is async. Proof verification runs inside the SDK's multi-threaded Tokio
//! runtime (`rs-sdk-ffi`'s `BigStackRuntime::block_on`), so the bridge uses
//! [`tokio::task::block_in_place`] (avoids the nested-runtime panic) plus the
//! ambient [`Handle::try_current`](tokio::runtime::Handle::try_current) of that
//! verify runtime. The source is constructed with the wallet manager, outside
//! any runtime, so the handle is resolved at call time (and a call from outside
//! a runtime returns an error rather than panicking).
//!
//! [`PlatformWalletManager`]: crate::manager::PlatformWalletManager
//! [`SpvRuntime::get_quorum_public_key`]: crate::spv::SpvRuntime::get_quorum_public_key

use std::future::Future;
use std::sync::Arc;

use dash_context_provider::ContextProvider;
use dash_context_provider::ContextProviderError;
use dashcore::sml::llmq_type::network::NetworkLLMQExt;
use dashcore::Network;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;
use tokio::runtime::RuntimeFlavor;

use crate::spv::SpvRuntime;

/// Hex-encode the first 8 bytes of a quorum hash for correlation in logs.
///
/// A prefix is enough to line a served key up with the proof that requested it
/// without dumping the full 32 bytes on every lookup.
fn hex_prefix(bytes: &[u8; 32]) -> String {
    use std::fmt::Write;
    bytes[..8].iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Context provider backed by an SPV client's synced masternode data.
///
/// Delegates quorum-key lookups to the shared [`SpvRuntime`]; the same runtime
/// the SPV client populates during sync is read live for each proof.
pub struct SpvQuorumSource {
    spv: Arc<SpvRuntime>,
    network: Network,
}

impl SpvQuorumSource {
    /// Create a new SPV quorum source.
    ///
    /// # Arguments
    ///
    /// * `spv` - Shared reference to the SPV runtime, obtained from
    ///   [`PlatformWalletManager::spv_arc`](crate::manager::PlatformWalletManager::spv_arc).
    /// * `network` - The Dash network (mainnet, testnet, devnet, etc.).
    pub fn new(spv: Arc<SpvRuntime>, network: Network) -> Self {
        Self { spv, network }
    }

    fn block_on_runtime<F, T>(&self, future: F) -> Result<T, ContextProviderError>
    where
        F: Future<Output = T>,
    {
        let handle = tokio::runtime::Handle::try_current().map_err(|_| {
            ContextProviderError::Generic(
                "SPV context provider called outside a Tokio runtime".to_string(),
            )
        })?;
        if handle.runtime_flavor() == RuntimeFlavor::CurrentThread {
            return Err(ContextProviderError::Generic(
                "SPV context provider requires a multi-threaded Tokio runtime".to_string(),
            ));
        }
        Ok(tokio::task::block_in_place(|| handle.block_on(future)))
    }

    /// Query dash-spv's existing progress snapshot without mirroring sync
    /// state in a parallel readiness flag.
    pub fn is_ready(&self) -> Result<bool, ContextProviderError> {
        self.block_on_runtime(self.spv.is_ready())
    }
}

impl ContextProvider for SpvQuorumSource {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        // Reject any quorum type other than this network's Platform quorum. The
        // proof's `quorum_type` is attacker-controlled and is folded into the
        // signature digest, so without this check a malicious DAPI endpoint plus
        // a compromised threshold of a lower-threshold quorum (e.g. LLMQ 50/60,
        // which needs only 30 signers, vs Platform's 100/67) could authenticate
        // forged Platform state. Pin the lookup to `network.platform_type()`.
        let platform_type = self.network.platform_type();
        if quorum_type != u32::from(u8::from(platform_type)) {
            return Err(ContextProviderError::InvalidQuorum(format!(
                "quorum type {quorum_type} is not the Platform quorum for {:?} \
                 (expected {platform_type:?})",
                self.network
            )));
        }

        // Bridge the sync trait method to the async runtime lookup. Proof
        // verification runs inside a Tokio runtime; a call from outside one
        // returns an error rather than panicking. The lookup is pure in-memory
        // (two brief RwLock reads, no network I/O); on write contention with SPV
        // sync it waits (fail-slow) rather than erroring.
        let result = self
            .block_on_runtime(self.spv.get_quorum_public_key(
                quorum_type,
                quorum_hash,
                core_chain_locked_height,
            ))?
            .map_err(|e| ContextProviderError::InvalidQuorum(e.to_string()));

        // The quorum public key is the trust root of every Platform proof this
        // SDK verifies; tracing which provider served it lets an operator
        // confirm proofs are validated against SPV-synced state rather than the
        // trusted HTTP fallback. Hash prefix only, to correlate without noise.
        match &result {
            Ok(_) => tracing::debug!(
                quorum_type,
                height = core_chain_locked_height,
                quorum_hash = %hex_prefix(&quorum_hash),
                "SPV context provider served quorum public key",
            ),
            Err(e) => tracing::debug!(
                quorum_type,
                height = core_chain_locked_height,
                quorum_hash = %hex_prefix(&quorum_hash),
                error = %e,
                "SPV context provider quorum lookup missed",
            ),
        }

        result
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
        // This provider is a quorum-only SPV source. A full SDK provider must
        // compose it with the local contract cache and embedded system
        // contracts; installing this source alone cannot resolve contracts.
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        // Token configuration is supplied by the same auxiliary local resolver
        // as data contracts, never by the SPV quorum source itself.
        Ok(None)
    }
}
