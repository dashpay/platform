//! Replay determinism across a protocol upgrade and a node restart.
//!
//! One seeded workload (identities, documents, top-ups and credit transfers
//! on the all-mutable DashPay contract) runs twice on a chain that starts at
//! the previous protocol version and upgrades to the latest one: once
//! continuously, and once stopped inside the upgrade window, reopened from
//! the persisted state and continued. Every block also re-runs
//! `process_proposal` as an independent validator, so proposer versus
//! validator parity is checked on every block of both runs.
//!
//! The two runs must agree on every application hash, every kept
//! transition's code and fee, the protocol version of every block, every
//! identity balance, the credit totals and the final root. The reopened run
//! is then recorded as a determinism artifact (see
//! `crate::determinism_artifact`) when `PLATFORM_DETERMINISM_ARTIFACT_DIR` is
//! set, so the `Determinism: cross-architecture replay` workflow can compare
//! the recording of one architecture with the recording of another.

#[cfg(test)]
mod tests {
    use crate::addresses_with_balance::AddressesWithBalance;
    use crate::determinism_artifact::{
        hex_lower, BlockRecord, Consensus, DeterminismArtifact, Diagnostic, Profile,
        TotalCreditsRecord, TransitionRecord, DETERMINISM_ARTIFACT_SCHEMA,
    };
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
    use crate::strategy::{
        ChainExecutionOutcome, ChainExecutionParameters, NetworkStrategy, StrategyRandomness,
        UpgradingInfo,
    };
    use dash_platform_macros::stack_size;
    use dpp::dash_to_duffs;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::{
        DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::{Identity, KeyType, Purpose, SecurityLevel};
    use dpp::state_transition::StateTransition;
    use dpp::tests::json_document::json_document_to_created_contract;
    use dpp::version::PlatformVersion;
    use drive_abci::abci::app::FullAbciApplication;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::rpc::core::MockCoreRPCLike;
    use drive_abci::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use std::collections::BTreeMap;
    use std::time::Instant;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{DocumentAction, DocumentOp, Operation, OperationType};
    use strategy_tests::{IdentityInsertInfo, KeyMaps, StartAddresses, StartIdentities, Strategy};
    use tenderdash_abci::proto::abci::ExecTxResult;

    /// Sixty blocks per epoch: `epoch_time_length_s` of 60 at one block per
    /// second. The proposers all propose the new version from block 1, so the
    /// upgrade locks in at the first epoch change and activates at the second.
    const BLOCKS_PER_EPOCH: u64 = 60;
    /// Blocks before the split. Past the first epoch change (the upgrade is
    /// locked in) and before the second (it has not activated).
    const FIRST_SEGMENT_BLOCKS: u64 = 70;
    /// Blocks after the split. Crosses the second epoch change, so the upgrade
    /// activates on a platform that was reopened from disk.
    const SECOND_SEGMENT_BLOCKS: u64 = 60;
    const SEED: u64 = 1401;
    /// `ExecTxResult::code` of a transition that hit an internal error.
    const INTERNAL_ERROR_CODE: u32 = 13;

    fn platform_config() -> PlatformConfig {
        PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_size: 30,
                ..Default::default()
            },
            // Chain locks need their own quorum type: the harness only signs
            // them, and the independent validator path only accepts them,
            // when the chain-lock quorums are distinct from the validator set.
            chain_lock: ChainLockConfig::default(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                epoch_time_length_s: BLOCKS_PER_EPOCH,
                ..Default::default()
            },
            block_spacing_ms: 1_000,
            // The full verification profile: blocks are signed and their
            // commit signatures verified, and instant lock signatures are
            // verified rather than skipped. Those are BLS paths, which is
            // exactly where an architecture could disagree, so the replay
            // keeps them on. It also persists the state the reopened run
            // needs. Only result-proof verification stays off, on the
            // strategy below.
            testing_configs: PlatformTestConfig::default(),
            ..Default::default()
        }
    }

    /// Ten start identities and one more per block, each with a critical
    /// transfer key so credit transfers can be signed; documents are created,
    /// replaced and deleted on DashPay contact requests; identities are
    /// topped up from signed instant locks. Withdrawals and tokens stay out:
    /// they need core RPC mocks and their own key setup and are covered by
    /// the comprehensive simulation.
    fn network_strategy(previous: u32, latest: u32) -> NetworkStrategy {
        let platform_version = PlatformVersion::get(previous)
            .expect("the previous protocol version must be known to this binary");
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
        let transfer_key: KeyMaps = [(
            Purpose::TRANSFER,
            [(SecurityLevel::CRITICAL, vec![KeyType::ECDSA_SECP256K1])].into(),
        )]
        .into();

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
                            times_per_block_range: 1..4,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(DocumentOp {
                            contract: contract.clone(),
                            action: DocumentAction::DocumentActionReplaceRandom,
                            document_type: document_type.clone(),
                        }),
                        frequency: Frequency {
                            times_per_block_range: 1..3,
                            chance_per_block: Some(0.7),
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
                            chance_per_block: Some(0.5),
                        },
                    },
                    Operation {
                        op_type: OperationType::IdentityTopUp(
                            dash_to_duffs!(1)..=dash_to_duffs!(1),
                        ),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: Some(0.5),
                        },
                    },
                    Operation {
                        op_type: OperationType::IdentityTransfer(None),
                        frequency: Frequency {
                            times_per_block_range: 1..3,
                            chance_per_block: Some(0.7),
                        },
                    },
                ],
                start_identities: StartIdentities {
                    number_of_identities: 10,
                    keys_per_identity: 3,
                    starting_balances: dash_to_duffs!(10),
                    extra_keys: transfer_key.clone(),
                    hard_coded: vec![],
                },
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    start_keys: 3,
                    extra_keys: transfer_key,
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
                },
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 50,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 4,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: previous,
                proposed_protocol_versions_with_weight: vec![(latest, 1)],
                upgrade_three_quarters_life: 0.0,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            // Proof verification of every result doubles the runtime and is
            // covered by the comprehensive simulation; this test is about
            // agreement between runs.
            verify_state_transition_results: false,
            independent_process_proposal_verification: true,
            sign_chain_locks: true,
            sign_instant_locks: true,
            ..Default::default()
        }
    }

    /// What one run leaves behind, in the fields two runs must agree on.
    struct RunRecord {
        consensus: Consensus,
        /// Consensus protocol version after the first segment.
        protocol_version_at_split: u32,
        /// Protocol version scheduled for the next epoch after the first
        /// segment.
        next_epoch_protocol_version_at_split: u32,
        elapsed_ms: u64,
    }

    fn consensus_record(
        abci_app: &FullAbciApplication<'_, MockCoreRPCLike>,
        identities: &[Identity],
        results_per_block: &BTreeMap<u64, Vec<(StateTransition, ExecTxResult)>>,
        app_hashes_per_block: &BTreeMap<u64, [u8; 32]>,
        protocol_versions_per_block: &BTreeMap<u64, u64>,
        reopened_at_height: u64,
    ) -> Consensus {
        let state = abci_app.platform.state.load();
        let platform_version = state
            .current_platform_version()
            .expect("expected the state's platform version");
        let drive = &abci_app.platform.drive;

        let identity_ids: Vec<[u8; 32]> = identities
            .iter()
            .map(|identity| identity.id().to_buffer())
            .collect();
        let balances = drive
            .fetch_identities_balances(&identity_ids, None, platform_version)
            .expect("expected to fetch identity balances");
        assert_eq!(
            balances.len(),
            identities.len(),
            "every identity the strategy created must have a balance"
        );

        let total_credits = drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("expected to calculate the total credits balance");
        assert!(
            total_credits
                .ok()
                .expect("the credit totals must not overflow"),
            "credits must be conserved at the last block: {total_credits}"
        );

        assert_eq!(
            app_hashes_per_block.len(),
            results_per_block.len(),
            "every finalized block must have an application hash and a result list"
        );
        let blocks = app_hashes_per_block
            .iter()
            .map(|(height, app_hash)| BlockRecord {
                height: *height,
                protocol_version: *protocol_versions_per_block
                    .get(height)
                    .expect("every finalized block must record its protocol version"),
                app_hash: hex_lower(app_hash),
                transitions: results_per_block
                    .get(height)
                    .expect("every finalized block must record its results")
                    .iter()
                    .map(|(transition, result)| TransitionRecord {
                        name: transition.name(),
                        code: result.code,
                        fee: result.gas_used,
                    })
                    .collect(),
            })
            .collect();

        Consensus {
            workload_seed: SEED,
            reopened_at_height,
            blocks,
            final_root_hash: hex_lower(
                &state
                    .last_committed_block_app_hash()
                    .expect("expected a committed block"),
            ),
            total_credits: TotalCreditsRecord::from(&total_credits),
            identity_balances: balances
                .into_iter()
                .map(|(id, balance)| (hex_lower(&id), balance))
                .collect(),
        }
    }

    /// Runs the first segment, then the second with the same workload. With
    /// `reopen` the platform is dropped between the segments and rebuilt from
    /// what it persisted, so the second segment, and the upgrade activation
    /// inside it, run on the saved state instead of the in-memory one.
    ///
    /// The continuation is workload-preserving: the mutated strategy of the
    /// first segment (its operations remapped to the deployed contract, its
    /// start contracts cleared so they are not deployed again), its
    /// identities, its signer and its nonce counters are handed to the second
    /// segment, and both runs reseed the second segment from the same entropy.
    async fn run_split(previous: u32, latest: u32, reopen: bool) -> RunRecord {
        let started = Instant::now();
        let config = platform_config();
        let strategy = network_strategy(previous, latest);
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(previous)
            .build_with_mock_rpc();

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
            app_hashes_per_block: first_app_hashes,
            protocol_versions_per_block: first_protocol_versions,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            FIRST_SEGMENT_BLOCKS,
            strategy.clone(),
            config.clone(),
            SEED,
            &mut None,
            &mut None,
        )
        .await;

        let state = abci_app.platform.state.load();
        assert_eq!(state.last_committed_block_height(), FIRST_SEGMENT_BLOCKS);
        let protocol_version_at_split = state.current_protocol_version_in_consensus();
        let next_epoch_protocol_version_at_split = state.next_epoch_protocol_version();
        drop(state);
        drop(abci_app);

        continued_strategy.strategy.start_contracts.clear();
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
                FIRST_SEGMENT_BLOCKS,
                "the reopened platform must resume from the persisted state"
            );
            assert_eq!(
                state.current_protocol_version_in_consensus(),
                protocol_version_at_split,
                "the persisted protocol version must survive the restart"
            );
            assert_eq!(
                state.next_epoch_protocol_version(),
                next_epoch_protocol_version_at_split,
                "the locked-in upgrade must survive the restart"
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
            app_hashes_per_block: second_app_hashes,
            protocol_versions_per_block: second_protocol_versions,
            ..
        } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start: FIRST_SEGMENT_BLOCKS + 1,
                core_height_start: 1,
                block_count: SECOND_SEGMENT_BLOCKS,
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
        let mut app_hashes_per_block = first_app_hashes;
        app_hashes_per_block.extend(second_app_hashes);
        let mut protocol_versions_per_block = first_protocol_versions;
        protocol_versions_per_block.extend(second_protocol_versions);

        let consensus = consensus_record(
            &abci_app,
            &identities,
            &results_per_block,
            &app_hashes_per_block,
            &protocol_versions_per_block,
            if reopen { FIRST_SEGMENT_BLOCKS } else { 0 },
        );
        assert_eq!(
            abci_app.platform.state.load().last_committed_block_height(),
            FIRST_SEGMENT_BLOCKS + SECOND_SEGMENT_BLOCKS
        );

        RunRecord {
            consensus,
            protocol_version_at_split,
            next_epoch_protocol_version_at_split,
            elapsed_ms: started.elapsed().as_millis() as u64,
        }
    }

    /// The kind of one executed transition, precise enough to tell a document
    /// create from an identity create. A `DocumentsBatch([Create, Delete])`
    /// contributes one `DocumentCreate` and one `DocumentDelete`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum TransitionKind {
        DataContractCreate,
        IdentityCreate,
        IdentityTopUp,
        IdentityCreditTransfer,
        DocumentCreate,
        DocumentReplace,
        DocumentDelete,
    }

    impl TransitionKind {
        /// The kinds the workload submits every block, so each must succeed
        /// on both sides of the activation.
        const RECURRING: [TransitionKind; 6] = [
            TransitionKind::IdentityCreate,
            TransitionKind::IdentityTopUp,
            TransitionKind::IdentityCreditTransfer,
            TransitionKind::DocumentCreate,
            TransitionKind::DocumentReplace,
            TransitionKind::DocumentDelete,
        ];

        /// Every kind a recorded name maps to. Unknown names fail loudly so
        /// a new operation in the workload has to be classified here.
        fn parse(name: &str) -> Result<Vec<TransitionKind>, String> {
            if let Some(inner) = name
                .strip_prefix("DocumentsBatch([")
                .and_then(|rest| rest.strip_suffix("])"))
            {
                return inner
                    .split(", ")
                    .filter(|part| !part.is_empty())
                    .map(|part| match part {
                        "Create" => Ok(TransitionKind::DocumentCreate),
                        "Replace" => Ok(TransitionKind::DocumentReplace),
                        "Delete" => Ok(TransitionKind::DocumentDelete),
                        other => Err(format!("unclassified batched transition {other:?}")),
                    })
                    .collect();
            }
            match name {
                "DataContractCreate" => Ok(vec![TransitionKind::DataContractCreate]),
                "IdentityCreate" => Ok(vec![TransitionKind::IdentityCreate]),
                "IdentityTopUp" => Ok(vec![TransitionKind::IdentityTopUp]),
                "IdentityCreditTransfer" => Ok(vec![TransitionKind::IdentityCreditTransfer]),
                other => Err(format!("unclassified transition {other:?}")),
            }
        }
    }

    /// Counts the successful transitions of each kind in `blocks`.
    fn successful_kinds(blocks: &[BlockRecord]) -> Result<BTreeMap<TransitionKind, usize>, String> {
        let mut counts = BTreeMap::new();
        for block in blocks {
            for transition in &block.transitions {
                if transition.code != 0 {
                    continue;
                }
                for kind in TransitionKind::parse(&transition.name)? {
                    *counts.entry(kind).or_insert(0) += 1;
                }
            }
        }
        Ok(counts)
    }

    /// The recorded blocks must show the upgrade activating strictly after
    /// the split, no internal errors, the contract deployed exactly once
    /// before activation, and every recurring workload operation succeeding
    /// both before and after activation. A run that quietly rejected or
    /// dropped a whole operation kind after the upgrade would still replay
    /// identically, so this is checked separately from the agreement between
    /// runs. Returns the activation height.
    fn check_workload_shape(
        blocks: &[BlockRecord],
        previous: u32,
        latest: u32,
    ) -> Result<u64, String> {
        let block = |height: u64| {
            blocks
                .iter()
                .find(|block| block.height == height)
                .ok_or_else(|| format!("block {height} must be recorded"))
        };
        if block(FIRST_SEGMENT_BLOCKS)?.protocol_version != previous as u64 {
            return Err(format!(
                "block {FIRST_SEGMENT_BLOCKS} must still run protocol version {previous}"
            ));
        }
        let last_height = FIRST_SEGMENT_BLOCKS + SECOND_SEGMENT_BLOCKS;
        if block(last_height)?.protocol_version != latest as u64 {
            return Err(format!(
                "the upgrade to {latest} must have activated by block {last_height}"
            ));
        }
        let activation_height = blocks
            .iter()
            .find(|block| block.protocol_version == latest as u64)
            .map(|block| block.height)
            .ok_or_else(|| "some block must run the new version".to_string())?;
        if activation_height <= FIRST_SEGMENT_BLOCKS {
            return Err(format!(
                "activation at {activation_height} must come after the split at {FIRST_SEGMENT_BLOCKS}"
            ));
        }

        for block in blocks {
            for transition in &block.transitions {
                if transition.code == INTERNAL_ERROR_CODE {
                    return Err(format!(
                        "block {} produced an internal error on {}",
                        block.height, transition.name
                    ));
                }
            }
        }

        let split = blocks.partition_point(|block| block.height < activation_height);
        let (before, after) = blocks.split_at(split);
        let before = successful_kinds(before)?;
        let after = successful_kinds(after)?;

        if before.get(&TransitionKind::DataContractCreate) != Some(&1) {
            return Err("the contract must be deployed exactly once before activation".to_string());
        }
        if after.contains_key(&TransitionKind::DataContractCreate) {
            return Err("no contract may be deployed after activation".to_string());
        }
        for kind in TransitionKind::RECURRING {
            if !before.contains_key(&kind) {
                return Err(format!(
                    "the workload must execute a successful {kind:?} before the upgrade activated"
                ));
            }
            if !after.contains_key(&kind) {
                return Err(format!(
                    "the workload must execute a successful {kind:?} after the upgrade activated"
                ));
            }
        }
        Ok(activation_height)
    }

    /// The upgrade must lock in before the split and activate after it, and
    /// the recorded blocks must pass `check_workload_shape`.
    fn assert_upgrade_and_workload_shape(run: &RunRecord, previous: u32, latest: u32) {
        assert_eq!(run.protocol_version_at_split, previous);
        assert_eq!(
            run.next_epoch_protocol_version_at_split, latest,
            "the upgrade must be locked in before the split"
        );
        check_workload_shape(&run.consensus.blocks, previous, latest)
            .unwrap_or_else(|reason| panic!("workload shape: {reason}"));
    }

    fn assert_runs_agree(continuous: &Consensus, reopened: &Consensus) {
        assert_eq!(continuous.blocks.len(), reopened.blocks.len());
        for (left, right) in continuous.blocks.iter().zip(reopened.blocks.iter()) {
            assert_eq!(left.height, right.height);
            assert_eq!(
                left.protocol_version, right.protocol_version,
                "protocol version differs at block {}",
                left.height
            );
            assert_eq!(
                left.transitions, right.transitions,
                "transition results differ at block {}",
                left.height
            );
            assert_eq!(
                left.app_hash, right.app_hash,
                "application hash differs at block {}",
                left.height
            );
        }
        assert_eq!(continuous.final_root_hash, reopened.final_root_hash);
        assert_eq!(continuous.total_credits, reopened.total_credits);
        assert_eq!(continuous.identity_balances, reopened.identity_balances);
    }

    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn should_replay_identically_from_saved_state_across_the_protocol_upgrade() {
        let latest = PlatformVersion::latest().protocol_version;
        let previous = latest
            .checked_sub(1)
            .expect("the latest protocol version must have a predecessor");
        assert!(
            PlatformVersion::get(previous).is_ok(),
            "the previous protocol version must be known to this binary"
        );

        let continuous = run_split(previous, latest, false).await;
        let reopened = run_split(previous, latest, true).await;

        assert_upgrade_and_workload_shape(&continuous, previous, latest);
        assert_upgrade_and_workload_shape(&reopened, previous, latest);
        assert_runs_agree(&continuous.consensus, &reopened.consensus);

        let artifact = DeterminismArtifact {
            schema: DETERMINISM_ARTIFACT_SCHEMA,
            profile: Profile::for_this_target(previous, latest),
            diagnostic: Diagnostic {
                elapsed_ms: reopened.elapsed_ms,
                block_count: reopened.consensus.blocks.len() as u64,
                engine_fuel: None,
            },
            consensus: reopened.consensus,
        };
        match artifact
            .write_if_requested()
            .expect("the artifact directory must be writable")
        {
            Some(path) => println!("determinism artifact written to {}", path.display()),
            None => println!(
                "determinism artifact not written: PLATFORM_DETERMINISM_ARTIFACT_DIR is unset"
            ),
        }
    }

    /// A synthetic recording with the split at `FIRST_SEGMENT_BLOCKS`, the
    /// activation at `activation` and one successful transition of every
    /// kind on both sides of it.
    fn synthetic_blocks(previous: u32, latest: u32, activation: u64) -> Vec<BlockRecord> {
        let last_height = FIRST_SEGMENT_BLOCKS + SECOND_SEGMENT_BLOCKS;
        let ok = |name: &str| TransitionRecord {
            name: name.to_string(),
            code: 0,
            fee: 1,
        };
        (1..=last_height)
            .map(|height| {
                let mut transitions = vec![
                    ok("IdentityCreate"),
                    ok("IdentityTopUp"),
                    ok("IdentityCreditTransfer"),
                    ok("DocumentsBatch([Create, Replace, Delete])"),
                ];
                if height == 2 {
                    transitions.push(ok("DataContractCreate"));
                }
                BlockRecord {
                    height,
                    protocol_version: if height >= activation {
                        latest as u64
                    } else {
                        previous as u64
                    },
                    app_hash: hex_lower(&[height as u8; 32]),
                    transitions,
                }
            })
            .collect()
    }

    #[test]
    fn should_accept_a_recording_with_every_operation_kind_on_both_sides_of_the_activation() {
        let blocks = synthetic_blocks(13, 14, FIRST_SEGMENT_BLOCKS + 51);
        assert_eq!(
            check_workload_shape(&blocks, 13, 14),
            Ok(FIRST_SEGMENT_BLOCKS + 51)
        );
    }

    #[test]
    fn should_reject_a_recording_that_lost_its_document_operations_after_the_activation() {
        let activation = FIRST_SEGMENT_BLOCKS + 51;
        let mut blocks = synthetic_blocks(13, 14, activation);
        for block in blocks.iter_mut().filter(|block| block.height >= activation) {
            block
                .transitions
                .retain(|transition| !transition.name.starts_with("DocumentsBatch"));
        }
        let reason = check_workload_shape(&blocks, 13, 14)
            .expect_err("dropping every post-activation document result must be caught");
        assert!(
            reason.contains("DocumentCreate") && reason.contains("after the upgrade activated"),
            "unexpected reason: {reason}"
        );

        // Identity creates alone must not stand in for document creates.
        let mut blocks = synthetic_blocks(13, 14, activation);
        for block in blocks.iter_mut().filter(|block| block.height >= activation) {
            block
                .transitions
                .retain(|transition| transition.name == "IdentityCreate");
        }
        let reason = check_workload_shape(&blocks, 13, 14)
            .expect_err("identity creates must not satisfy the document create check");
        assert!(
            reason.contains("after the upgrade activated"),
            "unexpected reason: {reason}"
        );
    }

    #[test]
    fn should_reject_a_recording_whose_top_ups_failed_after_the_activation() {
        let activation = FIRST_SEGMENT_BLOCKS + 51;
        let mut blocks = synthetic_blocks(13, 14, activation);
        for transition in blocks
            .iter_mut()
            .filter(|block| block.height >= activation)
            .flat_map(|block| block.transitions.iter_mut())
            .filter(|transition| transition.name == "IdentityTopUp")
        {
            transition.code = 10_000;
        }
        let reason = check_workload_shape(&blocks, 13, 14)
            .expect_err("rejected top-ups after activation must be caught");
        assert!(
            reason.contains("IdentityTopUp") && reason.contains("after the upgrade activated"),
            "unexpected reason: {reason}"
        );
    }

    #[test]
    fn should_reject_a_recording_that_activated_before_the_split_or_not_at_all() {
        let blocks = synthetic_blocks(13, 14, FIRST_SEGMENT_BLOCKS);
        let reason = check_workload_shape(&blocks, 13, 14)
            .expect_err("activation at the split must be caught");
        assert!(
            reason.contains("protocol version 13"),
            "unexpected reason: {reason}"
        );

        let blocks = synthetic_blocks(13, 14, FIRST_SEGMENT_BLOCKS + SECOND_SEGMENT_BLOCKS + 1);
        let reason = check_workload_shape(&blocks, 13, 14)
            .expect_err("a run that never activated must be caught");
        assert!(
            reason.contains("must have activated"),
            "unexpected reason: {reason}"
        );
    }

    #[test]
    fn should_reject_an_unclassified_transition_name() {
        let mut blocks = synthetic_blocks(13, 14, FIRST_SEGMENT_BLOCKS + 51);
        blocks[5].transitions.push(TransitionRecord {
            name: "MasternodeVote".to_string(),
            code: 0,
            fee: 0,
        });
        let reason = check_workload_shape(&blocks, 13, 14)
            .expect_err("a new operation kind must be classified before it counts");
        assert!(
            reason.contains("unclassified transition"),
            "unexpected reason: {reason}"
        );
    }
}
