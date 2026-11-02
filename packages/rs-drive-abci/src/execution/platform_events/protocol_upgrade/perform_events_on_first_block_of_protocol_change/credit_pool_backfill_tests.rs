use crate::test::helpers::setup::TestPlatformBuilder;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::fees::op::LowLevelDriveOperation;

#[test]
fn should_backfill_credit_notes_when_skipping_versions_and_leave_pv13_replay_unchanged() {
    let latest = PlatformVersion::latest();
    for previous in [11, 12, 13] {
        let old = PlatformVersion::get(previous).expect("old protocol");
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(previous)
            .build_with_mock_rpc()
            .set_genesis_state();
        let rho = [7; 32];
        if previous >= 12 {
            let tx = platform.drive.grove.start_transaction();
            let note =
                Drive::insert_note_op(rho, [1; 32], [2; 32], vec![3; 216], old).expect("note");
            platform
                .drive
                .grove_apply_batch(
                    LowLevelDriveOperation::grovedb_operations_batch_consume(note),
                    false,
                    Some(&tx),
                    &old.drive,
                )
                .expect("historical note");
            platform
                .drive
                .commit_transaction(tx, &old.drive)
                .expect("note commit");
        }
        if previous == 12 {
            let tx = platform.drive.grove.start_transaction();
            let pv13 = PlatformVersion::get(13).expect("PV13");
            platform
                .perform_events_on_first_block_of_protocol_change(
                    &platform.state.load(),
                    &BlockInfo::default(),
                    &tx,
                    previous,
                    pv13,
                )
                .expect("historical upgrade to PV13");
            assert!(!platform
                .drive
                .has_nullifier(&rho, Some(&tx), &mut vec![], pv13)
                .expect("unchanged historical gap"));
            drop(tx);
        }
        platform.drive.cache.data_contracts.clear_block_cache();
        let tx = platform.drive.grove.start_transaction();
        platform
            .perform_events_on_first_block_of_protocol_change(
                &platform.state.load(),
                &BlockInfo::default(),
                &tx,
                previous,
                latest,
            )
            .expect("version-skipping activation");
        assert_eq!(
            platform
                .drive
                .has_nullifier(&rho, Some(&tx), &mut vec![], latest)
                .expect("backfilled rho"),
            previous >= 12
        );
        platform
            .drive
            .commit_transaction(tx, &latest.drive)
            .expect("commit activation");
        let root = platform
            .drive
            .grove
            .root_hash(None, &latest.drive.grove_version)
            .value
            .expect("root");
        platform.drive.cache.data_contracts.clear_block_cache();
        let tx = platform.drive.grove.start_transaction();
        platform
            .perform_events_on_first_block_of_protocol_change(
                &platform.state.load(),
                &BlockInfo::default(),
                &tx,
                latest.protocol_version,
                latest,
            )
            .expect("same-version repeat");
        assert_eq!(
            platform
                .drive
                .grove
                .root_hash(Some(&tx), &latest.drive.grove_version)
                .value
                .expect("repeat root"),
            root
        );
    }
}

#[test]
fn should_keep_fresh_pv14_genesis_empty_when_backfill_is_repeated() {
    let pv = PlatformVersion::latest();
    let platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(pv.protocol_version)
        .build_with_mock_rpc()
        .set_genesis_state();
    let original = platform
        .drive
        .grove
        .root_hash(None, &pv.drive.grove_version)
        .value
        .expect("genesis root");
    let tx = platform.drive.grove.start_transaction();
    platform
        .drive
        .backfill_historical_credit_pool_nullifiers(&tx, pv)
        .expect("empty genesis union");
    assert_eq!(
        platform
            .drive
            .shielded_pool_notes_count(Some(&tx), &mut vec![], pv)
            .expect("empty note count"),
        0
    );
    assert_eq!(
        platform
            .drive
            .grove
            .root_hash(Some(&tx), &pv.drive.grove_version)
            .value
            .expect("unchanged root"),
        original
    );
}
