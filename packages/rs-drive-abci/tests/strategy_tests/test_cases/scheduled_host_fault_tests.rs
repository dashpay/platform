//! Three ways a block can fail, kept apart.
//!
//! Every masternode must execute every block to the same app hash, so a failure during block
//! execution has to be classified before anything is done about it. The book chapter on block
//! failure classes (`book/src/architecture/block-failure-classes.md`) describes the three
//! classes; these tests pin each one against the real block-execution entry points rather than
//! a unit that merely rejects a direct call.
//!
//! 1. **Paid failure.** A state transition fails deterministically on every node. It stays in
//!    the block, its owner is charged, the chain advances. For contracts this is a guest trap,
//!    a rejection or a failed native check; never a halt.
//! 2. **Node-local failure.** One node cannot execute a block the rest of the network can. That
//!    node stops until repaired, then re-executes the same block to the same app hash and
//!    commits it. Block validity never depended on that node.
//! 3. **Reproducible host fault in the scheduled phase.** Every node fails every proposal at
//!    the scheduled-event integration point, before any ordinary transaction is looked at,
//!    empty blocks included. No transaction removal can route around it; block production
//!    stops. The last test rehearses the recovery path that exists for it: an execution and
//!    fee identical hotfix on every node, a restart, and the replay of the very proposal that
//!    faulted.
//!
//! The fault is injected through `PlatformTestConfig::scheduled_event_host_fault`, a hook in
//! `run_block_proposal_v0` compiled only with the `testing-config` feature. Two platforms built
//! with the same seed are deterministic twins (every random draw is seeded and the mock Core
//! RPC is installed per platform), which is what lets a healthy node and a faulted node be
//! compared block for block.
#[cfg(test)]
mod tests {
    use crate::addresses_with_balance::AddressesWithBalance;
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
    use crate::masternodes::MasternodeListItemWithUpdates;
    use crate::strategy::{
        ChainExecutionOutcome, ChainExecutionParameters, FailureStrategy, NetworkStrategy,
        StrategyRandomness,
    };
    use dash_platform_macros::stack_size;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::QuorumHash;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::prelude::{Identifier, IdentityNonce};
    use dpp::serialization::PlatformSerializable;
    use dpp::tests::json_document::json_document_to_created_contract;
    use dpp::version::PlatformVersion;
    use drive_abci::abci::app::FullAbciApplication;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig, SCHEDULED_EVENT_HOST_FAULT_MESSAGE,
    };
    use drive_abci::mimic::test_quorum::TestQuorumInfo;
    use drive_abci::mimic::CHAIN_ID;
    use drive_abci::platform_types::platform::Platform;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::platform_types::signature_verification_quorum_set::{Quorums, SigningQuorum};
    use drive_abci::rpc::core::MockCoreRPCLike;
    use drive_abci::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::{BTreeMap, HashMap};
    use strategy_tests::frequency::Frequency;
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};
    use tenderdash_abci::proto::abci::response_process_proposal::ProposalStatus;
    use tenderdash_abci::proto::abci::tx_record::TxAction;
    use tenderdash_abci::proto::abci::{
        CommitInfo, RequestFinalizeBlock, RequestPrepareProposal, RequestProcessProposal,
        ResponseException, ResponsePrepareProposal,
    };
    use tenderdash_abci::proto::google::protobuf::Timestamp;
    use tenderdash_abci::proto::types::{
        Block, BlockId, Data, EvidenceList, Header, PartSetHeader,
    };
    use tenderdash_abci::proto::version::Consensus;
    use tenderdash_abci::Application;

    const STACK_SIZE: usize = 4 * 1024 * 1024;

    /// Short epochs so a single hand-built block can cross an epoch boundary.
    const EPOCH_TIME_LENGTH_S: u64 = 60;
    const BLOCK_SPACING_MS: u64 = 1_000;
    const MAX_TX_BYTES_PER_BLOCK: i64 = 40_000;
    /// The healthy prefix every twin runs before the rehearsal starts.
    const HEALTHY_PREFIX_BLOCKS: u64 = 5;
    const SEED: u64 = 7;

    /// One or two identity creates per block, so the healthy prefix and the hand-built
    /// blocks carry real signed transactions.
    fn strategy() -> NetworkStrategy {
        NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..3,
                        chance_per_block: None,
                    },
                    ..Default::default()
                },
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 50,
            extra_normal_mns: 0,
            validator_quorum_count: 10,
            chain_lock_quorum_count: 10,
            upgrading_info: None,
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        }
    }

    /// Platform state is stored to disk on every commit so a node can be reopened from its
    /// database, the way a restarted validator comes back. Commit signatures are not verified
    /// because the hand-built finalize requests below carry a zero signature.
    fn config(scheduled_event_host_fault: bool) -> PlatformConfig {
        PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                epoch_time_length_s: EPOCH_TIME_LENGTH_S,
                ..Default::default()
            },
            block_spacing_ms: BLOCK_SPACING_MS,
            testing_configs: PlatformTestConfig {
                store_platform_state: true,
                scheduled_event_host_fault,
                ..PlatformTestConfig::default_minimal_verifications()
            },
            ..Default::default()
        }
    }

    fn timestamp(time_ms: u64) -> Timestamp {
        Timestamp {
            seconds: (time_ms / 1000) as i64,
            nanos: ((time_ms % 1000) * 1_000_000) as i32,
        }
    }

    /// Everything a request for the next block needs from the chain, captured while the
    /// outcome is still alive so the requests can be built after it is gone.
    #[derive(Clone)]
    struct NextBlockInputs {
        height: u64,
        core_height: u32,
        proposer_pro_tx_hash: [u8; 32],
        quorum_hash: [u8; 32],
        time_ms: u64,
        proposed_app_version: u64,
    }

    /// The next block after `outcome`, proposed by the member at `proposer_slot` of the
    /// current quorum. The chain simulation walks the quorum in key order, one member per
    /// block, so slot `HEALTHY_PREFIX_BLOCKS` is the member it would pick next and later
    /// slots keep the proposer order ascending, which is what keeps the validator set from
    /// rotating under the hand-built block.
    fn next_block_inputs(
        outcome: &ChainExecutionOutcome,
        proposer_slot: usize,
        time_ms: u64,
        proposed_app_version: u64,
    ) -> NextBlockInputs {
        let platform_state = outcome.abci_app.platform.state.load();
        let quorum = outcome.current_quorum();
        let proposer = quorum
            .validator_map
            .values()
            .nth(proposer_slot)
            .expect("the quorum has a member at the requested slot");
        NextBlockInputs {
            height: platform_state.last_committed_block_height() + 1,
            core_height: platform_state.last_committed_core_height(),
            proposer_pro_tx_hash: proposer.pro_tx_hash.to_byte_array(),
            quorum_hash: quorum.quorum_hash.to_byte_array(),
            time_ms,
            proposed_app_version,
        }
    }

    /// The `RequestPrepareProposal` the node's own Tenderdash sends when it is the proposer.
    fn prepare_request(inputs: &NextBlockInputs, txs: Vec<Vec<u8>>) -> RequestPrepareProposal {
        RequestPrepareProposal {
            max_tx_bytes: MAX_TX_BYTES_PER_BLOCK,
            txs,
            local_last_commit: None,
            misbehavior: vec![],
            height: inputs.height as i64,
            time: Some(timestamp(inputs.time_ms)),
            next_validators_hash: [0u8; 32].to_vec(),
            round: 0,
            core_chain_locked_height: inputs.core_height,
            proposer_pro_tx_hash: inputs.proposer_pro_tx_hash.to_vec(),
            proposed_app_version: inputs.proposed_app_version,
            // Tenderdash sends 0 on prepare proposal; the app version it puts in the header
            // is the one that comes back in the response.
            version: Some(Consensus { block: 0, app: 0 }),
            quorum_hash: inputs.quorum_hash.to_vec(),
        }
    }

    /// The `RequestProcessProposal` every validator receives for a block that was never
    /// prepared by this node: the block as the network gossips it.
    fn process_request(
        inputs: &NextBlockInputs,
        round: i32,
        txs: Vec<Vec<u8>>,
        hash: [u8; 32],
    ) -> RequestProcessProposal {
        RequestProcessProposal {
            txs,
            proposed_last_commit: None,
            misbehavior: vec![],
            hash: hash.to_vec(),
            height: inputs.height as i64,
            time: Some(timestamp(inputs.time_ms)),
            next_validators_hash: [0u8; 32].to_vec(),
            round,
            core_chain_locked_height: inputs.core_height,
            core_chain_lock_update: None,
            proposer_pro_tx_hash: inputs.proposer_pro_tx_hash.to_vec(),
            proposed_app_version: inputs.proposed_app_version,
            version: Some(Consensus {
                block: 0,
                app: PlatformVersion::latest().protocol_version as u64,
            }),
            quorum_hash: inputs.quorum_hash.to_vec(),
        }
    }

    /// The `RequestProcessProposal` Tenderdash sends back for the block it just had prepared:
    /// the transactions the proposer kept, in order, the app version the response asked for in
    /// the header, and the block hash Tenderdash has computed by now.
    fn process_request_for_prepared_block(
        prepare_request: &RequestPrepareProposal,
        prepare_response: &ResponsePrepareProposal,
        hash: [u8; 32],
    ) -> RequestProcessProposal {
        let txs = prepare_response
            .tx_records
            .iter()
            .filter(|record| {
                record.action != TxAction::Removed as i32
                    && record.action != TxAction::Delayed as i32
            })
            .map(|record| record.tx.clone())
            .collect();

        RequestProcessProposal {
            txs,
            proposed_last_commit: None,
            misbehavior: vec![],
            hash: hash.to_vec(),
            height: prepare_request.height,
            time: prepare_request.time.clone(),
            next_validators_hash: prepare_request.next_validators_hash.clone(),
            round: prepare_request.round,
            core_chain_locked_height: prepare_response
                .core_chain_lock_update
                .as_ref()
                .map(|chain_lock| chain_lock.core_block_height)
                .unwrap_or(prepare_request.core_chain_locked_height),
            core_chain_lock_update: prepare_response.core_chain_lock_update.clone(),
            proposer_pro_tx_hash: prepare_request.proposer_pro_tx_hash.clone(),
            proposed_app_version: prepare_request.proposed_app_version,
            version: Some(Consensus {
                block: 0,
                app: prepare_response.app_version,
            }),
            quorum_hash: prepare_request.quorum_hash.clone(),
        }
    }

    /// The `RequestFinalizeBlock` Tenderdash sends once the block described by
    /// `process_request` has been voted in with app hash `app_hash`. Built the way the mimic
    /// block executor builds it: the header repeats the executed block's height, round, time,
    /// proposer, core height, app hash and proposed app version, the commit names the current
    /// validator set quorum, no vote extensions (the block carries no withdrawals), and the
    /// block signature is zero because commit signature verification is off in `config`.
    fn finalize_request(
        process_request: &RequestProcessProposal,
        app_hash: &[u8],
        quorum_hash: [u8; 32],
    ) -> RequestFinalizeBlock {
        RequestFinalizeBlock {
            commit: Some(CommitInfo {
                round: process_request.round,
                quorum_hash: quorum_hash.to_vec(),
                block_signature: [0u8; 96].to_vec(),
                threshold_vote_extensions: vec![],
            }),
            misbehavior: vec![],
            hash: process_request.hash.clone(),
            height: process_request.height,
            round: process_request.round,
            block: Some(Block {
                header: Some(Header {
                    version: process_request.version.clone(),
                    chain_id: CHAIN_ID.to_string(),
                    height: process_request.height,
                    time: process_request.time.clone(),
                    last_block_id: None,
                    last_commit_hash: [0u8; 32].to_vec(),
                    data_hash: [0u8; 32].to_vec(),
                    validators_hash: quorum_hash.to_vec(),
                    next_validators_hash: quorum_hash.to_vec(),
                    consensus_hash: [0u8; 32].to_vec(),
                    next_consensus_hash: [0u8; 32].to_vec(),
                    app_hash: app_hash.to_vec(),
                    results_hash: [0u8; 32].to_vec(),
                    evidence_hash: vec![],
                    proposed_app_version: process_request.proposed_app_version,
                    proposer_pro_tx_hash: process_request.proposer_pro_tx_hash.clone(),
                    core_chain_locked_height: process_request.core_chain_locked_height,
                }),
                data: Some(Data {
                    txs: process_request.txs.clone(),
                }),
                evidence: Some(EvidenceList { evidence: vec![] }),
                last_commit: None,
                core_chain_lock: None,
            }),
            block_id: Some(BlockId {
                hash: process_request.hash.clone(),
                part_set_header: Some(PartSetHeader {
                    total: 0,
                    hash: vec![0u8; 32],
                }),
                state_id: [0u8; 32].to_vec(),
            }),
        }
    }

    /// The exception must be the injected fault and nothing else: a different error would
    /// mean the request never reached the scheduled-event point.
    fn assert_injected(exception: ResponseException, what: &str) {
        assert!(
            exception.error.contains(SCHEDULED_EVENT_HOST_FAULT_MESSAGE),
            "{what}: expected the injected scheduled-event host fault, got: {}",
            exception.error
        );
    }

    fn committed_root(platform: &Platform<MockCoreRPCLike>) -> [u8; 32] {
        platform
            .drive
            .grove
            .root_hash(None, &PlatformVersion::latest().drive.grove_version)
            .unwrap()
            .expect("committed root hash")
    }

    fn committed_height(platform: &Platform<MockCoreRPCLike>) -> u64 {
        platform.state.load().last_committed_block_height()
    }

    fn committed_epoch_index(platform: &Platform<MockCoreRPCLike>) -> u16 {
        platform.state.load().last_committed_block_epoch().index
    }

    /// The protocol version votes recorded in committed state, read from the versions counter
    /// tree rather than from the in-memory cache.
    fn committed_votes(platform: &Platform<MockCoreRPCLike>) -> BTreeMap<u32, u64> {
        platform
            .drive
            .fetch_versions_with_counter(None, &PlatformVersion::latest().drive)
            .expect("committed version votes")
            .into_iter()
            .collect()
    }

    /// Reopens a platform from its own database, carrying the mock Core RPC over: the
    /// restart every hotfix ends with. `config` is the configuration the restarted binary
    /// runs with.
    fn reopen(
        platform: TempPlatform<MockCoreRPCLike>,
        config: PlatformConfig,
    ) -> TempPlatform<MockCoreRPCLike> {
        let TempPlatform {
            platform: mut old_platform,
            tempdir,
        } = platform;
        let core_rpc = std::mem::take(&mut old_platform.core_rpc);
        drop(old_platform);
        let mut reopened = TempPlatform::open_with_tempdir(tempdir, config);
        reopened.platform.core_rpc = core_rpc;
        reopened
    }

    /// The network the chain simulation needs to keep producing blocks: masternodes, quorums
    /// and the strategy. Cloned out of an outcome so the outcome can be dropped.
    struct ChainTemplate {
        proposers: Vec<MasternodeListItemWithUpdates>,
        validator_quorums: BTreeMap<QuorumHash, TestQuorumInfo>,
        instant_lock_quorums: Quorums<SigningQuorum>,
        strategy: NetworkStrategy,
    }

    fn chain_template(outcome: &ChainExecutionOutcome) -> ChainTemplate {
        ChainTemplate {
            proposers: outcome.proposers.clone(),
            validator_quorums: outcome.validator_quorums.clone(),
            instant_lock_quorums: outcome.instant_lock_quorums.clone(),
            strategy: outcome.strategy.clone(),
        }
    }

    /// The bookkeeping a node must carry to keep generating transactions after the hand-built
    /// block.
    struct Continuation {
        identities: Vec<Identity>,
        identity_nonce_counter: BTreeMap<Identifier, IdentityNonce>,
        identity_contract_nonce_counter: BTreeMap<(Identifier, Identifier), IdentityNonce>,
    }

    /// Real signed transactions for the block at `block_info`, generated once from the
    /// outcome's identities, signer and instant lock quorums. Returns the serialized
    /// transactions, the identities they create, and the mutated bookkeeping so a node can
    /// carry on producing blocks after this one is committed.
    async fn generate_transitions(
        outcome: &mut ChainExecutionOutcome<'_>,
        block_info: &BlockInfo,
    ) -> (Vec<Vec<u8>>, Vec<Identity>, Continuation) {
        let ChainExecutionOutcome {
            abci_app,
            identities,
            addresses_with_balance,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            strategy,
            signer,
            ..
        } = outcome;
        let mut current_identities = identities.clone();
        let mut current_addresses_with_balance = addresses_with_balance.clone();
        let mut current_identity_nonce_counter = identity_nonce_counter.clone();
        let mut current_identity_contract_nonce_counter = identity_contract_nonce_counter.clone();
        let mut current_votes = BTreeMap::new();
        let mut shielded_state = None;
        let identities_before = current_identities.len();

        let (state_transitions, finalize_block_operations) = strategy
            .state_transitions_for_block(
                abci_app.platform,
                1,
                block_info,
                &mut current_identities,
                &mut current_addresses_with_balance,
                &mut current_identity_nonce_counter,
                &mut current_identity_contract_nonce_counter,
                &mut current_votes,
                signer,
                &mut StdRng::seed_from_u64(block_info.height),
                instant_lock_quorums,
                &mut shielded_state,
            )
            .await;
        assert!(
            finalize_block_operations.is_empty(),
            "identity creates carry no finalize block operations"
        );
        assert!(
            !state_transitions.is_empty(),
            "test premise: the hand-built block must carry transactions"
        );
        signer.commit_block_keys();

        let txs = state_transitions
            .iter()
            .map(|transition| {
                transition
                    .serialize_to_bytes()
                    .expect("serialize state transition")
            })
            .collect();
        let created = current_identities[identities_before..].to_vec();

        (
            txs,
            created,
            Continuation {
                identities: current_identities,
                identity_nonce_counter: current_identity_nonce_counter,
                identity_contract_nonce_counter: current_identity_contract_nonce_counter,
            },
        )
    }

    /// Runs `block_count` more blocks on `abci_app` from the state a node reaches after the
    /// hand-built block at `inputs` was committed with `continuation`'s bookkeeping.
    async fn continue_after<'a>(
        abci_app: FullAbciApplication<'a, MockCoreRPCLike>,
        template: &ChainTemplate,
        inputs: &NextBlockInputs,
        continuation: &Continuation,
        block_count: u64,
    ) -> ChainExecutionOutcome<'a> {
        let current_validator_quorum_hash = abci_app
            .platform
            .state
            .load()
            .current_validator_set_quorum_hash();
        continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start: inputs.height + 1,
                core_height_start: inputs.core_height,
                block_count,
                proposers: template.proposers.clone(),
                validator_quorums: template.validator_quorums.clone(),
                current_validator_quorum_hash,
                instant_lock_quorums: template.instant_lock_quorums.clone(),
                current_proposer_versions: None,
                current_identity_nonce_counter: continuation.identity_nonce_counter.clone(),
                current_identity_contract_nonce_counter: continuation
                    .identity_contract_nonce_counter
                    .clone(),
                current_votes: BTreeMap::default(),
                start_time_ms: template.strategy.start_time_ms,
                current_time_ms: inputs.time_ms + BLOCK_SPACING_MS,
                current_identities: continuation.identities.clone(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            template.strategy.clone(),
            config(false),
            StrategyRandomness::SeedEntropy(SEED),
        )
        .await
    }

    /// Class 1. A contract update with a skipped property position fails at block 3 with a
    /// deterministic consensus error. The owner has an identity to pay with, so the transition
    /// stays in the block as a paid consensus error, is charged, and the chain goes on to block
    /// 10 as if nothing happened. This is what a guest trap, a rejection or a failed native
    /// check inside a contract call must look like: a result, never a halt.
    #[stack_size(STACK_SIZE)]
    #[test]
    async fn paid_failure_stays_in_the_block_pays_and_the_chain_advances() {
        const BAD_UPDATE_BLOCK: u64 = 3;
        const MISSING_POSITION_CODE: u32 = 10411;

        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let mut bad_update = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable-bad-update-skipped-position.json",
            2,
            false,
            platform_version,
        )
        .expect("expected to get contract from a json document");
        bad_update.data_contract_mut().set_version(2);

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(
                    contract,
                    Some(BTreeMap::from([(BAD_UPDATE_BLOCK, bad_update)])),
                )],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    ..Default::default()
                },
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            failure_testing: Some(FailureStrategy {
                deterministic_start_seed: None,
                dont_finalize_block: false,
                expect_every_block_errors_with_codes: vec![],
                rounds_before_successful_block: None,
                expect_specific_block_errors_with_codes: HashMap::from([(
                    BAD_UPDATE_BLOCK,
                    vec![MISSING_POSITION_CODE],
                )]),
            }),
            verify_state_transition_results: true,
            ..strategy()
        };
        let config = config(false);
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            10,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        )
        .await;

        let bad_update_results = outcome
            .state_transition_results_per_block
            .get(&BAD_UPDATE_BLOCK)
            .expect("block 3 must have results");
        let (_, bad_update_result) = bad_update_results
            .iter()
            .find(|(_, result)| result.code != 0)
            .expect("the bad update must be in block 3 with a non-zero code");
        assert_eq!(
            bad_update_result.code, MISSING_POSITION_CODE,
            "the failure must be the deterministic consensus error"
        );
        assert!(
            bad_update_result.gas_used > 0,
            "a paid failure is charged: gas used must be positive"
        );
        assert_eq!(
            committed_height(outcome.abci_app.platform),
            10,
            "the chain advances past a paid failure"
        );
    }

    /// Class 2. Twins A and B commit the same five blocks. A is armed and fails the block the
    /// network agreed on; B executes it. A's committed state is untouched by the failure.
    /// Repaired, A re-executes the identical block to B's app hash and transaction results,
    /// both finalize it through the normal handler, and both end up at the same committed root.
    /// Block validity never depended on A.
    #[stack_size(STACK_SIZE)]
    #[test]
    async fn node_local_fault_stops_only_that_node_and_it_commits_the_same_block_after_repair() {
        let mut platform_a = TestPlatformBuilder::new()
            .with_config(config(false))
            .build_with_mock_rpc();
        let mut platform_b = TestPlatformBuilder::new()
            .with_config(config(false))
            .build_with_mock_rpc();

        let mut outcome_a = run_chain_for_strategy(
            &mut platform_a,
            HEALTHY_PREFIX_BLOCKS,
            strategy(),
            config(false),
            SEED,
            &mut None,
            &mut None,
        )
        .await;
        let outcome_b = run_chain_for_strategy(
            &mut platform_b,
            HEALTHY_PREFIX_BLOCKS,
            strategy(),
            config(false),
            SEED,
            &mut None,
            &mut None,
        )
        .await;
        assert_eq!(
            committed_root(outcome_a.abci_app.platform),
            committed_root(outcome_b.abci_app.platform),
            "test premise: twins built from one seed commit the same prefix"
        );

        let block_time_ms = outcome_a.end_time_ms + BLOCK_SPACING_MS;
        let inputs = next_block_inputs(
            &outcome_a,
            HEALTHY_PREFIX_BLOCKS as usize,
            block_time_ms,
            PlatformVersion::latest().protocol_version as u64,
        );
        let block_info = BlockInfo {
            time_ms: block_time_ms,
            height: inputs.height,
            core_height: inputs.core_height,
            epoch: Epoch::new(outcome_a.end_epoch_index).expect("epoch"),
        };
        let (txs, _, _) = generate_transitions(&mut outcome_a, &block_info).await;
        let block = process_request(&inputs, 0, txs, [0x51u8; 32]);
        drop(outcome_a);

        // B, healthy, executes the network's block.
        let healthy = outcome_b
            .abci_app
            .process_proposal(block.clone())
            .expect("a healthy node processes the block");
        assert_eq!(healthy.status, ProposalStatus::Accept as i32);
        assert!(
            !healthy.tx_results.is_empty(),
            "test premise: the block carries transactions"
        );

        // A, faulted, cannot execute the same block. Its committed state is untouched.
        let root_before_fault = committed_root(&platform_a);
        platform_a.config.testing_configs.scheduled_event_host_fault = true;
        let faulted_app = FullAbciApplication::new(&platform_a);
        let exception = faulted_app
            .process_proposal(block.clone())
            .expect_err("the faulted node fails the block");
        assert_injected(exception, "faulted node process proposal");
        drop(faulted_app);
        assert_eq!(
            committed_root(&platform_a),
            root_before_fault,
            "a failed proposal leaves committed state untouched"
        );
        assert_eq!(committed_height(&platform_a), HEALTHY_PREFIX_BLOCKS);

        // Repair A and let it re-execute the block it could not process before.
        platform_a.config.testing_configs.scheduled_event_host_fault = false;
        let repaired_app = FullAbciApplication::new(&platform_a);
        let repaired = repaired_app
            .process_proposal(block.clone())
            .expect("the repaired node processes the block");
        assert_eq!(repaired.status, ProposalStatus::Accept as i32);
        assert_eq!(
            repaired.app_hash, healthy.app_hash,
            "the repaired node reaches the healthy node's app hash"
        );
        assert_eq!(
            repaired.tx_results, healthy.tx_results,
            "the repaired node reaches the healthy node's transaction results"
        );

        // Both nodes finalize the block through the normal handler.
        let finalize = finalize_request(&block, &healthy.app_hash, inputs.quorum_hash);
        repaired_app
            .finalize_block(finalize.clone())
            .expect("the repaired node finalizes the block");
        outcome_b
            .abci_app
            .finalize_block(finalize)
            .expect("the healthy node finalizes the block");
        drop(repaired_app);
        drop(outcome_b);

        assert_eq!(committed_height(&platform_a), inputs.height);
        assert_eq!(committed_height(&platform_b), inputs.height);
        assert_eq!(
            committed_root(&platform_a),
            committed_root(&platform_b),
            "both nodes commit the same block"
        );
        assert_eq!(committed_root(&platform_a).to_vec(), healthy.app_hash);
    }

    /// Class 3. Both twins are armed after the healthy prefix. Every attempt to produce the
    /// next block fails with the injected fault on both nodes: as proposer and as validator,
    /// in three rounds, from two proposers, with and without transactions, while signalling an
    /// upgrade, and past the epoch boundary. Nothing is committed, no upgrade vote is recorded,
    /// the next epoch protocol version does not move. This is the halt, and the reason a
    /// signalling upgrade cannot progress on a halted chain: the vote is written inside the
    /// block, tallied on an epoch-change block, and activated by a later block, and none of
    /// those blocks can be produced.
    #[stack_size(STACK_SIZE)]
    #[test]
    async fn reproducible_scheduled_fault_halts_every_node_before_ordinary_transactions() {
        let mut platform_a = TestPlatformBuilder::new()
            .with_config(config(false))
            .build_with_mock_rpc();
        let mut platform_b = TestPlatformBuilder::new()
            .with_config(config(false))
            .build_with_mock_rpc();

        let mut outcome_a = run_chain_for_strategy(
            &mut platform_a,
            HEALTHY_PREFIX_BLOCKS,
            strategy(),
            config(false),
            SEED,
            &mut None,
            &mut None,
        )
        .await;
        let outcome_b = run_chain_for_strategy(
            &mut platform_b,
            HEALTHY_PREFIX_BLOCKS,
            strategy(),
            config(false),
            SEED,
            &mut None,
            &mut None,
        )
        .await;
        assert_eq!(
            committed_root(outcome_a.abci_app.platform),
            committed_root(outcome_b.abci_app.platform),
            "test premise: twins built from one seed commit the same prefix"
        );

        let signalled_version = PlatformVersion::latest().protocol_version as u64 + 1;
        let next_block_time_ms = outcome_a.end_time_ms + BLOCK_SPACING_MS;
        // The healthy prefix ends well inside epoch 0; this block time lands in epoch 1.
        let epoch_boundary_time_ms =
            outcome_a.strategy.start_time_ms + EPOCH_TIME_LENGTH_S * 1000 + BLOCK_SPACING_MS;
        assert!(
            epoch_boundary_time_ms > next_block_time_ms,
            "test premise: the epoch boundary lies after the next block time"
        );

        let first_slot = HEALTHY_PREFIX_BLOCKS as usize;
        let inputs_by_proposer = [
            next_block_inputs(
                &outcome_a,
                first_slot,
                next_block_time_ms,
                signalled_version,
            ),
            next_block_inputs(
                &outcome_a,
                first_slot + 1,
                next_block_time_ms,
                signalled_version,
            ),
        ];
        let epoch_change_inputs = next_block_inputs(
            &outcome_a,
            first_slot + 2,
            epoch_boundary_time_ms,
            signalled_version,
        );
        let block_info = BlockInfo {
            time_ms: next_block_time_ms,
            height: inputs_by_proposer[0].height,
            core_height: inputs_by_proposer[0].core_height,
            epoch: Epoch::new(outcome_a.end_epoch_index).expect("epoch"),
        };
        let (txs, _, _) = generate_transitions(&mut outcome_a, &block_info).await;
        drop(outcome_a);
        drop(outcome_b);

        let roots_before = (committed_root(&platform_a), committed_root(&platform_b));
        let votes_before = (committed_votes(&platform_a), committed_votes(&platform_b));
        let next_versions_before = (
            platform_a.state.load().next_epoch_protocol_version(),
            platform_b.state.load().next_epoch_protocol_version(),
        );
        assert!(
            !votes_before.0.contains_key(&(signalled_version as u32)),
            "test premise: nobody has voted for the signalled version yet"
        );

        platform_a.config.testing_configs.scheduled_event_host_fault = true;
        platform_b.config.testing_configs.scheduled_event_host_fault = true;
        let nodes = [
            ("node A", FullAbciApplication::new(&platform_a)),
            ("node B", FullAbciApplication::new(&platform_b)),
        ];

        let transaction_sets: [Vec<Vec<u8>>; 2] = [vec![], txs];
        for (name, app) in nodes.iter() {
            for inputs in inputs_by_proposer.iter() {
                for round in 0..=2i32 {
                    for txs in transaction_sets.iter() {
                        let what = format!(
                            "{name} prepare proposal round {round} with {} transactions",
                            txs.len()
                        );
                        let mut prepare = prepare_request(inputs, txs.clone());
                        prepare.round = round;
                        let exception = app
                            .prepare_proposal(prepare)
                            .expect_err("every prepare proposal fails");
                        assert_injected(exception, &what);

                        let what = format!(
                            "{name} process proposal round {round} with {} transactions",
                            txs.len()
                        );
                        let exception = app
                            .process_proposal(process_request(
                                inputs,
                                round,
                                txs.clone(),
                                [round as u8 + 0x60; 32],
                            ))
                            .expect_err("every process proposal fails");
                        assert_injected(exception, &what);
                    }
                }
            }

            // The epoch-change block, the one that would tally the votes, cannot be produced
            // either: the fault sits after the epoch change events and before the ordinary
            // transactions, so the tally runs inside the failing proposal.
            let exception = app
                .prepare_proposal(prepare_request(&epoch_change_inputs, vec![]))
                .expect_err("the epoch change proposal fails");
            assert_injected(exception, &format!("{name} epoch change prepare proposal"));
            let exception = app
                .process_proposal(process_request(
                    &epoch_change_inputs,
                    0,
                    vec![],
                    [0x70u8; 32],
                ))
                .expect_err("the epoch change proposal fails");
            assert_injected(exception, &format!("{name} epoch change process proposal"));
        }
        drop(nodes);

        assert_eq!(
            (committed_root(&platform_a), committed_root(&platform_b)),
            roots_before,
            "no attempt reached committed state"
        );
        assert_eq!(
            (committed_height(&platform_a), committed_height(&platform_b)),
            (HEALTHY_PREFIX_BLOCKS, HEALTHY_PREFIX_BLOCKS)
        );
        assert_eq!(
            (committed_votes(&platform_a), committed_votes(&platform_b)),
            votes_before,
            "no upgrade vote is recorded by a proposal that fails"
        );
        assert_eq!(
            (
                platform_a.state.load().next_epoch_protocol_version(),
                platform_b.state.load().next_epoch_protocol_version(),
            ),
            next_versions_before,
            "the next epoch protocol version cannot move on a halted chain"
        );
    }

    /// The recovery path that exists for class 3, rehearsed end to end. Twins A (to be
    /// faulted) and C (control) share the healthy prefix. One concrete proposal R is built for
    /// the next height: non-empty, signalling an upgrade, and the first block of a new epoch.
    /// C executes R. A is armed and fails R as proposer and, twice, as validator; the attempts
    /// leave nothing behind. A is then "hotfixed": the fault is removed by an execution and fee
    /// identical change, and A restarts from disk. Tenderdash's write-ahead log re-delivers
    /// the unchanged R, and A executes it to C's app hash and transaction results. Both
    /// finalize R through the normal handler and continue with identical blocks.
    #[stack_size(STACK_SIZE)]
    #[test]
    async fn execution_identical_hotfix_replays_the_faulted_proposal_and_the_chain_continues() {
        let mut platform_a = TestPlatformBuilder::new()
            .with_config(config(false))
            .build_with_mock_rpc();
        let mut platform_c = TestPlatformBuilder::new()
            .with_config(config(false))
            .build_with_mock_rpc();

        let mut outcome_a = run_chain_for_strategy(
            &mut platform_a,
            HEALTHY_PREFIX_BLOCKS,
            strategy(),
            config(false),
            SEED,
            &mut None,
            &mut None,
        )
        .await;
        let outcome_c = run_chain_for_strategy(
            &mut platform_c,
            HEALTHY_PREFIX_BLOCKS,
            strategy(),
            config(false),
            SEED,
            &mut None,
            &mut None,
        )
        .await;
        assert_eq!(
            committed_root(outcome_a.abci_app.platform),
            committed_root(outcome_c.abci_app.platform),
            "test premise: twins built from one seed commit the same prefix"
        );
        assert_eq!(
            committed_epoch_index(outcome_a.abci_app.platform),
            0,
            "test premise: the healthy prefix ends in epoch 0"
        );
        let template = chain_template(&outcome_a);

        // 1. The concrete proposal R: real transactions, an upgrade signal, and a block time
        //    that crosses the 60 second epoch boundary.
        let signalled_version = PlatformVersion::latest().protocol_version as u64 + 1;
        let block_time_ms =
            outcome_a.strategy.start_time_ms + EPOCH_TIME_LENGTH_S * 1000 + BLOCK_SPACING_MS;
        assert!(
            block_time_ms > outcome_a.end_time_ms,
            "test premise: the epoch boundary lies after the healthy prefix"
        );
        let inputs = next_block_inputs(
            &outcome_a,
            HEALTHY_PREFIX_BLOCKS as usize,
            block_time_ms,
            signalled_version,
        );
        let block_info = BlockInfo {
            time_ms: block_time_ms,
            height: inputs.height,
            core_height: inputs.core_height,
            epoch: Epoch::new(1).expect("epoch"),
        };
        let (txs, created_identities, continuation) =
            generate_transitions(&mut outcome_a, &block_info).await;
        drop(outcome_a);
        let prepare = prepare_request(&inputs, txs);
        let prepare_response_c = outcome_c
            .abci_app
            .prepare_proposal(prepare.clone())
            .expect("the control node prepares R");
        let proposal =
            process_request_for_prepared_block(&prepare, &prepare_response_c, [0x52u8; 32]);
        assert!(
            !proposal.txs.is_empty(),
            "test premise: R carries transactions"
        );

        // 2. C executes R: this is the app hash and the results the network agrees on.
        let control = outcome_c
            .abci_app
            .process_proposal(proposal.clone())
            .expect("the control node processes R");
        assert_eq!(control.status, ProposalStatus::Accept as i32);

        // 3. A is faulted. As proposer and as validator, R fails, and a second delivery of the
        //    same message (the write-ahead log replaying on a restart that did not fix
        //    anything) fails the same way. Nothing reaches committed state, no vote is written.
        let root_before = committed_root(&platform_a);
        let votes_before = committed_votes(&platform_a);
        assert!(
            !votes_before.contains_key(&(signalled_version as u32)),
            "test premise: no vote for the signalled version exists yet"
        );
        platform_a.config.testing_configs.scheduled_event_host_fault = true;
        let faulted_app = FullAbciApplication::new(&platform_a);
        let exception = faulted_app
            .prepare_proposal(prepare.clone())
            .expect_err("the faulted node fails R as proposer");
        assert_injected(exception, "faulted node prepare proposal");
        for attempt in 1..=2 {
            let exception = faulted_app
                .process_proposal(proposal.clone())
                .expect_err("the faulted node fails R as validator");
            assert_injected(
                exception,
                &format!("faulted node process proposal {attempt}"),
            );
        }
        drop(faulted_app);
        assert_eq!(committed_root(&platform_a), root_before);
        assert_eq!(committed_height(&platform_a), HEALTHY_PREFIX_BLOCKS);
        assert_eq!(
            committed_votes(&platform_a),
            votes_before,
            "the failed attempts recorded no vote"
        );

        // 4. The hotfix: the fault is gone from the binary and the node restarts from disk. Its
        //    state is where the halt left it.
        let platform_a = reopen(platform_a, config(false));
        assert_eq!(committed_height(&platform_a), HEALTHY_PREFIX_BLOCKS);
        assert_eq!(committed_root(&platform_a), root_before);
        assert_eq!(
            platform_a.state.load().last_committed_block_app_hash(),
            Some(root_before),
            "the reopened state carries the pre-halt app hash"
        );

        // 5. The write-ahead log replays R unchanged. The hotfixed node executes it to the
        //    control node's app hash and transaction results.
        let recovered_app = FullAbciApplication::new(&platform_a);
        let recovered = recovered_app
            .process_proposal(proposal.clone())
            .expect("the hotfixed node processes the replayed R");
        assert_eq!(recovered.status, ProposalStatus::Accept as i32);
        assert_eq!(
            recovered.app_hash, control.app_hash,
            "the replayed proposal executes to the control node's app hash"
        );
        assert_eq!(
            recovered.tx_results, control.tx_results,
            "the replayed proposal executes to the control node's transaction results"
        );

        // 6. Both nodes finalize R through the normal handler.
        let finalize = finalize_request(&proposal, &control.app_hash, inputs.quorum_hash);
        recovered_app
            .finalize_block(finalize.clone())
            .expect("the hotfixed node finalizes R");
        outcome_c
            .abci_app
            .finalize_block(finalize)
            .expect("the control node finalizes R");

        let platform_c_ref = outcome_c.abci_app.platform;
        assert_eq!(committed_height(&platform_a), inputs.height);
        assert_eq!(committed_height(platform_c_ref), inputs.height);
        assert_eq!(
            committed_root(&platform_a),
            committed_root(platform_c_ref),
            "both nodes commit R to the same root"
        );
        assert_eq!(committed_root(&platform_a).to_vec(), control.app_hash);
        assert_eq!(
            committed_epoch_index(&platform_a),
            1,
            "R is the first block of epoch 1"
        );
        assert_eq!(committed_epoch_index(platform_c_ref), 1);
        // The epoch change cleared the prefix's votes; the only vote in state is the one the
        // committed R recorded. The failed attempts recorded none.
        let expected_votes = BTreeMap::from([(signalled_version as u32, 1u64)]);
        assert_eq!(committed_votes(&platform_a), expected_votes);
        assert_eq!(committed_votes(platform_c_ref), expected_votes);
        for identity in created_identities.iter() {
            let balance_a = platform_a
                .drive
                .fetch_identity_balance(identity.id().to_buffer(), None, PlatformVersion::latest())
                .expect("fetch balance on the hotfixed node");
            let balance_c = platform_c_ref
                .drive
                .fetch_identity_balance(identity.id().to_buffer(), None, PlatformVersion::latest())
                .expect("fetch balance on the control node");
            assert!(
                balance_a.is_some(),
                "the identity created by R exists on the hotfixed node"
            );
            assert_eq!(balance_a, balance_c);
        }

        // 7. The chain continues on both nodes with identical blocks.
        let ChainExecutionOutcome {
            abci_app: control_app,
            ..
        } = outcome_c;
        let continued_a = continue_after(recovered_app, &template, &inputs, &continuation, 5).await;
        let continued_c = continue_after(control_app, &template, &inputs, &continuation, 5).await;
        let continued_platform_c = continued_c.abci_app.platform;
        drop(continued_a);

        assert_eq!(committed_height(&platform_a), inputs.height + 5);
        assert_eq!(committed_height(continued_platform_c), inputs.height + 5);
        assert_eq!(
            committed_root(&platform_a),
            committed_root(continued_platform_c),
            "the hotfixed node and the control node stay in step after recovery"
        );
    }
}
