// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! The push-model [`ContextProvider`] `dash-sdk` verifies proofs against.
//!
//! The verifier asks for the public key of the quorum a proof names before
//! it checks the signature, so everything the embedder knows about the
//! trust anchor is enforced here, ahead of any SDK state moving: the
//! quorum type must be the network's Platform type, the quorum must be one
//! the embedder pushed from its mined final commitments, a local ChainLock
//! height must have been pushed, and the proof's signed core-chain-locked
//! height must not trail it by more than [`MAX_CORE_CHAINLOCK_LAG`]. There
//! is no ceiling: the signed height is inside the signed `StateId`, and a
//! node one ChainLock ahead of this one is honest.
//!
//! Failures map to `Status` by variant, never by message:
//! [`ContextProviderError::Config`] (no local anchor) is `Unavailable`,
//! [`ContextProviderError::InvalidQuorum`] (wrong type, unknown hash, stale)
//! is `Rejected`.
//!
//! Data contracts are the compiled-in DPNS and DashPay system contracts,
//! cached per protocol version with a negative entry for every other id.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

use dash_sdk::dpp::data_contract::TokenConfiguration;
use dash_sdk::dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::error::ContextProviderError;
use dash_sdk::platform::ContextProvider;

use crate::ffi;
use crate::sync::{read, write};

/// Core blocks a proof's signed core-chain-locked height may trail the
/// embedder's own best ChainLock: roughly half a day at 2.5 min/block,
/// generous so normal Platform lag never trips it, small enough that a
/// replayed proof is bounded.
pub const MAX_CORE_CHAINLOCK_LAG: u32 = 288;

#[derive(Default)]
struct Contracts {
    /// (contract id, protocol version) -> the compiled-in contract, or
    /// `None` for an id that is not a system contract.
    by_id: HashMap<(Identifier, u32), Option<Arc<DataContract>>>,
}

pub struct LocalContextProvider {
    platform_llmq_type: u32,
    /// quorum hash in proof byte order -> BLS public key.
    quorum_keys: RwLock<HashMap<[u8; 32], [u8; 48]>>,
    /// The embedder's best ChainLock height; 0 = not pushed yet.
    local_core_chain_locked_height: AtomicU32,
    contracts: RwLock<Contracts>,
}

impl LocalContextProvider {
    pub fn new(platform_llmq_type: u8) -> Self {
        LocalContextProvider {
            platform_llmq_type: u32::from(platform_llmq_type),
            quorum_keys: RwLock::new(HashMap::new()),
            local_core_chain_locked_height: AtomicU32::new(0),
            contracts: RwLock::new(Contracts::default()),
        }
    }

    /// Replaces the Platform quorum keys with `keys`. The embedder pushes
    /// the full active set on every update, so replacement (not merge)
    /// keeps rotated-out quorums from verifying proofs forever. The hashes
    /// arrive in the embedder's internal `uint256` byte order; proofs carry
    /// the reverse (the order `quorum info` prints), so they are reversed
    /// once here and C++ never learns the foreign representation.
    pub fn set_quorum_keys(&self, keys: &[ffi::QuorumKey]) {
        let keys = keys
            .iter()
            .map(|key| {
                let mut hash = key.hash;
                hash.reverse();
                (hash, key.pubkey)
            })
            .collect();
        *write(&self.quorum_keys) = keys;
    }

    /// Records the embedder's best ChainLock height, the anchor of the lag
    /// floor. Monotonic.
    pub fn set_local_core_chain_locked_height(&self, height: u32) {
        self.local_core_chain_locked_height
            .fetch_max(height, Ordering::Relaxed);
    }

    /// The pushed ChainLock height, 0 before the first push.
    pub fn local_core_chain_locked_height(&self) -> u32 {
        self.local_core_chain_locked_height.load(Ordering::Relaxed)
    }
}

/// The `Status` kind a provider refusal maps to: the missing anchor is the
/// embedder's state (`Unavailable`), everything else is the response's
/// fault (`Rejected`).
pub fn status_kind(error: &ContextProviderError) -> ffi::StatusKind {
    match error {
        ContextProviderError::Config(_) => ffi::StatusKind::Unavailable,
        _ => ffi::StatusKind::Rejected,
    }
}

impl ContextProvider for LocalContextProvider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        let cache_key = (*id, platform_version.protocol_version);
        if let Some(cached) = read(&self.contracts).by_id.get(&cache_key) {
            return Ok(cached.clone());
        }
        let found = match [SystemDataContract::DPNS, SystemDataContract::Dashpay]
            .into_iter()
            .find(|contract| contract.id() == *id)
        {
            Some(contract) => Some(Arc::new(
                load_system_data_contract(contract, platform_version)
                    .map_err(|e| ContextProviderError::DataContractFailure(e.to_string()))?,
            )),
            None => None,
        };
        write(&self.contracts)
            .by_id
            .insert(cache_key, found.clone());
        Ok(found)
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
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        if quorum_type != self.platform_llmq_type {
            return Err(ContextProviderError::InvalidQuorum(format!(
                "proof signed by quorum type {quorum_type}; Platform quorums on this network are \
                 type {}",
                self.platform_llmq_type
            )));
        }
        let local = self.local_core_chain_locked_height();
        if local == 0 {
            return Err(ContextProviderError::Config(
                "no local ChainLock anchor pushed yet".to_string(),
            ));
        }
        if local.saturating_sub(core_chain_locked_height) > MAX_CORE_CHAINLOCK_LAG {
            return Err(ContextProviderError::InvalidQuorum(format!(
                "stale proof: signed core chain locked height {core_chain_locked_height} trails \
                 the local ChainLock height {local} by more than {MAX_CORE_CHAINLOCK_LAG} blocks"
            )));
        }
        read(&self.quorum_keys)
            .get(&quorum_hash)
            .copied()
            .ok_or_else(|| {
                ContextProviderError::InvalidQuorum(format!(
                    "no locally known Platform quorum with hash {}",
                    hex::encode(quorum_hash)
                ))
            })
    }

    /// Only `verify_total_credits_in_system` consumes this, a query the
    /// embedder never issues.
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Err(ContextProviderError::Config(
            "the Platform activation height is not available through this binding".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;

    const PLATFORM_LLMQ: u8 = 106;

    fn provider_with_key(hash_core_order: [u8; 32]) -> LocalContextProvider {
        let provider = LocalContextProvider::new(PLATFORM_LLMQ);
        provider.set_quorum_keys(&[ffi::QuorumKey {
            hash: hash_core_order,
            pubkey: [7u8; 48],
        }]);
        provider
    }

    #[test]
    fn quorum_hash_is_reversed_from_core_order() {
        let mut core_order = [0u8; 32];
        core_order[0] = 0xaa;
        core_order[31] = 0xbb;
        let provider = provider_with_key(core_order);
        provider.set_local_core_chain_locked_height(1000);
        let mut proof_order = core_order;
        proof_order.reverse();
        assert_eq!(
            provider
                .get_quorum_public_key(u32::from(PLATFORM_LLMQ), proof_order, 1000)
                .expect("the pushed key in proof order"),
            [7u8; 48]
        );
        assert!(matches!(
            provider.get_quorum_public_key(u32::from(PLATFORM_LLMQ), core_order, 1000),
            Err(ContextProviderError::InvalidQuorum(_))
        ));
    }

    #[test]
    fn gates_run_before_the_key_lookup() {
        let provider = provider_with_key([1u8; 32]);
        // No anchor yet: refused as the embedder's fault.
        let error = provider
            .get_quorum_public_key(u32::from(PLATFORM_LLMQ), [1u8; 32], 5000)
            .unwrap_err();
        assert!(matches!(error, ContextProviderError::Config(_)));
        assert_eq!(status_kind(&error), ffi::StatusKind::Unavailable);

        provider.set_local_core_chain_locked_height(5000);
        // The wrong LLMQ type is refused even for a pushed hash.
        let error = provider
            .get_quorum_public_key(u32::from(PLATFORM_LLMQ) + 1, [1u8; 32], 5000)
            .unwrap_err();
        assert!(matches!(error, ContextProviderError::InvalidQuorum(_)));
        assert_eq!(status_kind(&error), ffi::StatusKind::Rejected);
    }

    #[test]
    fn chainlock_lag_floor_has_no_ceiling() {
        let provider = provider_with_key([1u8; 32]);
        provider.set_local_core_chain_locked_height(5000);
        let lookup = |signed: u32| {
            provider.get_quorum_public_key(u32::from(PLATFORM_LLMQ), [1u8; 32], signed)
        };
        assert!(lookup(5000 - MAX_CORE_CHAINLOCK_LAG).is_ok());
        assert!(matches!(
            lookup(5000 - MAX_CORE_CHAINLOCK_LAG - 1),
            Err(ContextProviderError::InvalidQuorum(_))
        ));
        // A node one ChainLock ahead of us is honest.
        assert!(lookup(5001).is_ok());
        // The anchor never moves backwards.
        provider.set_local_core_chain_locked_height(10);
        assert_eq!(provider.local_core_chain_locked_height(), 5000);
    }

    #[test]
    fn replacing_keys_forgets_rotated_out_quorums() {
        let provider = provider_with_key([1u8; 32]);
        provider.set_local_core_chain_locked_height(1);
        provider.set_quorum_keys(&[ffi::QuorumKey {
            hash: [2u8; 32],
            pubkey: [9u8; 48],
        }]);
        assert!(provider
            .get_quorum_public_key(u32::from(PLATFORM_LLMQ), [1u8; 32], 1)
            .is_err());
        assert_eq!(
            provider
                .get_quorum_public_key(u32::from(PLATFORM_LLMQ), [2u8; 32], 1)
                .expect("the replacement key"),
            [9u8; 48]
        );
    }

    #[test]
    fn serves_system_contracts_and_caches_negative_lookups() {
        let provider = LocalContextProvider::new(PLATFORM_LLMQ);
        let version = PlatformVersion::latest();
        for contract in [SystemDataContract::DPNS, SystemDataContract::Dashpay] {
            let served = provider
                .get_data_contract(&contract.id(), version)
                .unwrap()
                .expect("system contract");
            assert_eq!(served.id(), contract.id());
        }
        let unknown = Identifier::from([0x33u8; 32]);
        assert!(provider
            .get_data_contract(&unknown, version)
            .unwrap()
            .is_none());
        let cached = read(&provider.contracts);
        assert_eq!(cached.by_id.len(), 3);
        assert!(cached.by_id[&(unknown, version.protocol_version)].is_none());
    }

    #[test]
    fn activation_height_is_refused() {
        let provider = LocalContextProvider::new(PLATFORM_LLMQ);
        assert!(matches!(
            provider.get_platform_activation_height(),
            Err(ContextProviderError::Config(_))
        ));
    }
}
