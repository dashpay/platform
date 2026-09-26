//! Tests that run both batch apply generations through the dispatcher.
//!
//! Version 1 is selected only by a test-built drive version: no protocol
//! version references it until the fee decoder that consumes the typed cost
//! operation lands.

use crate::drive::system::misc_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::fee::refund_owner::{RefundOwner, RefundOwnersByIdentifier};
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::QualifiedGroveDbOp;
use grovedb::Element;
use grovedb_costs::storage_cost::removal::StorageRemovedBytes;
use grovedb_costs::storage_cost::removal::StorageRemovedBytes::{
    NoStorageRemoval, SectionedStorageRemoval,
};
use grovedb_costs::OperationCost;
use grovedb_path::SubtreePath;
use platform_version::version::drive_versions::DriveVersion;

const OWNER_ID: [u8; 32] = [0x22; 32];
const CONTRACT_ID: [u8; 32] = [0x11; 32];
const KEY: &[u8] = b"fix-08-flagged-item";

/// The latest drive version with both batch apply slots moved to version 1.
fn typed_drive_version() -> DriveVersion {
    let mut drive_version = PlatformVersion::latest().drive.clone();
    drive_version.grove_methods.apply.grove_apply_batch = 1;
    drive_version.grove_methods.apply.grove_apply_partial_batch = 1;
    drive_version
}

/// Inserts a flagged 200 byte item under the misc tree with the shipped
/// generation, which accepts the historical flag types.
fn insert_flagged_item(drive: &Drive, flags: &StorageFlags, drive_version: &DriveVersion) {
    let mut batch = GroveDbOpBatch::new();
    batch.add_insert(
        misc_path_vec(),
        KEY.to_vec(),
        Element::new_item_with_flags(vec![7u8; 200], flags.to_some_element_flags()),
    );
    drive
        .grove_apply_batch_with_add_costs(batch, false, None, &mut vec![], drive_version)
        .expect("should insert the flagged item");
}

fn delete_item(
    drive: &Drive,
    drive_version: &DriveVersion,
) -> Result<Vec<LowLevelDriveOperation>, Error> {
    let mut batch = GroveDbOpBatch::new();
    batch.add_delete(misc_path_vec(), KEY.to_vec());
    let mut drive_operations = vec![];
    drive.grove_apply_batch_with_add_costs(
        batch,
        false,
        None,
        &mut drive_operations,
        drive_version,
    )?;
    Ok(drive_operations)
}

fn root_hash(drive: &Drive, drive_version: &DriveVersion) -> [u8; 32] {
    drive
        .grove
        .root_hash(None, &drive_version.grove_version)
        .unwrap()
        .expect("should get root hash")
}

fn sectioned_removal(cost: &OperationCost) -> &StorageRemovedBytes {
    &cost.storage_cost.removed_bytes
}

#[test]
fn should_delete_an_identity_owned_item_identically_under_both_generations() {
    let shipped = PlatformVersion::latest().drive.clone();
    let typed = typed_drive_version();
    let flags = StorageFlags::SingleEpochOwned(3, OWNER_ID);

    let drive_v0 = setup_drive_with_initial_state_structure(None);
    insert_flagged_item(&drive_v0, &flags, &shipped);
    let drive_v1 = setup_drive_with_initial_state_structure(None);
    insert_flagged_item(&drive_v1, &flags, &shipped);
    assert_eq!(
        root_hash(&drive_v0, &shipped),
        root_hash(&drive_v1, &shipped)
    );

    let ops_v0 = delete_item(&drive_v0, &shipped).expect("v0 should delete");
    let ops_v1 = delete_item(&drive_v1, &typed).expect("v1 should delete");

    assert_eq!(root_hash(&drive_v0, &shipped), root_hash(&drive_v1, &typed));

    let [LowLevelDriveOperation::CalculatedCostOperation(cost_v0)] = ops_v0.as_slice() else {
        panic!("v0 should push one plain cost operation, got {:?}", ops_v0);
    };
    let [LowLevelDriveOperation::CalculatedCostOperationWithRefundOwners {
        cost: cost_v1,
        refund_owners,
    }] = ops_v1.as_slice()
    else {
        panic!("v1 should push one typed cost operation, got {:?}", ops_v1);
    };

    assert_eq!(cost_v0, cost_v1, "only the operation shape differs");
    let SectionedStorageRemoval(removal) = sectioned_removal(cost_v1) else {
        panic!("an owned delete sections its removed bytes");
    };
    assert_eq!(removal.keys().copied().collect::<Vec<_>>(), vec![OWNER_ID]);
    assert_eq!(
        *refund_owners,
        RefundOwnersByIdentifier::from([(
            OWNER_ID,
            RefundOwner::Identity(Identifier::from(OWNER_ID))
        )])
    );
}

#[test]
fn should_record_the_bucket_owner_when_a_bucket_owned_item_is_deleted_under_v1() {
    let typed = typed_drive_version();
    let owner = RefundOwner::ContractBucket {
        contract_id: Identifier::from(CONTRACT_ID),
        position: 7,
    };
    let flags = StorageFlags::new_single_epoch_for_owner(3, Some(owner));

    let drive = setup_drive_with_initial_state_structure(None);
    // the insert carries the flags through unchanged: no closure parses
    // flags on a plain insert
    insert_flagged_item(&drive, &flags, &typed);

    let ops = delete_item(&drive, &typed).expect("v1 should delete a bucket owned item");

    let [LowLevelDriveOperation::CalculatedCostOperationWithRefundOwners {
        cost,
        refund_owners,
    }] = ops.as_slice()
    else {
        panic!("v1 should push one typed cost operation, got {:?}", ops);
    };
    let SectionedStorageRemoval(removal) = sectioned_removal(cost) else {
        panic!("an owned delete sections its removed bytes");
    };
    assert_eq!(
        removal.keys().copied().collect::<Vec<_>>(),
        vec![owner.removal_key()]
    );
    let bytes_removed: u32 = removal[&owner.removal_key()].values().sum();
    assert!(bytes_removed >= 200, "the item value is at least 200 bytes");
    assert_eq!(
        *refund_owners,
        RefundOwnersByIdentifier::from([(owner.removal_key(), owner)])
    );
}

/// A replace whose new flags name a different owner transfers the bytes.
/// GroveDB prices a replace with the old flags still attached, so a smaller
/// payload in a later epoch takes the shrinking path, where the crate returns
/// the old single epoch flags untouched. The typed generation must still
/// transfer: the bytes freed by the shrink refund the bucket that paid for
/// them, and the delete afterwards refunds the identity, not the bucket.
#[test]
fn should_transfer_a_bucket_owned_item_to_an_identity_on_a_later_epoch_shrinking_replace() {
    let typed = typed_drive_version();
    let bucket = RefundOwner::ContractBucket {
        contract_id: Identifier::from(CONTRACT_ID),
        position: 7,
    };
    let identity = RefundOwner::Identity(Identifier::from(OWNER_ID));

    let drive = setup_drive_with_initial_state_structure(None);
    insert_flagged_item(
        &drive,
        &StorageFlags::new_single_epoch_for_owner(1, Some(bucket)),
        &typed,
    );

    let mut batch = GroveDbOpBatch::new();
    batch.push(QualifiedGroveDbOp::replace_op(
        misc_path_vec(),
        KEY.to_vec(),
        Element::new_item_with_flags(
            vec![7u8; 100],
            StorageFlags::new_single_epoch_for_owner(2, Some(identity)).to_some_element_flags(),
        ),
    ));
    let mut replace_operations = vec![];
    drive
        .grove_apply_batch_with_add_costs(batch, false, None, &mut replace_operations, &typed)
        .expect("v1 should replace across kinds");

    // the bytes freed by the shrink were paid for by the bucket
    let [LowLevelDriveOperation::CalculatedCostOperationWithRefundOwners {
        cost: shrink_cost,
        refund_owners: shrink_owners,
    }] = replace_operations.as_slice()
    else {
        panic!(
            "v1 should push one typed cost operation, got {:?}",
            replace_operations
        );
    };
    let SectionedStorageRemoval(shrink_removal) = sectioned_removal(shrink_cost) else {
        panic!("a shrinking replace sections its removed bytes");
    };
    assert_eq!(
        shrink_removal.keys().copied().collect::<Vec<_>>(),
        vec![bucket.removal_key()]
    );
    assert_eq!(
        *shrink_owners,
        RefundOwnersByIdentifier::from([(bucket.removal_key(), bucket)])
    );

    let stored = drive
        .grove
        .get(
            SubtreePath::from(misc_path_vec().as_slice()),
            KEY,
            None,
            &typed.grove_version,
        )
        .unwrap()
        .expect("item should exist");
    let flags = StorageFlags::map_some_element_flags_ref(stored.get_flags())
        .expect("flags should decode")
        .expect("flags should be present");
    assert_eq!(flags, StorageFlags::SingleEpochOwned(1, OWNER_ID));

    let ops = delete_item(&drive, &typed).expect("v1 should delete");
    let [LowLevelDriveOperation::CalculatedCostOperationWithRefundOwners {
        cost,
        refund_owners,
    }] = ops.as_slice()
    else {
        panic!("v1 should push one typed cost operation, got {:?}", ops);
    };
    let SectionedStorageRemoval(removal) = sectioned_removal(cost) else {
        panic!("an owned delete sections its removed bytes");
    };
    assert_eq!(removal.keys().copied().collect::<Vec<_>>(), vec![OWNER_ID]);
    assert_eq!(
        *refund_owners,
        RefundOwnersByIdentifier::from([(OWNER_ID, identity)])
    );
}

/// Replaces `KEY` under the typed generation with `payload_len` bytes and
/// the given flags, returning the pushed operations.
fn replace_item(
    drive: &Drive,
    payload_len: usize,
    flags: &StorageFlags,
    drive_version: &DriveVersion,
) -> Result<Vec<LowLevelDriveOperation>, Error> {
    let mut batch = GroveDbOpBatch::new();
    batch.push(QualifiedGroveDbOp::replace_op(
        misc_path_vec(),
        KEY.to_vec(),
        Element::new_item_with_flags(vec![7u8; payload_len], flags.to_some_element_flags()),
    ));
    let mut drive_operations = vec![];
    drive.grove_apply_batch_with_add_costs(
        batch,
        false,
        None,
        &mut drive_operations,
        drive_version,
    )?;
    Ok(drive_operations)
}

fn stored_flags(drive: &Drive, drive_version: &DriveVersion) -> StorageFlags {
    let stored = drive
        .grove
        .get(
            SubtreePath::from(misc_path_vec().as_slice()),
            KEY,
            None,
            &drive_version.grove_version,
        )
        .unwrap()
        .expect("item should exist");
    StorageFlags::map_some_element_flags_ref(stored.get_flags())
        .expect("flags should decode")
        .expect("flags should be present")
}

/// GroveDB prices a replace with the old flags attached and asks the update
/// closure whether the flags changed. A same-epoch replace across kinds
/// leaves the proposed flags as they are but changes the header width (35
/// bytes for an identity, 37 for a bucket), so the closure must report a
/// change or GroveDB keeps a stale price and rejects the write.
#[test]
fn should_replace_across_kinds_in_the_same_epoch_when_the_header_width_changes() {
    let typed = typed_drive_version();
    let bucket = RefundOwner::ContractBucket {
        contract_id: Identifier::from(CONTRACT_ID),
        position: 7,
    };
    let identity = RefundOwner::Identity(Identifier::from(OWNER_ID));

    // identity to bucket, payload one byte bigger
    let drive = setup_drive_with_initial_state_structure(None);
    insert_flagged_item(
        &drive,
        &StorageFlags::new_single_epoch_for_owner(1, Some(identity)),
        &typed,
    );
    let ops = replace_item(
        &drive,
        201,
        &StorageFlags::new_single_epoch_for_owner(1, Some(bucket)),
        &typed,
    )
    .expect("identity to bucket replace should apply");
    assert!(matches!(
        ops.as_slice(),
        [LowLevelDriveOperation::CalculatedCostOperationWithRefundOwners { .. }]
    ));
    assert_eq!(
        stored_flags(&drive, &typed),
        StorageFlags::SingleEpochContractBucket(1, CONTRACT_ID, 7)
    );
    let ops = delete_item(&drive, &typed).expect("v1 should delete");
    let [LowLevelDriveOperation::CalculatedCostOperationWithRefundOwners { refund_owners, .. }] =
        ops.as_slice()
    else {
        panic!("v1 should push one typed cost operation, got {:?}", ops);
    };
    assert_eq!(
        *refund_owners,
        RefundOwnersByIdentifier::from([(bucket.removal_key(), bucket)])
    );

    // bucket to identity, payload one byte bigger: priced with the old
    // header it looks like growth, with the new header it is a one byte
    // shrink, and the freed byte belongs to the bucket that paid for it
    let drive = setup_drive_with_initial_state_structure(None);
    insert_flagged_item(
        &drive,
        &StorageFlags::new_single_epoch_for_owner(1, Some(bucket)),
        &typed,
    );
    let ops = replace_item(
        &drive,
        201,
        &StorageFlags::new_single_epoch_for_owner(1, Some(identity)),
        &typed,
    )
    .expect("bucket to identity replace should apply");
    let [LowLevelDriveOperation::CalculatedCostOperationWithRefundOwners {
        cost,
        refund_owners,
    }] = ops.as_slice()
    else {
        panic!("v1 should push one typed cost operation, got {:?}", ops);
    };
    let SectionedStorageRemoval(removal) = sectioned_removal(cost) else {
        panic!("the one byte shrink is sectioned, got {:?}", cost);
    };
    assert_eq!(
        removal.keys().copied().collect::<Vec<_>>(),
        vec![bucket.removal_key()]
    );
    assert_eq!(
        *refund_owners,
        RefundOwnersByIdentifier::from([(bucket.removal_key(), bucket)])
    );
    assert_eq!(
        stored_flags(&drive, &typed),
        StorageFlags::SingleEpochOwned(1, OWNER_ID)
    );
}

/// A later-epoch replace whose payload shrinks by exactly the header growth
/// of the new owner kind nets to zero bytes. The first pricing pass sees a
/// shrink and transfers ownership; the recalculated pass sees the same size
/// and must resolve ownership the same way, or GroveDB's update loop
/// alternates between the two answers and never converges.
#[test]
fn should_converge_when_a_later_epoch_replace_nets_to_the_same_size_across_kinds() {
    let typed = typed_drive_version();
    let bucket = RefundOwner::ContractBucket {
        contract_id: Identifier::from(CONTRACT_ID),
        position: 7,
    };
    let identity = RefundOwner::Identity(Identifier::from(OWNER_ID));

    let drive = setup_drive_with_initial_state_structure(None);
    insert_flagged_item(
        &drive,
        &StorageFlags::new_single_epoch_for_owner(1, Some(identity)),
        &typed,
    );
    let ops = replace_item(
        &drive,
        198,
        &StorageFlags::new_single_epoch_for_owner(2, Some(bucket)),
        &typed,
    )
    .expect("a net same size replace across kinds should apply");

    let [LowLevelDriveOperation::CalculatedCostOperationWithRefundOwners { cost, .. }] =
        ops.as_slice()
    else {
        panic!("v1 should push one typed cost operation, got {:?}", ops);
    };
    assert_eq!(cost.storage_cost.added_bytes, 0, "nothing was added on net");
    assert_eq!(
        *sectioned_removal(cost),
        NoStorageRemoval,
        "nothing was freed on net"
    );
    // the bytes keep their original epoch and move to the new owner
    assert_eq!(
        stored_flags(&drive, &typed),
        StorageFlags::SingleEpochContractBucket(1, CONTRACT_ID, 7)
    );

    let ops = delete_item(&drive, &typed).expect("v1 should delete");
    let [LowLevelDriveOperation::CalculatedCostOperationWithRefundOwners { refund_owners, .. }] =
        ops.as_slice()
    else {
        panic!("v1 should push one typed cost operation, got {:?}", ops);
    };
    assert_eq!(
        *refund_owners,
        RefundOwnersByIdentifier::from([(bucket.removal_key(), bucket)])
    );
}

#[test]
fn should_fail_closed_when_a_bucket_owned_item_is_deleted_under_v0() {
    let shipped = PlatformVersion::latest().drive.clone();
    let typed = typed_drive_version();
    let owner = RefundOwner::ContractBucket {
        contract_id: Identifier::from(CONTRACT_ID),
        position: 7,
    };
    let flags = StorageFlags::new_single_epoch_for_owner(3, Some(owner));

    let drive = setup_drive_with_initial_state_structure(None);
    insert_flagged_item(&drive, &flags, &typed);
    let before = root_hash(&drive, &shipped);

    let mut batch = GroveDbOpBatch::new();
    batch.add_delete(misc_path_vec(), KEY.to_vec());
    let mut drive_operations = vec![];
    let error = drive
        .grove_apply_batch_with_add_costs(batch, false, None, &mut drive_operations, &shipped)
        .expect_err("v0 cannot attribute bucket owned bytes");

    assert!(
        error
            .to_string()
            .contains("unknown storage flags serialization"),
        "unexpected error: {}",
        error
    );
    assert_eq!(root_hash(&drive, &shipped), before, "nothing was applied");
    // the read cost incurred before the failure is still pushed, as for any
    // failed grove operation, but no removed bytes and no owners reach the
    // fee path
    for op in &drive_operations {
        let LowLevelDriveOperation::CalculatedCostOperation(cost) = op else {
            panic!("only plain read costs may be pushed, got {:?}", op);
        };
        assert_eq!(*sectioned_removal(cost), NoStorageRemoval);
    }

    // the item is still there and version 1 removes it with the recorded owner
    let ops = delete_item(&drive, &typed).expect("v1 should delete");
    assert!(matches!(
        ops.as_slice(),
        [LowLevelDriveOperation::CalculatedCostOperationWithRefundOwners { .. }]
    ));
}

#[test]
fn should_reject_an_unknown_generation() {
    let drive = setup_drive_with_initial_state_structure(None);
    let mut drive_version = PlatformVersion::latest().drive.clone();
    drive_version.grove_methods.apply.grove_apply_batch = 2;

    let mut batch = GroveDbOpBatch::new();
    batch.add_delete(misc_path_vec(), KEY.to_vec());

    let error = drive
        .grove_apply_batch_with_add_costs(batch, false, None, &mut vec![], &drive_version)
        .expect_err("version 2 does not exist");

    assert!(matches!(
        error,
        Error::Drive(crate::error::drive::DriveError::UnknownVersionMismatch { .. })
    ));
}
