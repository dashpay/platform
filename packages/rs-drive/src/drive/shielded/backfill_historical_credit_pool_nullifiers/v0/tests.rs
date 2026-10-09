use super::*;
use crate::drive::shielded::paths::{
    SHIELDED_ANCHORS_BY_HEIGHT_KEY, SHIELDED_ANCHORS_IN_POOL_KEY, SHIELDED_TOTAL_BALANCE_KEY,
};
use crate::fees::op::LowLevelDriveOperation;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use std::cell::Cell;

fn rho(value: u64) -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes[..8].copy_from_slice(&value.to_be_bytes());
    bytes
}

fn insert_notes(drive: &Drive, values: &[u64], pv: &PlatformVersion) {
    let tx = drive.grove.start_transaction();
    let mut ops = vec![];
    for (position, value) in values.iter().enumerate() {
        let mut cmx = [0; 32];
        cmx[..8].copy_from_slice(&(position as u64 + 1).to_le_bytes());
        ops.extend(
            Drive::insert_note_op(rho(*value), cmx, [0; 32], vec![7; 216], pv).expect("note ops"),
        );
    }
    drive
        .grove_apply_batch(
            LowLevelDriveOperation::grovedb_operations_batch_consume(ops),
            false,
            Some(&tx),
            &pv.drive,
        )
        .expect("historical notes");
    drive
        .commit_transaction(tx, &pv.drive)
        .expect("commit historical notes");
}

fn page(
    drive: &Drive,
    start: u64,
    limit: u16,
    tx: &Transaction,
    pv: &PlatformVersion,
) -> Result<RangePage, Error> {
    drive
        .grove
        .commitment_tree_get_range(
            shielded_credit_pool_path().as_slice(),
            &[SHIELDED_NOTES_KEY],
            start,
            limit,
            Some(tx),
            &pv.drive.grove_version,
        )
        .value
        .map_err(Error::from)
}

fn root(drive: &Drive, tx: Option<&Transaction>, pv: &PlatformVersion) -> [u8; 32] {
    drive
        .grove
        .root_hash(tx, &pv.drive.grove_version)
        .value
        .expect("root")
}

fn raw(drive: &Drive, key: u8, tx: &Transaction, pv: &PlatformVersion) -> Element {
    drive
        .grove
        .get_raw_optional(
            shielded_credit_pool_path().as_slice().into(),
            &[key],
            Some(tx),
            &pv.drive.grove_version,
        )
        .value
        .expect("raw element")
        .expect("existing element")
}

#[test]
fn should_union_full_and_partial_pages_preserving_existing_items_and_note_state() {
    let pv = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let mut values: Vec<_> = (0..4097).collect();
    values[2048] = 0;
    values[4096] = 2047;
    insert_notes(&drive, &values, pv);
    let tx = drive.grove.start_transaction();
    let existing = Element::Item(vec![77], Some(vec![88]));
    drive
        .grove
        .insert(
            shielded_credit_pool_nullifiers_path().as_slice(),
            &rho(0),
            existing.clone(),
            None,
            Some(&tx),
            &pv.drive.grove_version,
        )
        .value
        .expect("existing Item with flags");
    drive
        .grove
        .insert(
            shielded_credit_pool_nullifiers_path().as_slice(),
            &rho(9999),
            Element::new_item(vec![]),
            None,
            Some(&tx),
            &pv.drive.grove_version,
        )
        .value
        .expect("prior spent nullifier");
    let notes_before = page(&drive, 0, u16::MAX, &tx, pv).expect("all historical notes");
    let preserved_keys = [
        SHIELDED_NOTES_KEY,
        SHIELDED_TOTAL_BALANCE_KEY,
        SHIELDED_ANCHORS_IN_POOL_KEY,
        SHIELDED_ANCHORS_BY_HEIGHT_KEY,
    ];
    let preserved: Vec<_> = preserved_keys
        .iter()
        .map(|key| raw(&drive, *key, &tx, pv))
        .collect();
    let anchor_before = drive
        .grove
        .commitment_tree_anchor(
            shielded_credit_pool_path().as_slice(),
            &[SHIELDED_NOTES_KEY],
            Some(&tx),
            &pv.drive.grove_version,
        )
        .value
        .expect("frontier anchor");
    let batches = Cell::new(0);
    drive
        .backfill_credit_nullifiers_with_io_v0(
            |start, limit| {
                if start == 2048 {
                    assert!(drive
                        .has_nullifier(&rho(2047), Some(&tx), &mut vec![], pv)
                        .expect("prior page flushed"));
                    assert_eq!(
                        batches.get(),
                        1,
                        "partial first-page batch is applied before another page"
                    );
                }
                page(&drive, start, limit, &tx, pv)
            },
            |batch| {
                assert!(!batch.is_empty());
                assert!(batch.len() <= 2048);
                batches.set(batches.get() + 1);
                drive.grove_apply_batch(
                    GroveDbOpBatch { operations: batch },
                    false,
                    Some(&tx),
                    &pv.drive,
                )
            },
            &tx,
            pv,
        )
        .expect("complete union");
    assert_eq!(batches.get(), 2);
    for value in values.iter().chain(std::iter::once(&9999)) {
        assert!(drive
            .has_nullifier(&rho(*value), Some(&tx), &mut vec![], pv)
            .expect("union member"));
    }
    assert_eq!(
        drive
            .grove
            .get_raw_optional(
                shielded_credit_pool_nullifiers_path().as_slice().into(),
                &rho(0),
                Some(&tx),
                &pv.drive.grove_version
            )
            .value
            .expect("preserved Item"),
        Some(existing)
    );
    assert_eq!(
        page(&drive, 0, u16::MAX, &tx, pv).expect("notes after union"),
        notes_before
    );
    for (key, before) in preserved_keys.iter().zip(preserved) {
        assert_eq!(raw(&drive, *key, &tx, pv), before);
    }
    assert_eq!(
        drive
            .grove
            .commitment_tree_anchor(
                shielded_credit_pool_path().as_slice(),
                &[SHIELDED_NOTES_KEY],
                Some(&tx),
                &pv.drive.grove_version
            )
            .value
            .expect("unchanged frontier"),
        anchor_before
    );
    let mut expected_nullifiers: BTreeSet<_> = values.iter().copied().collect();
    expected_nullifiers.insert(9999);
    assert!(
        matches!(raw(&drive, SHIELDED_NULLIFIERS_KEY, &tx, pv),
        Element::ProvableCountTree(_, count, _) if count == expected_nullifiers.len() as u64),
        "membership plus exact count proves the union has no unrelated inserts"
    );
    let migrated_root = root(&drive, Some(&tx), pv);
    drive
        .backfill_credit_nullifiers_with_io_v0(
            |start, limit| page(&drive, start, limit, &tx, pv),
            |_| panic!("all-present union must apply no batch"),
            &tx,
            pv,
        )
        .expect("repeat union");
    assert_eq!(root(&drive, Some(&tx), pv), migrated_root);
}

#[test]
fn should_not_apply_empty_batches_and_should_validate_empty_pool_shape() {
    let pv = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let tx = drive.grove.start_transaction();
    let before = root(&drive, Some(&tx), pv);
    drive
        .backfill_credit_nullifiers_with_io_v0(
            |start, limit| page(&drive, start, limit, &tx, pv),
            |_| panic!("empty pool must apply no batch"),
            &tx,
            pv,
        )
        .expect("empty valid pool");
    assert_eq!(root(&drive, Some(&tx), pv), before);
    drive
        .grove
        .delete(
            shielded_credit_pool_path().as_slice(),
            &[SHIELDED_NULLIFIERS_KEY],
            None,
            Some(&tx),
            &pv.drive.grove_version,
        )
        .value
        .expect("remove empty fixture tree");
    drive
        .grove
        .insert(
            shielded_credit_pool_path().as_slice(),
            &[SHIELDED_NULLIFIERS_KEY],
            Element::empty_tree(),
            None,
            Some(&tx),
            &pv.drive.grove_version,
        )
        .value
        .expect("wrong-shaped empty tree");
    assert!(drive
        .backfill_historical_credit_pool_nullifiers(&tx, pv)
        .is_err());
}

#[test]
fn should_fail_on_existing_non_item_even_when_all_rhos_are_present() {
    let pv = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    insert_notes(&drive, &[1], pv);
    let tx = drive.grove.start_transaction();
    drive
        .grove
        .insert(
            shielded_credit_pool_nullifiers_path().as_slice(),
            &rho(1),
            Element::empty_tree(),
            None,
            Some(&tx),
            &pv.drive.grove_version,
        )
        .value
        .expect("malformed nullifier shape");
    let before = root(&drive, Some(&tx), pv);
    assert!(drive
        .backfill_historical_credit_pool_nullifiers(&tx, pv)
        .is_err());
    assert_eq!(root(&drive, Some(&tx), pv), before);
}

#[test]
fn should_reject_truncated_missing_reordered_and_count_drift_pages() {
    let row = vec![0; 96];
    let valid = RangePage {
        entries: vec![(0, row.clone()), (1, row.clone())],
        total_count: 2,
    };
    assert!(validated_page(valid.clone(), 0, 2, 2048).is_ok());
    for length in [0, 32, 64, 95] {
        let mut bad = valid.clone();
        bad.entries[1].1.truncate(length);
        assert!(validated_page(bad, 0, 2, 2048).is_err());
    }
    let mut missing = valid.clone();
    missing.entries.pop();
    assert!(validated_page(missing, 0, 2, 2048).is_err());
    let mut reordered = valid.clone();
    reordered.entries.swap(0, 1);
    assert!(validated_page(reordered, 0, 2, 2048).is_err());
    let mut gap = valid.clone();
    gap.entries[1].0 = 2;
    assert!(validated_page(gap, 0, 2, 2048).is_err());
    let mut drift = valid;
    drift.total_count = 3;
    assert!(validated_page(drift, 0, 2, 2048).is_err());
    assert!(validated_page(
        RangePage {
            entries: vec![],
            total_count: 0
        },
        u64::MAX,
        0,
        2048
    )
    .is_err());
    assert!(validated_page(
        RangePage {
            entries: vec![(u64::MAX - 1, row)],
            total_count: u64::MAX
        },
        u64::MAX - 1,
        u64::MAX,
        2048
    )
    .is_ok());
}

#[test]
fn should_drop_partial_writes_on_failure_and_retry_to_the_same_root() {
    let mut pv = PlatformVersion::latest().clone();
    pv.system_limits.credit_pool_nullifier_backfill_batch_size = 1;
    let drive = setup_drive_with_initial_state_structure(None);
    insert_notes(&drive, &[1, 2, 3], &pv);
    let original = root(&drive, None, &pv);
    let mut expected = None;
    for inject_failure in [true, false, false] {
        let tx = drive.grove.start_transaction();
        let mut batches = 0;
        let result = drive.backfill_credit_nullifiers_with_io_v0(
            |start, limit| page(&drive, start, limit, &tx, &pv),
            |batch| {
                assert!(!batch.is_empty());
                if inject_failure && batches == 1 {
                    assert!(drive
                        .has_nullifier(&rho(1), Some(&tx), &mut vec![], &pv)
                        .expect("first batch applied"));
                    return Err(Error::from(grovedb::Error::InvalidInput(
                        "injected storage write failure",
                    )));
                }
                batches += 1;
                drive.grove_apply_batch(
                    GroveDbOpBatch { operations: batch },
                    false,
                    Some(&tx),
                    &pv.drive,
                )
            },
            &tx,
            &pv,
        );
        if inject_failure {
            assert!(result.is_err());
            assert_eq!(batches, 1);
        } else {
            result.expect("successful retry");
            let candidate = root(&drive, Some(&tx), &pv);
            if let Some(expected) = expected {
                assert_eq!(candidate, expected);
            } else {
                expected = Some(candidate);
            }
        }
        assert_eq!(
            root(&drive, None, &pv),
            original,
            "candidate never publishes before commit"
        );
        drop(tx);
        assert_eq!(
            root(&drive, None, &pv),
            original,
            "dropping candidate rolls back every batch"
        );
    }
}

#[test]
fn should_abort_a_corrupt_later_page_after_a_nonempty_batch() {
    let pv = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    insert_notes(&drive, &(0..2049).collect::<Vec<_>>(), pv);
    let original = root(&drive, None, pv);
    for count_drift in [false, true] {
        let tx = drive.grove.start_transaction();
        let batches = Cell::new(0);
        let result = drive.backfill_credit_nullifiers_with_io_v0(
            |start, limit| {
                let mut result = page(&drive, start, limit, &tx, pv)?;
                if start > 0 {
                    assert_eq!(batches.get(), 1);
                    if count_drift {
                        result.total_count += 1;
                    } else {
                        result.entries[0].1.truncate(95);
                    }
                }
                Ok(result)
            },
            |batch| {
                batches.set(batches.get() + 1);
                drive.grove_apply_batch(
                    GroveDbOpBatch { operations: batch },
                    false,
                    Some(&tx),
                    &pv.drive,
                )
            },
            &tx,
            pv,
        );
        assert!(result.is_err());
        assert_eq!(batches.get(), 1);
        drop(tx);
        assert_eq!(root(&drive, None, pv), original);
    }
}

#[test]
fn should_reject_inactive_versions_and_keep_committed_union_after_reopen() {
    let pv = PlatformVersion::latest();
    let mut drive = setup_drive_with_initial_state_structure(None);
    insert_notes(&drive, &[1, 1, 2], pv);
    let tx = drive.grove.start_transaction();
    assert!(drive
        .backfill_historical_credit_pool_nullifiers(&tx, PlatformVersion::get(13).expect("PV13"))
        .is_err());
    drive
        .backfill_historical_credit_pool_nullifiers(&tx, pv)
        .expect("migration");
    drive
        .commit_transaction(tx, &pv.drive)
        .expect("commit migration");
    let expected = root(&drive, None, pv);
    let tempdir = drive.temp_dir.take().expect("owned test DB");
    drop(drive);
    let (reopened, _) = Drive::open(tempdir.path(), None).expect("reopen");
    let tx = reopened.grove.start_transaction();
    reopened
        .backfill_historical_credit_pool_nullifiers(&tx, pv)
        .expect("repeat after restart");
    assert_eq!(root(&reopened, Some(&tx), pv), expected);
    reopened
        .commit_transaction(tx, &pv.drive)
        .expect("commit no-op repeat");
    assert_eq!(root(&reopened, None, pv), expected);
}
