// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! `dash_context_provider::ContextProvider` backed by node-local state.
//!
//! `FromProof` verification (drive-proof-verifier) resolves everything it
//! needs about the network through this trait: the BLS public key of the
//! quorum that signed a proof, and the data contracts referenced by document
//! queries. The embedder serves both from local knowledge: quorum keys are
//! pushed across the bridge from synced LLMQ data, and the supported document
//! queries use the pinned DPNS and DashPay system contracts compiled into dpp.
//! Proof verification therefore never performs network fetches.
//!
//! The provider is process-global and supports one Platform network at a time.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use dash_context_provider::{ContextProvider, ContextProviderError};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use platform_version::version::PlatformVersion;

/// A quorum public key pushed from the node's LLMQ store. `quorum_hash` is
/// in the byte order DAPI proofs carry it (display order — the reverse of
/// the embedding application's internal hash order; the C++ adapter converts).
pub struct QuorumKey {
    pub quorum_hash: [u8; 32],
    pub public_key: [u8; 48],
}

#[derive(Default)]
struct State {
    /// (quorum_type, quorum_hash in proof byte order) -> BLS public key.
    quorum_keys: HashMap<(u32, [u8; 32]), [u8; 48]>,
    network: Option<Network>,
    /// Core height at which Platform activated (mn_rr). Embedders that do not
    /// use queries requiring this value may set it to 0.
    platform_activation_height: CoreBlockHeight,
    /// Lazily loaded pinned system contracts (DPNS, DashPay).
    contracts: HashMap<Identifier, Arc<DataContract>>,
}

/// Process-global provider instance.
pub struct LocalContextProvider {
    state: RwLock<State>,
}

static PROVIDER: OnceLock<LocalContextProvider> = OnceLock::new();

/// The process-global provider handed to every FromProof call.
pub fn provider() -> &'static LocalContextProvider {
    PROVIDER.get_or_init(|| LocalContextProvider {
        state: RwLock::new(State::default()),
    })
}

/// Replaces the stored key set of `quorum_type` with `keys`. The C++ client
/// pushes the full active Platform-LLMQ set on every masternode-list /
/// quorum update, so replacement (not merge) keeps rotated-out quorums from
/// verifying new proofs forever.
pub fn update_quorum_keys(quorum_type: u8, keys: Vec<QuorumKey>) {
    let mut state = provider().state.write().expect("provider lock poisoned");
    state
        .quorum_keys
        .retain(|(stored_type, _), _| *stored_type != u32::from(quorum_type));
    for key in keys {
        state
            .quorum_keys
            .insert((u32::from(quorum_type), key.quorum_hash), key.public_key);
    }
}

/// Parses a Dash network id ("main", "test", "regtest", "devnet") into
/// the dashcore `Network` FromProof expects.
pub fn parse_network(network_id: &str) -> Result<Network, String> {
    match network_id {
        "main" => Ok(Network::Mainnet),
        "test" => Ok(Network::Testnet),
        "regtest" => Ok(Network::Regtest),
        "devnet" => Ok(Network::Devnet),
        other => Err(format!("unknown network id {other:?}")),
    }
}

/// Sets the network and the Platform activation core height (0 = unknown;
/// see `State::platform_activation_height`).
pub fn set_context(network_id: &str, platform_activation_height: u32) -> Result<(), String> {
    let network = parse_network(network_id)?;
    let mut state = provider().state.write().expect("provider lock poisoned");
    state.network = Some(network);
    state.platform_activation_height = platform_activation_height;
    Ok(())
}

/// The network set via `set_context`.
pub fn network() -> Result<Network, String> {
    provider()
        .state
        .read()
        .expect("provider lock poisoned")
        .network
        .ok_or_else(|| "platform bridge context not initialized (set_context)".to_string())
}

impl ContextProvider for LocalContextProvider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        if let Some(contract) = self
            .state
            .read()
            .expect("provider lock poisoned")
            .contracts
            .get(id)
        {
            return Ok(Some(Arc::clone(contract)));
        }
        for system_contract in [SystemDataContract::DPNS, SystemDataContract::Dashpay] {
            let contract = load_system_data_contract(system_contract, platform_version)
                .map_err(|e| ContextProviderError::DataContractFailure(e.to_string()))?;
            if contract.id() == *id {
                let contract = Arc::new(contract);
                self.state
                    .write()
                    .expect("provider lock poisoned")
                    .contracts
                    .insert(*id, Arc::clone(&contract));
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
        // The locally synced LLMQ store only tracks currently valid quorums,
        // so the requested core height adds nothing to the lookup: a proof
        // signed by a quorum the node no longer knows fails verification.
        self.state
            .read()
            .expect("provider lock poisoned")
            .quorum_keys
            .get(&(quorum_type, quorum_hash))
            .copied()
            .ok_or_else(|| {
                ContextProviderError::InvalidQuorum(format!(
                    "no locally known quorum of type {} with hash {}",
                    quorum_type,
                    hex::encode(quorum_hash)
                ))
            })
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(self
            .state
            .read()
            .expect("provider lock poisoned")
            .platform_activation_height)
    }
}
