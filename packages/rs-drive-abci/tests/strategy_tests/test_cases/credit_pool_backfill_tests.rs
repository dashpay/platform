//! Historical CREDIT nullifiers across candidate ownership and protocol activation.
#[cfg(test)]
mod tests {
    use crate::addresses_with_balance::AddressesWithBalance;
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy, GENESIS_TIME_MS};
    use crate::strategy::{
        ChainExecutionOutcome, ChainExecutionParameters, NetworkStrategy, StrategyRandomness,
        UpgradingInfo,
    };
    use dash_platform_macros::stack_size;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Setters;
    use dpp::dashcore::hashes::Hash;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contracts::SystemDataContract;
    use dpp::system_data_contracts::load_system_data_contract;
    use drive::drive::contract::paths::{
        all_contracts_global_root_path, contract_other_path, CONTRACT_VERSION_KEY,
    };
    use drive::drive::Drive;
    use drive::fees::op::LowLevelDriveOperation;
    use drive::grovedb::operations::delete::DeleteOptions;
    use drive::grovedb::{Element, TransactionArg};
    use drive::util::storage_flags::StorageFlags;
    use drive_abci::abci::app::FullAbciApplication;
    use drive_abci::config::{
        ExecutionConfig, PlatformConfig, PlatformTestConfig, ValidatorSetConfig,
    };
    use drive_abci::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
    use drive_abci::execution::types::block_state_info::v0::BlockStateInfoV0Getters;
    use drive_abci::mimic::CHAIN_ID;
    use drive_abci::platform_types::platform::Platform;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::rpc::core::MockCoreRPCLike;
    use drive_abci::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use tenderdash_abci::proto::abci::response_process_proposal::ProposalStatus;
    use tenderdash_abci::proto::abci::{CommitInfo, RequestFinalizeBlock, RequestProcessProposal};
    use tenderdash_abci::proto::google::protobuf::Timestamp;
    use tenderdash_abci::proto::types::{
        Block, BlockId, Data, EvidenceList, Header, PartSetHeader,
    };
    use tenderdash_abci::proto::version::Consensus;
    use tenderdash_abci::proto::FromMillis;
    use tenderdash_abci::Application;

    /// The proposal of `round` for the next height, at chain-locked `core_chain_locked_height`
    fn proposal(
        outcome: &ChainExecutionOutcome,
        round: u32,
        core_chain_locked_height: u32,
        hash: [u8; 32],
    ) -> RequestProcessProposal {
        let platform_state = outcome.abci_app.platform.state.load();
        let height = platform_state.last_committed_block_height() + 1;
        let time_ms = outcome.end_time_ms + 1000 + round as u64 * 1000;
        let proposer = outcome.proposers[round as usize].pro_tx_hash();

        RequestProcessProposal {
            txs: vec![],
            proposed_last_commit: None,
            misbehavior: vec![],
            hash: hash.to_vec(),
            height: height as i64,
            time: Some(Timestamp::from_millis(time_ms).expect("expected a block time")),
            next_validators_hash: [0u8; 32].to_vec(),
            round: round as i32,
            core_chain_locked_height,
            core_chain_lock_update: None,
            proposer_pro_tx_hash: proposer.to_byte_array().to_vec(),
            proposed_app_version: PlatformVersion::latest().protocol_version as u64,
            version: Some(Consensus {
                block: 0,
                app: PlatformVersion::latest().protocol_version as u64,
            }),
            quorum_hash: outcome
                .current_quorum()
                .quorum_hash
                .to_byte_array()
                .to_vec(),
        }
    }

    /// The request Tenderdash finalizes `proposal` with once it is committed in its own round,
    /// with `app_hash` in the block header. The commit carries no withdrawal signatures, so the
    /// block must not have any withdrawal transactions to sign.
    fn finalize_request(
        proposal: &RequestProcessProposal,
        app_hash: Vec<u8>,
    ) -> RequestFinalizeBlock {
        RequestFinalizeBlock {
            commit: Some(CommitInfo {
                round: proposal.round,
                quorum_hash: proposal.quorum_hash.clone(),
                block_signature: vec![0u8; 96],
                threshold_vote_extensions: vec![],
            }),
            misbehavior: vec![],
            hash: proposal.hash.clone(),
            height: proposal.height,
            round: proposal.round,
            block: Some(Block {
                header: Some(Header {
                    version: proposal.version,
                    chain_id: CHAIN_ID.to_string(),
                    height: proposal.height,
                    time: proposal.time,
                    last_block_id: None,
                    last_commit_hash: vec![],
                    data_hash: vec![0u8; 32],
                    validators_hash: proposal.quorum_hash.clone(),
                    next_validators_hash: proposal.quorum_hash.clone(),
                    consensus_hash: vec![0u8; 32],
                    next_consensus_hash: vec![0u8; 32],
                    app_hash,
                    results_hash: vec![0u8; 32],
                    evidence_hash: vec![],
                    proposed_app_version: proposal.proposed_app_version,
                    proposer_pro_tx_hash: proposal.proposer_pro_tx_hash.clone(),
                    core_chain_locked_height: proposal.core_chain_locked_height,
                }),
                data: Some(Data {
                    txs: proposal.txs.clone(),
                }),
                evidence: Some(EvidenceList { evidence: vec![] }),
                last_commit: None,
                core_chain_lock: proposal.core_chain_lock_update.clone(),
            }),
            block_id: Some(BlockId {
                hash: proposal.hash.clone(),
                part_set_header: Some(PartSetHeader {
                    total: 0,
                    hash: vec![0u8; 32],
                }),
                state_id: vec![0u8; 32],
            }),
        }
    }

    fn take_application(
        outcome: ChainExecutionOutcome<'_>,
    ) -> FullAbciApplication<'_, MockCoreRPCLike> {
        outcome.abci_app
    }

    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn should_backfill_credit_nullifiers_when_accepted_activation_survives_a_rejected_round()
    {
        activation_lifecycle(false).await;
    }

    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn should_preserve_historical_token_history_flags_across_public_activation_candidates() {
        activation_lifecycle(true).await;
    }

    fn assert_token_history(
        platform: &Platform<MockCoreRPCLike>,
        tx: TransactionArg,
        version: u32,
        historical: bool,
    ) {
        let pv = if version == 1 {
            PlatformVersion::get(13).expect("PV13")
        } else {
            PlatformVersion::latest()
        };
        let stored = platform
            .drive
            .fetch_contract(
                SystemDataContract::TokenHistory.id().to_buffer(),
                None,
                None,
                tx,
                pv,
            )
            .value
            .expect("TokenHistory read")
            .expect("TokenHistory exists");
        assert_eq!(stored.contract.version(), version);
        let flags = historical.then(|| StorageFlags::new_single_epoch(0, Some([0; 32])));
        assert_eq!(
            stored.storage_flags.as_ref().map(StorageFlags::owner_id),
            flags.as_ref().map(StorageFlags::owner_id)
        );
        assert_eq!(
            stored.storage_flags.as_ref().map(StorageFlags::base_epoch),
            flags.as_ref().map(StorageFlags::base_epoch)
        );
        if version == 2 {
            if historical {
                assert_eq!(
                    stored
                        .storage_flags
                        .as_ref()
                        .and_then(StorageFlags::epoch_index_map)
                        .and_then(|allocations| allocations.get(&2)),
                    Some(&108)
                );
            }
            let (_, cached) = platform
                .drive
                .get_contract_with_fetch_info_and_fee(
                    SystemDataContract::TokenHistory.id().to_buffer(),
                    None,
                    false,
                    tx,
                    pv,
                )
                .expect("cache-aware TokenHistory read");
            assert_eq!(cached.expect("cached TokenHistory").contract.version(), 2);
            let id = SystemDataContract::TokenHistory.id().to_buffer();
            assert_eq!(
                platform
                    .drive
                    .grove
                    .get(
                        &contract_other_path(&id),
                        &[CONTRACT_VERSION_KEY],
                        tx,
                        &pv.drive.grove_version
                    )
                    .value
                    .expect("version Item"),
                Element::Item(
                    2u32.to_be_bytes().to_vec(),
                    flags.as_ref().map(StorageFlags::to_element_flags)
                )
            );
        }
    }

    async fn activation_lifecycle(historical_token_history: bool) {
        let old = PlatformVersion::get(13).expect("PV13");
        let latest = PlatformVersion::latest();
        let config = PlatformConfig {
            block_spacing_ms: 1000,
            validator_set: ValidatorSetConfig {
                quorum_size: 30,
                ..Default::default()
            },
            execution: ExecutionConfig {
                epoch_time_length_s: 60,
                ..Default::default()
            },
            testing_configs: PlatformTestConfig {
                store_platform_state: true,
                ..PlatformTestConfig::default_minimal_verifications()
            },
            ..Default::default()
        };
        let strategy = NetworkStrategy {
            total_hpmns: 50,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 13,
                proposed_protocol_versions_with_weight: vec![(latest.protocol_version, 1)],
                upgrade_three_quarters_life: 0.0,
            }),
            ..Default::default()
        };
        let mut observations = vec![];
        for (reopen_before_activation, reject_later_round, accept_alternate, restart_candidate) in [
            (false, false, false, false),
            (false, true, false, false),
            (true, true, false, false),
            (false, true, true, false),
            (false, false, false, true),
        ] {
            let mut platform = TestPlatformBuilder::new()
                .with_config(config.clone())
                .with_initial_protocol_version(13)
                .build_with_mock_rpc();
            let outcome = run_chain_for_strategy(
                &mut platform,
                119,
                strategy.clone(),
                config.clone(),
                13,
                &mut None,
                &mut None,
            )
            .await;
            let state = outcome.abci_app.platform.state.load();
            assert_eq!(state.current_protocol_version_in_consensus(), 13);
            assert_eq!(state.next_epoch_protocol_version(), latest.protocol_version);
            assert_eq!(state.last_committed_block_epoch().index, 1);
            drop(state);

            let transaction = outcome.abci_app.platform.drive.grove.start_transaction();
            if historical_token_history {
                let drive = &outcome.abci_app.platform.drive;
                drive
                    .grove
                    .delete(
                        &all_contracts_global_root_path(),
                        SystemDataContract::TokenHistory.id().as_bytes(),
                        Some(DeleteOptions {
                            allow_deleting_non_empty_trees: true,
                            ..Default::default()
                        }),
                        Some(&transaction),
                        &old.drive.grove_version,
                    )
                    .value
                    .expect("replace empty genesis fixture");
                let contract = load_system_data_contract(SystemDataContract::TokenHistory, old)
                    .expect("legacy TokenHistory");
                drive
                    .insert_contract(
                        &contract,
                        BlockInfo::default(),
                        true,
                        Some(&transaction),
                        old,
                    )
                    .expect("historical insertion path");
            }
            let mut notes = vec![];
            for (rho, cmx) in [([1u8; 32], [2u8; 32]), ([3u8; 32], [4u8; 32])] {
                notes.extend(
                    Drive::insert_note_op(rho, cmx, [5u8; 32], vec![6u8; 216], old)
                        .expect("historical note operations"),
                );
            }
            let drive = &outcome.abci_app.platform.drive;
            drive
                .grove_apply_batch(
                    LowLevelDriveOperation::grovedb_operations_batch_consume(notes),
                    false,
                    Some(&transaction),
                    &old.drive,
                )
                .expect("historical note storage");
            drive
                .commit_transaction(transaction, &old.drive)
                .expect("historical state commit");
            assert_token_history(outcome.abci_app.platform, None, 1, historical_token_history);
            let committed_root = drive
                .grove
                .root_hash(None, &old.drive.grove_version)
                .unwrap()
                .expect("committed root");
            let mut state = outcome.abci_app.platform.state.load().as_ref().clone();
            state
                .last_committed_block_info_mut()
                .as_mut()
                .expect("committed block")
                .set_app_hash(committed_root);
            outcome.abci_app.platform.state.store(Arc::new(state));
            for rho in [[1u8; 32], [3u8; 32]] {
                assert!(!drive
                    .has_nullifier(&rho, None, &mut vec![], old)
                    .expect("before activation"));
            }

            // One more old-protocol block commits the seeded notes together with coherent
            // persisted platform state, so restart observes the same app hash as a warm node.
            let parameters = ChainExecutionParameters {
                block_start: outcome
                    .abci_app
                    .platform
                    .state
                    .load()
                    .last_committed_block_height()
                    + 1,
                core_height_start: 1,
                block_count: 1,
                proposers: outcome.proposers.clone(),
                validator_quorums: outcome.validator_quorums.clone(),
                current_validator_quorum_hash: outcome.current_validator_quorum_hash,
                current_proposer_versions: Some(outcome.current_proposer_versions.clone()),
                current_identity_nonce_counter: outcome.identity_nonce_counter.clone(),
                current_identity_contract_nonce_counter: outcome
                    .identity_contract_nonce_counter
                    .clone(),
                current_votes: BTreeMap::new(),
                start_time_ms: GENESIS_TIME_MS,
                current_time_ms: outcome.end_time_ms,
                instant_lock_quorums: outcome.instant_lock_quorums.clone(),
                current_identities: vec![],
                current_addresses_with_balance: AddressesWithBalance::default(),
            };
            let abci_app = take_application(outcome);
            let outcome = continue_chain_for_strategy(
                abci_app,
                parameters,
                strategy.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(18),
            )
            .await;
            let committed_root = outcome
                .abci_app
                .platform
                .drive
                .grove
                .root_hash(None, &old.drive.grove_version)
                .value
                .expect("coherent old-protocol commit");
            assert_eq!(
                outcome
                    .abci_app
                    .platform
                    .state
                    .load()
                    .last_committed_block_app_hash(),
                Some(committed_root)
            );
            let core_height = outcome
                .abci_app
                .platform
                .state
                .load()
                .last_committed_core_height();
            let accepted = proposal(&outcome, 0, core_height, [0xA0; 32]);
            let mut rejected = proposal(&outcome, 1, core_height, [0xB0; 32]);
            rejected.txs = vec![vec![0u8; 10]];
            let alternate = proposal(&outcome, 2, core_height, [0xC0; 32]);
            drop(outcome);
            if reopen_before_activation {
                let TempPlatform {
                    platform: mut before_restart,
                    tempdir,
                } = platform;
                let rpc = std::mem::take(&mut before_restart.core_rpc);
                drop(before_restart);
                platform = TempPlatform::open_with_tempdir(tempdir, config.clone());
                platform.platform.core_rpc = rpc;
                assert_eq!(
                    platform.state.load().last_committed_block_app_hash(),
                    Some(committed_root),
                    "persisted state and warm state agree before activation"
                );
            }
            let mut abci_app = FullAbciApplication::new(&platform.platform);
            let response = abci_app
                .process_proposal(accepted.clone())
                .expect("candidate A");
            assert_eq!(response.status, ProposalStatus::Accept as i32);
            assert_token_history(abci_app.platform, None, 1, historical_token_history);
            let accepted_root = response.app_hash;
            assert_eq!(
                abci_app
                    .platform
                    .drive
                    .grove
                    .root_hash(None, &latest.drive.grove_version)
                    .unwrap()
                    .expect("unchanged committed root"),
                committed_root
            );
            for rho in [[1u8; 32], [3u8; 32]] {
                assert!(!abci_app
                    .platform
                    .drive
                    .has_nullifier(&rho, None, &mut vec![], latest)
                    .expect("uncommitted backfill"));
            }
            {
                let transaction = abci_app
                    .transaction
                    .read()
                    .expect("candidate transaction lock");
                assert_token_history(
                    abci_app.platform,
                    transaction.as_ref(),
                    2,
                    historical_token_history,
                );
                for rho in [[1u8; 32], [3u8; 32]] {
                    assert!(abci_app
                        .platform
                        .drive
                        .has_nullifier(&rho, transaction.as_ref(), &mut vec![], latest)
                        .expect("candidate union"));
                }
            }
            if restart_candidate {
                drop(abci_app);
                let TempPlatform {
                    platform: mut before_restart,
                    tempdir,
                } = platform;
                let rpc = std::mem::take(&mut before_restart.core_rpc);
                drop(before_restart);
                platform = TempPlatform::open_with_tempdir(tempdir, config.clone());
                platform.platform.core_rpc = rpc;
                assert_eq!(
                    platform
                        .state
                        .load()
                        .current_protocol_version_in_consensus(),
                    13
                );
                assert_eq!(
                    platform.state.load().last_committed_block_app_hash(),
                    Some(committed_root)
                );
                assert_eq!(
                    platform
                        .drive
                        .grove
                        .root_hash(None, &old.drive.grove_version)
                        .value
                        .expect("restart committed root"),
                    committed_root
                );
                for rho in [[1u8; 32], [3u8; 32]] {
                    assert!(!platform
                        .drive
                        .has_nullifier(&rho, None, &mut vec![], old)
                        .expect("dropped candidate union"));
                }
                abci_app = FullAbciApplication::new(&platform.platform);
                assert_token_history(abci_app.platform, None, 1, historical_token_history);
                let retry = abci_app
                    .process_proposal(accepted.clone())
                    .expect("retry A after restart");
                assert_eq!(retry.status, ProposalStatus::Accept as i32);
                assert_eq!(
                    retry.app_hash, accepted_root,
                    "restart must reproduce the same candidate root"
                );
                let transaction = abci_app.transaction.read().expect("retry transaction lock");
                assert_token_history(
                    abci_app.platform,
                    transaction.as_ref(),
                    2,
                    historical_token_history,
                );
                for rho in [[1u8; 32], [3u8; 32]] {
                    assert!(abci_app
                        .platform
                        .drive
                        .has_nullifier(&rho, transaction.as_ref(), &mut vec![], latest)
                        .expect("retried candidate union"));
                }
            }
            if reject_later_round {
                let response = abci_app.process_proposal(rejected).expect("candidate B");
                assert_eq!(response.status, ProposalStatus::Reject as i32);
                assert_token_history(abci_app.platform, None, 1, historical_token_history);
                assert!(
                    abci_app
                        .block_execution_context
                        .read()
                        .expect("context lock")
                        .as_ref()
                        .is_some_and(|context| context.block_state_info().round() == 1),
                    "B must be rejected after execution replaced A's context"
                );
                assert_eq!(
                    abci_app
                        .platform
                        .drive
                        .grove
                        .root_hash(None, &latest.drive.grove_version)
                        .unwrap()
                        .expect("committed root after rejection"),
                    committed_root
                );
            }
            if accept_alternate {
                let alternate_response = abci_app
                    .process_proposal(alternate)
                    .expect("alternate candidate");
                assert_eq!(alternate_response.status, ProposalStatus::Accept as i32);
                assert_eq!(
                    abci_app
                        .platform
                        .drive
                        .grove
                        .root_hash(None, &latest.drive.grove_version)
                        .value
                        .expect("no speculative commit"),
                    committed_root
                );
            }
            abci_app
                .finalize_block(finalize_request(&accepted, accepted_root.clone()))
                .expect("finalize accepted A after B's rejection");
            let state = abci_app.platform.state.load();
            assert_token_history(abci_app.platform, None, 2, historical_token_history);
            assert_eq!(
                state.current_protocol_version_in_consensus(),
                latest.protocol_version
            );
            assert_eq!(
                state.last_committed_block_app_hash().map(Vec::from),
                Some(accepted_root.clone())
            );
            drop(state);
            for rho in [[1u8; 32], [3u8; 32]] {
                assert!(abci_app
                    .platform
                    .drive
                    .has_nullifier(&rho, None, &mut vec![], latest)
                    .expect("committed backfill"));
            }
            observations.push(accepted_root);
            drop(abci_app);
            let TempPlatform {
                platform: before_restart,
                tempdir,
            } = platform;
            drop(before_restart);
            let reopened = TempPlatform::open_with_tempdir(tempdir, config.clone());
            assert_token_history(&reopened, None, 2, historical_token_history);
            assert_eq!(
                reopened
                    .state
                    .load()
                    .current_protocol_version_in_consensus(),
                latest.protocol_version
            );
            assert_eq!(
                reopened.state.load().next_epoch_protocol_version(),
                latest.protocol_version
            );
            for rho in [[1u8; 32], [3u8; 32]] {
                assert!(reopened
                    .drive
                    .has_nullifier(&rho, None, &mut vec![], latest)
                    .expect("backfill survives restart"));
            }
        }
        for observed in observations.iter().skip(1) {
            assert_eq!(*observed, observations[0], "clean, rejected/retried, reopened, alternate and restarted speculative candidates must finalize A identically");
        }
    }
}
