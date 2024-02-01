use crate::strategy::StrategyRandomness;
use dapi_grpc::platform::v0::get_identities_by_public_key_hashes_request::GetIdentitiesByPublicKeyHashesRequestV0;
use dapi_grpc::platform::v0::get_identities_by_public_key_hashes_response::PublicKeyHashIdentityEntry;
use dapi_grpc::platform::v0::{
    get_identities_by_public_key_hashes_request, get_identities_by_public_key_hashes_response,
    GetIdentitiesByPublicKeyHashesRequest, GetIdentitiesByPublicKeyHashesResponse, Proof,
};
use dashcore_rpc::dashcore_rpc_json::QuorumType;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::{Identity, PartialIdentity};
use dpp::serialization::PlatformDeserializable;
use dpp::validation::SimpleValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::verify::RootHash;
use drive::drive::Drive;
use drive_abci::abci::{AbciApplication, AbciError};
use drive_abci::rpc::core::MockCoreRPCLike;
use prost::Message;
use rand::prelude::SliceRandom;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, HashSet};
use strategy_tests::frequency::Frequency;
use tenderdash_abci::proto::google::protobuf::Timestamp;
use tenderdash_abci::proto::serializers::timestamp::ToMilis;
use tenderdash_abci::proto::types::{CanonicalVote, SignedMsgType, StateId};
use tenderdash_abci::signatures::{Hashable, Signable};

#[derive(Clone, Debug, Default)]
pub struct QueryStrategy {
    pub query_identities_by_public_key_hashes: Frequency,
}

/// ProofVerification contains trusted data from Platform chain (Tenderdash) needed to verify proofs at given `height`.
///
/// See https://github.com/dashpay/tenderdash/blob/v0.12-dev/spec/consensus/signing.md#block-signature-verification-on-light-client
#[derive(Debug, Clone)]
pub struct ProofVerification<'a> {
    /// Chain ID
    pub chain_id: String,

    /// Type of quorum
    pub quorum_type: QuorumType,

    /// Quorum hash
    pub quorum_hash: &'a [u8; 32],

    /// Commit height
    pub height: i64,

    /// Hash of CanonicalBlockID
    pub block_hash: &'a [u8; 32],

    /// Version of ABCI app used to generate this commit
    pub app_version: u64,

    /// App hash for the `height`
    pub app_hash: &'a [u8; 32],

    /// Core chain locked height in use when generating block
    pub core_chain_locked_height: u32,

    /// Block generation time
    pub time: Timestamp,

    /// Block signature
    pub signature: &'a [u8; 96],

    /// Threshold key used to verify the signature
    pub public_key: &'a dpp::bls_signatures::PublicKey,
}

impl<'a> ProofVerification<'a> {
    /// Verify proof signature
    ///
    /// Constructs new signature for provided state ID and checks if signature is still valid.
    ///
    /// Implements algorithm described at:
    /// https://github.com/dashpay/tenderdash/blob/v0.12-dev/spec/consensus/signing.md#block-signature-verification-on-light-client
    fn verify_signature(&self, state_id: StateId, round: u32) -> SimpleValidationResult<AbciError> {
        let state_id_hash =
            match state_id.calculate_msg_hash(&self.chain_id, self.height, round as i32) {
                Ok(s) => s,
                Err(e) => return SimpleValidationResult::new_with_error(AbciError::from(e)),
            };

        let v = CanonicalVote {
            block_id: self.block_hash.to_vec(),
            state_id: state_id_hash,
            chain_id: self.chain_id.clone(),
            height: self.height,
            round: round as i64,
            r#type: SignedMsgType::Precommit.into(),
        };

        let digest = match v.sign_digest(
            &self.chain_id,
            self.quorum_type as u8,
            self.quorum_hash,
            self.height,
            round as i32,
        ) {
            Ok(h) => h,
            Err(e) => return SimpleValidationResult::new_with_error(e.into()),
        };
        // We could have received a fake commit, so signature validation needs to be returned if error as a simple validation result
        let signature = match dpp::bls_signatures::Signature::from_bytes(self.signature) {
            Ok(signature) => signature,
            Err(e) => {
                return SimpleValidationResult::new_with_error(
                    AbciError::BlsErrorOfTenderdashThresholdMechanism(
                        e,
                        format!("Malformed signature data: {}", hex::encode(self.signature)),
                    ),
                )
            }
        };
        tracing::trace!(
            digest=hex::encode(&digest),
            ?state_id,
            commit = ?v,
            verification_context = ?self,
            "Proof verification"
        );
        match self.public_key.verify(&signature, &digest) {
            true => SimpleValidationResult::default(),
            false => {
                SimpleValidationResult::new_with_error(AbciError::BadCommitSignature(format!(
                    "commit signature {} is wrong",
                    hex::encode(signature.to_bytes().as_slice())
                )))
            }
        }
    }

    /// Verify proof returned by the Platform.
    pub fn verify_proof(&self, app_hash: &[u8], proof: Proof) -> SimpleValidationResult<AbciError> {
        tracing::debug!(?proof, app_hash = hex::encode(app_hash), "verifying proof");

        if self.app_hash != app_hash {
            return SimpleValidationResult::new_with_error(AbciError::InvalidState(
                "Invalid root app hash".to_string(),
            ));
        };

        if proof.signature != self.signature {
            tracing::error!(?proof.signature,?self.signature, "proof signature mismatch");
            return SimpleValidationResult::new_with_error(AbciError::BadCommitSignature(
                "Proof signature mismatch".to_string(),
            ));
        };

        let state_id = StateId {
            app_hash: app_hash.to_vec(),
            app_version: self.app_version,
            core_chain_locked_height: self.core_chain_locked_height,
            height: self.height as u64,
            time: self.time.to_milis(),
        };

        self.verify_signature(state_id, proof.round)
    }
}

impl QueryStrategy {
    pub(crate) fn query_chain_for_strategy(
        &self,
        proof_verification: &ProofVerification,
        current_identities: &Vec<Identity>,
        abci_app: &AbciApplication<MockCoreRPCLike>,
        seed: StrategyRandomness,
        platform_version: &PlatformVersion,
    ) {
        let mut rng = match seed {
            StrategyRandomness::SeedEntropy(seed) => StdRng::seed_from_u64(seed),
            StrategyRandomness::RNGEntropy(rng) => rng,
        };
        let QueryStrategy {
            query_identities_by_public_key_hashes,
        } = self;
        if query_identities_by_public_key_hashes.is_set() {
            Self::query_identities_by_public_key_hashes(
                proof_verification,
                current_identities,
                query_identities_by_public_key_hashes,
                abci_app,
                &mut rng,
                platform_version,
            );
        }
    }

    pub(crate) fn query_identities_by_public_key_hashes(
        proof_verification: &ProofVerification,
        current_identities: &Vec<Identity>,
        frequency: &Frequency,
        abci_app: &AbciApplication<MockCoreRPCLike>,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) {
        let events = frequency.events_if_hit(rng);

        for _i in 0..events {
            let identity_count = rng.gen_range(1..10);
            let chosen_identities = current_identities.choose_multiple(rng, identity_count);
            let public_key_hashes = chosen_identities
                .into_iter()
                .filter_map(|identity| {
                    let unique_public_keys: Vec<_> = identity
                        .public_keys()
                        .iter()
                        .filter(|(_, public_key)| public_key.key_type().is_unique_key_type())
                        .collect();

                    if unique_public_keys.is_empty() {
                        None
                    } else {
                        let key_num = rng.gen_range(0..unique_public_keys.len());
                        let public_key = unique_public_keys[key_num].1;
                        Some((
                            public_key.hash().unwrap(),
                            identity.clone().into_partial_identity_info_no_balance(),
                        ))
                    }
                })
                .collect::<HashMap<[u8; 20], PartialIdentity>>();

            let prove: bool = rng.gen();

            let request = GetIdentitiesByPublicKeyHashesRequest {
                version: Some(get_identities_by_public_key_hashes_request::Version::V0(
                    GetIdentitiesByPublicKeyHashesRequestV0 {
                        public_key_hashes: public_key_hashes
                            .keys()
                            .map(|hash| hash.to_vec())
                            .collect(),
                        prove,
                    },
                )),
            };
            let encoded_request = request.encode_to_vec();
            let query_validation_result = abci_app
                .platform
                .query(
                    "/identities/by-public-key-hash",
                    encoded_request.as_slice(),
                    platform_version,
                )
                .expect("expected to run query");

            assert!(
                query_validation_result.errors.is_empty(),
                "{:?}",
                query_validation_result.errors
            );

            let query_data = query_validation_result
                .into_data()
                .expect("expected data on query_validation_result");
            let response = GetIdentitiesByPublicKeyHashesResponse::decode(query_data.as_slice())
                .expect("expected to deserialize");

            let versioned_result = response.version.expect("expected a result");
            match versioned_result {
                get_identities_by_public_key_hashes_response::Version::V0(v0) => {
                    let result = v0.result.expect("expected a result");

                    match result {
                get_identities_by_public_key_hashes_response::get_identities_by_public_key_hashes_response_v0::Result::Proof(proof) => {
                    let (proof_root_hash, identities): (
                        RootHash,
                        HashMap<[u8; 20], Option<Identity>>,
                    ) = Drive::verify_full_identities_by_public_key_hashes(
                        &proof.grovedb_proof,
                        public_key_hashes
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .as_slice(),
                        platform_version,
                    )
                    .expect("expected to verify proof");
                    let identities: HashMap<[u8; 20], PartialIdentity> = identities
                        .into_iter()
                        .map(|(k, v)| {
                            (
                                k,
                                v.expect("expect an identity")
                                    .into_partial_identity_info_no_balance(),
                            )
                        })
                        .collect();
                    assert_eq!(proof_verification.app_hash, &proof_root_hash);
                    assert!(proof_verification
                        .verify_proof(&proof_root_hash, proof)
                        .is_valid());
                    assert_eq!(identities, public_key_hashes);
                }
                get_identities_by_public_key_hashes_response::get_identities_by_public_key_hashes_response_v0::Result::Identities(data) => {
                    let identities_returned = data
                        .identity_entries
                        .into_iter()
                        .map(|entry| {
                            let PublicKeyHashIdentityEntry{  value, .. } = entry;
                            Identity::deserialize_from_bytes(&value.expect("expected a value"))
                                .expect("expected to deserialize identity")
                                .id()
                        })
                        .collect::<HashSet<_>>();
                    assert_eq!(
                        identities_returned,
                        public_key_hashes
                            .values()
                            .map(|partial_identity| partial_identity.id)
                            .collect()
                    );
                }
            }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::run_chain_for_strategy;

    use crate::strategy::NetworkStrategy;

    use dapi_grpc::platform::v0::get_epochs_info_request::{GetEpochsInfoRequestV0, Version};
    use dapi_grpc::platform::v0::{
        get_epochs_info_response, GetEpochsInfoRequest, GetEpochsInfoResponse,
    };
    use dpp::block::epoch::EpochIndex;
    use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;

    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;

    use dpp::version::PlatformVersion;
    use drive_abci::config::{ExecutionConfig, PlatformConfig, PlatformTestConfig};

    use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;

    use drive_abci::test::helpers::setup::TestPlatformBuilder;

    use strategy_tests::Strategy;

    use crate::strategy::CoreHeightIncrease::RandomCoreHeightIncrease;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{BlockHash, ChainLock};
    use tenderdash_abci::proto::types::CoreChainLock;
    use tenderdash_abci::Application;

    macro_rules! extract_single_variant_or_panic {
        ($expression:expr, $pattern:pat, $binding:ident) => {
            match $expression {
                $pattern => $binding,
                // _ => panic!(
                //     "Expected pattern {} but got another variant",
                //     stringify!($pattern)
                // ),
            }
        };
    }

    macro_rules! extract_variant_or_panic {
        ($expression:expr, $pattern:pat, $binding:ident) => {
            match $expression {
                $pattern => $binding,
                _ => panic!(
                    "Expected pattern {} but got another variant",
                    stringify!($pattern)
                ),
            }
        };
    }

    #[test]
    fn run_chain_query_epoch_info() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: vec![],
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                },
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..3,
                chance_per_block: Some(0.5),
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let hour_in_ms = 1000 * 60 * 60;
        let config = PlatformConfig {
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 100,
                ..Default::default()
            },
            block_spacing_ms: hour_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 1000, strategy, config, 15);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");

        let request = GetEpochsInfoRequest {
            version: Some(Version::V0(GetEpochsInfoRequestV0 {
                start_epoch: None,
                count: 8,
                ascending: true,
                prove: false,
            })),
        }
        .encode_to_vec();

        let platform_state = outcome
            .abci_app
            .platform
            .state
            .read()
            .expect("expected to read state");
        let protocol_version = platform_state.current_protocol_version_in_consensus();
        drop(platform_state);
        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected to get current platform version");
        let validation_result = outcome
            .abci_app
            .platform
            .query("/epochInfos", &request, platform_version)
            .expect("expected query to succeed");
        let response = GetEpochsInfoResponse::decode(
            validation_result.data.expect("expected data").as_slice(),
        )
        .expect("expected to decode response");

        let result = extract_single_variant_or_panic!(
            response.version.expect("expected a versioned response"),
            get_epochs_info_response::Version::V0(inner),
            inner
        )
        .result
        .expect("expected a result");

        let epoch_infos = extract_variant_or_panic!(
            result,
            get_epochs_info_response::get_epochs_info_response_v0::Result::Epochs(inner),
            inner
        );

        // we should have 5 epochs worth of infos

        assert_eq!(epoch_infos.epoch_infos.len(), 5)
    }

    #[test]
    fn run_chain_query_epoch_info_latest() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: vec![],
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                },
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..3,
                chance_per_block: Some(0.5),
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let hour_in_ms = 1000 * 60 * 60;
        let config = PlatformConfig {
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 100,
                ..Default::default()
            },
            block_spacing_ms: hour_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 1000, strategy, config, 15);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");

        let request = GetEpochsInfoRequest {
            version: Some(Version::V0(GetEpochsInfoRequestV0 {
                start_epoch: None,
                count: 1,
                ascending: false,
                prove: false,
            })),
        }
        .encode_to_vec();

        let platform_state = outcome
            .abci_app
            .platform
            .state
            .read()
            .expect("expected to read state");
        let protocol_version = platform_state.current_protocol_version_in_consensus();
        drop(platform_state);
        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected to get current platform version");
        let validation_result = outcome
            .abci_app
            .platform
            .query("/epochInfos", &request, platform_version)
            .expect("expected query to succeed");
        let response = GetEpochsInfoResponse::decode(
            validation_result.data.expect("expected data").as_slice(),
        )
        .expect("expected to decode response");

        let result = extract_single_variant_or_panic!(
            response.version.expect("expected a versioned response"),
            get_epochs_info_response::Version::V0(inner),
            inner
        )
        .result
        .expect("expected a result");

        let epoch_infos = extract_variant_or_panic!(
            result,
            get_epochs_info_response::get_epochs_info_response_v0::Result::Epochs(inner),
            inner
        );

        // we should have 5 epochs worth of infos

        assert_eq!(epoch_infos.epoch_infos.len(), 1);
        assert_eq!(epoch_infos.epoch_infos.first().unwrap().number, 4);
    }

    #[test]
    fn run_chain_prove_epoch_info() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: vec![],
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                },
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..3,
                chance_per_block: Some(0.5),
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let hour_in_ms = 1000 * 60 * 60;
        let config = PlatformConfig {
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 100,
                ..Default::default()
            },
            block_spacing_ms: hour_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 1000, strategy, config, 15);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");

        let request = GetEpochsInfoRequest {
            version: Some(Version::V0(GetEpochsInfoRequestV0 {
                start_epoch: None,
                count: 8,
                ascending: true,
                prove: true,
            })),
        }
        .encode_to_vec();

        let platform_state = outcome
            .abci_app
            .platform
            .state
            .read()
            .expect("expected to read state");
        let protocol_version = platform_state.current_protocol_version_in_consensus();
        let current_epoch = platform_state.last_committed_block_epoch_ref().index;
        drop(platform_state);
        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected to get current platform version");
        let validation_result = outcome
            .abci_app
            .platform
            .query("/epochInfos", &request, platform_version)
            .expect("expected query to succeed");
        let response = GetEpochsInfoResponse::decode(
            validation_result.data.expect("expected data").as_slice(),
        )
        .expect("expected to decode response");

        let result = extract_single_variant_or_panic!(
            response.version.expect("expected a versioned response"),
            get_epochs_info_response::Version::V0(inner),
            inner
        )
        .result
        .expect("expected a result");

        let epoch_infos_proof = extract_variant_or_panic!(
            result,
            get_epochs_info_response::get_epochs_info_response_v0::Result::Proof(inner),
            inner
        );

        let epoch_infos = Drive::verify_epoch_infos(
            epoch_infos_proof.grovedb_proof.as_slice(),
            current_epoch,
            None,
            8,
            true,
            platform_version,
        )
        .expect("expected to verify current epochs")
        .1;

        // we should have 5 epochs worth of infos

        assert_eq!(epoch_infos.len(), 5);

        let request = GetEpochsInfoRequest {
            version: Some(Version::V0(GetEpochsInfoRequestV0 {
                start_epoch: None,
                count: 1,
                ascending: false,
                prove: true,
            })),
        }
        .encode_to_vec();

        let validation_result = outcome
            .abci_app
            .platform
            .query("/epochInfos", &request, platform_version)
            .expect("expected query to succeed");
        let response = GetEpochsInfoResponse::decode(
            validation_result.data.expect("expected data").as_slice(),
        )
        .expect("expected to decode response");

        let get_epochs_info_response::Version::V0(response_v0) =
            response.version.expect("expected a versioned response");

        let result = response_v0.result.expect("expected a result");

        let metadata = response_v0.metadata.expect("expected metadata");

        let epoch_infos_proof = extract_variant_or_panic!(
            result,
            get_epochs_info_response::get_epochs_info_response_v0::Result::Proof(inner),
            inner
        );

        let epoch_infos = Drive::verify_epoch_infos(
            epoch_infos_proof.grovedb_proof.as_slice(),
            metadata.epoch as EpochIndex,
            None,
            1,
            false,
            platform_version,
        )
        .expect("expected to verify current epochs")
        .1;

        assert_eq!(epoch_infos.len(), 1);
        assert_eq!(epoch_infos.first().unwrap().index(), 4);
    }
}
