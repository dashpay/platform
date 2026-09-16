//! Tests that run both partial batch apply generations through the
//! dispatcher. Version 1 is selected only by a test-built drive version.

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
use grovedb::Element;
use grovedb_costs::storage_cost::removal::StorageRemovedBytes::{
    NoStorageRemoval, SectionedStorageRemoval,
};
use platform_version::version::drive_versions::DriveVersion;

const OWNER_ID: [u8; 32] = [0x33; 32];
const CONTRACT_ID: [u8; 32] = [0x44; 32];
const KEY: &[u8] = b"fix-08-partial-flagged-item";

fn typed_drive_version() -> DriveVersion {
    let mut drive_version = PlatformVersion::latest().drive.clone();
    drive_version.grove_methods.apply.grove_apply_batch = 1;
    drive_version.grove_methods.apply.grove_apply_partial_batch = 1;
    drive_version
}

fn insert_flagged_item(drive: &Drive, flags: &StorageFlags, drive_version: &DriveVersion) {
    let mut batch = GroveDbOpBatch::new();
    batch.add_insert(
        misc_path_vec(),
        KEY.to_vec(),
        Element::new_item_with_flags(vec![9u8; 150], flags.to_some_element_flags()),
    );
    drive
        .grove_apply_batch_with_add_costs(batch, false, None, &mut vec![], drive_version)
        .expect("should insert the flagged item");
}

fn delete_item_partially(
    drive: &Drive,
    drive_version: &DriveVersion,
) -> Result<Vec<LowLevelDriveOperation>, Error> {
    let mut batch = GroveDbOpBatch::new();
    batch.add_delete(misc_path_vec(), KEY.to_vec());
    let mut drive_operations = vec![];
    drive.grove_apply_partial_batch_with_add_costs(
        batch,
        false,
        None,
        |_cost, _ops_by_level| Ok(vec![]),
        &mut drive_operations,
        drive_version,
    )?;
    Ok(drive_operations)
}

#[test]
fn should_push_the_same_cost_for_an_identity_owned_delete_under_both_generations() {
    let shipped = PlatformVersion::latest().drive.clone();
    let typed = typed_drive_version();
    let flags = StorageFlags::SingleEpochOwned(2, OWNER_ID);

    let drive_v0 = setup_drive_with_initial_state_structure(None);
    insert_flagged_item(&drive_v0, &flags, &shipped);
    let drive_v1 = setup_drive_with_initial_state_structure(None);
    insert_flagged_item(&drive_v1, &flags, &shipped);

    let ops_v0 = delete_item_partially(&drive_v0, &shipped).expect("v0 should delete");
    let ops_v1 = delete_item_partially(&drive_v1, &typed).expect("v1 should delete");

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
    assert_eq!(cost_v0, cost_v1);
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
        position: 1,
    };
    let flags = StorageFlags::new_single_epoch_for_owner(2, Some(owner));

    let drive = setup_drive_with_initial_state_structure(None);
    insert_flagged_item(&drive, &flags, &typed);

    let ops = delete_item_partially(&drive, &typed).expect("v1 should delete");
    let [LowLevelDriveOperation::CalculatedCostOperationWithRefundOwners {
        cost,
        refund_owners,
    }] = ops.as_slice()
    else {
        panic!("v1 should push one typed cost operation, got {:?}", ops);
    };
    let SectionedStorageRemoval(removal) = &cost.storage_cost.removed_bytes else {
        panic!("an owned delete sections its removed bytes");
    };
    assert_eq!(
        removal.keys().copied().collect::<Vec<_>>(),
        vec![owner.removal_key()]
    );
    assert_eq!(
        *refund_owners,
        RefundOwnersByIdentifier::from([(owner.removal_key(), owner)])
    );
}

/// The shipped partial batch generation is bound to the crate's flags type,
/// which does not know the bucket type bytes, so a bucket-owned removal fails
/// the batch: nothing is applied and no cost operation is pushed. Without
/// this, the bucket's carrier key would reach the shipped fee decoder as a
/// plain sectioned removal and be read as an identity.
#[test]
fn should_fail_closed_when_a_bucket_owned_item_is_deleted_under_v0() {
    let shipped = PlatformVersion::latest().drive.clone();
    let typed = typed_drive_version();
    let owner = RefundOwner::ContractBucket {
        contract_id: Identifier::from(CONTRACT_ID),
        position: 1,
    };
    let flags = StorageFlags::new_single_epoch_for_owner(2, Some(owner));

    let drive = setup_drive_with_initial_state_structure(None);
    insert_flagged_item(&drive, &flags, &typed);
    let before = drive
        .grove
        .root_hash(None, &shipped.grove_version)
        .unwrap()
        .expect("should get root hash");

    let mut batch = GroveDbOpBatch::new();
    batch.add_delete(misc_path_vec(), KEY.to_vec());
    let mut drive_operations = vec![];
    let error = drive
        .grove_apply_partial_batch_with_add_costs(
            batch,
            false,
            None,
            |_cost, _ops_by_level| Ok(vec![]),
            &mut drive_operations,
            &shipped,
        )
        .expect_err("v0 cannot attribute bucket owned bytes");

    assert!(
        error
            .to_string()
            .contains("unknown storage flags serialization"),
        "unexpected error: {}",
        error
    );
    // the read cost incurred before the failure is still pushed, as for any
    // failed grove operation, but no removed bytes and no owners reach the
    // fee path
    for op in &drive_operations {
        let LowLevelDriveOperation::CalculatedCostOperation(cost) = op else {
            panic!("only plain read costs may be pushed, got {:?}", op);
        };
        assert_eq!(cost.storage_cost.removed_bytes, NoStorageRemoval);
    }
    let after = drive
        .grove
        .root_hash(None, &shipped.grove_version)
        .unwrap()
        .expect("should get root hash");
    assert_eq!(after, before, "nothing was applied");

    // the item is still there and version 1 can still remove it
    let ops = delete_item_partially(&drive, &typed).expect("v1 should delete");
    assert!(matches!(
        ops.as_slice(),
        [LowLevelDriveOperation::CalculatedCostOperationWithRefundOwners { .. }]
    ));
}
