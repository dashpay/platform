//! Implementation of validator set updates, required by Tenderdash.
//!
//!

mod error;

pub use error::ValidatorSetError;

use dashcore_rpc::dashcore::hashes::{sha256, Hash, HashEngine};
use dashcore_rpc::dashcore_rpc_json::{
    ExtendedQuorumDetails, QuorumHash, QuorumInfoResult, QuorumType,
};
use tenderdash_abci::proto::{abci, crypto as abci_crypto};

use crate::{
    config::PlatformConfig,
    rpc::core::{CoreHeight, CoreRPCLike, QuorumListExtendedInfo},
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
                power: 100,      // FIXME: double-check
                pub_key: pubkey, // FIXME: double-check if it should be pub_key_share
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
    ) -> Result<Self, ValidatorSetError> {
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
        .ok_or(ValidatorSetError::NoQuorumAtHeight(
            Some(core_height),
            quorum_type.to_owned(),
        ))?;

        let entropy = if seed.is_none() {
            Vec::new()
        } else {
            seed.unwrap()
        };

        let quorum =
            Self::choose_random_quorum(config, core_height, &quorum_type, &quorums, &entropy)?;
        let quorum_info = client
            .get_quorum_info(quorum_type.clone().into(), &quorum.into(), Some(false))
            .map_err(|e| ValidatorSetError::RpcError(e))?;

        Ok(Self { quorum_info })
    }

    /// Returns quorum to use at provided height
    fn choose_random_quorum(
        config: &PlatformConfig,
        core_height: CoreHeight,
        quorum_type: &QuorumType,
        quorums_extended_info: &Vec<QuorumListExtendedInfo>,
        entropy: &Vec<u8>,
    ) -> Result<Quorum, ValidatorSetError> {
        // read some config
        let rotation_block_interval: CoreHeight = config.validator_set_quorum_rotation_block_count;
        let min_valid_members = config.core.min_quorum_valid_members;
        let dkg_interval = config.core.dkg_interval;

        let min_ttl: CoreHeight = rotation_block_interval * 3;

        let number_of_quorums = quorums_extended_info.len() as u32;
        if number_of_quorums == 0 {
            return Err(ValidatorSetError::NoQuorumAtHeight(
                None,
                quorum_type.to_owned(),
            ));
        }

        // First, convert dashcore rpc quorum info into our Quorum struct
        let quorums = quorums_extended_info
            .into_iter()
            .flatten()
            .map(|(hash, details)| Quorum::new(hash, details, entropy))
            .collect::<Vec<Quorum>>();

        // Now, let's filter quorums. We use iter() to not consume `quorums`, needed later
        let mut filtered_quorums = quorums
            .iter()
            .filter(|item| {
                item.num_valid_members >= min_valid_members
                    && item.quorum_ttl(core_height, dkg_interval, number_of_quorums) > min_ttl
            })
            .collect::<Vec<&Quorum>>();

        // if there is no "vital" quorums, we choose among others with default min quorum size
        if filtered_quorums.len() == 0 {
            filtered_quorums = quorums.iter().collect::<Vec<&Quorum>>();
        }

        // Now we select the final quorum, based on some scoring algorithm.
        filtered_quorums.sort();
        let winner =
            filtered_quorums
                .into_iter()
                .next()
                .ok_or(ValidatorSetError::NoQuorumAtHeight(
                    Some(core_height),
                    quorum_type.to_owned(),
                ))?;

        Ok(winner.to_owned())
    }
}

impl From<Quorum> for QuorumHash {
    fn from(value: Quorum) -> Self {
        QuorumHash(value.quorum_hash)
    }
}
/// Quorum info with additional weight details. Easy to sort by weight.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
struct Quorum {
    // ensure weight is first, as it metters when sorting
    weight: Vec<u8>,

    quorum_hash: Vec<u8>,

    creation_height: u32,
    quorum_index: Option<u32>,
    // mined_block_hash: BlockHash,
    num_valid_members: u32,
    health_ratio: i32,
}

impl Quorum {
    fn new(
        quorum_hash: &QuorumHash,
        quorum_details: &ExtendedQuorumDetails,
        entropy: &Vec<u8>,
    ) -> Self {
        Quorum {
            weight: Self::calculate_weight(quorum_hash, entropy),

            quorum_hash: quorum_hash.0.clone(),
            creation_height: quorum_details.creation_height,
            // To avoid playing with floats, which don't implement Ord, we just multiply health ratio by 10^6
            health_ratio: (quorum_details.health_ratio * 1000000.0).round() as i32,
            num_valid_members: quorum_details.num_valid_members,
            quorum_index: quorum_details.quorum_index,
        }
    }

    /// Calculate weight to use when sorting.
    fn calculate_weight(quorum_hash: &QuorumHash, entropy: &Vec<u8>) -> Vec<u8> {
        let mut hash = quorum_hash.0.clone();
        hash.extend(entropy);

        let mut hasher = sha256::HashEngine::default();
        hasher.input(&hash);
        sha256::Hash::from_engine(hasher).to_vec()
    }

    /// Calculate estimated quorum time-to-live
    fn quorum_ttl(
        &self,
        core_height: CoreHeight,
        dkg_interval: u32,
        number_of_quorums: u32,
    ) -> u32 {
        let quorum_remove_height: CoreHeight =
            self.creation_height + (dkg_interval * number_of_quorums);
        if quorum_remove_height <= core_height {
            return 0;
        }
        let how_much_in_rest: CoreHeight = quorum_remove_height - core_height;
        let quorum_ttl: u32 = how_much_in_rest * 5 / 2; // multiply by 2.5, round down

        quorum_ttl
    }
}

#[cfg(test)]
mod tests {
    use dashcore_rpc::dashcore::{hashes::Hash, BlockHash};
    use dashcore_rpc::dashcore_rpc_json::{ExtendedQuorumDetails, QuorumHash, QuorumInfoResult};
    use tenderdash_abci::proto::abci::ValidatorSetUpdate;

    use crate::{config::PlatformConfig, rpc::core::QuorumListExtendedInfo};

    fn generate_quorums_extended_info(n: u32) -> QuorumListExtendedInfo {
        let mut quorums = QuorumListExtendedInfo::new();

        for i in 0..n {
            let i_bytes = [i as u8; 32];

            let hash = QuorumHash(i_bytes.to_vec());

            let details = ExtendedQuorumDetails {
                creation_height: i,
                health_ratio: (i as f32) / (n as f32),
                mined_block_hash: BlockHash::from_slice(&i_bytes).unwrap(),
                num_valid_members: i,
                quorum_index: Some(i),
            };

            quorums
                .insert(hash.clone(), details)
                .map(|v| panic!("duplicate record {:?}={:?}", hash, v));
        }
        quorums
    }

    #[test]
    fn test_new_random_at_height() {
        const CORE_HEIGHT: u32 = 2000;
        let quorum_type = dashcore_rpc::dashcore_rpc_json::QuorumType::Llmq50_60;

        let config = PlatformConfig::default();
        let mut client = crate::rpc::core::MockCoreRPCLike::new();
        client
            .expect_get_quorum_listextended()
            .returning(move |_| {
                Ok(dashcore_rpc::dashcore_rpc_json::QuorumListResult {
                    llmq_50_60: Some(vec![generate_quorums_extended_info(100)]),
                    llmq_400_60: None,
                    llmq_400_85: None,
                    llmq_100_67: None,
                    llmq_60_75: None,
                    llmq_25_67: None,
                    // for devnets only
                    llmq_devnet: None,
                    llmq_devnet_platform: None,
                    // for devnets only. rotated version (v2) for devnets
                    llmq_devnet_dip0024: None,
                    // for testing only
                    llmq_test: None,
                    llmq_test_instantsend: None,
                    llmq_test_v17: None,
                    llmq_test_dip0024: None,
                    llmq_test_platform: None,
                    // TODO: simplify with ..Default::default() when it's implemented by dashcore_rpc
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
        assert_eq!(vsu.quorum_hash, [17u8; 32]);
    }
}
