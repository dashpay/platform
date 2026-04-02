//! SPV-based Context Provider
//!
//! Pure Rust implementation that reads quorum data directly from a
//! [`MasternodeListEngine`], with no FFI calls.
//!
//! # Architecture
//!
//! The [`SpvContextProvider`] holds an `Arc<RwLock<MasternodeListEngine>>`
//! (shared with the SPV client) and reads quorum public keys by looking up
//! the masternode list closest to the requested core chain-locked height.
//!
//! This design eliminates the need for FFI round-trips: the same in-memory
//! masternode list engine that the SPV client populates during sync is read
//! directly by the Platform SDK's proof verifier.
//!
//! # Usage
//!
//! ```ignore
//! use std::sync::Arc;
//! use tokio::sync::RwLock;
//! use dash_spv::MasternodeListEngine;
//! use dashcore::Network;
//! use platform_wallet::spv_context_provider::SpvContextProvider;
//!
//! let engine: Arc<RwLock<MasternodeListEngine>> = /* from DashSpvClient */;
//! let provider = SpvContextProvider::new(engine, Network::Testnet);
//! ```

use std::sync::Arc;

use dash_context_provider::ContextProvider;
use dash_context_provider::ContextProviderError;
use dash_spv::LLMQType;
use dash_spv::MasternodeListEngine;
use dashcore::hashes::Hash;
use dashcore::Network;
use dashcore::QuorumHash;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;
use tokio::sync::RwLock;

/// Context provider backed by an SPV client's synced masternode data.
///
/// Reads quorum public keys directly from the [`MasternodeListEngine`]
/// without any FFI calls. The engine is shared with the SPV client via
/// `Arc<RwLock<...>>`, so all data stays in-process.
pub struct SpvContextProvider {
    masternode_engine: Arc<RwLock<MasternodeListEngine>>,
    network: Network,
}

impl SpvContextProvider {
    /// Create a new SPV context provider.
    ///
    /// # Arguments
    ///
    /// * `masternode_engine` - Shared reference to the masternode list engine,
    ///   typically obtained from [`DashSpvClient::masternode_list_engine()`].
    /// * `network` - The Dash network (mainnet, testnet, devnet, etc.).
    pub fn new(masternode_engine: Arc<RwLock<MasternodeListEngine>>, network: Network) -> Self {
        Self {
            masternode_engine,
            network,
        }
    }
}

impl ContextProvider for SpvContextProvider {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        let quorum_type_u8 = u8::try_from(quorum_type).map_err(|_| {
            ContextProviderError::InvalidQuorum(format!(
                "Quorum type {} exceeds u8 range",
                quorum_type
            ))
        })?;
        let llmq_type: LLMQType = quorum_type_u8.into();
        let quorum_hash = QuorumHash::from_byte_array(quorum_hash);

        // Use try_read() instead of blocking_read() because this sync method
        // may be called from within a Tokio async context (proof verification
        // happens inside async tasks). blocking_read() would panic in that case.
        let engine = self.masternode_engine.try_read().map_err(|_| {
            ContextProviderError::Generic(
                "Masternode engine lock is busy; retry quorum lookup".to_string(),
            )
        })?;
        let (before, _after) = engine.masternode_lists_around_height(core_chain_locked_height);

        let ml = before.ok_or_else(|| {
            ContextProviderError::InvalidQuorum(format!(
                "No masternode list found at or before height {}",
                core_chain_locked_height
            ))
        })?;

        let list_height = ml.known_height;

        let quorums = ml.quorums.get(&llmq_type).ok_or_else(|| {
            ContextProviderError::InvalidQuorum(format!(
                "No quorums of type {} found at list height {} (requested {})",
                quorum_type, list_height, core_chain_locked_height
            ))
        })?;

        let quorum = quorums.get(&quorum_hash).ok_or_else(|| {
            ContextProviderError::InvalidQuorum(format!(
                "Quorum not found: type {} at list height {} (requested {}) \
                 with hash {:x} (masternode list has {} quorums of this type)",
                quorum_type,
                list_height,
                core_chain_locked_height,
                quorum_hash,
                quorums.len()
            ))
        })?;

        let pubkey_bytes: &[u8; 48] = quorum.quorum_entry.quorum_public_key.as_ref();
        Ok(*pubkey_bytes)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        match self.network {
            Network::Mainnet => Ok(1_888_888),
            Network::Testnet => Ok(1_289_520),
            Network::Devnet => Ok(1),
            _ => Err(ContextProviderError::Generic(format!(
                "Platform activation height unknown for network {:?}",
                self.network
            ))),
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
