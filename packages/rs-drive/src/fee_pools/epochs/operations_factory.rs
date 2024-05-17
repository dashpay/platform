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

//! Epoch Operations
//!
//! Defines and implements in `Epoch` functions relevant to epoch management.
//!

use crate::drive::batch::GroveDbOpBatch;
use crate::drive::credit_pools::paths::pools_vec_path;
use crate::drive::Drive;
use crate::error::Error;

use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::fee_pools::epochs::epoch_key_constants::{
    KEY_FEE_MULTIPLIER, KEY_POOL_PROCESSING_FEES, KEY_POOL_STORAGE_FEES, KEY_PROPOSERS,
    KEY_PROTOCOL_VERSION, KEY_START_BLOCK_CORE_HEIGHT, KEY_START_BLOCK_HEIGHT, KEY_START_TIME,
};
use crate::fee_pools::epochs::paths::EpochProposers;
use dpp::balances::credits::Creditable;
use dpp::block::epoch::Epoch;
use dpp::fee::Credits;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::PlatformVersion;
use grovedb::batch::GroveDbOp;
use grovedb::{Element, TransactionArg};

/// Operations on Epochs
pub trait EpochOperations {
    /// Updates the given proposer's block count to the current + 1
    fn increment_proposer_block_count_operation(
        &self,
        drive: &Drive,
        proposer_pro_tx_hash: &[u8; 32],
        cached_previous_block_count: Option<u64>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<GroveDbOp, Error>;
    /// Adds to the groveDB op batch operations to insert an empty tree into the epoch
    fn add_init_empty_without_storage_operations(&self, batch: &mut GroveDbOpBatch);
    /// Adds to the groveDB op batch operations to insert an empty tree into the epoch
    /// and sets the storage distribution pool to 0.
    fn add_init_empty_operations(&self, batch: &mut GroveDbOpBatch) -> Result<(), Error>;
    /// Adds to the groveDB op batch initialization operations for the epoch.
    fn add_init_current_operations(
        &self,
        multiplier: f64,
        start_block_height: u64, // TODO Many method in drive needs block time and height. Maybe we need DTO for drive as well which will contain block information
        start_block_core_height: u32,
        start_time_ms: u64,
        batch: &mut GroveDbOpBatch,
    );
    /// Adds to the groveDB op batch operations signifying that the epoch distribution fees were paid out.
    fn add_mark_as_paid_operations(&self, batch: &mut GroveDbOpBatch);
    /// Update Epoch's protocol version
    fn update_protocol_version_operation(&self, protocol_version: ProtocolVersion) -> GroveDbOp;
    /// Returns a groveDB op which updates the epoch start time.
    fn update_start_time_operation(&self, time_ms: u64) -> GroveDbOp;
    /// Returns a groveDB op which updates the epoch start block height.
    fn update_start_block_height_operation(&self, start_block_height: u64) -> GroveDbOp;
    /// Returns a groveDB op which updates the epoch start block height.
    fn update_start_block_core_height_operation(&self, start_block_core_height: u32) -> GroveDbOp;
    /// Returns a groveDB op which updates the epoch fee multiplier.
    fn update_fee_multiplier_operation(&self, multiplier: f64) -> GroveDbOp;
    /// Returns a groveDB op which updates the epoch processing credits for distribution.
    fn update_processing_fee_pool_operation(
        &self,
        processing_fee: Credits,
    ) -> Result<GroveDbOp, Error>;
    /// Returns a groveDB op which deletes the epoch processing credits for distribution tree.
    fn delete_processing_credits_for_distribution_operation(&self) -> GroveDbOp;
    /// Returns a groveDB op which updates the epoch storage credits for distribution.
    fn update_storage_fee_pool_operation(&self, storage_fee: Credits) -> Result<GroveDbOp, Error>;
    /// Returns a groveDB op which deletes the epoch storage credits for distribution tree.
    fn delete_storage_credits_for_distribution_operation(&self) -> GroveDbOp;
    /// Returns a groveDB op which updates the given epoch proposer's block count.
    fn update_proposer_block_count_operation(
        &self,
        proposer_pro_tx_hash: &[u8; 32],
        block_count: u64,
    ) -> GroveDbOp;
    /// Returns a groveDB op which inserts an empty tree into the epoch proposers path.
    fn init_proposers_tree_operation(&self) -> GroveDbOp;
    /// Returns a groveDB op which deletes the epoch proposers tree.
    fn delete_proposers_tree_operation(&self) -> GroveDbOp;
    /// Adds a groveDB op to the batch which deletes the given epoch proposers from the proposers tree.
    fn add_delete_proposers_operations(
        &self,
        pro_tx_hashes: Vec<Vec<u8>>,
        batch: &mut GroveDbOpBatch,
    );
}

impl EpochOperations for Epoch {
    /// Updates the given proposer's block count to the current + 1
    fn increment_proposer_block_count_operation(
        &self,
        drive: &Drive,
        proposer_pro_tx_hash: &[u8; 32],
        cached_previous_block_count: Option<u64>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<GroveDbOp, Error> {
        // get current proposer's block count
        let proposed_block_count = if let Some(block_count) = cached_previous_block_count {
            block_count
        } else {
            drive
                .get_epochs_proposer_block_count(
                    self,
                    proposer_pro_tx_hash,
                    transaction,
                    platform_version,
                )
                .or_else(|e| match e {
                    Error::GroveDB(grovedb::Error::PathKeyNotFound(_)) => Ok(0u64),
                    _ => Err(e),
                })?
        };

        let operation = self
            .update_proposer_block_count_operation(proposer_pro_tx_hash, proposed_block_count + 1);

        Ok(operation)
    }

    /// Adds to the groveDB op batch operations to insert an empty tree into the epoch
    fn add_init_empty_without_storage_operations(&self, batch: &mut GroveDbOpBatch) {
        batch.add_insert_empty_sum_tree(pools_vec_path(), self.key.to_vec());
    }

    /// Adds to the groveDB op batch operations to insert an empty tree into the epoch
    /// and sets the storage distribution pool to 0.
    fn add_init_empty_operations(&self, batch: &mut GroveDbOpBatch) -> Result<(), Error> {
        self.add_init_empty_without_storage_operations(batch);

        // init storage fee item to 0
        batch.push(self.update_storage_fee_pool_operation(0)?);

        Ok(())
    }

    /// Adds to the groveDB op batch initialization operations for the epoch.
    fn add_init_current_operations(
        &self,
        multiplier: f64,
        start_block_height: u64, // TODO Many method in drive needs block time and height. Maybe we need DTO for drive as well which will contain block information
        start_block_core_height: u32,
        start_time_ms: u64,
        batch: &mut GroveDbOpBatch,
    ) {
        batch.push(self.update_start_block_height_operation(start_block_height));

        batch.push(self.update_start_block_core_height_operation(start_block_core_height));

        batch.push(self.init_proposers_tree_operation());

        batch.push(self.update_fee_multiplier_operation(multiplier));

        batch.push(self.update_start_time_operation(start_time_ms));
    }

    /// Adds to the groveDB op batch operations signifying that the epoch distribution fees were paid out.
    fn add_mark_as_paid_operations(&self, batch: &mut GroveDbOpBatch) {
        batch.push(self.delete_proposers_tree_operation());

        batch.push(self.delete_storage_credits_for_distribution_operation());

        batch.push(self.delete_processing_credits_for_distribution_operation());
    }

    /// Returns a groveDB op which updates the epoch start time.
    fn update_protocol_version_operation(&self, protocol_version: ProtocolVersion) -> GroveDbOp {
        GroveDbOp::insert_op(
            self.get_path_vec(),
            KEY_PROTOCOL_VERSION.to_vec(),
            Element::Item(protocol_version.to_be_bytes().to_vec(), None),
        )
    }

    /// Returns a groveDB op which updates the epoch start time.
    fn update_start_time_operation(&self, time_ms: u64) -> GroveDbOp {
        GroveDbOp::insert_op(
            self.get_path_vec(),
            KEY_START_TIME.to_vec(),
            Element::Item(time_ms.to_be_bytes().to_vec(), None),
        )
    }

    /// Returns a groveDB op which updates the epoch start block height.
    fn update_start_block_height_operation(&self, start_block_height: u64) -> GroveDbOp {
        GroveDbOp::insert_op(
            self.get_path_vec(),
            KEY_START_BLOCK_HEIGHT.to_vec(),
            Element::Item(start_block_height.to_be_bytes().to_vec(), None),
        )
    }

    /// Returns a groveDB op which updates the epoch start block core height.
    fn update_start_block_core_height_operation(&self, start_block_core_height: u32) -> GroveDbOp {
        GroveDbOp::insert_op(
            self.get_path_vec(),
            KEY_START_BLOCK_CORE_HEIGHT.to_vec(),
            Element::Item(start_block_core_height.to_be_bytes().to_vec(), None),
        )
    }

    /// Returns a groveDB op which updates the epoch fee multiplier.
    fn update_fee_multiplier_operation(&self, multiplier: f64) -> GroveDbOp {
        GroveDbOp::insert_op(
            self.get_path_vec(),
            KEY_FEE_MULTIPLIER.to_vec(),
            Element::Item(multiplier.to_be_bytes().to_vec(), None),
        )
    }

    /// Returns a groveDB op which updates the epoch processing credits for distribution.
    fn update_processing_fee_pool_operation(
        &self,
        processing_fee: Credits,
    ) -> Result<GroveDbOp, Error> {
        Ok(GroveDbOp::insert_op(
            self.get_path_vec(),
            KEY_POOL_PROCESSING_FEES.to_vec(),
            Element::new_sum_item(processing_fee.to_signed()?),
        ))
    }

    /// Returns a groveDB op which deletes the epoch processing credits for distribution tree.
    fn delete_processing_credits_for_distribution_operation(&self) -> GroveDbOp {
        GroveDbOp::delete_op(self.get_path_vec(), KEY_POOL_PROCESSING_FEES.to_vec())
    }

    /// Returns a groveDB op which updates the epoch storage credits for distribution.
    fn update_storage_fee_pool_operation(&self, storage_fee: Credits) -> Result<GroveDbOp, Error> {
        Ok(GroveDbOp::insert_op(
            self.get_path_vec(),
            KEY_POOL_STORAGE_FEES.to_vec(),
            Element::new_sum_item(storage_fee.to_signed()?),
        ))
    }

    /// Returns a groveDB op which deletes the epoch storage credits for distribution tree.
    fn delete_storage_credits_for_distribution_operation(&self) -> GroveDbOp {
        GroveDbOp::delete_op(self.get_path_vec(), KEY_POOL_STORAGE_FEES.to_vec())
    }

    /// Returns a groveDB op which updates the given epoch proposer's block count.
    fn update_proposer_block_count_operation(
        &self,
        proposer_pro_tx_hash: &[u8; 32],
        block_count: u64,
    ) -> GroveDbOp {
        GroveDbOp::insert_op(
            self.get_proposers_path_vec(),
            proposer_pro_tx_hash.to_vec(),
            Element::Item(block_count.to_be_bytes().to_vec(), None),
        )
    }

    /// Returns a groveDB op which inserts an empty tree into the epoch proposers path.
    fn init_proposers_tree_operation(&self) -> GroveDbOp {
        GroveDbOp::insert_op(
            self.get_path_vec(),
            KEY_PROPOSERS.to_vec(),
            Element::empty_tree(),
        )
    }

    /// Returns a groveDB op which deletes the epoch proposers tree.
    fn delete_proposers_tree_operation(&self) -> GroveDbOp {
        GroveDbOp::delete_tree_op(self.get_path_vec(), KEY_PROPOSERS.to_vec(), false)
    }

    /// Adds a groveDB op to the batch which deletes the given epoch proposers from the proposers tree.
    fn add_delete_proposers_operations(
        &self,
        pro_tx_hashes: Vec<Vec<u8>>,
        batch: &mut GroveDbOpBatch,
    ) {
        for pro_tx_hash in pro_tx_hashes.into_iter() {
            batch.add_delete(self.get_proposers_path_vec(), pro_tx_hash);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};
    use chrono::Utc;
    use dpp::version::PlatformVersion;

    mod increment_proposer_block_count_operation {
        use super::*;

        #[test]
        fn test_increment_block_count_to_1_if_proposers_tree_is_not_committed() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::first();

            let pro_tx_hash: [u8; 32] = rand::random();

            let epoch = Epoch::new(0).unwrap();

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch.init_proposers_tree_operation());

            batch.push(
                epoch
                    .increment_proposer_block_count_operation(
                        &drive,
                        &pro_tx_hash,
                        Some(0),
                        Some(&transaction),
                        platform_version,
                    )
                    .expect("should increment proposer block count operations"),
            );

            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let stored_block_count = drive
                .get_epochs_proposer_block_count(
                    &epoch,
                    &pro_tx_hash,
                    Some(&transaction),
                    platform_version,
                )
                .expect("should get proposer block count");

            assert_eq!(stored_block_count, 1);
        }

        #[test]
        fn test_existing_block_count_is_incremented() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::first();

            let pro_tx_hash: [u8; 32] = rand::random();

            let epoch = Epoch::new(1).unwrap();

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch.init_proposers_tree_operation());

            // Apply proposers tree
            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch.update_proposer_block_count_operation(&pro_tx_hash, 1));

            // Apply proposer block count
            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let mut batch = GroveDbOpBatch::new();

            batch.push(
                epoch
                    .increment_proposer_block_count_operation(
                        &drive,
                        &pro_tx_hash,
                        None,
                        Some(&transaction),
                        platform_version,
                    )
                    .expect("should update proposer block count"),
            );

            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let stored_block_count = drive
                .get_epochs_proposer_block_count(
                    &epoch,
                    &pro_tx_hash,
                    Some(&transaction),
                    platform_version,
                )
                .expect("should get proposer block count");

            assert_eq!(stored_block_count, 2);
        }
    }

    mod add_init_empty_operations {
        use super::*;

        #[test]
        fn test_error_if_fee_pools_not_initialized() {
            let drive = setup_drive(None);
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::first();

            let epoch = Epoch::new(1042).unwrap();

            let mut batch = GroveDbOpBatch::new();

            epoch
                .add_init_empty_operations(&mut batch)
                .expect("should init empty epoch");

            let result =
                drive.grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive);

            assert!(matches!(
                result,
                Err(Error::GroveDB(grovedb::Error::InvalidPath(_)))
            ));
        }

        #[test]
        fn test_values_are_set() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::first();

            let epoch = Epoch::new(1042).unwrap();

            let mut batch = GroveDbOpBatch::new();

            epoch
                .add_init_empty_operations(&mut batch)
                .expect("should init empty epoch");

            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let storage_fee = drive
                .get_epoch_storage_credits_for_distribution(
                    &epoch,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to get storage credits in epoch pool");

            assert_eq!(storage_fee, 0);
        }
    }

    mod add_init_current_operations {
        use super::*;

        #[test]
        fn test_values_are_set() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::first();

            let epoch = Epoch::new(1042).unwrap();

            let multiplier = 42.0;
            let start_time = 1;
            let start_block_height = 2;
            let start_block_core_height = 5;

            let mut batch = GroveDbOpBatch::new();

            epoch
                .add_init_empty_operations(&mut batch)
                .expect("should init empty epoch");

            epoch.add_init_current_operations(
                multiplier,
                start_block_height,
                start_block_core_height,
                start_time,
                &mut batch,
            );

            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let stored_multiplier = drive
                .get_epoch_fee_multiplier(&epoch, Some(&transaction), platform_version)
                .expect("should get multiplier");

            assert_eq!(stored_multiplier, multiplier);

            let stored_start_time = drive
                .get_epoch_start_time(&epoch, Some(&transaction), platform_version)
                .expect("should get start time");

            assert_eq!(stored_start_time, start_time);

            let stored_block_height = drive
                .get_epoch_start_block_height(&epoch, Some(&transaction), platform_version)
                .expect("should get start block height");

            assert_eq!(stored_block_height, start_block_height);

            let stored_block_core_height = drive
                .get_epoch_start_block_core_height(&epoch, Some(&transaction), platform_version)
                .expect("should get start block core height");

            assert_eq!(stored_block_core_height, start_block_core_height);

            drive
                .get_epoch_processing_credits_for_distribution(
                    &epoch,
                    Some(&transaction),
                    platform_version,
                )
                .expect_err("should not get processing fee");

            let proposers = drive
                .get_epoch_proposers(&epoch, Some(1), Some(&transaction), platform_version)
                .expect("should get proposers");

            assert_eq!(proposers, vec!());
        }
    }

    mod add_mark_as_paid_operations {
        use super::*;

        #[test]
        fn test_values_are_deleted() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::first();

            let epoch = Epoch::new(0).unwrap();

            let mut batch = GroveDbOpBatch::new();

            epoch.add_init_current_operations(1.0, 2, 5, 3, &mut batch);

            // Apply init current
            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let mut batch = GroveDbOpBatch::new();

            epoch.add_mark_as_paid_operations(&mut batch);

            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let result = drive
                .grove
                .get(
                    &epoch.get_path(),
                    KEY_PROPOSERS.as_slice(),
                    Some(&transaction),
                )
                .unwrap();

            assert!(matches!(result, Err(grovedb::Error::PathKeyNotFound(_))));

            let result = drive.get_epoch_processing_credits_for_distribution(
                &epoch,
                Some(&transaction),
                platform_version,
            );

            assert!(matches!(
                result,
                Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_)))
            ));

            let result = drive.get_epoch_storage_credits_for_distribution(
                &epoch,
                Some(&transaction),
                platform_version,
            );

            assert!(matches!(
                result,
                Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_)))
            ));
        }
    }

    mod update_proposer_block_count {
        use super::*;

        #[test]
        fn test_value_is_set() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::first();

            let pro_tx_hash: [u8; 32] = rand::random();
            let block_count = 42;

            let epoch = Epoch::new(0).unwrap();

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch.init_proposers_tree_operation());

            batch.push(epoch.update_proposer_block_count_operation(&pro_tx_hash, block_count));

            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let stored_block_count = drive
                .get_epochs_proposer_block_count(
                    &epoch,
                    &pro_tx_hash,
                    Some(&transaction),
                    platform_version,
                )
                .expect("should get proposer block count");

            assert_eq!(stored_block_count, block_count);
        }
    }

    #[test]
    fn test_update_start_time() {
        let drive = setup_drive_with_initial_state_structure();
        let transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::first();

        let epoch_tree = Epoch::new(0).unwrap();

        let start_time_ms: u64 = Utc::now().timestamp_millis() as u64;

        let mut batch = GroveDbOpBatch::new();

        batch.push(epoch_tree.update_start_time_operation(start_time_ms));

        drive
            .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
            .expect("should apply batch");

        let actual_start_time_ms = drive
            .get_epoch_start_time(&epoch_tree, Some(&transaction), platform_version)
            .expect("should get start time");

        assert_eq!(start_time_ms, actual_start_time_ms);
    }

    #[test]
    fn test_update_epoch_start_block_height() {
        let drive = setup_drive_with_initial_state_structure();
        let transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::first();

        let epoch = Epoch::new(0).unwrap();

        let start_block_height = 1;

        let op = epoch.update_start_block_height_operation(start_block_height);

        drive
            .grove_apply_operation(op, false, Some(&transaction), &platform_version.drive)
            .expect("should apply batch");

        let actual_start_block_height = drive
            .get_epoch_start_block_height(&epoch, Some(&transaction), platform_version)
            .expect("should get start block height");

        assert_eq!(start_block_height, actual_start_block_height);
    }

    #[test]
    fn test_update_epoch_start_block_core_height() {
        let drive = setup_drive_with_initial_state_structure();
        let transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::first();

        let epoch = Epoch::new(0).unwrap();

        let start_block_height = 1;

        let op = epoch.update_start_block_core_height_operation(start_block_height);

        drive
            .grove_apply_operation(op, false, Some(&transaction), &platform_version.drive)
            .expect("should apply batch");

        let actual_start_block_height = drive
            .get_epoch_start_block_core_height(&epoch, Some(&transaction), platform_version)
            .expect("should get start block core height");

        assert_eq!(start_block_height, actual_start_block_height);
    }

    mod update_epoch_processing_credits_for_distribution {
        use super::*;

        #[test]
        fn test_error_if_epoch_tree_is_not_initiated() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::first();

            let epoch = Epoch::new(7000).unwrap();

            let op = epoch
                .update_processing_fee_pool_operation(42)
                .expect("should return operation");

            let result =
                drive.grove_apply_operation(op, false, Some(&transaction), &platform_version.drive);

            assert!(matches!(
                result,
                Err(Error::GroveDB(grovedb::Error::InvalidPath(_)))
            ));
        }

        #[test]
        fn test_value_is_set() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::first();

            let epoch = Epoch::new(0).unwrap();

            let processing_fee: u64 = 42;

            let op = epoch
                .update_processing_fee_pool_operation(42)
                .expect("should return operation");

            drive
                .grove_apply_operation(op, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let stored_processing_fee = drive
                .get_epoch_processing_credits_for_distribution(
                    &epoch,
                    Some(&transaction),
                    platform_version,
                )
                .expect("should get processing fee");

            assert_eq!(stored_processing_fee, processing_fee);
        }
    }

    mod update_epoch_storage_credits_for_distribution {
        use super::*;

        #[test]
        fn test_error_if_epoch_tree_is_not_initiated() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::first();

            let epoch = Epoch::new(7000).unwrap();

            let op = epoch
                .update_storage_fee_pool_operation(42)
                .expect("should return operation");

            let result =
                drive.grove_apply_operation(op, false, Some(&transaction), &platform_version.drive);

            assert!(matches!(
                result,
                Err(Error::GroveDB(grovedb::Error::InvalidPath(_)))
            ));
        }

        #[test]
        fn test_value_is_set() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::first();

            let epoch = Epoch::new(0).unwrap();

            let storage_fee = 42;

            let op = epoch
                .update_storage_fee_pool_operation(storage_fee)
                .expect("should return operation");

            drive
                .grove_apply_operation(op, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let stored_storage_fee = drive
                .get_epoch_storage_credits_for_distribution(
                    &epoch,
                    Some(&transaction),
                    platform_version,
                )
                .expect("should get storage fee");

            assert_eq!(stored_storage_fee, storage_fee);
        }
    }

    mod delete_proposers_tree {
        use super::*;

        #[test]
        fn test_values_has_been_deleted() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::first();

            let epoch = Epoch::new(0).unwrap();

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch.init_proposers_tree_operation());

            // Apply proposers tree
            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch.delete_proposers_tree_operation());

            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let error = drive
                .grove
                .get(
                    &epoch.get_path(),
                    KEY_PROPOSERS.as_slice(),
                    Some(&transaction),
                )
                .unwrap()
                .expect_err("expect tree not exists");

            match error {
                grovedb::Error::PathKeyNotFound(_) => {}
                _ => panic!("invalid error type"),
            }
        }
    }

    mod delete_proposers {
        use super::*;

        #[test]
        fn test_values_are_being_deleted() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = Epoch::new(0).unwrap();

            let platform_version = PlatformVersion::first();

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch.init_proposers_tree_operation());

            // Apply proposers tree
            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let pro_tx_hashes: Vec<[u8; 32]> = (0..10).map(|_| rand::random()).collect();

            let mut batch = GroveDbOpBatch::new();

            for pro_tx_hash in pro_tx_hashes.iter() {
                batch.push(epoch.update_proposer_block_count_operation(pro_tx_hash, 1));
            }

            // Apply proposers block count updates
            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let mut stored_proposers = drive
                .get_epoch_proposers(&epoch, Some(20), Some(&transaction), platform_version)
                .expect("should get proposers");

            let mut awaited_result = pro_tx_hashes
                .iter()
                .map(|hash| (hash.to_vec(), 1))
                .collect::<Vec<(Vec<u8>, u64)>>();

            // sort both result to be able to compare them
            stored_proposers.sort();
            awaited_result.sort();

            assert_eq!(stored_proposers, awaited_result);

            let deleted_pro_tx_hashes = vec![
                awaited_result.first().unwrap().0.clone(),
                awaited_result.get(1).unwrap().0.clone(),
            ];

            // remove items we deleted
            awaited_result.remove(0);
            awaited_result.remove(1);

            let mut batch = GroveDbOpBatch::new();

            epoch.add_delete_proposers_operations(deleted_pro_tx_hashes, &mut batch);

            // Apply proposers deletion
            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let stored_proposers = drive
                .get_epoch_proposers(&epoch, Some(20), Some(&transaction), platform_version)
                .expect("should get proposers");

            let mut stored_hexes: Vec<String> = stored_proposers
                .iter()
                .map(|(hash, _)| hex::encode(hash))
                .collect();

            let mut awaited_hexes: Vec<String> = stored_proposers
                .iter()
                .map(|(hash, _)| hex::encode(hash))
                .collect();

            stored_hexes.sort();
            awaited_hexes.sort();

            assert_eq!(stored_hexes, awaited_hexes);
        }
    }
}
