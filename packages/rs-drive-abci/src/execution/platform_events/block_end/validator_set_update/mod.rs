mod v0;
mod v1;
mod v2;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::execution::types::block_execution_context::BlockExecutionContext;

use crate::platform_types::platform::Platform;

use crate::platform_types::platform_state::PlatformState;

use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for validator set rotations and performs rotations if necessary.
    ///
    /// This function is a version handler that directs to specific version implementations
    /// of the validator_set_update function.
    ///
    /// # Arguments
    ///
    /// * `platform_state` - A `PlatformState` reference.
    /// * `block_execution_context` - A mutable `BlockExecutionContext` reference.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<Option<ValidatorSetUpdate>, Error>` - If the rotation is successful, it returns `Ok(Some(ValidatorSetUpdate))`
    ///   If there is no update, it returns `Ok(None)`. If there is an error, it returns `Error`.
    ///
    pub fn validator_set_update(
        &self,
        proposer_pro_tx_hash: [u8; 32],
        platform_state: &PlatformState,
        block_execution_context: &mut BlockExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ValidatorSetUpdate>, Error> {
        match platform_version
            .drive_abci
            .methods
            .block_end
            .validator_set_update
        {
            0 => self.validator_set_update_v0(
                proposer_pro_tx_hash,
                platform_state,
                block_execution_context,
            ),
            1 => self.validator_set_update_v1(
                proposer_pro_tx_hash,
                platform_state,
                block_execution_context,
            ),
            2 => self.validator_set_update_v2(
                proposer_pro_tx_hash,
                platform_state,
                block_execution_context,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "validator_set_update".to_string(),
                known_versions: vec![0, 1, 2],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0;
    use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
    use crate::execution::types::block_state_info::BlockStateInfo;
    use crate::platform_types::epoch_info::v0::EpochInfoV0;
    use crate::platform_types::epoch_info::EpochInfo;
    use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use std::collections::BTreeMap;

    fn make_block_execution_context(block_platform_state: PlatformState) -> BlockExecutionContext {
        BlockExecutionContext::V0(BlockExecutionContextV0 {
            block_state_info: BlockStateInfo::V0(BlockStateInfoV0 {
                height: 1,
                round: 0,
                block_time_ms: 1_000_000,
                previous_block_time_ms: None,
                proposer_pro_tx_hash: [0u8; 32],
                core_chain_locked_height: 1,
                block_hash: None,
                app_hash: None,
            }),
            epoch_info: EpochInfo::V0(EpochInfoV0 {
                current_epoch_index: 0,
                previous_epoch_index: None,
                is_epoch_change: false,
            }),
            unsigned_withdrawal_transactions: UnsignedWithdrawalTxs::default(),
            block_address_balance_changes: BTreeMap::new(),
            block_platform_state,
            proposer_results: None,
        })
    }

    #[test]
    fn test_dispatcher_unknown_version_returns_error() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut modified_version = platform_version.clone();
        modified_version
            .drive_abci
            .methods
            .block_end
            .validator_set_update = 255;

        let platform_state = platform.state.load();
        let block_platform_state = platform_state.as_ref().clone();

        let mut block_execution_context = make_block_execution_context(block_platform_state);

        let result = platform.validator_set_update(
            [0u8; 32],
            &platform_state,
            &mut block_execution_context,
            &modified_version,
        );

        assert!(result.is_err());
        match result {
            Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            })) => {
                assert_eq!(method, "validator_set_update");
                assert_eq!(known_versions, vec![0, 1, 2]);
                assert_eq!(received, 255);
            }
            _ => panic!("expected UnknownVersionMismatch error"),
        }
    }

    mod v2_tests {
        use super::*;
        use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
        use crate::platform_types::platform_state::PlatformStateV0Methods;
        use dpp::block::block_info::BlockInfo;
        use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
        use dpp::block::extended_block_info::ExtendedBlockInfo;
        use dpp::bls_signatures::{Bls12381G2Impl, SecretKey};
        use dpp::core_types::validator::v0::ValidatorV0;
        use dpp::core_types::validator_set::v0::ValidatorSetV0;
        use dpp::core_types::validator_set::ValidatorSet;
        use dpp::dashcore::hashes::Hash;
        use dpp::dashcore::{ProTxHash, PubkeyHash, QuorumHash};
        use indexmap::IndexMap;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        /// Initialize a tracing subscriber at DEBUG level so that tracing::debug!
        /// macro arguments are fully evaluated, improving code coverage metrics.
        fn init_debug_tracing() -> tracing::subscriber::DefaultGuard {
            let subscriber = tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .with_test_writer()
                .finish();
            tracing::subscriber::set_default(subscriber)
        }

        /// Helper to create a ValidatorSetV0 with members whose ProTxHash values
        /// are derived from the given byte seeds. Members are inserted into a BTreeMap,
        /// so they are sorted by ProTxHash.
        fn make_validator_set(
            quorum_hash: QuorumHash,
            member_seeds: &[u8],
            rng: &mut StdRng,
        ) -> ValidatorSet {
            let threshold_public_key = SecretKey::<Bls12381G2Impl>::random(&mut *rng).public_key();
            let mut members = BTreeMap::new();
            for &seed in member_seeds {
                let mut hash_bytes = [0u8; 32];
                hash_bytes[31] = seed;
                let pro_tx_hash = ProTxHash::from_byte_array(hash_bytes);
                let public_key = Some(SecretKey::<Bls12381G2Impl>::random(&mut *rng).public_key());
                let node_id = PubkeyHash::from_byte_array([seed; 20]);
                let validator = ValidatorV0 {
                    pro_tx_hash,
                    public_key,
                    node_ip: format!("10.0.0.{}", seed),
                    node_id,
                    core_port: 9999,
                    platform_http_port: 1443,
                    platform_p2p_port: 26656,
                    is_banned: false,
                };
                members.insert(pro_tx_hash, validator);
            }
            ValidatorSet::V0(ValidatorSetV0 {
                quorum_hash,
                quorum_index: None,
                core_height: 100,
                members,
                threshold_public_key,
            })
        }

        fn quorum_hash_from_seed(seed: u8) -> QuorumHash {
            let mut bytes = [0u8; 32];
            bytes[31] = seed;
            QuorumHash::from_byte_array(bytes)
        }

        fn make_extended_block_info(
            quorum_hash: [u8; 32],
            proposer_pro_tx_hash: [u8; 32],
            height: u64,
        ) -> ExtendedBlockInfo {
            ExtendedBlockInfo::V0(ExtendedBlockInfoV0 {
                basic_info: BlockInfo {
                    time_ms: 1_000_000,
                    height,
                    core_height: 100,
                    epoch: Default::default(),
                },
                app_hash: [0u8; 32],
                quorum_hash,
                block_id_hash: [0u8; 32],
                proposer_pro_tx_hash,
                signature: [0u8; 96],
                round: 0,
            })
        }

        /// No rotation needed: proposer is mid-quorum, same quorum, no changes.
        /// Covers the else branch at line 200 and the "no validator set update" path at line 212.
        #[test]
        fn v2_no_rotation_no_changes() {
            let _guard = init_debug_tracing();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(42);
            let qh = quorum_hash_from_seed(1);
            // Members: seeds 10, 20, 30. BTreeMap sorts by ProTxHash bytes.
            // [0..0,10] < [0..0,20] < [0..0,30]
            let vs = make_validator_set(qh, &[10, 20, 30], &mut rng);

            let mut validator_sets = IndexMap::new();
            validator_sets.insert(qh, vs);

            // Set up platform_state: current quorum = qh, proposer = member 20 (middle)
            let mut platform_state = platform.state.load().as_ref().clone();
            platform_state.set_current_validator_set_quorum_hash(qh);
            platform_state.set_validator_sets(validator_sets.clone());
            // last_committed info: same quorum, proposer 10 (before 20 = no wrap)
            let mut proposer_bytes = [0u8; 32];
            proposer_bytes[31] = 10;
            platform_state.set_last_committed_block_info(Some(make_extended_block_info(
                *qh.as_byte_array(),
                proposer_bytes,
                5,
            )));

            // block_platform_state is same as platform_state
            let block_platform_state = platform_state.clone();
            let mut block_execution_context = make_block_execution_context(block_platform_state);

            // proposer is member 20 (not last member = 30, not wrapping around)
            let mut proposer = [0u8; 32];
            proposer[31] = 20;

            let result = platform
                .validator_set_update_v2(proposer, &platform_state, &mut block_execution_context)
                .expect("should succeed");

            assert!(result.is_none(), "no rotation and no changes expected");
        }

        /// No rotation but validator set has changed (e.g., IP changed).
        /// Covers the "validator set update without rotation" path at line 204-211.
        #[test]
        fn v2_no_rotation_with_validator_change() {
            let _guard = init_debug_tracing();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(43);
            let qh = quorum_hash_from_seed(1);
            let vs = make_validator_set(qh, &[10, 20, 30], &mut rng);

            let mut validator_sets = IndexMap::new();
            validator_sets.insert(qh, vs.clone());

            // platform_state has original validator set
            let mut platform_state = platform.state.load().as_ref().clone();
            platform_state.set_current_validator_set_quorum_hash(qh);
            platform_state.set_validator_sets(validator_sets.clone());
            let mut proposer_bytes = [0u8; 32];
            proposer_bytes[31] = 10;
            platform_state.set_last_committed_block_info(Some(make_extended_block_info(
                *qh.as_byte_array(),
                proposer_bytes,
                5,
            )));

            // block_platform_state has a modified validator set (different IP for one member)
            let mut modified_vs = vs.clone();
            let ValidatorSet::V0(ref mut v0) = modified_vs;
            let first_key = *v0.members.keys().next().unwrap();
            v0.members.get_mut(&first_key).unwrap().node_ip = "10.0.0.99".to_string();
            let mut block_validator_sets = IndexMap::new();
            block_validator_sets.insert(qh, modified_vs);

            let mut block_platform_state = platform_state.clone();
            block_platform_state.set_validator_sets(block_validator_sets);

            let mut block_execution_context = make_block_execution_context(block_platform_state);

            // proposer is member 20 (middle, not triggering rotation)
            let mut proposer = [0u8; 32];
            proposer[31] = 20;

            let result = platform
                .validator_set_update_v2(proposer, &platform_state, &mut block_execution_context)
                .expect("should succeed");

            assert!(
                result.is_some(),
                "should return update due to validator change"
            );
        }

        /// Rotation triggered because proposer is the last member of the quorum.
        /// With 2 quorums, should rotate to the next one.
        /// Covers lines 46-62 (last member match) and 134-172 (rotation to next quorum).
        #[test]
        fn v2_rotation_last_member_two_quorums() {
            let _guard = init_debug_tracing();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(44);
            let qh1 = quorum_hash_from_seed(1);
            let qh2 = quorum_hash_from_seed(2);
            let vs1 = make_validator_set(qh1, &[10, 20, 30], &mut rng);
            let vs2 = make_validator_set(qh2, &[40, 50, 60], &mut rng);

            let mut validator_sets = IndexMap::new();
            validator_sets.insert(qh1, vs1.clone());
            validator_sets.insert(qh2, vs2.clone());

            let mut platform_state = platform.state.load().as_ref().clone();
            platform_state.set_current_validator_set_quorum_hash(qh1);
            platform_state.set_validator_sets(validator_sets.clone());
            // Set last committed block info so last_committed_quorum_hash works
            let mut proposer_bytes = [0u8; 32];
            proposer_bytes[31] = 20;
            platform_state.set_last_committed_block_info(Some(make_extended_block_info(
                *qh1.as_byte_array(),
                proposer_bytes,
                5,
            )));

            // block_platform_state has same validator sets
            let block_platform_state = platform_state.clone();
            let mut block_execution_context = make_block_execution_context(block_platform_state);

            // Proposer is the LAST member (seed 30 -> highest ProTxHash in BTreeMap)
            let mut proposer = [0u8; 32];
            proposer[31] = 30;

            let result = platform
                .validator_set_update_v2(proposer, &platform_state, &mut block_execution_context)
                .expect("should succeed");

            assert!(result.is_some(), "should rotate to next quorum");

            // Verify the next quorum hash was set on block execution context
            let next_qh = block_execution_context
                .block_platform_state()
                .next_validator_set_quorum_hash();
            assert_eq!(*next_qh, Some(qh2));
        }

        /// Rotation triggered because the validator set has no members (empty quorum).
        /// With only 1 quorum in system -> quorum_count == 1 -> Ok(None).
        /// Covers lines 63-70 (no members) and line 133 (single quorum = no rotation).
        #[test]
        fn v2_rotation_empty_members_single_quorum() {
            let _guard = init_debug_tracing();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(45);
            let qh = quorum_hash_from_seed(1);
            let threshold_pk = SecretKey::<Bls12381G2Impl>::random(&mut rng).public_key();
            let empty_vs = ValidatorSet::V0(ValidatorSetV0 {
                quorum_hash: qh,
                quorum_index: None,
                core_height: 100,
                members: BTreeMap::new(),
                threshold_public_key: threshold_pk,
            });

            let mut validator_sets = IndexMap::new();
            validator_sets.insert(qh, empty_vs);

            let mut platform_state = platform.state.load().as_ref().clone();
            platform_state.set_current_validator_set_quorum_hash(qh);
            platform_state.set_validator_sets(validator_sets.clone());

            let block_platform_state = platform_state.clone();
            let mut block_execution_context = make_block_execution_context(block_platform_state);

            let result = platform
                .validator_set_update_v2([0u8; 32], &platform_state, &mut block_execution_context)
                .expect("should succeed");

            // Only one quorum, so no rotation target
            assert!(result.is_none(), "single quorum means no rotation target");
        }

        /// Rotation triggered because proposer wraps around (new proposer < last proposer)
        /// with more than one quorum available.
        /// Covers lines 76-99 (wrap-around detection).
        #[test]
        fn v2_rotation_proposer_wrap_around() {
            let _guard = init_debug_tracing();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(46);
            let qh1 = quorum_hash_from_seed(1);
            let qh2 = quorum_hash_from_seed(2);
            let vs1 = make_validator_set(qh1, &[10, 20, 30], &mut rng);
            let vs2 = make_validator_set(qh2, &[40, 50, 60], &mut rng);

            let mut validator_sets = IndexMap::new();
            validator_sets.insert(qh1, vs1);
            validator_sets.insert(qh2, vs2);

            let mut platform_state = platform.state.load().as_ref().clone();
            platform_state.set_current_validator_set_quorum_hash(qh1);
            platform_state.set_validator_sets(validator_sets.clone());

            // last committed: same quorum (qh1), proposer = 20
            let mut last_proposer = [0u8; 32];
            last_proposer[31] = 20;
            platform_state.set_last_committed_block_info(Some(make_extended_block_info(
                *qh1.as_byte_array(),
                last_proposer,
                5,
            )));

            let block_platform_state = platform_state.clone();
            let mut block_execution_context = make_block_execution_context(block_platform_state);

            // New proposer = 10 which is LESS than last proposer = 20 -> wrap around
            // Same quorum and >1 quorum in system -> triggers rotation
            let mut proposer = [0u8; 32];
            proposer[31] = 10;

            let result = platform
                .validator_set_update_v2(proposer, &platform_state, &mut block_execution_context)
                .expect("should succeed");

            assert!(result.is_some(), "should rotate due to wrap-around");
        }

        /// Rotation triggered because the current quorum is not in the new block's validator sets.
        /// Covers lines 100-113 (quorum removed from block state).
        #[test]
        fn v2_rotation_quorum_removed() {
            let _guard = init_debug_tracing();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(47);
            let qh1 = quorum_hash_from_seed(1);
            let qh2 = quorum_hash_from_seed(2);
            let qh3 = quorum_hash_from_seed(3);
            let vs1 = make_validator_set(qh1, &[10, 20, 30], &mut rng);
            let vs2 = make_validator_set(qh2, &[40, 50, 60], &mut rng);
            let vs3 = make_validator_set(qh3, &[70, 80, 90], &mut rng);

            // platform_state has qh1, qh2
            let mut platform_validator_sets = IndexMap::new();
            platform_validator_sets.insert(qh1, vs1);
            platform_validator_sets.insert(qh2, vs2);

            let mut platform_state = platform.state.load().as_ref().clone();
            platform_state.set_current_validator_set_quorum_hash(qh1);
            platform_state.set_validator_sets(platform_validator_sets);

            // block_platform_state does NOT have qh1, has qh2, qh3 instead
            let mut block_validator_sets = IndexMap::new();
            block_validator_sets.insert(qh2, make_validator_set(qh2, &[40, 50, 60], &mut rng));
            block_validator_sets.insert(qh3, vs3);

            let mut block_platform_state = platform_state.clone();
            block_platform_state.set_validator_sets(block_validator_sets);

            let mut block_execution_context = make_block_execution_context(block_platform_state);

            let result = platform
                .validator_set_update_v2([0u8; 32], &platform_state, &mut block_execution_context)
                .expect("should succeed");

            assert!(
                result.is_some(),
                "should rotate because current quorum was removed"
            );
            // Should rotate to qh2 (next index after qh1 that still exists in block state)
            let next_qh = block_execution_context
                .block_platform_state()
                .next_validator_set_quorum_hash();
            assert_eq!(*next_qh, Some(qh2));
        }

        /// All quorums changed: platform_state quorums don't exist in block state.
        /// Falls through the rotation loop and hits the "all quorums changed" fallback.
        /// Covers lines 179-194.
        #[test]
        fn v2_rotation_all_quorums_changed() {
            let _guard = init_debug_tracing();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(48);
            let qh1 = quorum_hash_from_seed(1);
            let qh2 = quorum_hash_from_seed(2);
            let vs1 = make_validator_set(qh1, &[10, 20, 30], &mut rng);
            let vs2 = make_validator_set(qh2, &[40, 50, 60], &mut rng);

            let mut platform_validator_sets = IndexMap::new();
            platform_validator_sets.insert(qh1, vs1);
            platform_validator_sets.insert(qh2, vs2);

            let mut platform_state = platform.state.load().as_ref().clone();
            platform_state.set_current_validator_set_quorum_hash(qh1);
            platform_state.set_validator_sets(platform_validator_sets);

            // block state has entirely different quorums
            let qh3 = quorum_hash_from_seed(3);
            let qh4 = quorum_hash_from_seed(4);
            let vs3 = make_validator_set(qh3, &[70, 80, 90], &mut rng);
            let vs4 = make_validator_set(qh4, &[100, 110, 120], &mut rng);

            let mut block_validator_sets = IndexMap::new();
            block_validator_sets.insert(qh3, vs3);
            block_validator_sets.insert(qh4, vs4);

            let mut block_platform_state = platform_state.clone();
            block_platform_state.set_validator_sets(block_validator_sets);

            let mut block_execution_context = make_block_execution_context(block_platform_state);

            // Trigger rotation via quorum removal (qh1 not in block state)
            let result = platform
                .validator_set_update_v2([0u8; 32], &platform_state, &mut block_execution_context)
                .expect("should succeed");

            assert!(result.is_some(), "should rotate to first of new quorums");
            // Should pick the first quorum in block state
            let next_qh = block_execution_context
                .block_platform_state()
                .next_validator_set_quorum_hash();
            assert_eq!(*next_qh, Some(qh3));
        }

        /// Rotation with > 10 quorums: tests the `oldest_quorum_index_we_can_go_to = count - 2`
        /// branch.
        /// Covers lines 136-142 (count > 10 branch).
        #[test]
        fn v2_rotation_more_than_ten_quorums() {
            let _guard = init_debug_tracing();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49);

            // Create 12 quorums
            let mut validator_sets = IndexMap::new();
            let mut quorum_hashes = Vec::new();
            for i in 1..=12u8 {
                let qh = quorum_hash_from_seed(i);
                quorum_hashes.push(qh);
                let base_seed = i * 10;
                let vs =
                    make_validator_set(qh, &[base_seed, base_seed + 1, base_seed + 2], &mut rng);
                validator_sets.insert(qh, vs);
            }

            let mut platform_state = platform.state.load().as_ref().clone();
            platform_state.set_current_validator_set_quorum_hash(quorum_hashes[0]);
            platform_state.set_validator_sets(validator_sets.clone());

            let block_platform_state = platform_state.clone();
            let mut block_execution_context = make_block_execution_context(block_platform_state);

            // Propose the last member of qh[0] to trigger rotation via last-member path
            // The members are sorted by ProTxHash. For seeds [10,11,12],
            // the last is seed 12.
            let mut proposer = [0u8; 32];
            proposer[31] = 12;

            let result = platform
                .validator_set_update_v2(proposer, &platform_state, &mut block_execution_context)
                .expect("should succeed");

            assert!(result.is_some(), "should rotate with >10 quorums");
            // With >10 quorums, oldest_quorum_index_we_can_go_to = 12 - 2 = 10
            // current index = 0, next = 1, which is quorum_hashes[1]
            let next_qh = block_execution_context
                .block_platform_state()
                .next_validator_set_quorum_hash();
            assert_eq!(*next_qh, Some(quorum_hashes[1]));
        }

        /// Rotation with > 10 quorums where current quorum is near the end,
        /// so index wraps back to 0.
        /// Covers the wrap-around in lines 143-147 (index + 1 >= oldest_quorum_index_we_can_go_to).
        #[test]
        fn v2_rotation_more_than_ten_quorums_wrap_to_zero() {
            let _guard = init_debug_tracing();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(50);

            // Create 12 quorums
            let mut validator_sets = IndexMap::new();
            let mut quorum_hashes = Vec::new();
            for i in 1..=12u8 {
                let qh = quorum_hash_from_seed(i);
                quorum_hashes.push(qh);
                let base_seed = i * 10;
                let vs =
                    make_validator_set(qh, &[base_seed, base_seed + 1, base_seed + 2], &mut rng);
                validator_sets.insert(qh, vs);
            }

            // Set current quorum to index 9 (0-based), so index+1 = 10 >= count-2 = 10
            // This should wrap to index 0
            let mut platform_state = platform.state.load().as_ref().clone();
            platform_state.set_current_validator_set_quorum_hash(quorum_hashes[9]);
            platform_state.set_validator_sets(validator_sets.clone());

            let block_platform_state = platform_state.clone();
            let mut block_execution_context = make_block_execution_context(block_platform_state);

            // Trigger rotation: use last member of quorum at index 9
            // quorum_hashes[9] has seeds [100, 101, 102], last = 102
            let mut proposer = [0u8; 32];
            proposer[31] = 102;

            let result = platform
                .validator_set_update_v2(proposer, &platform_state, &mut block_execution_context)
                .expect("should succeed");

            assert!(result.is_some(), "should rotate with wrap to index 0");
            let next_qh = block_execution_context
                .block_platform_state()
                .next_validator_set_quorum_hash();
            // Should wrap to index 0 = quorum_hashes[0]
            assert_eq!(*next_qh, Some(quorum_hashes[0]));
        }

        /// Rotation loop: next quorum in platform_state was removed from block state,
        /// so the loop continues to the one after.
        /// Covers the loop continuation path at lines 173-176.
        #[test]
        fn v2_rotation_skip_removed_quorum_in_loop() {
            let _guard = init_debug_tracing();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(51);
            let qh1 = quorum_hash_from_seed(1);
            let qh2 = quorum_hash_from_seed(2);
            let qh3 = quorum_hash_from_seed(3);
            let vs1 = make_validator_set(qh1, &[10, 20, 30], &mut rng);
            let vs2 = make_validator_set(qh2, &[40, 50, 60], &mut rng);
            let vs3 = make_validator_set(qh3, &[70, 80, 90], &mut rng);

            // platform_state has all three quorums
            let mut platform_validator_sets = IndexMap::new();
            platform_validator_sets.insert(qh1, vs1);
            platform_validator_sets.insert(qh2, vs2);
            platform_validator_sets.insert(qh3, vs3.clone());

            let mut platform_state = platform.state.load().as_ref().clone();
            platform_state.set_current_validator_set_quorum_hash(qh1);
            platform_state.set_validator_sets(platform_validator_sets);

            // block state has qh1 and qh3 but NOT qh2
            // So when rotating from qh1, index goes to qh2 (not found), then qh3 (found)
            let mut block_validator_sets = IndexMap::new();
            block_validator_sets.insert(qh1, make_validator_set(qh1, &[10, 20, 30], &mut rng));
            block_validator_sets.insert(qh3, make_validator_set(qh3, &[70, 80, 90], &mut rng));

            let mut block_platform_state = platform_state.clone();
            block_platform_state.set_validator_sets(block_validator_sets);

            let mut block_execution_context = make_block_execution_context(block_platform_state);

            // Trigger rotation via last member of qh1 (seed 30)
            let mut proposer = [0u8; 32];
            proposer[31] = 30;

            let result = platform
                .validator_set_update_v2(proposer, &platform_state, &mut block_execution_context)
                .expect("should succeed");

            assert!(
                result.is_some(),
                "should skip removed qh2 and rotate to qh3"
            );
            let next_qh = block_execution_context
                .block_platform_state()
                .next_validator_set_quorum_hash();
            assert_eq!(*next_qh, Some(qh3));
        }

        /// Rotation with empty block state quorums: no new quorums to choose from.
        /// Covers line 196-197 ("no new quorums to choose from" -> Ok(None)).
        #[test]
        fn v2_rotation_no_new_quorums_available() {
            let _guard = init_debug_tracing();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(52);
            let qh1 = quorum_hash_from_seed(1);
            let qh2 = quorum_hash_from_seed(2);
            let vs1 = make_validator_set(qh1, &[10, 20, 30], &mut rng);
            let vs2 = make_validator_set(qh2, &[40, 50, 60], &mut rng);

            let mut platform_validator_sets = IndexMap::new();
            platform_validator_sets.insert(qh1, vs1);
            platform_validator_sets.insert(qh2, vs2);

            let mut platform_state = platform.state.load().as_ref().clone();
            platform_state.set_current_validator_set_quorum_hash(qh1);
            platform_state.set_validator_sets(platform_validator_sets);

            // block state has NO quorums at all
            let mut block_platform_state = platform_state.clone();
            block_platform_state.set_validator_sets(IndexMap::new());

            let mut block_execution_context = make_block_execution_context(block_platform_state);

            let result = platform
                .validator_set_update_v2([0u8; 32], &platform_state, &mut block_execution_context)
                .expect("should succeed");

            // All quorums changed path, block state empty, no first() -> Ok(None)
            assert!(
                result.is_none(),
                "no new quorums available, should return None"
            );
        }

        /// Rotation triggered, but quorum_count == 0 in platform_state.
        /// This requires an unusual setup: current quorum hash is in validator_sets
        /// but after get_index_of succeeds, the len() is 0. This is actually impossible
        /// in practice (if get_index_of succeeds, len >= 1), so let's test the
        /// CorruptedCachedState error when current quorum hash is NOT in platform_state.
        /// Covers lines 119-126 (error when current quorum not found in platform_state).
        #[test]
        fn v2_rotation_current_quorum_not_in_platform_state() {
            let _guard = init_debug_tracing();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(53);
            let qh1 = quorum_hash_from_seed(1);
            let qh2 = quorum_hash_from_seed(2);
            let vs2 = make_validator_set(qh2, &[40, 50, 60], &mut rng);

            // platform_state: current quorum is qh1, but validator_sets only has qh2
            let mut platform_validator_sets = IndexMap::new();
            platform_validator_sets.insert(qh2, vs2.clone());

            let mut platform_state = platform.state.load().as_ref().clone();
            platform_state.set_current_validator_set_quorum_hash(qh1);
            platform_state.set_validator_sets(platform_validator_sets);

            // block_platform_state has qh2 but not qh1 -> triggers rotation
            let mut block_validator_sets = IndexMap::new();
            block_validator_sets.insert(qh2, vs2);

            let mut block_platform_state = platform_state.clone();
            block_platform_state.set_validator_sets(block_validator_sets);

            let mut block_execution_context = make_block_execution_context(block_platform_state);

            let result = platform.validator_set_update_v2(
                [0u8; 32],
                &platform_state,
                &mut block_execution_context,
            );

            assert!(
                result.is_err(),
                "should error: current quorum not in platform_state validator_sets"
            );
            match result {
                Err(Error::Execution(ExecutionError::CorruptedCachedState(msg))) => {
                    assert!(
                        msg.contains("perform_rotation"),
                        "error should mention perform_rotation: {}",
                        msg
                    );
                }
                other => panic!("expected CorruptedCachedState error, got {:?}", other),
            }
        }

        /// Rotation with exactly 1 quorum: last member triggers rotation but
        /// there's only one quorum so result is Ok(None).
        /// Covers line 133 (count == 1 -> no rotation possible).
        #[test]
        fn v2_rotation_single_quorum_last_member() {
            let _guard = init_debug_tracing();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(54);
            let qh = quorum_hash_from_seed(1);
            let vs = make_validator_set(qh, &[10, 20, 30], &mut rng);

            let mut validator_sets = IndexMap::new();
            validator_sets.insert(qh, vs);

            let mut platform_state = platform.state.load().as_ref().clone();
            platform_state.set_current_validator_set_quorum_hash(qh);
            platform_state.set_validator_sets(validator_sets.clone());

            let block_platform_state = platform_state.clone();
            let mut block_execution_context = make_block_execution_context(block_platform_state);

            // Proposer is the last member (seed 30)
            let mut proposer = [0u8; 32];
            proposer[31] = 30;

            let result = platform
                .validator_set_update_v2(proposer, &platform_state, &mut block_execution_context)
                .expect("should succeed");

            assert!(
                result.is_none(),
                "single quorum: rotation triggers but no target"
            );
        }

        /// Wrap-around detection does NOT trigger with only 1 quorum.
        /// Even though new_proposer < last_proposer, validator_sets().len() == 1.
        /// Covers the `platform_state.validator_sets().len() > 1` guard at line 81.
        #[test]
        fn v2_wrap_around_not_triggered_single_quorum() {
            let _guard = init_debug_tracing();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(55);
            let qh = quorum_hash_from_seed(1);
            let vs = make_validator_set(qh, &[10, 20, 30], &mut rng);

            let mut validator_sets = IndexMap::new();
            validator_sets.insert(qh, vs);

            let mut platform_state = platform.state.load().as_ref().clone();
            platform_state.set_current_validator_set_quorum_hash(qh);
            platform_state.set_validator_sets(validator_sets.clone());
            // Last committed on same quorum, proposer = 20
            let mut last_proposer = [0u8; 32];
            last_proposer[31] = 20;
            platform_state.set_last_committed_block_info(Some(make_extended_block_info(
                *qh.as_byte_array(),
                last_proposer,
                5,
            )));

            let block_platform_state = platform_state.clone();
            let mut block_execution_context = make_block_execution_context(block_platform_state);

            // New proposer = 10 < last proposer = 20, but only 1 quorum -> no rotation
            let mut proposer = [0u8; 32];
            proposer[31] = 10;

            let result = platform
                .validator_set_update_v2(proposer, &platform_state, &mut block_execution_context)
                .expect("should succeed");

            assert!(
                result.is_none(),
                "wrap-around should not trigger rotation with single quorum"
            );
        }

        /// Wrap-around detection does NOT trigger when quorum changed between blocks
        /// (last_committed_quorum_hash != current_validator_set_quorum_hash).
        /// Covers the first condition at line 76-79.
        #[test]
        fn v2_wrap_around_not_triggered_different_quorum() {
            let _guard = init_debug_tracing();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(56);
            let qh1 = quorum_hash_from_seed(1);
            let qh2 = quorum_hash_from_seed(2);
            let vs1 = make_validator_set(qh1, &[10, 20, 30], &mut rng);
            let vs2 = make_validator_set(qh2, &[40, 50, 60], &mut rng);

            let mut validator_sets = IndexMap::new();
            validator_sets.insert(qh1, vs1);
            validator_sets.insert(qh2, vs2);

            let mut platform_state = platform.state.load().as_ref().clone();
            platform_state.set_current_validator_set_quorum_hash(qh1);
            platform_state.set_validator_sets(validator_sets.clone());
            // Last committed was on DIFFERENT quorum (qh2), proposer = 20
            let mut last_proposer = [0u8; 32];
            last_proposer[31] = 20;
            platform_state.set_last_committed_block_info(Some(make_extended_block_info(
                *qh2.as_byte_array(),
                last_proposer,
                5,
            )));

            let block_platform_state = platform_state.clone();
            let mut block_execution_context = make_block_execution_context(block_platform_state);

            // New proposer = 10 < last proposer = 20 but different quorum -> no wrap
            let mut proposer = [0u8; 32];
            proposer[31] = 10;

            let result = platform
                .validator_set_update_v2(proposer, &platform_state, &mut block_execution_context)
                .expect("should succeed");

            assert!(
                result.is_none(),
                "wrap-around should not trigger when last block was on different quorum"
            );
        }

        /// run_block_proposal v1 (protocol v15) moves `validator_set_update` from AFTER
        /// the root-hash computation (its v0 position) to BEFORE it, so the reduced
        /// platform state written into the replicated state can carry the post-rotation
        /// next validator set. The only observable differences between the two call
        /// sites are (a) `block_state_info.app_hash` being set and (b) grovedb having
        /// received additional writes in between. Rotation reads neither, and this test
        /// proves it: for rotation-triggering and non-triggering scenarios alike, the
        /// rotation outcome (returned update and resulting next validator set quorum
        /// hash) is identical whether or not the app hash was set and grovedb was
        /// written to before the call.
        #[test]
        fn v2_rotation_outcome_is_independent_of_root_hash_ordering() {
            use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0MutableGetters;
            use crate::execution::types::block_state_info::v0::BlockStateInfoV0Setters;
            use dpp::reduced_platform_state::v0::ReducedBlockInfoV0;

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();
            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(57);
            let qh1 = quorum_hash_from_seed(1);
            let qh2 = quorum_hash_from_seed(2);
            let vs1 = make_validator_set(qh1, &[10, 20, 30], &mut rng);
            let vs2 = make_validator_set(qh2, &[40, 50, 60], &mut rng);

            let mut validator_sets = IndexMap::new();
            validator_sets.insert(qh1, vs1);
            validator_sets.insert(qh2, vs2);

            // Scenarios: (proposer seed, last committed proposer seed, description)
            // - proposer 20 after 10: mid-quorum, no rotation
            // - proposer 30 after 20: last member, rotation to qh2
            // - proposer 10 after 20: wrap-around, rotation to qh2
            let scenarios: [(u8, u8, &str); 3] = [
                (20, 10, "no rotation"),
                (30, 20, "rotation on last member"),
                (10, 20, "rotation on wrap-around"),
            ];

            for (proposer_seed, last_proposer_seed, description) in scenarios {
                let mut platform_state = platform.state.load().as_ref().clone();
                platform_state.set_current_validator_set_quorum_hash(qh1);
                platform_state.set_validator_sets(validator_sets.clone());
                let mut last_proposer = [0u8; 32];
                last_proposer[31] = last_proposer_seed;
                platform_state.set_last_committed_block_info(Some(make_extended_block_info(
                    *qh1.as_byte_array(),
                    last_proposer,
                    5,
                )));

                let mut proposer = [0u8; 32];
                proposer[31] = proposer_seed;

                // v1 ordering: rotation runs before the root hash exists and before any
                // reduced-state write.
                let mut context_before_root_hash =
                    make_block_execution_context(platform_state.clone());
                let update_before = platform
                    .validator_set_update_v2(
                        proposer,
                        &platform_state,
                        &mut context_before_root_hash,
                    )
                    .expect("should succeed before root hash");

                // v0 ordering: by the time rotation runs, the app hash has been computed
                // and set, and grovedb has received the block's writes (simulated here by
                // a committed reduced-state write).
                let reduced_platform_state = platform_state.to_reduced_platform_state(
                    ReducedBlockInfoV0 {
                        basic_info: BlockInfo::default(),
                        app_hash: None,
                        quorum_hash: (*qh1.as_byte_array()).into(),
                        block_id_hash: None,
                        proposer_pro_tx_hash: proposer.into(),
                        signature: None,
                        round: 0,
                    },
                    1,
                );
                platform
                    .store_reduced_platform_state(&reduced_platform_state, None, platform_version)
                    .expect("should store reduced platform state");
                let mut context_after_root_hash =
                    make_block_execution_context(platform_state.clone());
                context_after_root_hash
                    .block_state_info_mut()
                    .set_app_hash(Some([9u8; 32]));
                let update_after = platform
                    .validator_set_update_v2(
                        proposer,
                        &platform_state,
                        &mut context_after_root_hash,
                    )
                    .expect("should succeed after root hash");

                assert_eq!(
                    update_before, update_after,
                    "validator set update must not depend on call ordering ({})",
                    description
                );
                assert_eq!(
                    context_before_root_hash
                        .block_platform_state()
                        .next_validator_set_quorum_hash(),
                    context_after_root_hash
                        .block_platform_state()
                        .next_validator_set_quorum_hash(),
                    "next validator set quorum hash must not depend on call ordering ({})",
                    description
                );
                assert_eq!(
                    context_before_root_hash
                        .block_platform_state()
                        .current_validator_set_quorum_hash(),
                    context_after_root_hash
                        .block_platform_state()
                        .current_validator_set_quorum_hash(),
                    "current validator set quorum hash must not depend on call ordering ({})",
                    description
                );
            }
        }
    }
}
