// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

#[cfg(feature = "server")]
use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
#[cfg(feature = "server")]
use crate::drive::batch::GroveDbOpBatch;
#[cfg(feature = "server")]
use crate::drive::credit_pools::paths::pools_vec_path;
#[cfg(feature = "server")]
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fee_pools::epochs::operations_factory::EpochOperations;
#[cfg(feature = "server")]
use crate::fee_pools::epochs_root_tree_key_constants::{
    KEY_PENDING_EPOCH_REFUNDS, KEY_STORAGE_FEE_POOL, KEY_UNPAID_EPOCH_INDEX,
};
#[cfg(feature = "server")]
use dpp::balances::credits::Creditable;
#[cfg(feature = "server")]
use dpp::block::epoch::{Epoch, EpochIndex};
#[cfg(feature = "server")]
use dpp::fee::epoch::{perpetual_storage_epochs, GENESIS_EPOCH_INDEX};
#[cfg(feature = "server")]
use dpp::fee::Credits;
use dpp::util::deserializer::ProtocolVersion;
#[cfg(feature = "server")]
use grovedb::batch::GroveDbOp;
#[cfg(feature = "server")]
use grovedb::Element;

#[cfg(any(feature = "server", feature = "verify"))]
/// Epochs module
pub mod epochs;

#[cfg(any(feature = "server", feature = "verify"))]
/// Epochs root tree key constants module
pub mod epochs_root_tree_key_constants;

#[cfg(feature = "server")]
/// Adds the operations to groveDB op batch to create the fee pool trees
pub fn add_create_fee_pool_trees_operations(
    batch: &mut GroveDbOpBatch,
    epochs_per_era: u16,
    protocol_version: ProtocolVersion,
) -> Result<(), Error> {
    // Init storage credit pool
    batch.push(update_storage_fee_distribution_pool_operation(0)?);

    // Init next epoch to pay
    batch.push(update_unpaid_epoch_index_operation(GENESIS_EPOCH_INDEX));

    add_create_pending_epoch_refunds_tree_operations(batch);

    // We need to insert 50 era worth of epochs,
    // with 40 epochs per era that's 2000 epochs
    // however this is configurable
    for i in GENESIS_EPOCH_INDEX..perpetual_storage_epochs(epochs_per_era) {
        let epoch = Epoch::new(i)?;
        epoch.add_init_empty_operations(batch)?;
    }

    let genesis_epoch = Epoch::new(GENESIS_EPOCH_INDEX)?;

    // Initial protocol version for genesis epoch
    batch.push(genesis_epoch.update_protocol_version_operation(protocol_version));

    Ok(())
}

#[cfg(feature = "server")]
/// Adds operations to batch to create pending pool updates tree
pub fn add_create_pending_epoch_refunds_tree_operations(batch: &mut GroveDbOpBatch) {
    batch.add_insert_empty_sum_tree(pools_vec_path(), KEY_PENDING_EPOCH_REFUNDS.to_vec());
}

#[cfg(feature = "server")]
/// Updates the storage fee distribution pool with a new storage fee
pub fn update_storage_fee_distribution_pool_operation(
    storage_fee: Credits,
) -> Result<GroveDbOp, Error> {
    Ok(GroveDbOp::insert_op(
        pools_vec_path(),
        KEY_STORAGE_FEE_POOL.to_vec(),
        Element::new_sum_item(storage_fee.to_signed()?),
    ))
}

#[cfg(feature = "server")]
/// Updates the unpaid epoch index
pub fn update_unpaid_epoch_index_operation(epoch_index: EpochIndex) -> GroveDbOp {
    GroveDbOp::insert_op(
        pools_vec_path(),
        KEY_UNPAID_EPOCH_INDEX.to_vec(),
        Element::new_item(epoch_index.to_be_bytes().to_vec()),
    )
}

// TODD: Find tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;

    mod add_create_fee_pool_trees_operations {
        use super::*;
        use dpp::version::PlatformVersion;

        #[test]
        fn test_values_are_set() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();
            let platform_version = PlatformVersion::latest();

            let storage_fee_pool = drive
                .get_storage_fees_from_distribution_pool(Some(&transaction), platform_version)
                .expect("should get storage fee pool");

            assert_eq!(storage_fee_pool, 0u64);
        }

        #[test]
        fn test_epoch_trees_are_created() {
            let drive = setup_drive_with_initial_state_structure();
            let platform_version = PlatformVersion::latest();
            let transaction = drive.grove.start_transaction();

            let perpetual_storage_epochs = perpetual_storage_epochs(drive.config.epochs_per_era);

            for epoch_index in 0..perpetual_storage_epochs {
                let epoch = Epoch::new(epoch_index).unwrap();

                let storage_fee = drive
                    .get_epoch_storage_credits_for_distribution(
                        &epoch,
                        Some(&transaction),
                        platform_version,
                    )
                    .expect("should get storage fee");

                assert_eq!(storage_fee, 0);
            }

            let epoch = Epoch::new(perpetual_storage_epochs).unwrap(); // 1001th epochs pool

            let result = drive.get_epoch_storage_credits_for_distribution(
                &epoch,
                Some(&transaction),
                platform_version,
            );

            assert!(matches!(result, Err(Error::GroveDB(_))));
        }
    }

    mod update_storage_fee_distribution_pool_operation {
        use super::*;
        use dpp::version::PlatformVersion;

        #[test]
        fn test_update_and_get_value() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::latest();

            let storage_fee = 42;

            let mut batch = GroveDbOpBatch::new();

            batch.push(
                update_storage_fee_distribution_pool_operation(storage_fee)
                    .expect("should return operation"),
            );

            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let stored_storage_fee = drive
                .get_storage_fees_from_distribution_pool(Some(&transaction), platform_version)
                .expect("should get storage fee pool");

            assert_eq!(storage_fee, stored_storage_fee);
        }
    }
}
