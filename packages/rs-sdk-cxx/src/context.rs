// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! The SDK's [`ContextProvider`] over the embedder's `CoreContext`: quorum
//! keys are pulled from the embedder on every proof, with no cache and no
//! other source; DPNS and DashPay contracts come from dpp.

use std::sync::Arc;

use dash_sdk::dpp::data_contract::TokenConfiguration;
use dash_sdk::dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::error::ContextProviderError;
use dash_sdk::platform::ContextProvider;

use crate::ffi;
use crate::marshal::reversed;
use crate::runtime::blocking;

// SAFETY: `CoreContext` is documented as callable from any thread; its
// callbacks take the embedder's own locks.
unsafe impl Send for ffi::CoreContext {}
unsafe impl Sync for ffi::CoreContext {}

/// Where quorum keys come from: the embedder's `CoreContext`, or a fake in
/// tests. Both answer `None` for anything they do not hold.
pub trait QuorumSource: Send + Sync {
    fn quorum_public_key(&self, llmq_type: u8, hash: &[u8; 32], height: u32) -> Option<[u8; 48]>;
    fn platform_activation_height(&self) -> Option<u32>;
}

impl QuorumSource for cxx::UniquePtr<ffi::CoreContext> {
    fn quorum_public_key(&self, llmq_type: u8, hash: &[u8; 32], height: u32) -> Option<[u8; 48]> {
        let mut key = [0u8; 48];
        self.QuorumPublicKey(llmq_type, hash, height, &mut key)
            .then_some(key)
    }

    fn platform_activation_height(&self) -> Option<u32> {
        let mut height = 0;
        self.PlatformActivationHeight(&mut height).then_some(height)
    }
}

pub struct CoreContextProvider<S> {
    pub source: S,
    pub platform_llmq_type: u8,
}

impl<S: QuorumSource> ContextProvider for CoreContextProvider<S> {
    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        [SystemDataContract::DPNS, SystemDataContract::Dashpay]
            .into_iter()
            .find(|contract| contract.id() == *id)
            .map(|contract| {
                load_system_data_contract(contract, platform_version)
                    .map(Arc::new)
                    .map_err(|e| ContextProviderError::DataContractFailure(e.to_string()))
            })
            .transpose()
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }

    /// `quorum_hash` arrives in proof byte order, the reverse of the
    /// embedder's internal `uint256` order. The type is range-checked before
    /// it is narrowed and must be the configured Platform type.
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        if quorum_type != u32::from(self.platform_llmq_type) {
            return Err(ContextProviderError::InvalidQuorum(format!(
                "proof signed by LLMQ type {quorum_type}; Platform uses type {}",
                self.platform_llmq_type
            )));
        }
        let internal_order = reversed(&quorum_hash);
        blocking(|| {
            self.source.quorum_public_key(
                self.platform_llmq_type,
                &internal_order,
                core_chain_locked_height,
            )
        })
        .ok_or_else(|| {
            ContextProviderError::InvalidQuorum(
                "no local Platform quorum with that hash".to_string(),
            )
        })
    }

    /// A refusal is the local source's gap, not a bad proof. (Its only SDK
    /// consumer, total credits, is not bridged.)
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        blocking(|| self.source.platform_activation_height()).ok_or_else(|| {
            ContextProviderError::QuorumSourceUnavailable(
                "the Platform activation height is unknown".to_string(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    const PLATFORM_LLMQ: u8 = 106;

    /// Holds one key, under its hash in the embedder's byte order, and
    /// records every lookup it is asked for.
    struct OneQuorum {
        hash: [u8; 32],
        asked: Mutex<Vec<(u8, [u8; 32], u32)>>,
    }

    impl QuorumSource for OneQuorum {
        fn quorum_public_key(
            &self,
            llmq_type: u8,
            hash: &[u8; 32],
            height: u32,
        ) -> Option<[u8; 48]> {
            self.asked.lock().unwrap().push((llmq_type, *hash, height));
            (*hash == self.hash).then_some([7; 48])
        }

        fn platform_activation_height(&self) -> Option<u32> {
            None
        }
    }

    fn provider(hash: [u8; 32]) -> CoreContextProvider<OneQuorum> {
        CoreContextProvider {
            source: OneQuorum {
                hash,
                asked: Mutex::new(Vec::new()),
            },
            platform_llmq_type: PLATFORM_LLMQ,
        }
    }

    #[test]
    fn quorum_lookups_check_the_type_and_pass_the_hash_in_core_order() {
        let mut core_order = [0u8; 32];
        core_order[0] = 0xaa;
        let provider = provider(core_order);
        let mut proof_order = core_order;
        proof_order.reverse();
        let platform = u32::from(PLATFORM_LLMQ);

        assert_eq!(
            provider
                .get_quorum_public_key(platform, proof_order, 9)
                .unwrap(),
            [7; 48]
        );
        // Unknown to the embedder: refused, with no fallback.
        assert!(matches!(
            provider.get_quorum_public_key(platform, core_order, 9),
            Err(ContextProviderError::InvalidQuorum(_))
        ));
        // Another type, including one that would wrap to the Platform type
        // when narrowed to u8, never reaches the embedder.
        for other in [platform + 1, platform + 256, u32::MAX] {
            assert!(matches!(
                provider.get_quorum_public_key(other, proof_order, 9),
                Err(ContextProviderError::InvalidQuorum(_))
            ));
        }
        assert_eq!(
            *provider.source.asked.lock().unwrap(),
            vec![
                (PLATFORM_LLMQ, core_order, 9),
                (PLATFORM_LLMQ, proof_order, 9)
            ]
        );
        assert!(matches!(
            provider.get_platform_activation_height(),
            Err(ContextProviderError::QuorumSourceUnavailable(_))
        ));
    }

    #[test]
    fn system_contracts_are_served_locally() {
        let provider = provider([0; 32]);
        let version = PlatformVersion::latest();
        for contract in [SystemDataContract::DPNS, SystemDataContract::Dashpay] {
            assert!(provider
                .get_data_contract(&contract.id(), version)
                .unwrap()
                .is_some());
        }
        let unknown = Identifier::from([0x33u8; 32]);
        assert!(provider
            .get_data_contract(&unknown, version)
            .unwrap()
            .is_none());
    }
}
