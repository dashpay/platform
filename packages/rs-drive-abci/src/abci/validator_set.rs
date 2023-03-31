//! Implementation of validator set updates, required by Tenderdash.
//!
//!

use dashcore_rpc::dashcore_rpc_json::{QuorumHash, QuorumInfoResult, QuorumType};
use tenderdash_abci::proto::{abci, crypto as abci_crypto};

use crate::{
    config::PlatformConfig,
    rpc::core::{CoreHeight, CoreRPCLike},
};

/// ValidatorSet contains validators that should be in use at a given height.
///
/// You can easily convert ValidatorSet into [tenderdash_abci::proto::abci::ValdiatorSetUpdate] using [From].
pub struct ValidatorSet {
    quorum_info: QuorumInfoResult,
}

/// Helper function to convert bytes into bls12381 public key, as required by Tenderdash
fn u8_to_bls12381_pubkey(public_key: Vec<u8>) -> abci_crypto::PublicKey {
    abci_crypto::PublicKey {
        sum: Some(tenderdash_abci::proto::crypto::public_key::Sum::Bls12381(
            public_key,
        )),
    }
}

impl From<ValidatorSet> for tenderdash_abci::proto::abci::ValidatorSetUpdate {
    fn from(value: ValidatorSet) -> Self {
        let mut validator_updates: Vec<abci::ValidatorUpdate> = Vec::new();
        for validator in value.quorum_info.members {
            if !validator.valid {
                continue;
            }
            let pubkey = validator.pub_key_share.map(|k| u8_to_bls12381_pubkey(k));
            let vu = abci::ValidatorUpdate {
                node_address: Default::default(),
                power: 100,      // TODO: double-check
                pub_key: pubkey, // TODO: double-check if it should be pub_key_share
                pro_tx_hash: validator.pro_tx_hash.0,
            };

            validator_updates.push(vu);
        }

        Self {
            validator_updates,
            threshold_public_key: Some(u8_to_bls12381_pubkey(value.quorum_info.quorum_public_key)),
            quorum_hash: value.quorum_info.quorum_hash.0,
        }
    }
}

impl ValidatorSet {
    /// Retrieve quorums from Dash Core at provided height and extract validator set information from it.
    ///
    /// ## Arguments
    ///
    /// - `client` - Core RPC client
    /// - `config` - platform configuration
    /// - `core_height` - height of dash core for which we create validator set
    /// - `quorum_type` - type of LLMQ quorum
    /// - `seed` - additional information that can be included in the selection algorithm to make it non-deterministic.
    ///   Use `None` to make it deterministic.
    pub(crate) fn new_at_height_with_seed<C: CoreRPCLike>(
        client: C,
        config: &PlatformConfig,
        core_height: CoreHeight,
        quorum_type: &QuorumType,
        seed: Option<Vec<u8>>,
    ) -> Result<Self, ValSetError> {
        // TODO: parse QuorumInfoResult to return
        let quorums = client.get_quorum_listextended(Some(core_height))?;
        let quorums = match quorum_type {
            QuorumType::Llmq50_60 => quorums.llmq_50_60,
            QuorumType::Llmq400_60 => quorums.llmq_400_60,
            QuorumType::Llmq400_85 => quorums.llmq_400_85,
            QuorumType::Llmq100_67 => quorums.llmq_100_67,
            QuorumType::Llmq60_75 => panic!("unsupported quorum type {:?}", quorum_type),
            QuorumType::LlmqTest => quorums.llmq_test,
            QuorumType::LlmqDevnet => panic!("unsupported quorum type {:?}", quorum_type),
            QuorumType::LlmqTestV17 => quorums.llmq_test_v17,
            QuorumType::LlmqTestDip0024 => quorums.llmq_test_dip0024,
            QuorumType::LlmqTestInstantsend => quorums.llmq_test_instantsend,
            QuorumType::LlmqDevnetDip0024 => panic!("unsupported quorum type {:?}", quorum_type),
            QuorumType::LlmqTestPlatform => quorums.llmq_test_platform,
            QuorumType::LlmqDevnetPlatform => panic!("unsupported quorum type {:?}", quorum_type),
            QuorumType::UNKNOWN => panic!("unsupported quorum type {:?}", quorum_type),
            // no default here, so if the list of quorums changes, we will detect it during build
        }
        .ok_or(ValSetError::NoQuorumAtHeight(
            Some(core_height),
            quorum_type.to_owned(),
        ))?;

        let entropy = if seed.is_none() {
            Vec::new()
        } else {
            seed.unwrap()
        };

        let quorum_hash =
            Self::choose_random_quorum(config, core_height, &quorum_type, &quorums, &entropy)?;
        let quorum_info = client
            .get_quorum_info(quorum_type.clone().into(), &quorum_hash, Some(false))
            .map_err(|e| ValSetError::RpcError(e))?;

        Ok(Self { quorum_info })
    }

    /// Returns best quorum to use
    /// TODO: replace Vec<QuorumHash> with Vec<QuorumListExtendedInfo>
    fn choose_random_quorum(
        config: &PlatformConfig,
        core_height: CoreHeight,
        quorum_type: &QuorumType,
        quorums_extended_info: &Vec<QuorumHash>,
        entropy: &Vec<u8>,
    ) -> Result<QuorumHash, ValSetError> {
        // TODO: migrate to config file
        const DKG_INTERVAL: CoreHeight = 24;
        const MIN_QUORUM_VALID_MEMBERS: CoreHeight = 3;
        let rotation_block_interval: CoreHeight = config.validator_set_quorum_rotation_block_count;
        let min_ttl: CoreHeight = rotation_block_interval * 3;

        let number_of_quorums = quorums_extended_info.len() as u32;
        if number_of_quorums == 0 {
            return Err(ValSetError::NoQuorumAtHeight(None, quorum_type.to_owned()));
        }

        // let quorum_hash = quorums_extended_info
        //     .first()
        //     .ok_or(ValSetError::NoQuorumAtHeight(None, quorum_type.to_owned()))?;

        let quorums_weighted = quorums_extended_info
            .iter()
            .map(|item| QuorumWeighted::new(item, entropy))
            .collect::<Vec<QuorumWeighted>>();

        let mut final_quorums = quorums_weighted
            .iter()
            .filter(|_item| {
                // TODO: read the items below from quorums_extended_info
                let num_valid_members = 3;

                num_valid_members >= MIN_QUORUM_VALID_MEMBERS
            })
            .filter(|item| item.quorum_ttl(core_height, DKG_INTERVAL, number_of_quorums) > min_ttl)
            .collect::<Vec<&QuorumWeighted>>();
        // let mut filtered_quorums = &filtered_quorums;
        // if there is no "vital" quorums, we choose among others with default min quorum size
        if final_quorums.len() == 0 {
            final_quorums = quorums_weighted.iter().collect::<Vec<&QuorumWeighted>>();
        }

        // Now we select best quorum, based on some scoring algorithm.
        final_quorums.sort();
        let winner = final_quorums.first().ok_or(ValSetError::NoQuorumAtHeight(
            Some(core_height),
            quorum_type.to_owned(),
        ))?;

        // TODO: when dashcore_rpc is updated, use winner.unwrap().quorum_details.quorum_hash below
        Ok(QuorumHash(winner.quorum_details.to_owned()))
    }
}

/// Quorum info with additional weight details. Easy to sort by weight.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct QuorumWeighted {
    // ensure weight is first, as it metters when sorting
    weight: Vec<u8>,
    quorum_details: Vec<u8>,
}

impl QuorumWeighted {
    fn new(quorum_details: &QuorumHash, entropy: &Vec<u8>) -> Self {
        QuorumWeighted {
            quorum_details: quorum_details.0.to_owned(),
            weight: Self::calculate_weight(quorum_details, entropy),
        }
    }

    /// Calculate weight to use when sorting.
    fn calculate_weight(quorum_hash: &QuorumHash, entropy: &Vec<u8>) -> Vec<u8> {
        let mut hash = quorum_hash.0.clone();
        hash.extend(entropy);

        hash
    }

    /// Calculate estimated quorum time-to-live
    fn quorum_ttl(
        &self,
        core_height: CoreHeight,
        dkg_interval: u32,
        number_of_quorums: u32,
    ) -> u32 {
        // TODO: read the item below from quorums_extended_info
        let creation_height: CoreHeight = 2000;

        let quorum_remove_height: CoreHeight = creation_height + (dkg_interval * number_of_quorums);
        let how_much_in_rest: CoreHeight = quorum_remove_height - core_height;
        let quorum_ttl: u32 = how_much_in_rest * 5 / 2; // multiply by 2.5, round down

        quorum_ttl
    }
}

/// Error returned by Core RPC endpoint
#[derive(Debug, thiserror::Error)]
pub enum ValSetError {
    #[error{"Core RPC returned error: {0}"}]
    /// Error returned by RPC interface
    RpcError(#[from] dashcore_rpc::Error),

    /// Requested height is not found
    #[error{"No quorum of type {1:?} at core height {0:?} found"}]
    NoQuorumAtHeight(Option<CoreHeight>, QuorumType),

    /// Quorum with given hash not found
    #[error{"No quorum with hash {0:?} of type {1:?} found"}]
    QuorumNotFound(QuorumHash, QuorumType),
}

#[cfg(test)]
mod tests {
    use dashcore_rpc::dashcore_rpc_json::{QuorumHash, QuorumInfoResult};
    use tenderdash_abci::proto::abci::ValidatorSetUpdate;

    use crate::config::PlatformConfig;

    #[test]
    fn test_new_random_at_height() {
        const CORE_HEIGHT: u32 = 2000;
        let quorum_type = dashcore_rpc::dashcore_rpc_json::QuorumType::Llmq50_60;
        let quorum1 = QuorumHash(vec![1u8; 32]);
        let winning_quorum = quorum1.clone();

        let config = PlatformConfig::default();
        let mut client = crate::rpc::core::MockCoreRPCLike::new();
        client
            .expect_get_quorum_listextended()
            .returning(move |_| {
                Ok(dashcore_rpc::dashcore_rpc_json::QuorumListResult {
                    llmq_50_60: Some(vec![quorum1.clone()]),
                    llmq_400_60: None,
                    llmq_400_85: None,
                    llmq_100_67: None,
                    llmq_test: None,
                    llmq_test_instantsend: None,
                    llmq_test_v17: None,
                    llmq_test_dip0024: None,
                    llmq_test_platform: None,
                })
            })
            .once();

        client
            .expect_get_quorum_info()
            .returning(|quorum_type, quorum_hash, _| {
                Ok(QuorumInfoResult {
                    height: CORE_HEIGHT as u64,
                    quorum_type,
                    mined_block: vec![],
                    quorum_hash: quorum_hash.clone(),
                    quorum_index: 1,
                    quorum_public_key: vec![],
                    members: vec![],
                    secret_key_share: None,
                })
            })
            .once();

        let vset =
            super::ValidatorSet::new_at_height_with_seed(client, &config, 2000, &quorum_type, None)
                .expect("failed to fetch validator set");

        let vsu = ValidatorSetUpdate::from(vset);
        assert_eq!(vsu.quorum_hash, winning_quorum.0);
    }
}
