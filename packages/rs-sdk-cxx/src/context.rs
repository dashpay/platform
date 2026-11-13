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

/// What a [`QuorumSource`] knows of the quorum a proof names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuorumAnswer {
    /// The quorum could have signed; its BLS public key.
    Found([u8; 48]),
    /// The source's complete evidence shows it could not have signed.
    Invalid,
    /// The source cannot decide now: no fault of the proof.
    Unavailable,
}

/// Where quorum keys come from: the embedder's `CoreContext`, or a fake in
/// tests.
pub trait QuorumSource: Send + Sync {
    fn quorum_public_key(&self, llmq_type: u8, hash: &[u8; 32], height: u32) -> QuorumAnswer;
    fn platform_activation_height(&self) -> Option<u32>;
}

impl QuorumSource for cxx::UniquePtr<ffi::CoreContext> {
    fn quorum_public_key(&self, llmq_type: u8, hash: &[u8; 32], height: u32) -> QuorumAnswer {
        let mut key = [0u8; 48];
        let lookup = self.QuorumPublicKey(llmq_type, hash, height, &mut key);
        answer(lookup, key)
    }

    fn platform_activation_height(&self) -> Option<u32> {
        let mut height = 0;
        self.PlatformActivationHeight(&mut height).then_some(height)
    }
}

/// The embedder's lookup: a value outside the enum is its fault, never the
/// proof's.
fn answer(lookup: ffi::QuorumLookup, key: [u8; 48]) -> QuorumAnswer {
    match lookup {
        ffi::QuorumLookup::Found => QuorumAnswer::Found(key),
        ffi::QuorumLookup::Invalid => QuorumAnswer::Invalid,
        _ => QuorumAnswer::Unavailable,
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
    /// it is narrowed and must be the configured Platform type. The signed
    /// height goes to the embedder unchanged. `Unavailable` is the source's
    /// gap (`QuorumSourceUnavailable`, which bans no endpoint); only
    /// `Invalid` indicts the proof.
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
        let answer = blocking(|| {
            self.source.quorum_public_key(
                self.platform_llmq_type,
                &internal_order,
                core_chain_locked_height,
            )
        });
        match answer {
            QuorumAnswer::Found(key) => Ok(key),
            QuorumAnswer::Invalid => Err(ContextProviderError::InvalidQuorum(format!(
                "no local Platform quorum with that hash could sign at Core height \
                 {core_chain_locked_height}"
            ))),
            QuorumAnswer::Unavailable => {
                Err(ContextProviderError::QuorumSourceUnavailable(format!(
                    "the local quorum data cannot decide this quorum at Core height \
                     {core_chain_locked_height} yet"
                )))
            }
        }
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

    /// Finds one quorum and knows another invalid, under their hashes in the
    /// embedder's byte order; cannot decide any other. Records every lookup.
    struct Scripted {
        found: [u8; 32],
        invalid: [u8; 32],
        asked: Mutex<Vec<(u8, [u8; 32], u32)>>,
    }

    impl QuorumSource for Scripted {
        fn quorum_public_key(&self, llmq_type: u8, hash: &[u8; 32], height: u32) -> QuorumAnswer {
            self.asked.lock().unwrap().push((llmq_type, *hash, height));
            if *hash == self.found {
                QuorumAnswer::Found([7; 48])
            } else if *hash == self.invalid {
                QuorumAnswer::Invalid
            } else {
                QuorumAnswer::Unavailable
            }
        }

        fn platform_activation_height(&self) -> Option<u32> {
            None
        }
    }

    fn hash(first: u8) -> [u8; 32] {
        let mut hash = [0u8; 32];
        hash[0] = first;
        hash
    }

    fn provider() -> CoreContextProvider<Scripted> {
        CoreContextProvider {
            source: Scripted {
                found: hash(0xaa),
                invalid: hash(0xbb),
                asked: Mutex::new(Vec::new()),
            },
            platform_llmq_type: PLATFORM_LLMQ,
        }
    }

    /// Found gives the key, Invalid indicts the proof, Unavailable is the
    /// source's gap. The hash is reversed once and the signed height passed
    /// as is.
    #[test]
    fn should_map_each_quorum_answer_to_its_own_result() {
        let provider = provider();
        let platform = u32::from(PLATFORM_LLMQ);
        let signed_height = 4_000_001;

        assert_eq!(
            provider
                .get_quorum_public_key(platform, reversed(&hash(0xaa)), signed_height)
                .unwrap(),
            [7; 48]
        );
        assert!(matches!(
            provider.get_quorum_public_key(platform, reversed(&hash(0xbb)), signed_height),
            Err(ContextProviderError::InvalidQuorum(_))
        ));
        assert!(matches!(
            provider.get_quorum_public_key(platform, reversed(&hash(0xcc)), signed_height),
            Err(ContextProviderError::QuorumSourceUnavailable(_))
        ));
        assert_eq!(
            *provider.source.asked.lock().unwrap(),
            vec![
                (PLATFORM_LLMQ, hash(0xaa), signed_height),
                (PLATFORM_LLMQ, hash(0xbb), signed_height),
                (PLATFORM_LLMQ, hash(0xcc), signed_height),
            ]
        );
    }

    #[test]
    fn should_never_ask_the_embedder_about_another_quorum_type() {
        let provider = provider();
        let platform = u32::from(PLATFORM_LLMQ);
        // Including one that would wrap to the Platform type when narrowed.
        for other in [platform + 1, platform + 256, u32::MAX] {
            assert!(matches!(
                provider.get_quorum_public_key(other, reversed(&hash(0xaa)), 9),
                Err(ContextProviderError::InvalidQuorum(_))
            ));
        }
        assert!(provider.source.asked.lock().unwrap().is_empty());
        assert!(matches!(
            provider.get_platform_activation_height(),
            Err(ContextProviderError::QuorumSourceUnavailable(_))
        ));
    }

    /// A lookup value the enum does not name is the embedder's fault.
    #[test]
    fn should_read_an_unknown_embedder_lookup_as_unavailable() {
        assert_eq!(
            answer(ffi::QuorumLookup::Found, [3; 48]),
            QuorumAnswer::Found([3; 48])
        );
        assert_eq!(
            answer(ffi::QuorumLookup::Invalid, [3; 48]),
            QuorumAnswer::Invalid
        );
        for repr in [2, 3, 0xff] {
            assert_eq!(
                answer(ffi::QuorumLookup { repr }, [3; 48]),
                QuorumAnswer::Unavailable
            );
        }
    }

    #[test]
    fn system_contracts_are_served_locally() {
        let provider = provider();
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
