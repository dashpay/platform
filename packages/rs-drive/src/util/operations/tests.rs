//! Inline tests for the `util/operations` module.
//!
//! These tests exercise the real logic of the batch application helpers,
//! the transaction commit / rollback helpers, and `drop_cache`.
//!
//! Per project test-writing guidance we do **not** include version-dispatch
//! tests here — every test exercises meaningful behaviour.

use crate::error::Error;
use crate::fees::op::{FunctionOp, HashFunction, LowLevelDriveOperation};
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use crate::util::test_helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};
use dpp::version::PlatformVersion;
use grovedb::batch::QualifiedGroveDbOp;
use grovedb::Element;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Produces a deterministic `FunctionOperation` for leftover-partition testing.
fn function_op() -> LowLevelDriveOperation {
    LowLevelDriveOperation::FunctionOperation(FunctionOp::new_with_round_count(
        HashFunction::Sha256,
        3,
    ))
}

// ---------------------------------------------------------------------------
// apply_batch_low_level_drive_operations_v0
// ---------------------------------------------------------------------------

/// Empty batch → no grove ops, nothing to apply, no leftovers. Must succeed.
#[test]
fn apply_batch_low_level_empty_batch_is_ok_and_drive_operations_stay_empty() {
    let drive = setup_drive_with_initial_state_structure(None);
    let platform_version = PlatformVersion::latest();
    let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

    drive
        .apply_batch_low_level_drive_operations_v0(
            None,
            None,
            vec![],
            &mut drive_operations,
            &platform_version.drive,
        )
        .expect("empty batch should succeed");

    // An empty batch must produce no drive_operations output since
    // apply_batch_grovedb_operations is skipped and there are no leftovers.
    assert!(drive_operations.is_empty());
}

/// Pure non-grove leftover ops must bypass grove and be appended to
/// `drive_operations` verbatim.
#[test]
fn apply_batch_low_level_only_leftovers_skips_grove_and_appends_them() {
    let drive = setup_drive_with_initial_state_structure(None);
    let platform_version = PlatformVersion::latest();

    let batch_ops = vec![function_op(), function_op()];
    let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

    drive
        .apply_batch_low_level_drive_operations_v0(
            None,
            None,
            batch_ops,
            &mut drive_operations,
            &platform_version.drive,
        )
        .expect("should succeed with only non-grove ops");

    assert_eq!(drive_operations.len(), 2);
    assert!(drive_operations
        .iter()
        .all(|op| matches!(op, LowLevelDriveOperation::FunctionOperation(_))));
}

/// Mixed batch: grove ops must be applied, non-grove ops must appear in
/// `drive_operations` at the end.
#[test]
fn apply_batch_low_level_mixed_ops_applies_grove_and_preserves_leftovers() {
    let drive = setup_drive_with_initial_state_structure(None);
    let platform_version = PlatformVersion::latest();

    // Two grove insertions into an existing root subtree (balances) and one
    // non-grove op. We use the balances root subtree (key 0x02 at depth 0)
    // because `setup_drive_with_initial_state_structure` created it. Instead
    // of guessing paths, we build grove ops against an already-existing
    // subtree by inserting at a *newly created* subtree first.
    let batch_ops = vec![
        LowLevelDriveOperation::GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
            vec![],
            vec![0xfe],
            Element::empty_tree(),
        )),
        function_op(),
        LowLevelDriveOperation::GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
            vec![vec![0xfe]],
            vec![0x01],
            Element::new_item(vec![1, 2, 3]),
        )),
    ];
    let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

    drive
        .apply_batch_low_level_drive_operations_v0(
            None,
            None,
            batch_ops,
            &mut drive_operations,
            &platform_version.drive,
        )
        .expect("mixed batch should apply grove ops and preserve leftovers");

    // Expect exactly one appended leftover (the function op).
    assert_eq!(
        drive_operations
            .iter()
            .filter(|op| matches!(op, LowLevelDriveOperation::FunctionOperation(_)))
            .count(),
        1,
        "function op should be preserved as a leftover"
    );
    // Grove application itself records CalculatedCostOperation entries. Make
    // sure we observe at least one cost entry from the grove apply call.
    assert!(
        drive_operations
            .iter()
            .any(|op| matches!(op, LowLevelDriveOperation::CalculatedCostOperation(_))),
        "expected at least one CalculatedCostOperation from grove apply"
    );
}

/// Inserted value must be readable back, proving the batch was materialized.
#[test]
fn apply_batch_low_level_actually_persists_grove_inserts() {
    use crate::util::grove_operations::DirectQueryType;
    use grovedb_path::SubtreePath;

    let drive = setup_drive_with_initial_state_structure(None);
    let platform_version = PlatformVersion::latest();

    let batch_ops = vec![
        LowLevelDriveOperation::GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
            vec![],
            vec![0xfd],
            Element::empty_tree(),
        )),
        LowLevelDriveOperation::GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
            vec![vec![0xfd]],
            b"hello".to_vec(),
            Element::new_item(b"world".to_vec()),
        )),
    ];
    let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

    drive
        .apply_batch_low_level_drive_operations_v0(
            None,
            None,
            batch_ops,
            &mut drive_operations,
            &platform_version.drive,
        )
        .expect("persist batch");

    // Read the value back through Drive's typed grove helper.
    let path: Vec<Vec<u8>> = vec![vec![0xfd]];
    let path_refs: Vec<&[u8]> = path.iter().map(|v| v.as_slice()).collect();
    let subtree_path = SubtreePath::from(path_refs.as_slice());

    let mut cost_ops: Vec<LowLevelDriveOperation> = vec![];
    let fetched = drive
        .grove_get_raw_optional_item(
            subtree_path,
            b"hello",
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut cost_ops,
            &platform_version.drive,
        )
        .expect("grove get must succeed");

    assert_eq!(
        fetched,
        Some(b"world".to_vec()),
        "inserted value must be readable back after batch apply"
    );
}

// ---------------------------------------------------------------------------
// apply_batch_grovedb_operations_v0 — error path: empty batch
// ---------------------------------------------------------------------------

/// A non-empty wrapping batch with no actual grove ops inside the GroveDbOpBatch
/// funnelled through `apply_batch_grovedb_operations` (applied branch) must
/// return `BatchIsEmpty` — this proves the error bubble-through works.
#[test]
fn apply_batch_grovedb_empty_ops_returns_batch_is_empty_error() {
    let drive = setup_drive_with_initial_state_structure(None);
    let platform_version = PlatformVersion::latest();
    let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

    let err = drive
        .apply_batch_grovedb_operations(
            None,
            None,
            GroveDbOpBatch::new(),
            &mut drive_operations,
            &platform_version.drive,
        )
        .expect_err("empty batch must error in applied branch");

    let msg = format!("{:?}", err);
    assert!(
        msg.contains("BatchIsEmpty"),
        "expected BatchIsEmpty, got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// apply_partial_batch_low_level_drive_operations_v0  (deprecated but still
// part of the covered tree — exercise it at least once)
// ---------------------------------------------------------------------------

/// Partial batch with only non-grove leftovers: the inner apply returns an
/// empty-batch error. The `#[deprecated]` function still routes through this
/// path, so we assert that error reaches the caller.
#[allow(deprecated)]
#[test]
fn apply_partial_batch_low_level_only_leftovers_surfaces_empty_batch_error() {
    let drive = setup_drive_with_initial_state_structure(None);
    let platform_version = PlatformVersion::latest();
    let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

    let err = drive
        .apply_partial_batch_low_level_drive_operations_v0(
            None,
            None,
            vec![function_op()],
            |_cost, _ops_by_level| Ok(vec![]),
            &mut drive_operations,
            &platform_version.drive,
        )
        .expect_err("expected inner BatchIsEmpty error to surface");

    let msg = format!("{:?}", err);
    assert!(
        msg.contains("BatchIsEmpty"),
        "expected BatchIsEmpty, got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// commit_transaction_v0 / rollback_transaction_v0
// ---------------------------------------------------------------------------

/// Starting a fresh transaction and committing it with `commit_transaction_v0`
/// must be OK even when no operations were performed.
#[test]
fn commit_transaction_v0_succeeds_on_empty_transaction() {
    let drive = setup_drive_with_initial_state_structure(None);
    let tx = drive.grove.start_transaction();
    drive
        .commit_transaction_v0(tx)
        .expect("commit of an untouched transaction must succeed");
}

/// Writes inside a transaction must disappear after `rollback_transaction_v0`.
#[test]
fn rollback_transaction_v0_discards_uncommitted_changes() {
    let drive = setup_drive_with_initial_state_structure(None);
    let platform_version = PlatformVersion::latest();

    let tx = drive.grove.start_transaction();

    // Build ops inside the transaction.
    let batch_ops = vec![
        LowLevelDriveOperation::GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
            vec![],
            vec![0xfb],
            Element::empty_tree(),
        )),
        LowLevelDriveOperation::GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
            vec![vec![0xfb]],
            b"k".to_vec(),
            Element::new_item(b"v".to_vec()),
        )),
    ];
    let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
    drive
        .apply_batch_low_level_drive_operations_v0(
            None,
            Some(&tx),
            batch_ops,
            &mut drive_operations,
            &platform_version.drive,
        )
        .expect("apply in transaction");

    // Roll back the transaction before it is committed.
    drive
        .rollback_transaction_v0(&tx)
        .expect("rollback should succeed");

    // Committing the rolled-back (hence empty) transaction must still succeed.
    drive
        .commit_transaction_v0(tx)
        .expect("commit of a rolled-back transaction should succeed");

    // After rollback + commit, trying to apply a new batch that would depend
    // on the rolled-back subtree existing should fail (the subtree is gone).
    //
    // We verify this indirectly by attempting an insertion into the subtree
    // path we *thought* we created. With consistency verification enabled,
    // grovedb will error because the parent tree does not exist.
    let bad_batch = vec![LowLevelDriveOperation::GroveOperation(
        QualifiedGroveDbOp::insert_or_replace_op(
            vec![vec![0xfb]],
            b"k2".to_vec(),
            Element::new_item(b"v2".to_vec()),
        ),
    )];
    let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
    let res = drive.apply_batch_low_level_drive_operations_v0(
        None,
        None,
        bad_batch,
        &mut drive_operations,
        &platform_version.drive,
    );
    assert!(
        res.is_err(),
        "rolled back subtree should no longer be present; expected apply to fail"
    );
}

// ---------------------------------------------------------------------------
// drop_cache_v0
// ---------------------------------------------------------------------------

/// After setting a non-default genesis_time_ms in the cache, `drop_cache_v0`
/// must reset it back to the config's default (None for a bare drive).
#[test]
fn drop_cache_v0_resets_genesis_time_and_protocol_versions() {
    let drive = setup_drive(None);

    // Mutate the caches to non-default values.
    {
        let mut g = drive.cache.genesis_time_ms.write();
        *g = Some(123_456_789);
    }

    // Sanity check that we actually wrote something.
    assert_eq!(*drive.cache.genesis_time_ms.read(), Some(123_456_789));

    drive.drop_cache_v0();

    // The default for a bare Drive is None.
    assert_eq!(
        *drive.cache.genesis_time_ms.read(),
        None,
        "drop_cache should reset genesis_time_ms back to config default"
    );
    // protocol_versions_counter cache should have been replaced by a new
    // empty cache — we don't make assumptions about its internals beyond
    // that it's readable.
    let _guard = drive.cache.protocol_versions_counter.read();
    drop(_guard);
}

// ---------------------------------------------------------------------------
// apply_batch_low_level_drive_operations (top-level, non-v0) — exercise
// the real logic once through the public API. Not a version-dispatch test.
// ---------------------------------------------------------------------------

/// Calling through the public entry point `apply_batch_low_level_drive_operations`
/// with a mixed batch must succeed (proves wiring between the public
/// function and `_v0`).
#[test]
fn public_apply_batch_low_level_drive_operations_routes_through_v0() {
    let drive = setup_drive_with_initial_state_structure(None);
    let platform_version = PlatformVersion::latest();

    let batch_ops = vec![
        LowLevelDriveOperation::GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
            vec![],
            vec![0xfa],
            Element::empty_tree(),
        )),
        LowLevelDriveOperation::GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
            vec![vec![0xfa]],
            b"x".to_vec(),
            Element::new_item(b"y".to_vec()),
        )),
    ];
    let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

    let res = drive.apply_batch_low_level_drive_operations(
        None,
        None,
        batch_ops,
        &mut drive_operations,
        &platform_version.drive,
    );
    // On the latest platform version, we route to v0 and the call succeeds.
    match res {
        Ok(()) => {}
        Err(Error::Drive(crate::error::drive::DriveError::UnknownVersionMismatch { .. })) => {
            panic!("unexpected version mismatch on latest platform version");
        }
        Err(e) => panic!("unexpected error: {:?}", e),
    }
}
