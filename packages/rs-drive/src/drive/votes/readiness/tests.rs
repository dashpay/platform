//! Tests of the compilation readiness storage, all through the dispatchers at the latest
//! platform version.

use crate::drive::prefunded_specialized_balances::{
    prefunded_specialized_balances_path, PREFUNDED_BALANCES_FOR_READINESS,
    PREFUNDED_BALANCES_FOR_VOTING,
};
use crate::drive::votes::paths::{
    readiness_contract_tree_path, readiness_deadline_tree_path_vec, readiness_round_tree_path,
    vote_root_path, READINESS_CURRENT_ROUND_POINTER_KEY, READINESS_ROUND_RECORD_KEY,
    READINESS_ROUND_REPORTS_TREE_KEY, READINESS_ROUND_SCAN_CURSOR_KEY, READINESS_TREE_KEY,
};
use crate::drive::votes::readiness::{
    ReadinessCleanupOutcome, ReadinessRoundFunding, RetiredReadinessRound,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use crate::util::test_helpers::test_utils::identities::create_test_identity_with_rng;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::payer::ReadinessPayer;
use dpp::voting::readiness::report_record::ReadinessReportRecord;
use dpp::voting::readiness::round::{ReadinessEvaluation, ReadinessRound, ReadinessRoundOpening};
use dpp::voting::readiness::scan_cursor::ReadinessScanCursor;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::collections::HashMap;

const INITIAL_FUNDING: Credits = 20_000_000;
const CLEANUP_RESERVE: Credits = 1_000_000;
const PAYER_BALANCE: Credits = 100_000_000;
const MIN_WAIT_MS: u64 = 120_000;
const MAX_WAIT_MS: u64 = 3_600_000;

const FUNDING: ReadinessRoundFunding = ReadinessRoundFunding {
    initial_funding: INITIAL_FUNDING,
    cleanup_reserve: CLEANUP_RESERVE,
};

fn block_info(time_ms: u64, height: u64, core_height: u32) -> BlockInfo {
    BlockInfo {
        time_ms,
        height,
        core_height,
        epoch: Epoch::new(0).expect("epoch"),
    }
}

fn opening(contract_id: [u8; 32], payer: Identifier, height: u64) -> ReadinessRoundOpening {
    ReadinessRoundOpening {
        contract_id: Identifier::from(contract_id),
        version: 1,
        bundle_digest: [0xD1u8; 32],
        preparation_profile: 1,
        accepted_at_ms: 1_000_000,
        accepted_at_height: height,
        payer: ReadinessPayer::Identity(payer),
    }
}

/// A drive at the latest version with one funded payer identity.
fn setup() -> (Drive, Identifier) {
    let drive = setup_drive_with_initial_state_structure(None);
    let platform_version = PlatformVersion::latest();
    let mut rng = StdRng::seed_from_u64(7);
    let payer =
        create_test_identity_with_rng(&drive, [0xAAu8; 32], &mut rng, None, platform_version)
            .expect("payer")
            .id();
    drive
        .add_to_identity_balance(
            payer.to_buffer(),
            PAYER_BALANCE,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("fund payer");
    (drive, payer)
}

fn apply(
    drive: &Drive,
    operations: Vec<LowLevelDriveOperation>,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> FeeResult {
    let mut drive_operations = vec![];
    drive
        .apply_batch_low_level_drive_operations(
            None,
            transaction,
            operations,
            &mut drive_operations,
            &platform_version.drive,
        )
        .expect("apply");
    Drive::calculate_fee(
        None,
        Some(drive_operations),
        &Epoch::new(0).expect("epoch"),
        drive.config.epochs_per_era,
        platform_version,
        None,
    )
    .expect("fee")
}

fn estimate(
    drive: &Drive,
    layer_info: HashMap<KeyInfoPath, EstimatedLayerInformation>,
    operations: Vec<LowLevelDriveOperation>,
    platform_version: &PlatformVersion,
) -> FeeResult {
    let mut drive_operations = vec![];
    drive
        .apply_batch_low_level_drive_operations(
            Some(layer_info),
            None,
            operations,
            &mut drive_operations,
            &platform_version.drive,
        )
        .expect("estimate");
    Drive::calculate_fee(
        None,
        Some(drive_operations),
        &Epoch::new(0).expect("epoch"),
        drive.config.epochs_per_era,
        platform_version,
        None,
    )
    .expect("fee")
}

fn open_round(
    drive: &Drive,
    contract_id: [u8; 32],
    payer: Identifier,
    height: u64,
    transaction: TransactionArg,
) -> (ReadinessRound, FeeResult) {
    let platform_version = PlatformVersion::latest();
    let (round, operations) = drive
        .open_readiness_round_operations(
            opening(contract_id, payer, height),
            FUNDING,
            &block_info(1_000_000, height, 100),
            &mut None,
            transaction,
            platform_version,
        )
        .expect("open");
    let fee = apply(drive, operations, transaction, platform_version);
    (round, fee)
}

fn insert_reports(
    drive: &Drive,
    round: &ReadinessRound,
    from: u32,
    to: u32,
    transaction: TransactionArg,
) {
    let platform_version = PlatformVersion::latest();
    let record = ReadinessReportRecord::new(round.accepted_at_height() + 1, 1, platform_version)
        .expect("record");
    let contract_id = round.contract_id().to_buffer();
    let mut operations = vec![];
    for i in from..to {
        let (inserted, ops) = drive
            .insert_readiness_report_operations(
                contract_id,
                round.round_id(),
                pro_tx_hash(i),
                &record,
                &mut None,
                transaction,
                platform_version,
            )
            .expect("insert");
        assert!(inserted);
        operations.extend(ops);
        if operations.len() >= 256 {
            apply(
                drive,
                std::mem::take(&mut operations),
                transaction,
                platform_version,
            );
        }
    }
    if !operations.is_empty() {
        apply(drive, operations, transaction, platform_version);
    }
}

fn pro_tx_hash(i: u32) -> [u8; 32] {
    let mut hash = [0u8; 32];
    hash[..4].copy_from_slice(&i.to_be_bytes());
    hash[31] = 0x77;
    hash
}

fn payer_balance(drive: &Drive, payer: Identifier, transaction: TransactionArg) -> Credits {
    drive
        .fetch_identity_balance(payer.to_buffer(), transaction, PlatformVersion::latest())
        .expect("balance")
        .expect("payer exists")
}

fn pool_credits(drive: &Drive, transaction: TransactionArg) -> Credits {
    drive
        .get_epoch_processing_credits_for_distribution(
            &Epoch::new(0).expect("epoch"),
            transaction,
            PlatformVersion::latest(),
        )
        .expect("pool")
}

fn conservation_holds(drive: &Drive, transaction: TransactionArg) -> bool {
    let totals = drive
        .calculate_total_credits_balance(transaction, &PlatformVersion::latest().drive)
        .expect("totals");
    totals.ok().expect("verdict")
}

mod genesis {
    use super::*;

    #[test]
    fn should_not_create_the_readiness_structures_at_protocol_version_14() {
        let platform_version = PlatformVersion::get(14).expect("v14");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let grove_version = &platform_version.drive.grove_version;
        assert!(drive
            .grove
            .get(
                &vote_root_path(),
                &[READINESS_TREE_KEY as u8],
                None,
                grove_version
            )
            .unwrap()
            .is_err());
        assert!(drive
            .grove
            .get(
                &prefunded_specialized_balances_path(),
                &[PREFUNDED_BALANCES_FOR_READINESS],
                None,
                grove_version
            )
            .unwrap()
            .is_err());
        assert!(drive
            .grove
            .get(
                &prefunded_specialized_balances_path(),
                &[PREFUNDED_BALANCES_FOR_VOTING],
                None,
                grove_version
            )
            .unwrap()
            .is_ok());
    }

    #[test]
    fn should_create_the_readiness_structures_at_the_latest_protocol_version() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let grove_version = &platform_version.drive.grove_version;
        let readiness = drive
            .grove
            .get(
                &vote_root_path(),
                &[READINESS_TREE_KEY as u8],
                None,
                grove_version,
            )
            .unwrap()
            .expect("readiness tree");
        assert!(matches!(readiness, Element::Tree(..)));
        let funds = drive
            .grove
            .get(
                &prefunded_specialized_balances_path(),
                &[PREFUNDED_BALANCES_FOR_READINESS],
                None,
                grove_version,
            )
            .unwrap()
            .expect("readiness fund tree");
        assert!(matches!(funds, Element::SumTree(..)));
        assert_eq!(
            drive
                .fetch_readiness_rounds_page(None, 10, None, platform_version)
                .expect("page"),
            Vec::<[u8; 32]>::new()
        );
        assert_eq!(
            drive
                .fetch_retired_readiness_round(None, platform_version)
                .expect("retired"),
            None
        );
        assert_eq!(
            drive
                .fetch_readiness_rounds_due(u64::MAX, 10, None, platform_version)
                .expect("due"),
            vec![]
        );
    }

    #[test]
    fn should_report_the_readiness_methods_inactive_at_protocol_version_14() {
        let platform_version = PlatformVersion::get(14).expect("v14");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let result = drive.fetch_readiness_round([1u8; 32], None, platform_version);
        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::VersionNotActive { .. }))
            ),
            "{result:?}"
        );
        let result =
            drive.add_readiness_fund(Identifier::from([1u8; 32]), 1, None, platform_version);
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::VersionNotActive { .. }))
        ));
    }
}

mod funds {
    use super::*;

    #[test]
    fn should_add_deduct_and_empty_a_readiness_fund_on_its_own_tree() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let fund_id = Identifier::from([0xF1u8; 32]);

        drive
            .add_readiness_fund(fund_id, 5_000, None, platform_version)
            .expect("add");
        drive
            .add_readiness_fund(fund_id, 500, None, platform_version)
            .expect("top up");
        assert_eq!(
            drive
                .fetch_readiness_fund(fund_id.to_buffer(), None, platform_version)
                .expect("fetch"),
            Some(5_500)
        );
        // The voting family does not see it.
        assert_eq!(
            drive
                .fetch_prefunded_specialized_balance(fund_id.to_buffer(), None, platform_version)
                .expect("fetch voting"),
            None
        );

        let operations = drive
            .deduct_from_readiness_fund_operations(
                fund_id,
                1_500,
                0,
                &mut None,
                None,
                platform_version,
            )
            .expect("deduct");
        apply(&drive, operations, None, platform_version);
        assert_eq!(
            drive
                .fetch_readiness_fund(fund_id.to_buffer(), None, platform_version)
                .expect("fetch"),
            Some(4_000)
        );

        let (credits, operations) = drive
            .empty_readiness_fund_operations(fund_id, true, &mut None, None, platform_version)
            .expect("empty");
        assert_eq!(credits, 4_000);
        apply(&drive, operations, None, platform_version);
        assert_eq!(
            drive
                .fetch_readiness_fund(fund_id.to_buffer(), None, platform_version)
                .expect("fetch"),
            None
        );
    }

    #[test]
    fn should_refuse_a_deduction_below_zero_or_into_the_cleanup_reserve() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let fund_id = Identifier::from([0xF2u8; 32]);
        drive
            .add_readiness_fund(fund_id, 2_000, None, platform_version)
            .expect("add");

        let result = drive.deduct_from_readiness_fund_operations(
            fund_id,
            2_001,
            0,
            &mut None,
            None,
            platform_version,
        );
        assert!(matches!(
            result,
            Err(Error::Drive(
                DriveError::PrefundedSpecializedBalanceNotEnough(2_000, 2_001)
            ))
        ));

        let result = drive.deduct_from_readiness_fund_operations(
            fund_id,
            1_001,
            1_000,
            &mut None,
            None,
            platform_version,
        );
        assert!(
            matches!(
                result,
                Err(Error::Drive(
                    DriveError::PrefundedSpecializedBalanceNotEnough(1_000, 1_001)
                ))
            ),
            "{result:?}"
        );

        let result = drive.deduct_from_readiness_fund_operations(
            Identifier::from([0xF3u8; 32]),
            1,
            0,
            &mut None,
            None,
            platform_version,
        );
        assert!(matches!(
            result,
            Err(Error::Drive(
                DriveError::PrefundedSpecializedBalanceDoesNotExist(_)
            ))
        ));
    }

    #[test]
    fn should_prove_and_verify_a_readiness_fund_present_and_absent() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let fund_id = Identifier::from([0xF4u8; 32]);
        drive
            .add_readiness_fund(fund_id, 777, None, platform_version)
            .expect("add");

        let proof = drive
            .prove_readiness_fund(fund_id.to_buffer(), None, platform_version)
            .expect("prove");
        let (_, balance) =
            Drive::verify_readiness_fund(&proof, fund_id.to_buffer(), false, platform_version)
                .expect("verify");
        assert_eq!(balance, Some(777));

        let absent = [0xF5u8; 32];
        let proof = drive
            .prove_readiness_fund(absent, None, platform_version)
            .expect("prove absent");
        let (_, balance) =
            Drive::verify_readiness_fund(&proof, absent, false, platform_version).expect("verify");
        assert_eq!(balance, None);
    }

    #[test]
    fn should_count_readiness_funds_in_credit_conservation() {
        let (drive, payer) = setup();
        let platform_version = PlatformVersion::latest();
        drive
            .add_to_system_credits(PAYER_BALANCE, None, platform_version)
            .expect("system credits");
        assert!(conservation_holds(&drive, None));

        let (round, _) = open_round(&drive, [1u8; 32], payer, 10, None);
        assert_eq!(
            drive
                .fetch_readiness_fund(round.funding_id().to_buffer(), None, platform_version)
                .expect("fund"),
            Some(INITIAL_FUNDING)
        );
        assert_eq!(
            payer_balance(&drive, payer, None),
            PAYER_BALANCE - INITIAL_FUNDING
        );
        assert!(conservation_holds(&drive, None));

        // Spend part of it into the pool, as a report verification fee would.
        let mut operations = drive
            .deduct_from_readiness_fund_operations(
                round.funding_id(),
                10_000,
                CLEANUP_RESERVE,
                &mut None,
                None,
                platform_version,
            )
            .expect("deduct");
        let mut reads = vec![];
        operations.push(
            drive
                .add_readiness_pool_credit_operation(
                    &block_info(1, 1, 1),
                    10_000,
                    true,
                    None,
                    &mut reads,
                    platform_version,
                )
                .expect("pool"),
        );
        apply(&drive, operations, None, platform_version);
        assert_eq!(pool_credits(&drive, None), 10_000);
        assert!(conservation_holds(&drive, None));
    }
}

mod rounds {
    use super::*;

    #[test]
    fn should_open_a_round_with_its_pointer_record_count_tree_and_fund() {
        let (drive, payer) = setup();
        let platform_version = PlatformVersion::latest();
        let contract_id = [1u8; 32];

        let (round, _) = open_round(&drive, contract_id, payer, 10, None);

        let fetched = drive
            .fetch_readiness_round(contract_id, None, platform_version)
            .expect("fetch")
            .expect("round");
        assert_eq!(fetched, round);
        assert!(fetched.is_pending());
        assert_eq!(fetched.last_evaluated(), None);
        assert_eq!(
            drive
                .fetch_readiness_round_raw_count(
                    contract_id,
                    round.round_id(),
                    None,
                    platform_version
                )
                .expect("count"),
            0
        );
        assert_eq!(
            drive
                .fetch_readiness_fund(round.funding_id().to_buffer(), None, platform_version)
                .expect("fund"),
            Some(INITIAL_FUNDING)
        );
        assert_eq!(
            payer_balance(&drive, payer, None),
            PAYER_BALANCE - INITIAL_FUNDING
        );
        assert_eq!(
            drive
                .fetch_readiness_rounds_page(None, 10, None, platform_version)
                .expect("page"),
            vec![contract_id]
        );
        assert_eq!(
            drive
                .fetch_readiness_scan_cursor(contract_id, round.round_id(), None, platform_version)
                .expect("cursor"),
            None
        );
    }

    #[test]
    fn should_refuse_to_open_a_round_the_payer_cannot_fund() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let mut rng = StdRng::seed_from_u64(8);
        let payer =
            create_test_identity_with_rng(&drive, [0xABu8; 32], &mut rng, None, platform_version)
                .expect("payer")
                .id();
        let result = drive.open_readiness_round_operations(
            opening([1u8; 32], payer, 10),
            FUNDING,
            &block_info(1_000_000, 10, 100),
            &mut None,
            None,
            platform_version,
        );
        assert!(
            matches!(
                result,
                Err(Error::Identity(
                    crate::error::identity::IdentityError::IdentityInsufficientBalance(_)
                ))
            ),
            "{result:?}"
        );
    }

    #[test]
    fn should_insert_a_report_once_and_count_distinct_reporters() {
        let (drive, payer) = setup();
        let platform_version = PlatformVersion::latest();
        let contract_id = [1u8; 32];
        let (round, _) = open_round(&drive, contract_id, payer, 10, None);
        let record = ReadinessReportRecord::new(11, 1, platform_version).expect("record");

        let (inserted, operations) = drive
            .insert_readiness_report_operations(
                contract_id,
                round.round_id(),
                pro_tx_hash(1),
                &record,
                &mut None,
                None,
                platform_version,
            )
            .expect("insert");
        assert!(inserted);
        apply(&drive, operations, None, platform_version);

        // A retransmission: not new, no write.
        let (inserted, operations) = drive
            .insert_readiness_report_operations(
                contract_id,
                round.round_id(),
                pro_tx_hash(1),
                &record,
                &mut None,
                None,
                platform_version,
            )
            .expect("insert again");
        assert!(!inserted);
        assert!(operations
            .iter()
            .all(|operation| !matches!(operation, LowLevelDriveOperation::GroveOperation(_))));

        insert_reports(&drive, &round, 2, 5, None);
        assert_eq!(
            drive
                .fetch_readiness_round_raw_count(
                    contract_id,
                    round.round_id(),
                    None,
                    platform_version
                )
                .expect("count"),
            4
        );

        // Pages come back in key order and `after` continues from the last key.
        let page = drive
            .fetch_readiness_reports_page(
                contract_id,
                round.round_id(),
                None,
                3,
                None,
                platform_version,
            )
            .expect("page");
        assert_eq!(
            page.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
            vec![pro_tx_hash(1), pro_tx_hash(2), pro_tx_hash(3)]
        );
        assert_eq!(page[0].1, record);
        let page = drive
            .fetch_readiness_reports_page(
                contract_id,
                round.round_id(),
                Some(pro_tx_hash(3)),
                3,
                None,
                platform_version,
            )
            .expect("page");
        assert_eq!(
            page.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
            vec![pro_tx_hash(4)]
        );

        // Proof of one report.
        let proof = drive
            .prove_readiness_report(
                contract_id,
                round.round_id(),
                pro_tx_hash(2),
                None,
                platform_version,
            )
            .expect("prove");
        let (_, proved) = Drive::verify_readiness_report(
            &proof,
            contract_id,
            round.round_id(),
            pro_tx_hash(2),
            false,
            platform_version,
        )
        .expect("verify");
        assert_eq!(proved, Some(record.clone()));
        let proof = drive
            .prove_readiness_report(
                contract_id,
                round.round_id(),
                pro_tx_hash(9),
                None,
                platform_version,
            )
            .expect("prove absent");
        let (_, proved) = Drive::verify_readiness_report(
            &proof,
            contract_id,
            round.round_id(),
            pro_tx_hash(9),
            false,
            platform_version,
        )
        .expect("verify absent");
        assert_eq!(proved, None);
    }

    #[test]
    fn should_prune_named_reports_and_decrement_the_count() {
        let (drive, payer) = setup();
        let platform_version = PlatformVersion::latest();
        let contract_id = [1u8; 32];
        let (round, _) = open_round(&drive, contract_id, payer, 10, None);
        insert_reports(&drive, &round, 0, 6, None);

        let operations = drive
            .prune_readiness_reports_operations(
                contract_id,
                round.round_id(),
                &[pro_tx_hash(1), pro_tx_hash(4)],
                &mut None,
                None,
                platform_version,
            )
            .expect("prune");
        apply(&drive, operations, None, platform_version);
        assert_eq!(
            drive
                .fetch_readiness_round_raw_count(
                    contract_id,
                    round.round_id(),
                    None,
                    platform_version
                )
                .expect("count"),
            4
        );
        let page = drive
            .fetch_readiness_reports_page(
                contract_id,
                round.round_id(),
                None,
                10,
                None,
                platform_version,
            )
            .expect("page");
        assert_eq!(
            page.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
            vec![
                pro_tx_hash(0),
                pro_tx_hash(2),
                pro_tx_hash(3),
                pro_tx_hash(5)
            ]
        );
    }

    #[test]
    fn should_store_advance_and_clear_a_scan_cursor() {
        let (drive, payer) = setup();
        let platform_version = PlatformVersion::latest();
        let contract_id = [1u8; 32];
        let (round, _) = open_round(&drive, contract_id, payer, 10, None);

        let mut cursor = ReadinessScanCursor::new(100, 400, platform_version).expect("cursor");
        let operations = drive
            .store_readiness_scan_cursor_operations(
                contract_id,
                round.round_id(),
                &cursor,
                &mut None,
                None,
                platform_version,
            )
            .expect("store");
        apply(&drive, operations, None, platform_version);
        cursor
            .advance(pro_tx_hash(511), 512, 500, 12)
            .expect("advance");
        let operations = drive
            .store_readiness_scan_cursor_operations(
                contract_id,
                round.round_id(),
                &cursor,
                &mut None,
                None,
                platform_version,
            )
            .expect("store again");
        apply(&drive, operations, None, platform_version);
        assert_eq!(
            drive
                .fetch_readiness_scan_cursor(contract_id, round.round_id(), None, platform_version)
                .expect("fetch"),
            Some(cursor)
        );

        let operations = drive
            .clear_readiness_scan_cursor_operations(
                contract_id,
                round.round_id(),
                &mut None,
                None,
                platform_version,
            )
            .expect("clear");
        apply(&drive, operations, None, platform_version);
        assert_eq!(
            drive
                .fetch_readiness_scan_cursor(contract_id, round.round_id(), None, platform_version)
                .expect("fetch"),
            None
        );
        // Clearing an absent cursor writes nothing.
        let operations = drive
            .clear_readiness_scan_cursor_operations(
                contract_id,
                round.round_id(),
                &mut None,
                None,
                platform_version,
            )
            .expect("clear absent");
        assert!(operations
            .iter()
            .all(|operation| !matches!(operation, LowLevelDriveOperation::GroveOperation(_))));
    }

    #[test]
    fn should_persist_the_evaluation_mark_and_the_fairness_cursor() {
        let (drive, payer) = setup();
        let platform_version = PlatformVersion::latest();
        let contract_id = [1u8; 32];
        let (mut round, _) = open_round(&drive, contract_id, payer, 10, None);

        round.set_evaluation(
            ReadinessEvaluation {
                core_height: 100,
                raw_count: 4,
            },
            true,
        );
        let operations = drive
            .update_readiness_round_evaluation_operations(&round, &mut None, None, platform_version)
            .expect("update");
        apply(&drive, operations, None, platform_version);
        let fetched = drive
            .fetch_readiness_round(contract_id, None, platform_version)
            .expect("fetch")
            .expect("round");
        assert_eq!(fetched, round);
        assert!(fetched.awaiting_funding());

        assert_eq!(
            drive
                .fetch_readiness_evaluation_cursor(None, platform_version)
                .expect("cursor"),
            None
        );
        let operations = drive
            .store_readiness_evaluation_cursor_operations(
                Some(contract_id),
                &mut None,
                None,
                platform_version,
            )
            .expect("store");
        apply(&drive, operations, None, platform_version);
        assert_eq!(
            drive
                .fetch_readiness_evaluation_cursor(None, platform_version)
                .expect("cursor"),
            Some(contract_id)
        );
        let operations = drive
            .store_readiness_evaluation_cursor_operations(None, &mut None, None, platform_version)
            .expect("clear");
        apply(&drive, operations, None, platform_version);
        assert_eq!(
            drive
                .fetch_readiness_evaluation_cursor(None, platform_version)
                .expect("cursor"),
            None
        );
    }

    #[test]
    fn should_record_a_crossing_queue_its_deadline_and_find_it_when_due() {
        let (drive, payer) = setup();
        let platform_version = PlatformVersion::latest();
        let contract_id = [1u8; 32];
        let (mut round, _) = open_round(&drive, contract_id, payer, 10, None);
        let cursor = ReadinessScanCursor::new(100, 5, platform_version).expect("cursor");
        let operations = drive
            .store_readiness_scan_cursor_operations(
                contract_id,
                round.round_id(),
                &cursor,
                &mut None,
                None,
                platform_version,
            )
            .expect("store");
        apply(&drive, operations, None, platform_version);

        let crossing_ms = 1_000_000 + 600_000;
        let (deadline_ms, operations) = drive
            .record_readiness_crossing_operations(
                &mut round,
                crossing_ms,
                MIN_WAIT_MS,
                MAX_WAIT_MS,
                &mut None,
                None,
                platform_version,
            )
            .expect("crossing");
        assert_eq!(deadline_ms, crossing_ms + 600_000);
        apply(&drive, operations, None, platform_version);

        let fetched = drive
            .fetch_readiness_round(contract_id, None, platform_version)
            .expect("fetch")
            .expect("round");
        assert!(!fetched.is_pending());
        assert_eq!(fetched.deadline_ms(), Some(deadline_ms));
        assert_eq!(
            drive
                .fetch_readiness_scan_cursor(contract_id, round.round_id(), None, platform_version)
                .expect("cursor"),
            None
        );
        assert_eq!(
            drive
                .fetch_readiness_rounds_due(deadline_ms - 1, 10, None, platform_version)
                .expect("due"),
            vec![]
        );
        let due = drive
            .fetch_readiness_rounds_due(deadline_ms, 10, None, platform_version)
            .expect("due");
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].deadline_ms, deadline_ms);
        assert_eq!(due[0].contract_id, contract_id);
        assert_eq!(due[0].round_id, round.round_id());

        // A second crossing on the same round is a code error.
        let result = drive.record_readiness_crossing_operations(
            &mut round,
            crossing_ms + 1,
            MIN_WAIT_MS,
            MAX_WAIT_MS,
            &mut None,
            None,
            platform_version,
        );
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
        ));
    }

    #[test]
    fn should_prove_and_verify_a_round_present_and_absent() {
        let (drive, payer) = setup();
        let platform_version = PlatformVersion::latest();
        let contract_id = [1u8; 32];

        let proof = drive
            .prove_readiness_round(contract_id, None, platform_version)
            .expect("prove absent");
        let (_, verified) =
            Drive::verify_readiness_round(&proof, contract_id, false, platform_version)
                .expect("verify absent");
        assert_eq!(verified, None);

        let (round, _) = open_round(&drive, contract_id, payer, 10, None);
        insert_reports(&drive, &round, 0, 3, None);
        let proof = drive
            .prove_readiness_round(contract_id, None, platform_version)
            .expect("prove");
        let (_, verified) =
            Drive::verify_readiness_round(&proof, contract_id, false, platform_version)
                .expect("verify");
        let verified = verified.expect("round");
        assert_eq!(verified.round, round);
        assert_eq!(verified.raw_count, 3);
    }
}

mod retirement {
    use super::*;

    #[test]
    fn should_replace_a_round_in_one_batch_and_net_the_payer_settlement() {
        let (drive, payer) = setup();
        let platform_version = PlatformVersion::latest();
        let contract_id = [1u8; 32];
        let (mut first, _) = open_round(&drive, contract_id, payer, 10, None);
        insert_reports(&drive, &first, 0, 5, None);
        let (_, operations) = drive
            .record_readiness_crossing_operations(
                &mut first,
                1_500_000,
                MIN_WAIT_MS,
                MAX_WAIT_MS,
                &mut None,
                None,
                platform_version,
            )
            .expect("crossing");
        apply(&drive, operations, None, platform_version);
        let first_deadline = first.deadline_ms().expect("deadline");
        let balance_before = payer_balance(&drive, payer, None);

        // Replacement at a later height: a different round id for the same bundle.
        let (second, _) = open_round(&drive, contract_id, payer, 20, None);
        assert_ne!(second.round_id(), first.round_id());

        // Pointer swapped, new round empty, old round unreachable through the pointer.
        let current = drive
            .fetch_readiness_round(contract_id, None, platform_version)
            .expect("fetch")
            .expect("round");
        assert_eq!(current, second);
        assert_eq!(
            drive
                .fetch_readiness_round_raw_count(
                    contract_id,
                    second.round_id(),
                    None,
                    platform_version
                )
                .expect("count"),
            0
        );
        // The old subtree is still on disk under its own key until the cleanup drains it.
        assert_eq!(
            drive
                .fetch_readiness_round_raw_count(
                    contract_id,
                    first.round_id(),
                    None,
                    platform_version
                )
                .expect("count"),
            5
        );
        assert_eq!(
            drive
                .fetch_retired_readiness_round(None, platform_version)
                .expect("retired"),
            Some(RetiredReadinessRound {
                round_id: first.round_id(),
                contract_id
            })
        );
        // The old deadline entry is gone.
        assert_eq!(
            drive
                .fetch_readiness_rounds_due(first_deadline, 10, None, platform_version)
                .expect("due"),
            vec![]
        );
        // Old fund settled: cleanup reserve to the pool, the remainder netted against the new
        // funding in one balance write.
        assert_eq!(
            drive
                .fetch_readiness_fund(first.funding_id().to_buffer(), None, platform_version)
                .expect("old fund"),
            None
        );
        assert_eq!(
            drive
                .fetch_readiness_fund(second.funding_id().to_buffer(), None, platform_version)
                .expect("new fund"),
            Some(INITIAL_FUNDING)
        );
        assert_eq!(pool_credits(&drive, None), CLEANUP_RESERVE);
        assert_eq!(
            payer_balance(&drive, payer, None),
            balance_before + (INITIAL_FUNDING - CLEANUP_RESERVE) - INITIAL_FUNDING
        );
        // Old reports cannot be inserted through the pointer: the pointer names the new round.
        let record = ReadinessReportRecord::new(21, 1, platform_version).expect("record");
        let current_round_id = drive
            .fetch_readiness_current_round_id_operations(
                contract_id,
                None,
                &mut vec![],
                platform_version,
            )
            .expect("pointer")
            .expect("pointer present");
        assert_eq!(current_round_id, second.round_id());
        let (inserted, operations) = drive
            .insert_readiness_report_operations(
                contract_id,
                current_round_id,
                pro_tx_hash(0),
                &record,
                &mut None,
                None,
                platform_version,
            )
            .expect("insert");
        assert!(
            inserted,
            "a reporter of the old round starts over in the new one"
        );
        apply(&drive, operations, None, platform_version);
    }

    /// Retiring a round never opens its reports tree, so replacing a round holding 2,000
    /// reports costs the same as replacing an empty one up to one bounded read: fetching
    /// the retiring round's record loads its merk root node, and the reports count tree
    /// element in that node carries a 32-byte root key and a wider count only once reports
    /// exist. The storage fee is identical; the processing fee differs by that read alone.
    #[test]
    fn should_charge_the_same_for_replacing_a_full_round_and_an_empty_one() {
        let platform_version = PlatformVersion::latest();
        let contract_id = [1u8; 32];

        let (drive_full, payer_full) = setup();
        let (full, _) = open_round(&drive_full, contract_id, payer_full, 10, None);
        insert_reports(&drive_full, &full, 0, 2_000, None);
        let (_, fee_full) = open_round(&drive_full, contract_id, payer_full, 20, None);

        let (drive_empty, payer_empty) = setup();
        open_round(&drive_empty, contract_id, payer_empty, 10, None);
        let (_, fee_empty) = open_round(&drive_empty, contract_id, payer_empty, 20, None);

        assert_eq!(fee_full.storage_fee, fee_empty.storage_fee);
        assert!(
            fee_full.processing_fee >= fee_empty.processing_fee,
            "reading a populated round cannot be cheaper than reading an empty one"
        );
        // One loaded merk node: at most a 32-byte root key, a varint count and their
        // framing, priced per loaded byte.
        const ONE_ROOT_NODE_READ: Credits = 2_000;
        assert!(
            fee_full.processing_fee - fee_empty.processing_fee <= ONE_ROOT_NODE_READ,
            "full {} vs empty {}: the replacement cost grew with the report count",
            fee_full.processing_fee,
            fee_empty.processing_fee
        );

        // And the estimate covers both.
        let mut layer_info = Some(HashMap::new());
        let (_, operations) = drive_full
            .open_readiness_round_operations(
                opening(contract_id, payer_full, 30),
                FUNDING,
                &block_info(1_000_000, 30, 100),
                &mut layer_info,
                None,
                platform_version,
            )
            .expect("estimate ops");
        let estimated = estimate(
            &drive_full,
            layer_info.expect("layer info"),
            operations,
            platform_version,
        );
        assert!(
            estimated.processing_fee >= fee_full.processing_fee,
            "estimated processing {} < applied {}",
            estimated.processing_fee,
            fee_full.processing_fee
        );
        assert!(
            estimated.storage_fee >= fee_full.storage_fee,
            "estimated storage {} < applied {}",
            estimated.storage_fee,
            fee_full.storage_fee
        );
    }

    #[test]
    fn should_cancel_a_round_refund_the_remainder_and_keep_the_votes_tree_shape() {
        let (drive, payer) = setup();
        let platform_version = PlatformVersion::latest();
        let contract_id = [1u8; 32];
        let (round, _) = open_round(&drive, contract_id, payer, 10, None);
        insert_reports(&drive, &round, 0, 3, None);
        let balance_before = payer_balance(&drive, payer, None);

        let (cancelled, operations) = drive
            .cancel_readiness_round_operations(
                contract_id,
                CLEANUP_RESERVE,
                &block_info(1_100_000, 11, 100),
                &mut None,
                None,
                platform_version,
            )
            .expect("cancel");
        assert_eq!(cancelled, Some(round.clone()));
        apply(&drive, operations, None, platform_version);

        assert_eq!(
            drive
                .fetch_readiness_round(contract_id, None, platform_version)
                .expect("fetch"),
            None
        );
        assert_eq!(
            payer_balance(&drive, payer, None),
            balance_before + INITIAL_FUNDING - CLEANUP_RESERVE
        );
        assert_eq!(pool_credits(&drive, None), CLEANUP_RESERVE);
        assert_eq!(
            drive
                .fetch_retired_readiness_round(None, platform_version)
                .expect("retired"),
            Some(RetiredReadinessRound {
                round_id: round.round_id(),
                contract_id
            })
        );
        // The votes tree still has its four children.
        let grove_version = &platform_version.drive.grove_version;
        for key in [b'd', b'c', b'e', READINESS_TREE_KEY as u8] {
            assert!(drive
                .grove
                .get(&vote_root_path(), &[key], None, grove_version)
                .unwrap()
                .is_ok());
        }
        // Cancelling again is a no-op.
        let (cancelled, operations) = drive
            .cancel_readiness_round_operations(
                contract_id,
                CLEANUP_RESERVE,
                &block_info(1_100_000, 12, 100),
                &mut None,
                None,
                platform_version,
            )
            .expect("cancel again");
        assert_eq!(cancelled, None);
        assert!(operations
            .iter()
            .all(|operation| !matches!(operation, LowLevelDriveOperation::GroveOperation(_))));
    }

    #[test]
    fn should_activate_a_crossed_round_and_refuse_a_pending_or_stale_one() {
        let (drive, payer) = setup();
        let platform_version = PlatformVersion::latest();
        let contract_id = [1u8; 32];
        let (mut round, _) = open_round(&drive, contract_id, payer, 10, None);

        let result = drive.activate_readiness_round_operations(
            contract_id,
            round.round_id(),
            CLEANUP_RESERVE,
            &block_info(2_000_000, 20, 100),
            &mut None,
            None,
            platform_version,
        );
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));

        let (deadline_ms, operations) = drive
            .record_readiness_crossing_operations(
                &mut round,
                1_500_000,
                MIN_WAIT_MS,
                MAX_WAIT_MS,
                &mut None,
                None,
                platform_version,
            )
            .expect("crossing");
        apply(&drive, operations, None, platform_version);
        let balance_before = payer_balance(&drive, payer, None);

        let result = drive.activate_readiness_round_operations(
            contract_id,
            [9u8; 32],
            CLEANUP_RESERVE,
            &block_info(deadline_ms, 20, 100),
            &mut None,
            None,
            platform_version,
        );
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));

        let (activated, operations) = drive
            .activate_readiness_round_operations(
                contract_id,
                round.round_id(),
                CLEANUP_RESERVE,
                &block_info(deadline_ms, 20, 100),
                &mut None,
                None,
                platform_version,
            )
            .expect("activate");
        assert_eq!(activated, round);
        apply(&drive, operations, None, platform_version);

        assert_eq!(
            drive
                .fetch_readiness_round(contract_id, None, platform_version)
                .expect("fetch"),
            None
        );
        assert_eq!(
            drive
                .fetch_readiness_rounds_due(deadline_ms, 10, None, platform_version)
                .expect("due"),
            vec![]
        );
        assert_eq!(
            payer_balance(&drive, payer, None),
            balance_before + INITIAL_FUNDING - CLEANUP_RESERVE
        );
        assert_eq!(pool_credits(&drive, None), CLEANUP_RESERVE);
    }

    #[test]
    fn should_leave_no_partial_subtree_when_an_opening_batch_is_rolled_back() {
        let (drive, payer) = setup();
        let platform_version = PlatformVersion::latest();
        let contract_id = [1u8; 32];
        let grove_version = &platform_version.drive.grove_version;

        let transaction = drive.grove.start_transaction();
        open_round(&drive, contract_id, payer, 10, Some(&transaction));
        assert!(drive
            .fetch_readiness_round(contract_id, Some(&transaction), platform_version)
            .expect("fetch")
            .is_some());
        drive
            .grove
            .rollback_transaction(&transaction)
            .expect("rollback");

        assert_eq!(
            drive
                .fetch_readiness_round(contract_id, None, platform_version)
                .expect("fetch"),
            None
        );
        let contracts_path = crate::drive::votes::paths::readiness_contracts_tree_path();
        assert!(drive
            .grove
            .get(&contracts_path, &contract_id, None, grove_version)
            .unwrap()
            .is_err());
        assert_eq!(payer_balance(&drive, payer, None), PAYER_BALANCE);
    }
}

mod cleanup {
    use super::*;

    fn cleanup_step(
        drive: &Drive,
        round_id: [u8; 32],
        contract_id: [u8; 32],
        max_deletes: u16,
    ) -> (ReadinessCleanupOutcome, FeeResult) {
        let platform_version = PlatformVersion::latest();
        let (outcome, operations) = drive
            .cleanup_retired_readiness_round_operations(
                round_id,
                contract_id,
                max_deletes,
                &mut None,
                None,
                platform_version,
            )
            .expect("cleanup");
        let fee = apply(drive, operations, None, platform_version);
        (outcome, fee)
    }

    #[test]
    fn should_drain_a_retired_round_in_bounded_steps_and_remove_its_trees() {
        let (drive, payer) = setup();
        let platform_version = PlatformVersion::latest();
        let contract_id = [1u8; 32];
        let grove_version = &platform_version.drive.grove_version;
        let (old, _) = open_round(&drive, contract_id, payer, 10, None);
        insert_reports(&drive, &old, 0, 2_000, None);
        let cursor = ReadinessScanCursor::new(100, 5, platform_version).expect("cursor");
        let operations = drive
            .store_readiness_scan_cursor_operations(
                contract_id,
                old.round_id(),
                &cursor,
                &mut None,
                None,
                platform_version,
            )
            .expect("store");
        apply(&drive, operations, None, platform_version);
        // Cancel so that no live round remains and the contract tree can go too.
        let (_, operations) = drive
            .cancel_readiness_round_operations(
                contract_id,
                CLEANUP_RESERVE,
                &block_info(1_100_000, 11, 100),
                &mut None,
                None,
                platform_version,
            )
            .expect("cancel");
        apply(&drive, operations, None, platform_version);

        // The estimation branch prices `max_deletes` deletes plus the fixed tail and grows
        // with `max_deletes`. It is a dry run of the step's shape, not a bound on the applied
        // cost: the average-case model charges tree propagation once per layer per batch,
        // while a real merk loads the path of every deleted key. Nothing compares the two;
        // the block event applies the step as bounded work paid by the flat cleanup reserve.
        let estimate_for = |max_deletes: u16| {
            let mut estimated_layer_info = Some(HashMap::new());
            let (outcome, estimated_operations) = drive
                .cleanup_retired_readiness_round_operations(
                    old.round_id(),
                    contract_id,
                    max_deletes,
                    &mut estimated_layer_info,
                    None,
                    platform_version,
                )
                .expect("estimate");
            assert!(outcome.finished);
            assert_eq!(outcome.reports_deleted, max_deletes as u64);
            estimate(
                &drive,
                estimated_layer_info.expect("layer info"),
                estimated_operations,
                platform_version,
            )
        };
        let estimate_small = estimate_for(8);
        let estimate_large = estimate_for(512);
        assert!(estimate_large.processing_fee > estimate_small.processing_fee);

        let mut steps = 0;
        let mut previous_fee: Option<FeeResult> = None;
        loop {
            let (outcome, fee) = cleanup_step(&drive, old.round_id(), contract_id, 512);
            steps += 1;
            assert!(fee.processing_fee > 0);
            if let Some(previous) = previous_fee.as_ref() {
                if !outcome.finished {
                    // Full steps delete the same number of reports and cost about the same.
                    let (low, high) = if previous.processing_fee <= fee.processing_fee {
                        (previous.processing_fee, fee.processing_fee)
                    } else {
                        (fee.processing_fee, previous.processing_fee)
                    };
                    assert!(
                        high <= low + low / 4,
                        "step {steps}: {high} is not within a quarter of the previous step's {low}"
                    );
                }
            }
            previous_fee = Some(fee);
            if outcome.finished {
                assert_eq!(outcome.reports_deleted, 2_000 - 512 * 3);
                break;
            }
            assert_eq!(outcome.reports_deleted, 512);
            assert!(steps < 4, "the round should drain in four steps");
        }
        assert_eq!(steps, 4);

        assert_eq!(
            drive
                .fetch_retired_readiness_round(None, platform_version)
                .expect("retired"),
            None
        );
        let old_round_id = old.round_id();
        let round_path = readiness_round_tree_path(&contract_id, &old_round_id);
        for key in [
            READINESS_ROUND_RECORD_KEY,
            READINESS_ROUND_REPORTS_TREE_KEY,
            READINESS_ROUND_SCAN_CURSOR_KEY,
        ] {
            assert!(drive
                .grove
                .get(&round_path, &[key], None, grove_version)
                .unwrap()
                .is_err());
        }
        let contract_path = readiness_contract_tree_path(&contract_id);
        assert!(drive
            .grove
            .get(&contract_path, &old_round_id, None, grove_version)
            .unwrap()
            .is_err());
        assert!(drive
            .grove
            .get(
                &contract_path,
                &[READINESS_CURRENT_ROUND_POINTER_KEY as u8],
                None,
                grove_version
            )
            .unwrap()
            .is_err());
        let contracts_path = crate::drive::votes::paths::readiness_contracts_tree_path();
        assert!(drive
            .grove
            .get(&contracts_path, &contract_id, None, grove_version)
            .unwrap()
            .is_err());
        assert_eq!(
            drive
                .fetch_readiness_rounds_page(None, 10, None, platform_version)
                .expect("page"),
            Vec::<[u8; 32]>::new()
        );
    }

    #[test]
    fn should_keep_the_contract_tree_while_a_live_round_remains_and_queue_in_order() {
        let (drive, payer) = setup();
        let platform_version = PlatformVersion::latest();
        let contract_id = [1u8; 32];
        let grove_version = &platform_version.drive.grove_version;
        let (first, _) = open_round(&drive, contract_id, payer, 10, None);
        insert_reports(&drive, &first, 0, 3, None);
        let (second, _) = open_round(&drive, contract_id, payer, 20, None);
        insert_reports(&drive, &second, 0, 2, None);
        let (third, _) = open_round(&drive, contract_id, payer, 30, None);

        // Queue order is by round id (the key), so drain whichever comes first.
        let queued = drive
            .fetch_retired_readiness_round(None, platform_version)
            .expect("retired")
            .expect("first entry")
            .round_id;
        assert!(queued == first.round_id() || queued == second.round_id());

        let (outcome, _) = cleanup_step(&drive, queued, contract_id, 512);
        assert!(outcome.finished);
        let queued_next = drive
            .fetch_retired_readiness_round(None, platform_version)
            .expect("retired")
            .expect("second entry")
            .round_id;
        assert_ne!(queued_next, queued);
        let (outcome, _) = cleanup_step(&drive, queued_next, contract_id, 512);
        assert!(outcome.finished);
        assert_eq!(
            drive
                .fetch_retired_readiness_round(None, platform_version)
                .expect("retired"),
            None
        );

        // The live round and its contract tree are untouched.
        let current = drive
            .fetch_readiness_round(contract_id, None, platform_version)
            .expect("fetch")
            .expect("round");
        assert_eq!(current, third);
        let contract_path = readiness_contract_tree_path(&contract_id);
        assert!(drive
            .grove
            .get(&contract_path, &third.round_id(), None, grove_version)
            .unwrap()
            .is_ok());
        assert!(drive
            .grove
            .get(&contract_path, &first.round_id(), None, grove_version)
            .unwrap()
            .is_err());
    }

    #[test]
    fn should_keep_a_shared_deadline_tree_when_only_one_of_its_rounds_retires() {
        let (drive, payer) = setup();
        let platform_version = PlatformVersion::latest();
        let grove_version = &platform_version.drive.grove_version;
        let mut rounds = vec![];
        for contract in 1u8..=2 {
            let contract_id = [contract; 32];
            let (mut round, _) = open_round(&drive, contract_id, payer, 10, None);
            let (_, operations) = drive
                .record_readiness_crossing_operations(
                    &mut round,
                    1_500_000,
                    MIN_WAIT_MS,
                    MAX_WAIT_MS,
                    &mut None,
                    None,
                    platform_version,
                )
                .expect("crossing");
            apply(&drive, operations, None, platform_version);
            rounds.push(round);
        }
        let deadline_ms = rounds[0].deadline_ms().expect("deadline");
        assert_eq!(rounds[1].deadline_ms(), Some(deadline_ms));

        let (_, operations) = drive
            .cancel_readiness_round_operations(
                [1u8; 32],
                CLEANUP_RESERVE,
                &block_info(1_600_000, 16, 100),
                &mut None,
                None,
                platform_version,
            )
            .expect("cancel");
        apply(&drive, operations, None, platform_version);

        let due = drive
            .fetch_readiness_rounds_due(deadline_ms, 10, None, platform_version)
            .expect("due");
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].contract_id, [2u8; 32]);
        let deadline_path = readiness_deadline_tree_path_vec(deadline_ms);
        assert!(drive
            .grove
            .get(deadline_path.as_slice(), &[2u8; 32], None, grove_version)
            .unwrap()
            .is_ok());
    }
}
