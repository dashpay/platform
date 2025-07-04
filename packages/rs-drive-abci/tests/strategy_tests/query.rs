use crate::strategy::StrategyRandomness;
use dapi_grpc::platform::v0::get_identity_by_public_key_hash_request::GetIdentityByPublicKeyHashRequestV0;
use dapi_grpc::platform::v0::{
    get_finalized_epoch_infos_request, get_finalized_epoch_infos_response,
    get_finalized_epoch_infos_response::get_finalized_epoch_infos_response_v0,
    GetFinalizedEpochInfosRequest,
};
use dapi_grpc::platform::v0::{
    get_identity_by_public_key_hash_request, get_identity_by_public_key_hash_response,
    GetIdentityByPublicKeyHashRequest, Proof,
};
use dashcore_rpc::dashcore_rpc_json::QuorumType;
use dpp::bls_signatures::{Bls12381G2Impl, BlsError, Pairing, Signature};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::{Identity, PartialIdentity};
use dpp::serialization::PlatformDeserializable;
use dpp::validation::SimpleValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::verify::RootHash;
use drive_abci::abci::app::FullAbciApplication;
use drive_abci::abci::AbciError;
use drive_abci::rpc::core::MockCoreRPCLike;
use rand::prelude::SliceRandom;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use strategy_tests::frequency::Frequency;
use tenderdash_abci::proto::google::protobuf::Timestamp;
use tenderdash_abci::proto::types::{CanonicalVote, SignedMsgType, StateId};
use tenderdash_abci::proto::ToMillis;
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
    pub public_key: &'a dpp::bls_signatures::PublicKey<Bls12381G2Impl>,
}

impl ProofVerification<'_> {
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

        let digest = match v.calculate_sign_hash(
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
        let signature =
            match <Bls12381G2Impl as Pairing>::Signature::from_compressed(self.signature)
                .into_option()
            {
                Some(signature) => Signature::Basic(signature),
                None => {
                    return SimpleValidationResult::new_with_error(
                        AbciError::BlsErrorOfTenderdashThresholdMechanism(
                            BlsError::InvalidSignature,
                            format!("malformed signature data: {}", hex::encode(self.signature)),
                        ),
                    );
                }
            };
        tracing::trace!(
            digest=hex::encode(&digest),
            ?state_id,
            commit = ?v,
            verification_context = ?self,
            "Proof verification"
        );
        match signature.verify(self.public_key, &digest) {
            Ok(_) => SimpleValidationResult::default(),
            Err(e) => SimpleValidationResult::new_with_error(AbciError::BadCommitSignature(
                format!("commit signature {} is wrong: {}", signature, e),
            )),
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
            time: self.time.to_millis().expect("time as milliseconds"),
        };

        self.verify_signature(state_id, proof.round)
    }
}

impl QueryStrategy {
    pub(crate) fn query_chain_for_strategy(
        &self,
        proof_verification: &ProofVerification,
        current_identities: &[Identity],
        abci_app: &FullAbciApplication<MockCoreRPCLike>,
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
        current_identities: &[Identity],
        frequency: &Frequency,
        abci_app: &FullAbciApplication<MockCoreRPCLike>,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) {
        let events = frequency.events_if_hit(rng);

        let platform_state = abci_app.platform.state.load();

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
                            public_key.public_key_hash().unwrap(),
                            identity.clone().into_partial_identity_info_no_balance(),
                        ))
                    }
                })
                .collect::<HashMap<[u8; 20], PartialIdentity>>();

            let prove: bool = rng.gen();

            for (key_hash, expected_identity) in public_key_hashes {
                let request = GetIdentityByPublicKeyHashRequest {
                    version: Some(get_identity_by_public_key_hash_request::Version::V0(
                        GetIdentityByPublicKeyHashRequestV0 {
                            public_key_hash: key_hash.to_vec(),
                            prove,
                        },
                    )),
                };

                let query_validation_result = abci_app
                    .platform
                    .query_identity_by_public_key_hash(request, &platform_state, platform_version)
                    .expect("expected to run query");

                assert!(
                    query_validation_result.errors.is_empty(),
                    "{:?}",
                    query_validation_result.errors
                );

                let response = query_validation_result
                    .into_data()
                    .expect("expected data on query_validation_result");

                let versioned_result = response.version.expect("expected a result");
                match versioned_result {
                    get_identity_by_public_key_hash_response::Version::V0(v0) => {
                        let result = v0.result.expect("expected a result");

                        match result {
                            get_identity_by_public_key_hash_response::get_identity_by_public_key_hash_response_v0::Result::Proof(proof) => {
                                let (proof_root_hash, identity): (
                                    RootHash,
                                    Option<Identity>,
                                ) = Drive::verify_full_identity_by_unique_public_key_hash(
                                    &proof.grovedb_proof,
                                    key_hash,
                                    platform_version,
                                )
                                    .expect("expected to verify proof");
                                let identity = identity.expect("expected an identity")
                                    .into_partial_identity_info_no_balance();
                                assert_eq!(proof_verification.app_hash, &proof_root_hash);
                                assert!(proof_verification
                                    .verify_proof(&proof_root_hash, proof)
                                    .is_valid());
                                assert_eq!(identity, expected_identity);
                            }
                            get_identity_by_public_key_hash_response::get_identity_by_public_key_hash_response_v0::Result::Identity(data) => {
                                let identity_id = Identity::deserialize_from_bytes(&data)
                                    .expect("expected to deserialize identity").id();

                                assert_eq!(identity_id, expected_identity.id);
                            }
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
    use dapi_grpc::platform::v0::get_finalized_epoch_infos_request::GetFinalizedEpochInfosRequestV0;
    use dapi_grpc::platform::v0::{get_epochs_info_response, GetEpochsInfoRequest};
    use dpp::block::epoch::EpochIndex;
    use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;

    use dpp::version::PlatformVersion;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;

    use drive_abci::test::helpers::setup::TestPlatformBuilder;

    use strategy_tests::{IdentityInsertInfo, StartIdentities, Strategy};

    use crate::strategy::CoreHeightIncrease::RandomCoreHeightIncrease;

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
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
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
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: hour_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            1000,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let nodes_with_no_balance = outcome
            .masternode_identity_balances
            .iter()
            .filter(|(_, balance)| *balance == &0)
            .collect::<Vec<_>>();
        assert_eq!(
            nodes_with_no_balance.len(),
            0,
            "all masternodes should have a balance"
        );

        let request = GetEpochsInfoRequest {
            version: Some(Version::V0(GetEpochsInfoRequestV0 {
                start_epoch: None,
                count: 8,
                ascending: true,
                prove: false,
            })),
        };

        let platform_state = outcome.abci_app.platform.state.load();

        let protocol_version = platform_state.current_protocol_version_in_consensus();

        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected to get current platform version");

        let validation_result = outcome
            .abci_app
            .platform
            .query_epoch_infos(request, &platform_state, platform_version)
            .expect("expected query to succeed");

        let response = validation_result.into_data().expect("expected data");

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
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
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
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: hour_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            1000,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );
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
        };

        let platform_state = outcome.abci_app.platform.state.load();

        let protocol_version = platform_state.current_protocol_version_in_consensus();

        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected to get current platform version");

        let validation_result = outcome
            .abci_app
            .platform
            .query_epoch_infos(request, &platform_state, platform_version)
            .expect("expected query to succeed");

        let response = validation_result.into_data().expect("expected data");

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
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
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
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: hour_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            1000,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );
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
        };

        let platform_state = outcome.abci_app.platform.state.load();

        let protocol_version = platform_state.current_protocol_version_in_consensus();

        let current_epoch = platform_state.last_committed_block_epoch_ref().index;

        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected to get current platform version");

        let validation_result = outcome
            .abci_app
            .platform
            .query_epoch_infos(request, &platform_state, platform_version)
            .expect("expected query to succeed");

        let response = validation_result.data.expect("expected data");

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
        };

        let validation_result = outcome
            .abci_app
            .platform
            .query_epoch_infos(request, &platform_state, platform_version)
            .expect("expected query to succeed");

        let response = validation_result.data.expect("expected data");

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

    #[test]
    fn run_chain_prove_finalized_epoch_infos() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
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
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: hour_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            1000,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");

        let platform_state = outcome.abci_app.platform.state.load();

        let protocol_version = platform_state.current_protocol_version_in_consensus();

        let current_epoch = platform_state.last_committed_block_epoch_ref().index;

        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected to get current platform version");

        // Test getting finalized epoch infos
        let request = GetFinalizedEpochInfosRequest {
            version: Some(get_finalized_epoch_infos_request::Version::V0(
                GetFinalizedEpochInfosRequestV0 {
                    start_epoch_index: 0,
                    start_epoch_index_included: true,
                    end_epoch_index: current_epoch as u32,
                    end_epoch_index_included: false,
                    prove: true,
                },
            )),
        };

        let validation_result = outcome
            .abci_app
            .platform
            .query_finalized_epoch_infos(request, &platform_state, platform_version)
            .expect("expected query to succeed");

        let response = validation_result.data.expect("expected data");

        match response.version.expect("expected version") {
            get_finalized_epoch_infos_response::Version::V0(v0) => {
                match v0.result.expect("expected result") {
                    get_finalized_epoch_infos_response_v0::Result::Proof(proof) => {
                        // Verify we got a proof response
                        assert!(!proof.grovedb_proof.is_empty());
                        assert!(!proof.quorum_hash.is_empty());
                        assert!(!proof.signature.is_empty());

                        // Verify the proof
                        let (root_hash, finalized_epoch_infos) =
                            Drive::verify_finalized_epoch_infos(
                                &proof.grovedb_proof,
                                0,
                                true,
                                current_epoch - 1,
                                false,
                                platform_version,
                            )
                            .expect("expected to verify finalized epoch infos");

                        assert!(!finalized_epoch_infos.is_empty());

                        // All returned epochs should be finalized (< current epoch)
                        for (epoch_index, _) in &finalized_epoch_infos {
                            assert!(*epoch_index < current_epoch);
                        }
                    }
                    _ => panic!("expected proof"),
                }
            }
        }

        // Test without proof
        let request_no_proof = GetFinalizedEpochInfosRequest {
            version: Some(get_finalized_epoch_infos_request::Version::V0(
                GetFinalizedEpochInfosRequestV0 {
                    start_epoch_index: 0,
                    start_epoch_index_included: true,
                    end_epoch_index: current_epoch as u32,
                    end_epoch_index_included: false,
                    prove: false,
                },
            )),
        };

        let validation_result_no_proof = outcome
            .abci_app
            .platform
            .query_finalized_epoch_infos(request_no_proof, &platform_state, platform_version)
            .expect("expected query to succeed");

        let response_no_proof = validation_result_no_proof.data.expect("expected data");

        match response_no_proof.version.expect("expected version") {
            get_finalized_epoch_infos_response::Version::V0(v0) => {
                match v0.result.expect("expected result") {
                    get_finalized_epoch_infos_response_v0::Result::Epochs(epochs) => {
                        // Verify we got epoch data
                        assert!(!epochs.finalized_epoch_infos.is_empty());

                        // Verify the epochs are in the expected range
                        for epoch_info in &epochs.finalized_epoch_infos {
                            assert!(epoch_info.number < current_epoch as u32);
                            assert!(epoch_info.first_block_height > 0);
                            assert!(epoch_info.first_core_block_height > 0);
                            assert!(epoch_info.protocol_version > 0);
                            assert!(epoch_info.fee_multiplier >= 1.0);

                            // Check that block proposers exist
                            assert!(
                                !epoch_info.block_proposers.is_empty(),
                                "epoch {} should have block proposers",
                                epoch_info.number
                            );
                        }
                    }
                    _ => panic!("expected epochs"),
                }
            }
        }
    }
}
