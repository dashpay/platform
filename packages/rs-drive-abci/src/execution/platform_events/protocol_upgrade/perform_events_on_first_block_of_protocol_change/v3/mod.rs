use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::version::ProtocolVersion;
use drive::drive::Drive;
use drive::grovedb::batch::GroveOp;
use drive::grovedb::Transaction;
use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use drive::util::batch::GroveDbOpBatch;

impl<C> Platform<C> {
    /// Runs the protocol change events of generation 2 and then the transition to protocol
    /// version 17 when the chain crosses it: the compilation readiness structures are
    /// created through the same helper genesis uses.
    ///
    /// The transition runs after the cache clear and refresh of generation 2 because it writes
    /// no contract; it touches only the `[Votes] / r` and `[PreFundedSpecializedBalances] / 129`
    /// trees, which no cache holds.
    pub(super) fn perform_events_on_first_block_of_protocol_change_v3(
        &self,
        platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: &Transaction,
        previous_protocol_version: ProtocolVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.perform_events_on_first_block_of_protocol_change_v2(
            platform_state,
            block_info,
            transaction,
            previous_protocol_version,
            platform_version,
        )?;

        if previous_protocol_version < 17 && platform_version.protocol_version >= 17 {
            self.transition_to_version_17_compilation_readiness(transaction, platform_version)?;
        }

        Ok(())
    }

    /// When transitioning to version 17 we add the compilation readiness tree under
    /// `[Votes]` with its contracts, deadlines and retired-rounds children, and the
    /// readiness fund sum tree under `[PreFundedSpecializedBalances]`.
    ///
    /// CONSENSUS-CRITICAL: the elements come from the shared helper
    /// `Drive::add_readiness_structure_operations`, which genesis (vote setup generation 1)
    /// also calls, so a fresh genesis-v17 node and an in-place-upgraded v17 node hold
    /// byte-identical subtrees. Every element is inserted only if absent, so a retried block
    /// after a rejected proposal, or a second run in the same transaction, is a no-op.
    pub(super) fn transition_to_version_17_compilation_readiness(
        &self,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut batch = GroveDbOpBatch::new();
        Drive::add_readiness_structure_operations(&mut batch);
        for operation in batch {
            let GroveOp::InsertOrReplace { element } = operation.op else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "the readiness structure helper emits only inserts",
                )));
            };
            let path = operation.path.to_path();
            let key = operation
                .key
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "the readiness structure helper emits only keyed inserts",
                )))?
                .get_key();
            self.drive.grove_insert_if_not_exists(
                path.as_slice().into(),
                &key,
                element,
                Some(transaction),
                None,
                &platform_version.drive,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::PlatformVersion;
    use dpp::voting::readiness::payer::ReadinessPayer;
    use dpp::voting::readiness::round::ReadinessRoundOpening;
    use drive::drive::prefunded_specialized_balances::{
        prefunded_specialized_balances_path, PREFUNDED_BALANCES_FOR_READINESS,
    };
    use drive::drive::votes::paths::{
        readiness_tree_path_vec, vote_root_path, READINESS_CONTRACTS_TREE_KEY,
        READINESS_DEADLINES_TREE_KEY, READINESS_RETIRED_ROUNDS_TREE_KEY, READINESS_TREE_KEY,
    };
    use drive::drive::votes::readiness::ReadinessRoundFunding;
    use drive::grovedb::{Element, PathQuery, Query, SizedQuery};
    use drive::query::QueryResultType;
    use platform_version::version::v16::PLATFORM_V16;
    use platform_version::version::v17::PLATFORM_V17;
    use rand::rngs::StdRng;
    use rand::{RngCore, SeedableRng};

    const INITIAL_FUNDING: u64 = 20_000_000;
    const CLEANUP_RESERVE: u64 = 1_000_000;

    fn block_info() -> BlockInfo {
        BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 100,
            epoch: Epoch::new(0).expect("expected epoch"),
        }
    }

    /// Reads every element one level under `path`, failing loudly on a query error.
    fn read_level(
        platform: &crate::platform_types::platform::Platform<crate::rpc::core::MockCoreRPCLike>,
        transaction: drive::grovedb::TransactionArg,
        path: Vec<Vec<u8>>,
        platform_version: &PlatformVersion,
    ) -> Vec<(Vec<u8>, Element)> {
        let mut query = Query::new();
        query.insert_all();
        let path_query = PathQuery::new(path, SizedQuery::new(query, None, None));
        let (results, _) = platform
            .drive
            .grove_get_raw_path_query(
                &path_query,
                transaction,
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected the level to be readable");
        results.to_key_elements()
    }

    /// CONSENSUS-CRITICAL equivalence guard for the v16 -> v17 boundary.
    ///
    /// The readiness structures are built two ways that MUST be byte-identical:
    ///
    ///  * GENESIS path: vote setup generation 1 inside `create_initial_state_structure`.
    ///  * UPGRADE path: `Platform::transition_to_version_17_compilation_readiness` at the
    ///    activation block on a node born at v16.
    ///
    /// Both go through `Drive::add_readiness_structure_operations`; this pins that they
    /// keep doing so, element by element under `[Votes] / r` and for
    /// `[PreFundedSpecializedBalances] / 129`.
    #[test]
    fn test_genesis_v17_and_upgrade_to_v17_build_identical_readiness_structures() {
        let platform_version_17 = &PLATFORM_V17;
        let grove_version = &platform_version_17.drive.grove_version;

        let platform_genesis = TestPlatformBuilder::new()
            .with_initial_protocol_version(PLATFORM_V17.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_upgraded = TestPlatformBuilder::new()
            .with_initial_protocol_version(PLATFORM_V16.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();

        assert!(
            platform_upgraded
                .drive
                .grove
                .get(
                    &vote_root_path(),
                    &[READINESS_TREE_KEY as u8],
                    None,
                    grove_version
                )
                .unwrap()
                .is_err(),
            "v16 genesis must not contain the readiness tree before the upgrade"
        );
        assert!(
            platform_upgraded
                .drive
                .grove
                .get(
                    &prefunded_specialized_balances_path(),
                    &[PREFUNDED_BALANCES_FOR_READINESS],
                    None,
                    grove_version
                )
                .unwrap()
                .is_err(),
            "v16 genesis must not contain the readiness fund tree before the upgrade"
        );

        let transaction = platform_upgraded.drive.grove.start_transaction();
        platform_upgraded
            .transition_to_version_17_compilation_readiness(&transaction, platform_version_17)
            .expect("upgrade: the readiness transition should succeed");

        for (path, key) in [
            (vote_root_path().to_vec(), READINESS_TREE_KEY as u8),
            (
                prefunded_specialized_balances_path().to_vec(),
                PREFUNDED_BALANCES_FOR_READINESS,
            ),
        ] {
            let genesis_element = platform_genesis
                .drive
                .grove
                .get(path.as_slice(), &[key], None, grove_version)
                .unwrap()
                .expect("genesis: element");
            let upgraded_element = platform_upgraded
                .drive
                .grove
                .get(path.as_slice(), &[key], Some(&transaction), grove_version)
                .unwrap()
                .expect("upgrade: element");
            assert_eq!(
                genesis_element, upgraded_element,
                "CONSENSUS FORK: the element at key {key} differs between a fresh genesis-v17 \
                 node and an in-place-upgraded v17 node"
            );
        }

        let genesis_children = read_level(
            &platform_genesis,
            None,
            readiness_tree_path_vec(),
            platform_version_17,
        );
        let upgraded_children = read_level(
            &platform_upgraded,
            Some(&transaction),
            readiness_tree_path_vec(),
            platform_version_17,
        );
        assert_eq!(
            genesis_children, upgraded_children,
            "CONSENSUS FORK: the [Votes] / r children differ between genesis and upgrade"
        );
        assert_eq!(
            genesis_children
                .iter()
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>(),
            vec![
                vec![READINESS_CONTRACTS_TREE_KEY],
                vec![READINESS_DEADLINES_TREE_KEY],
                vec![READINESS_RETIRED_ROUNDS_TREE_KEY],
            ]
        );
    }

    /// A rejected proposal drops its transaction; the retried block runs the transition
    /// again on the same state and must produce the same root, and a second run in one
    /// transaction changes nothing.
    #[test]
    fn test_transition_to_version_17_is_idempotent_and_replayable() {
        let platform_version_17 = &PLATFORM_V17;
        let grove_version = &platform_version_17.drive.grove_version;
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(PLATFORM_V16.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();

        let dropped = platform.drive.grove.start_transaction();
        platform
            .transition_to_version_17_compilation_readiness(&dropped, platform_version_17)
            .expect("expected the first attempt to succeed");
        let root_after_first = platform
            .drive
            .grove
            .root_hash(Some(&dropped), grove_version)
            .unwrap()
            .expect("expected a root hash");
        platform
            .drive
            .grove
            .rollback_transaction(&dropped)
            .expect("expected to roll back");

        let retried = platform.drive.grove.start_transaction();
        platform
            .transition_to_version_17_compilation_readiness(&retried, platform_version_17)
            .expect("expected the retry to succeed");
        platform
            .transition_to_version_17_compilation_readiness(&retried, platform_version_17)
            .expect("expected a second run in the same transaction to be a no-op");
        let root_after_retry = platform
            .drive
            .grove
            .root_hash(Some(&retried), grove_version)
            .unwrap()
            .expect("expected a root hash");
        assert_eq!(root_after_first, root_after_retry);
    }

    /// The dispatcher selects generation 3 at protocol version 17, which runs the readiness
    /// transition when crossing 17 and skips it when the previous version is already 17. The
    /// generation selected at 16 does not know the structures.
    #[test]
    fn test_transition_from_version_16_triggers_17_only_once() {
        let platform_version_17 = &PLATFORM_V17;
        let grove_version = &platform_version_17.drive.grove_version;
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(PLATFORM_V16.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();
        let platform_state = platform.state.load();
        let block_info = block_info();

        // At 16 the hook knows nothing of readiness.
        platform
            .perform_events_on_first_block_of_protocol_change(
                &platform_state,
                &block_info,
                &transaction,
                PLATFORM_V16.protocol_version - 1,
                &PLATFORM_V16,
            )
            .expect("expected the transition to 16 to succeed");
        assert!(platform
            .drive
            .grove
            .get(
                &vote_root_path(),
                &[READINESS_TREE_KEY as u8],
                Some(&transaction),
                grove_version,
            )
            .unwrap()
            .is_err());

        platform
            .perform_events_on_first_block_of_protocol_change(
                &platform_state,
                &block_info,
                &transaction,
                PLATFORM_V16.protocol_version,
                platform_version_17,
            )
            .expect("expected the transition from 16 to succeed");
        assert!(platform
            .drive
            .grove
            .get(
                &vote_root_path(),
                &[READINESS_TREE_KEY as u8],
                Some(&transaction),
                grove_version,
            )
            .unwrap()
            .is_ok());

        let root_before = platform
            .drive
            .grove
            .root_hash(Some(&transaction), grove_version)
            .unwrap()
            .expect("expected a root hash");
        platform
            .perform_events_on_first_block_of_protocol_change(
                &platform_state,
                &block_info,
                &transaction,
                platform_version_17.protocol_version,
                platform_version_17,
            )
            .expect("expected a same-version run to succeed");
        let root_after = platform
            .drive
            .grove
            .root_hash(Some(&transaction), grove_version)
            .unwrap()
            .expect("expected a root hash");
        assert_eq!(root_before, root_after);
    }

    /// Funds, deducts, cancels and conserves on a database born at 17 and on one upgraded
    /// from 16, so both shapes behave the same under the readiness fund flows.
    #[test]
    fn test_readiness_fund_flows_on_fresh_and_upgraded_databases() {
        let platform_version_17 = &PLATFORM_V17;
        for born_at in [PLATFORM_V16.protocol_version, PLATFORM_V17.protocol_version] {
            let platform = TestPlatformBuilder::new()
                .with_initial_protocol_version(born_at)
                .build_with_mock_rpc()
                .set_genesis_state();
            let transaction = platform.drive.grove.start_transaction();
            if born_at < PLATFORM_V17.protocol_version {
                platform
                    .transition_to_version_17_compilation_readiness(
                        &transaction,
                        platform_version_17,
                    )
                    .expect("upgrade");
            }

            // A payer with a balance, mirrored into the system credits so conservation holds.
            let mut rng = StdRng::seed_from_u64(born_at as u64);
            let payer = Identity::random_identity(1, Some(rng.next_u64()), platform_version_17)
                .expect("payer");
            let payer_id = payer.id();
            let payer_balance = payer.balance();
            platform
                .drive
                .add_new_identity(
                    payer,
                    false,
                    &block_info(),
                    true,
                    Some(&transaction),
                    platform_version_17,
                )
                .expect("add payer");
            platform
                .drive
                .add_to_system_credits(payer_balance, Some(&transaction), platform_version_17)
                .expect("system credits");
            assert!(platform
                .drive
                .calculate_total_credits_balance(Some(&transaction), &platform_version_17.drive)
                .expect("totals")
                .ok()
                .expect("verdict"));

            let contract_id = [born_at as u8; 32];
            let opening = ReadinessRoundOpening {
                contract_id: Identifier::from(contract_id),
                version: 1,
                bundle_digest: [0xD1u8; 32],
                preparation_profile: 1,
                accepted_at_ms: block_info().time_ms,
                accepted_at_height: block_info().height,
                payer: ReadinessPayer::Identity(payer_id),
            };
            let (round, operations) = platform
                .drive
                .open_readiness_round_operations(
                    opening,
                    ReadinessRoundFunding {
                        initial_funding: INITIAL_FUNDING,
                        cleanup_reserve: CLEANUP_RESERVE,
                    },
                    &block_info(),
                    &mut None,
                    Some(&transaction),
                    platform_version_17,
                )
                .expect("open");
            platform
                .drive
                .apply_batch_low_level_drive_operations(
                    None,
                    Some(&transaction),
                    operations,
                    &mut vec![],
                    &platform_version_17.drive,
                )
                .expect("apply open");
            assert_eq!(
                platform
                    .drive
                    .fetch_readiness_fund(
                        round.funding_id().to_buffer(),
                        Some(&transaction),
                        platform_version_17
                    )
                    .expect("fund"),
                Some(INITIAL_FUNDING)
            );
            assert!(platform
                .drive
                .calculate_total_credits_balance(Some(&transaction), &platform_version_17.drive)
                .expect("totals")
                .ok()
                .expect("verdict"));

            // A verification fee moves from the fund to the pool.
            let mut operations = platform
                .drive
                .deduct_from_readiness_fund_operations(
                    round.funding_id(),
                    10_000,
                    CLEANUP_RESERVE,
                    &mut None,
                    Some(&transaction),
                    platform_version_17,
                )
                .expect("deduct");
            let mut reads = vec![];
            operations.push(
                platform
                    .drive
                    .add_readiness_pool_credit_operation(
                        &block_info(),
                        10_000,
                        true,
                        Some(&transaction),
                        &mut reads,
                        platform_version_17,
                    )
                    .expect("pool"),
            );
            platform
                .drive
                .apply_batch_low_level_drive_operations(
                    None,
                    Some(&transaction),
                    operations,
                    &mut vec![],
                    &platform_version_17.drive,
                )
                .expect("apply deduct");
            assert!(platform
                .drive
                .calculate_total_credits_balance(Some(&transaction), &platform_version_17.drive)
                .expect("totals")
                .ok()
                .expect("verdict"));

            // Cancellation refunds the remainder minus the reserve.
            let (cancelled, operations) = platform
                .drive
                .cancel_readiness_round_operations(
                    contract_id,
                    CLEANUP_RESERVE,
                    &block_info(),
                    &mut None,
                    Some(&transaction),
                    platform_version_17,
                )
                .expect("cancel");
            assert!(cancelled.is_some());
            platform
                .drive
                .apply_batch_low_level_drive_operations(
                    None,
                    Some(&transaction),
                    operations,
                    &mut vec![],
                    &platform_version_17.drive,
                )
                .expect("apply cancel");
            let balance = platform
                .drive
                .fetch_identity_balance(
                    payer_id.to_buffer(),
                    Some(&transaction),
                    platform_version_17,
                )
                .expect("balance")
                .expect("payer");
            assert_eq!(balance, payer_balance - 10_000 - CLEANUP_RESERVE);
            assert!(platform
                .drive
                .calculate_total_credits_balance(Some(&transaction), &platform_version_17.drive)
                .expect("totals")
                .ok()
                .expect("verdict"));
        }
    }
}
