// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! `dash_context_provider::ContextProvider` backed by node-local state.
//!
//! `dash-sdk` resolves everything it needs about the network through this
//! trait: the BLS public key of the quorum that signed a proof, and the data
//! contracts referenced by document queries. The embedder serves both from
//! local knowledge: quorum keys are pushed across the bridge from synced
//! LLMQ data, and the supported document queries use the pinned DPNS and
//! DashPay system contracts compiled into dpp. Nothing here fetches from a
//! trusted HTTP service.

use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use dash_context_provider::{ContextProvider, ContextProviderError};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use platform_version::version::PlatformVersion;

/// A quorum public key pushed from the node's LLMQ store. `quorum_hash` is
/// in the byte order DAPI proofs carry it (display order: the hex `quorum
/// info` prints, the reverse of the embedding application's internal
/// uint256 byte order).
pub struct QuorumKey {
    pub quorum_hash: [u8; 32],
    pub public_key: [u8; 48],
}

/// The network context the embedder installs before anything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub network: Network,
    /// The LLMQ type Platform quorums use on this network. A proof naming any
    /// other quorum type is rejected before its key is looked up, so the set
    /// of keys the embedder pushes for other purposes can never sign
    /// Platform state.
    pub platform_quorum_type: u32,
    /// Tenderdash chain id of the network. It enters the quorum signature
    /// preimage, so a verified response carrying another id is a signed
    /// response from another chain.
    pub tenderdash_chain_id: String,
    /// Lowest protocol version this network can be running; the SDK ratchets
    /// upward from it as verified responses report newer versions.
    pub protocol_version: u32,
    /// Core height at which Platform activated (mn_rr). Embedders that do not
    /// use queries requiring this value may set it to 0.
    pub platform_activation_height: CoreBlockHeight,
}

#[derive(Default)]
struct State {
    context: Option<Context>,
    /// quorum_hash (proof byte order) -> BLS public key, for the Platform
    /// quorum type only.
    quorum_keys: HashMap<[u8; 32], [u8; 48]>,
    /// Lazily loaded pinned system contracts (DPNS, DashPay), keyed by id and
    /// the protocol version they were loaded for.
    contracts: HashMap<(Identifier, u32), Arc<DataContract>>,
}

/// Node-local context shared between the SDK (through `ContextProvider`)
/// and the embedder, which updates it as its chain state advances.
#[derive(Default)]
pub struct LocalContextProvider {
    state: RwLock<State>,
}

// A poisoned lock means a bridge call panicked while holding it. The state is
// plain data (no invariants span a write), so keep serving it rather than
// turning every later verification into an error.
fn read(state: &RwLock<State>) -> RwLockReadGuard<'_, State> {
    state.read().unwrap_or_else(PoisonError::into_inner)
}

fn write(state: &RwLock<State>) -> RwLockWriteGuard<'_, State> {
    state.write().unwrap_or_else(PoisonError::into_inner)
}

/// Parses a Dash network id ("main", "test", "regtest", "devnet") into
/// the dashcore `Network` the SDK expects.
pub fn parse_network(network_id: &str) -> Result<Network, String> {
    match network_id {
        "main" => Ok(Network::Mainnet),
        "test" => Ok(Network::Testnet),
        "regtest" => Ok(Network::Regtest),
        "devnet" => Ok(Network::Devnet),
        other => Err(format!("unknown network id {other:?}")),
    }
}

impl LocalContextProvider {
    /// Installs the network context. Replaces any previous context and drops
    /// the stored quorum keys, which belonged to it.
    pub fn set_context(&self, context: Context) -> Result<(), String> {
        PlatformVersion::get(context.protocol_version).map_err(|e| {
            format!(
                "protocol version {} is unknown to this build: {e}",
                context.protocol_version
            )
        })?;
        let mut state = write(&self.state);
        state.context = Some(context);
        state.quorum_keys.clear();
        Ok(())
    }

    /// The context set via [`Self::set_context`].
    pub fn context(&self) -> Result<Context, String> {
        read(&self.state)
            .context
            .clone()
            .ok_or_else(|| "platform bridge context not initialized (set_context)".to_string())
    }

    /// Replaces the stored Platform quorum keys with `keys`. The embedder
    /// pushes the full active Platform-LLMQ set on every masternode-list /
    /// quorum update, so replacement (not merge) keeps rotated-out quorums
    /// from verifying new proofs forever.
    pub fn update_quorum_keys(&self, keys: Vec<QuorumKey>) -> Result<(), String> {
        let mut state = write(&self.state);
        if state.context.is_none() {
            return Err("platform bridge context not initialized (set_context)".to_string());
        }
        state.quorum_keys = keys
            .into_iter()
            .map(|key| (key.quorum_hash, key.public_key))
            .collect();
        Ok(())
    }
}

impl ContextProvider for LocalContextProvider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        let cache_key = (*id, platform_version.protocol_version);
        if let Some(contract) = read(&self.state).contracts.get(&cache_key) {
            return Ok(Some(Arc::clone(contract)));
        }
        for system_contract in [SystemDataContract::DPNS, SystemDataContract::Dashpay] {
            let contract = load_system_data_contract(system_contract, platform_version)
                .map_err(|e| ContextProviderError::DataContractFailure(e.to_string()))?;
            if contract.id() == *id {
                let contract = Arc::new(contract);
                write(&self.state)
                    .contracts
                    .insert(cache_key, Arc::clone(&contract));
                return Ok(Some(contract));
            }
        }
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Err(ContextProviderError::Generic(
            "token configurations are not available through this binding".to_string(),
        ))
    }

    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        let state = read(&self.state);
        let context = state
            .context
            .as_ref()
            .ok_or_else(|| ContextProviderError::Config("set_context not called".to_string()))?;
        // The response names the quorum type; only the network's Platform
        // type may sign Platform state.
        if quorum_type != context.platform_quorum_type {
            return Err(ContextProviderError::InvalidQuorum(format!(
                "proof signed by quorum type {quorum_type}; Platform quorums on this network are \
                 type {}",
                context.platform_quorum_type
            )));
        }
        // The locally synced LLMQ store only tracks currently valid quorums,
        // so the requested core height adds nothing to the lookup: a proof
        // signed by a quorum the node no longer knows fails verification.
        state.quorum_keys.get(&quorum_hash).copied().ok_or_else(|| {
            ContextProviderError::InvalidQuorum(format!(
                "no locally known Platform quorum with hash {}",
                hex::encode(quorum_hash)
            ))
        })
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        read(&self.state)
            .context
            .as_ref()
            .map(|context| context.platform_activation_height)
            .ok_or_else(|| ContextProviderError::Config("set_context not called".to_string()))
    }
}
