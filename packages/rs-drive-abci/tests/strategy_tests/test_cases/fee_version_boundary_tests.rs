//! Replay coverage across a fee-generation boundary.
//!
//! The doubled-storage mock protocol version (`TEST_PLATFORM_V4`) is the
//! latest shipped tables with a fee generation that exists only under
//! `mock-versions`, so upgrading to it changes nothing but the storage rate
//! and the fee history entry the epoch-change hook records. Each simulation
//! runs one workload twice: continuously, and stopped at the same block,
//! reopened from the persisted state and continued. The two runs must agree
//! on every root hash, every transition result, every identity balance and
//! the fee history; every block also re-runs `process_proposal` as an
//! independent validator, which is the proposer versus validator parity.

#[cfg(test)]
mod tests {
    use crate::addresses_with_balance::AddressesWithBalance;
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
    use crate::strategy::{
        ChainExecutionOutcome, ChainExecutionParameters, NetworkStrategy, StrategyRandomness,
        UpgradingInfo,
    };
    use dash_platform_macros::stack_size;
    use dpp::block::epoch::EpochIndex;
    use dpp::dash_to_duffs;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::{
        DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::fee::Credits;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::platform_value::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
    use dpp::state_transition::batch_transition::batched_transition::document_transition::{
        DocumentTransition, DocumentTransitionV0Methods,
    };
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
    use dpp::state_transition::StateTransition;
    use dpp::tests::json_document::json_document_to_created_contract;
    use dpp::version::fee::FeeVersionNumber;
    use dpp::version::PlatformVersion;
    use drive_abci::abci::app::FullAbciApplication;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use platform_version::version::mocks::fee_doubled_storage_test::TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE;
    use platform_version::version::mocks::v4_test::TEST_PROTOCOL_VERSION_4;
    use std::collections::{BTreeMap, BTreeSet};
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{DocumentAction, DocumentOp, Operation, OperationType};
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};
    use tenderdash_abci::proto::abci::ExecTxResult;

    /// Sixty blocks per epoch: `epoch_time_length_s` of 60 at one block per second.
    const BLOCKS_PER_EPOCH: u64 = 60;
    const SEED: u64 = 41;

    /// What a run leaves behind, in the fields the two runs must agree on.
    #[derive(Debug, PartialEq, Eq)]
    struct Snapshot {
        root_hash: [u8; 32],
        height: u64,
        protocol_version: u32,
        /// Per block: the code and gas of every transition the proposer kept.
        results_per_block: BTreeMap<u64, Vec<(u32, i64)>>,
        /// Per block of the second segment: the bytes of every transition the
        /// strategy submitted, which proves the two runs executed one workload.
        submitted_per_block: BTreeMap<u64, Vec<Vec<u8>>>,
        balances: BTreeMap<[u8; 32], Credits>,
        fee_history: Vec<(EpochIndex, FeeVersionNumber)>,
    }

    /// Document ids created or deleted by the kept transitions of a run,
    /// keyed by block.
    #[derive(Default)]
    struct DocumentActivity {
        created: BTreeMap<u64, BTreeSet<Identifier>>,
        deleted: BTreeMap<u64, BTreeSet<Identifier>>,
    }

    fn document_activity(
        results_per_block: &BTreeMap<u64, Vec<(StateTransition, ExecTxResult)>>,
    ) -> DocumentActivity {
        let mut activity = DocumentActivity::default();
        for (height, results) in results_per_block {
            for (state_transition, result) in results {
                let StateTransition::Batch(batch) = state_transition else {
                    continue;
                };
                if result.code != 0 {
                    continue;
                }
                for transition in batch.transitions_iter() {
                    let BatchedTransitionRef::Document(document_transition) = transition else {
                        continue;
                    };
                    let bucket = match document_transition {
                        DocumentTransition::Create(_) => &mut activity.created,
                        DocumentTransition::Delete(_) => &mut activity.deleted,
                        _ => continue,
                    };
                    bucket
                        .entry(*height)
                        .or_default()
                        .insert(document_transition.get_id());
                }
            }
        }
        activity
    }

    fn platform_config() -> PlatformConfig {
        PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_size: 30,
                ..Default::default()
            },
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                epoch_time_length_s: BLOCKS_PER_EPOCH,
                ..Default::default()
            },
            block_spacing_ms: 1_000,
            testing_configs: PlatformTestConfig {
                store_platform_state: true,
                ..PlatformTestConfig::default_minimal_verifications()
            },
            ..Default::default()
        }
    }

    /// One identity per block, one to two random inserts and one delete per
    /// block on the all-mutable DashPay contract, so documents are written and
    /// removed on both sides of every boundary the tests cross.
    fn network_strategy(upgrading_info: Option<UpgradingInfo>) -> NetworkStrategy {
        let platform_version = PlatformVersion::latest();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");
        let contract = created_contract.data_contract();
        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected a contactRequest document type")
            .to_owned_document_type();

        NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract.clone(), None)],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(DocumentOp {
                            contract: contract.clone(),
                            action: DocumentAction::DocumentActionInsertRandom(
                                DocumentFieldFillType::FillIfNotRequired,
                                DocumentFieldFillSize::AnyDocumentFillSize,
                            ),
                            document_type: document_type.clone(),
                        }),
                        frequency: Frequency {
                            times_per_block_range: 1..3,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(DocumentOp {
                            contract: contract.clone(),
                            action: DocumentAction::DocumentActionDelete,
                            document_type,
                        }),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
                    ..Default::default()
                },
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 50,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info,
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            independent_process_proposal_verification: true,
            ..Default::default()
        }
    }

    fn snapshot(
        abci_app: &FullAbciApplication<'_, drive_abci::rpc::core::MockCoreRPCLike>,
        identities: &[dpp::identity::Identity],
        results_per_block: &BTreeMap<u64, Vec<(StateTransition, ExecTxResult)>>,
        submitted_per_block: &BTreeMap<u64, Vec<StateTransition>>,
        second_segment_start: u64,
    ) -> Snapshot {
        let state = abci_app.platform.state.load();
        let platform_version = state
            .current_platform_version()
            .expect("expected the state's platform version");
        let balances = abci_app
            .platform
            .drive
            .fetch_identities_balances(
                &identities
                    .iter()
                    .map(|identity| identity.id().to_buffer())
                    .collect(),
                None,
                platform_version,
            )
            .expect("expected to fetch identity balances");
        assert_eq!(
            balances.len(),
            identities.len(),
            "every identity the strategy created must have a balance"
        );
        Snapshot {
            root_hash: state
                .last_committed_block_app_hash()
                .expect("expected a committed block"),
            height: state.last_committed_block_height(),
            protocol_version: state.current_protocol_version_in_consensus(),
            results_per_block: results_per_block
                .iter()
                .map(|(height, results)| {
                    (
                        *height,
                        results
                            .iter()
                            .map(|(_, result)| (result.code, result.gas_used))
                            .collect(),
                    )
                })
                .collect(),
            submitted_per_block: submitted_per_block
                .iter()
                .filter(|(height, _)| **height >= second_segment_start)
                .map(|(height, transitions)| {
                    (
                        *height,
                        transitions
                            .iter()
                            .map(|transition| {
                                transition
                                    .serialize_to_bytes()
                                    .expect("expected to serialize the transition")
                            })
                            .collect(),
                    )
                })
                .collect(),
            balances,
            fee_history: state
                .previous_fee_versions()
                .iter()
                .map(|(epoch_index, fee_version)| (*epoch_index, fee_version.fee_version_number))
                .collect(),
        }
    }

    struct SplitRun {
        snapshot: Snapshot,
        activity: DocumentActivity,
        /// Fee history as the first segment left it.
        fee_history_at_split: Vec<(EpochIndex, FeeVersionNumber)>,
        protocol_version_at_split: u32,
    }

    /// Runs `first_blocks` blocks, then `second_blocks` more with the same
    /// workload. With `reopen` the platform is dropped between the segments
    /// and rebuilt from what it persisted, so the second segment runs on the
    /// saved state instead of the in-memory one.
    ///
    /// The continuation is workload-preserving: the mutated strategy of the
    /// first segment (its start contracts already deployed and its operations
    /// remapped to the deployed contract), its identities, its signer and its
    /// nonce counters are handed to the second segment, and both runs reseed
    /// the second segment from the same entropy.
    async fn run_split(
        mut platform: TempPlatform<drive_abci::rpc::core::MockCoreRPCLike>,
        config: PlatformConfig,
        strategy: NetworkStrategy,
        first_blocks: u64,
        second_blocks: u64,
        reopen: bool,
    ) -> SplitRun {
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums,
            current_validator_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            identities,
            signer,
            strategy: mut continued_strategy,
            state_transition_results_per_block: first_results,
            state_transitions_per_block: first_submitted,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            first_blocks,
            strategy.clone(),
            config.clone(),
            SEED,
            &mut None,
            &mut None,
        )
        .await;

        let state = abci_app.platform.state.load();
        assert_eq!(state.last_committed_block_height(), first_blocks);
        let fee_history_at_split = state
            .previous_fee_versions()
            .iter()
            .map(|(epoch_index, fee_version)| (*epoch_index, fee_version.fee_version_number))
            .collect::<Vec<_>>();
        let protocol_version_at_split = state.current_protocol_version_in_consensus();
        drop(state);
        drop(abci_app);

        assert!(
            continued_strategy.strategy.start_contracts.is_empty(),
            "the first segment deploys the start contracts; the continuation must not redeploy them"
        );
        continued_strategy.strategy.signer = Some(signer);

        let platform = if reopen {
            let TempPlatform {
                platform: mut platform_before_restart,
                tempdir,
            } = platform;
            let core_rpc = std::mem::take(&mut platform_before_restart.core_rpc);
            drop(platform_before_restart);
            let mut reopened = TempPlatform::open_with_tempdir(tempdir, config.clone());
            reopened.platform.core_rpc = core_rpc;
            let state = reopened.state.load();
            assert_eq!(
                state.last_committed_block_height(),
                first_blocks,
                "the reopened platform must resume from the persisted state"
            );
            assert_eq!(
                state
                    .previous_fee_versions()
                    .iter()
                    .map(|(epoch_index, fee_version)| (
                        *epoch_index,
                        fee_version.fee_version_number
                    ))
                    .collect::<Vec<_>>(),
                fee_history_at_split,
                "the fee history must survive the saved-state round trip"
            );
            assert_eq!(
                state.current_protocol_version_in_consensus(),
                protocol_version_at_split
            );
            drop(state);
            reopened
        } else {
            platform
        };
        let abci_app = FullAbciApplication::new(&platform.platform);

        let ChainExecutionOutcome {
            abci_app,
            identities,
            state_transition_results_per_block: second_results,
            state_transitions_per_block: second_submitted,
            ..
        } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start: first_blocks + 1,
                core_height_start: 1,
                block_count: second_blocks,
                proposers,
                validator_quorums,
                current_validator_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: strategy.start_time_ms,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: identities,
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            continued_strategy,
            config,
            StrategyRandomness::SeedEntropy(SEED + 1),
        )
        .await;

        let mut results_per_block = first_results;
        results_per_block.extend(second_results);
        let mut submitted_per_block = first_submitted;
        submitted_per_block.extend(second_submitted);

        let snapshot = snapshot(
            &abci_app,
            &identities,
            &results_per_block,
            &submitted_per_block,
            first_blocks + 1,
        );
        assert_eq!(snapshot.height, first_blocks + second_blocks);
        let activity = document_activity(&results_per_block);

        SplitRun {
            snapshot,
            activity,
            fee_history_at_split,
            protocol_version_at_split,
        }
    }

    fn assert_no_internal_errors(snapshot: &Snapshot) {
        const INTERNAL_ERROR_CODE: u32 = 13;
        for (height, results) in &snapshot.results_per_block {
            assert!(
                results.iter().all(|(code, _)| *code != INTERNAL_ERROR_CODE),
                "block {height} produced an internal error: {results:?}"
            );
        }
    }

    /// At least one document written before `split` was deleted at or after
    /// `from`, so the history-driven refund path priced bytes that were
    /// persisted before the restart.
    fn assert_deleted_earlier_document(activity: &DocumentActivity, split: u64, from: u64) {
        let created_before_split = activity
            .created
            .range(..=split)
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect::<BTreeSet<_>>();
        let deleted_after = activity
            .deleted
            .range(from..)
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect::<BTreeSet<_>>();
        assert!(
            !created_before_split.is_empty(),
            "expected documents to be created before block {split}"
        );
        assert!(
            deleted_after
                .iter()
                .any(|id| created_before_split.contains(id)),
            "expected a document created before block {split} to be deleted from block {from}; \
             deleted {deleted_after:?}"
        );
    }

    fn assert_runs_agree(continuous: &SplitRun, reopened: &SplitRun) {
        assert_eq!(
            continuous.snapshot.submitted_per_block, reopened.snapshot.submitted_per_block,
            "both runs must submit the same transitions after the split"
        );
        assert_eq!(
            continuous.snapshot.results_per_block, reopened.snapshot.results_per_block,
            "both runs must accept the same transitions with the same fees"
        );
        assert_eq!(continuous.snapshot.balances, reopened.snapshot.balances);
        assert_eq!(
            continuous.snapshot.fee_history,
            reopened.snapshot.fee_history
        );
        assert_eq!(
            continuous.snapshot.root_hash, reopened.snapshot.root_hash,
            "the continuous run and the run reopened from saved state must end on one root hash"
        );
        assert_eq!(continuous.snapshot, reopened.snapshot);
        assert_no_internal_errors(&continuous.snapshot);
    }

    /// Every proposer votes for the mock from block 1, so epoch 0 collects
    /// the votes, the first block of epoch 1 locks the mock in as the next
    /// version, and the first block of epoch 2 activates it and records the
    /// doubled-storage generation at epoch 2. The split at block 130 sits
    /// nine blocks after activation, inside the new generation.
    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn run_chain_upgrade_across_fee_version_boundary_keeps_replay_and_saved_state_in_agreement(
    ) {
        let latest = PlatformVersion::latest();
        let genesis_number = latest.fee_version.fee_version_number;
        let activation_block = 2 * BLOCKS_PER_EPOCH + 1;
        let first_blocks = 130;
        let second_blocks = 30;
        assert!(first_blocks > activation_block);

        let strategy = network_strategy(Some(UpgradingInfo {
            current_protocol_version: latest.protocol_version,
            proposed_protocol_versions_with_weight: vec![(TEST_PROTOCOL_VERSION_4, 1)],
            upgrade_three_quarters_life: 0.0,
        }));
        let config = platform_config();

        let mut runs = Vec::with_capacity(2);
        for reopen in [false, true] {
            let platform = TestPlatformBuilder::new()
                .with_config(config.clone())
                .with_initial_protocol_version(latest.protocol_version)
                .build_with_mock_rpc();
            let run = run_split(
                platform,
                config.clone(),
                strategy.clone(),
                first_blocks,
                second_blocks,
                reopen,
            )
            .await;

            assert_eq!(run.protocol_version_at_split, TEST_PROTOCOL_VERSION_4);
            assert_eq!(run.snapshot.protocol_version, TEST_PROTOCOL_VERSION_4);
            assert_eq!(
                run.fee_history_at_split,
                vec![
                    (0, genesis_number),
                    (2, TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE)
                ],
                "the genesis generation is recorded at init chain and the doubled generation at \
                 the epoch that activated it"
            );
            assert_eq!(run.snapshot.fee_history, run.fee_history_at_split);
            assert_deleted_earlier_document(&run.activity, first_blocks, activation_block);
            assert_deleted_earlier_document(&run.activity, first_blocks, first_blocks + 1);
            runs.push(run);
        }

        let reopened = runs.pop().expect("reopened run");
        let continuous = runs.pop().expect("continuous run");
        assert_runs_agree(&continuous, &reopened);
    }

    /// A chain started at the mock: the initial state records the doubled
    /// generation for the genesis epoch, and the epoch changes at blocks 61
    /// and 121 (the second one after the restart) leave the history alone
    /// because the generation does not change.
    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn run_chain_started_at_a_new_fee_generation_records_the_genesis_generation_and_survives_a_restart(
    ) {
        let first_blocks = 70;
        let second_blocks = 60;

        let strategy = network_strategy(None);
        let config = platform_config();

        let mut runs = Vec::with_capacity(2);
        for reopen in [false, true] {
            let platform = TestPlatformBuilder::new()
                .with_config(config.clone())
                .with_initial_protocol_version(TEST_PROTOCOL_VERSION_4)
                .build_with_mock_rpc();
            let run = run_split(
                platform,
                config.clone(),
                strategy.clone(),
                first_blocks,
                second_blocks,
                reopen,
            )
            .await;

            assert_eq!(run.protocol_version_at_split, TEST_PROTOCOL_VERSION_4);
            assert_eq!(run.snapshot.protocol_version, TEST_PROTOCOL_VERSION_4);
            assert_eq!(
                run.fee_history_at_split,
                vec![(0, TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE)]
            );
            assert_eq!(
                run.snapshot.fee_history,
                vec![(0, TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE)],
                "an epoch change within one generation must not add a history entry"
            );
            assert_deleted_earlier_document(&run.activity, first_blocks, first_blocks + 1);
            runs.push(run);
        }

        let reopened = runs.pop().expect("reopened run");
        let continuous = runs.pop().expect("continuous run");
        assert_runs_agree(&continuous, &reopened);
    }
}
