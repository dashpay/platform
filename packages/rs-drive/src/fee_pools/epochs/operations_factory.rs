use crate::drive::batch::GroveDbOpBatch;
use crate::drive::fee_pools::pools_vec_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee_pools::epochs::epoch_key_constants::{
    KEY_FEE_MULTIPLIER, KEY_POOL_PROCESSING_FEES, KEY_POOL_STORAGE_FEES, KEY_START_BLOCK_HEIGHT,
    KEY_START_TIME,
};
use crate::fee_pools::epochs::{epoch_key_constants, Epoch};
use grovedb::batch::Op::Insert;
use grovedb::batch::{GroveDbOp, Op};
use grovedb::{Element, TransactionArg};

impl Epoch {
    pub fn increment_proposer_block_count_operation(
        &self,
        drive: &Drive,
        proposer_pro_tx_hash: &[u8; 32],
        cached_previous_block_count: Option<u64>,
        transaction: TransactionArg,
    ) -> Result<GroveDbOp, Error> {
        // get current proposer's block count
        let proposed_block_count = match cached_previous_block_count {
            Some(block_count) => block_count,
            None => drive
                .get_epochs_proposer_block_count(self, proposer_pro_tx_hash, transaction)
                .or_else(|e| match e {
                    Error::GroveDB(grovedb::Error::PathKeyNotFound(_)) => Ok(0u64),
                    _ => Err(e),
                })?,
        };

        Ok(self
            .update_proposer_block_count_operation(proposer_pro_tx_hash, proposed_block_count + 1))
    }

    pub fn add_init_empty_operations(&self, batch: &mut GroveDbOpBatch) {
        batch.add_insert_empty_tree(pools_vec_path(), self.key.to_vec());

        // init storage fee item to 0
        batch.push(self.update_storage_credits_for_distribution_operation(0));
    }

    pub fn add_init_current_operations(
        &self,
        multiplier: f64,
        start_block_height: u64, // TODO Many method in drive needs block time and height. Maybe we need DTO for drive as well which will contain block information
        start_time_ms: u64,
        batch: &mut GroveDbOpBatch,
    ) {
        batch.push(self.update_start_block_height_operation(start_block_height));

        batch.push(self.init_proposers_tree_operation());

        batch.push(self.update_fee_multiplier_operation(multiplier));

        batch.push(self.update_start_time_operation(start_time_ms));
    }

    pub fn add_mark_as_paid_operations(&self, batch: &mut GroveDbOpBatch) {
        batch.push(self.delete_proposers_tree_operation());

        batch.push(self.delete_storage_credits_for_distribution_operation());

        batch.push(self.delete_processing_credits_for_distribution_operation());
    }

    pub fn update_start_time_operation(&self, time_ms: u64) -> GroveDbOp {
        GroveDbOp {
            path: self.get_vec_path(),
            key: KEY_START_TIME.to_vec(),
            op: Insert {
                element: Element::Item(time_ms.to_be_bytes().to_vec(), None),
            },
        }
    }

    pub fn update_start_block_height_operation(&self, start_block_height: u64) -> GroveDbOp {
        GroveDbOp {
            path: self.get_vec_path(),
            key: KEY_START_BLOCK_HEIGHT.to_vec(),
            op: Insert {
                element: Element::Item(start_block_height.to_be_bytes().to_vec(), None),
            },
        }
    }

    pub fn update_fee_multiplier_operation(&self, multiplier: f64) -> GroveDbOp {
        GroveDbOp {
            path: self.get_vec_path(),
            key: KEY_FEE_MULTIPLIER.to_vec(),
            op: Insert {
                element: Element::Item(multiplier.to_be_bytes().to_vec(), None),
            },
        }
    }

    pub fn update_processing_credits_for_distribution_operation(
        &self,
        processing_fee: u64,
    ) -> GroveDbOp {
        GroveDbOp {
            path: self.get_vec_path(),
            key: KEY_POOL_PROCESSING_FEES.to_vec(),
            op: Insert {
                element: Element::new_item(processing_fee.to_be_bytes().to_vec()),
            },
        }
    }

    pub fn delete_processing_credits_for_distribution_operation(&self) -> GroveDbOp {
        GroveDbOp {
            path: self.get_vec_path(),
            key: KEY_POOL_PROCESSING_FEES.to_vec(),
            op: Op::Delete,
        }
    }

    pub fn update_storage_credits_for_distribution_operation(&self, storage_fee: u64) -> GroveDbOp {
        GroveDbOp {
            path: self.get_vec_path(),
            key: KEY_POOL_STORAGE_FEES.to_vec(),
            op: Insert {
                element: Element::new_item(storage_fee.to_be_bytes().to_vec()),
            },
        }
    }

    pub fn delete_storage_credits_for_distribution_operation(&self) -> GroveDbOp {
        GroveDbOp {
            path: self.get_vec_path(),
            key: KEY_POOL_STORAGE_FEES.to_vec(),
            op: Op::Delete,
        }
    }

    pub(crate) fn update_proposer_block_count_operation(
        &self,
        proposer_pro_tx_hash: &[u8; 32],
        block_count: u64,
    ) -> GroveDbOp {
        GroveDbOp {
            path: self.get_proposers_vec_path(),
            key: proposer_pro_tx_hash.to_vec(),
            op: Insert {
                element: Element::Item(block_count.to_be_bytes().to_vec(), None),
            },
        }
    }

    pub fn init_proposers_tree_operation(&self) -> GroveDbOp {
        GroveDbOp {
            path: self.get_vec_path(),
            key: epoch_key_constants::KEY_PROPOSERS.to_vec(),
            op: Insert {
                element: Element::empty_tree(),
            },
        }
    }

    pub fn delete_proposers_tree_operation(&self) -> GroveDbOp {
        GroveDbOp {
            path: self.get_vec_path(),
            key: epoch_key_constants::KEY_PROPOSERS.to_vec(),
            op: Op::Delete,
        }
    }

    pub fn add_delete_proposers_operations(
        &self,
        pro_tx_hashes: Vec<Vec<u8>>,
        batch: &mut GroveDbOpBatch,
    ) {
        for pro_tx_hash in pro_tx_hashes.into_iter() {
            batch.add_delete(self.get_proposers_vec_path(), pro_tx_hash);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;
    use crate::drive::batch::GroveDbOpBatch;
    use crate::fee_pools::epochs::Epoch;
    use chrono::Utc;

    mod increment_proposer_block_count_operation {
        #[test]
        fn test_increment_block_count_to_1_if_proposers_tree_is_not_committed() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let pro_tx_hash: [u8; 32] = rand::random();

            let epoch = super::Epoch::new(0);

            let mut batch = super::GroveDbOpBatch::new();

            batch.push(epoch.init_proposers_tree_operation());

            batch.push(
                epoch
                    .increment_proposer_block_count_operation(
                        &drive,
                        &pro_tx_hash,
                        Some(0),
                        Some(&transaction),
                    )
                    .expect("should increment proposer block count operations"),
            );

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let stored_block_count = drive
                .get_epochs_proposer_block_count(&epoch, &pro_tx_hash, Some(&transaction))
                .expect("should get proposer block count");

            assert_eq!(stored_block_count, 1);
        }

        #[test]
        fn test_existing_block_count_is_incremented() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let pro_tx_hash: [u8; 32] = rand::random();

            let epoch = super::Epoch::new(1);

            let mut batch = super::GroveDbOpBatch::new();

            batch.push(epoch.init_proposers_tree_operation());

            // Apply proposers tree
            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let mut batch = super::GroveDbOpBatch::new();

            batch.push(epoch.update_proposer_block_count_operation(&pro_tx_hash, 1));

            // Apply proposer block count
            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let mut batch = super::GroveDbOpBatch::new();

            batch.push(
                epoch
                    .increment_proposer_block_count_operation(
                        &drive,
                        &pro_tx_hash,
                        None,
                        Some(&transaction),
                    )
                    .expect("should update proposer block count"),
            );

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let stored_block_count = drive
                .get_epochs_proposer_block_count(&epoch, &pro_tx_hash, Some(&transaction))
                .expect("should get proposer block count");

            assert_eq!(stored_block_count, 2);
        }
    }

    mod add_init_empty_operations {
        use crate::common::helpers::setup::setup_drive;
        use crate::error;

        #[test]
        fn test_error_if_fee_pools_not_initialized() {
            let drive = setup_drive(None);
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(1042);

            let mut batch = super::GroveDbOpBatch::new();

            epoch.add_init_empty_operations(&mut batch);

            match drive.grove_apply_batch(batch, false, Some(&transaction)) {
                Ok(_) => assert!(false, "should not be able to init epochs without FeePools"),
                Err(e) => match e {
                    error::Error::GroveDB(grovedb::Error::PathKeyNotFound(_)) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_values_are_set() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(1042);

            let mut batch = super::GroveDbOpBatch::new();

            epoch.add_init_empty_operations(&mut batch);

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let storage_fee = drive
                .get_epoch_storage_credits_for_distribution(&epoch, Some(&transaction))
                .expect("expected to get storage credits in epoch pool");

            assert_eq!(storage_fee, 0);
        }
    }

    mod add_init_current_operations {

        #[test]
        fn test_values_are_set() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(1042);

            let multiplier = 42.0;
            let start_time = 1;
            let start_block_height = 2;

            let mut batch = super::GroveDbOpBatch::new();

            epoch.add_init_empty_operations(&mut batch);

            epoch.add_init_current_operations(
                multiplier,
                start_block_height,
                start_time,
                &mut batch,
            );

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let stored_multiplier = drive
                .get_epoch_fee_multiplier(&epoch, Some(&transaction))
                .expect("should get multiplier");

            assert_eq!(stored_multiplier, multiplier);

            let stored_start_time = drive
                .get_epoch_start_time(&epoch, Some(&transaction))
                .expect("should get start time");

            assert_eq!(stored_start_time, start_time);

            let stored_block_height = drive
                .get_epoch_start_block_height(&epoch, Some(&transaction))
                .expect("should get start block height");

            assert_eq!(stored_block_height, start_block_height);

            drive
                .get_epoch_processing_credits_for_distribution(&epoch, Some(&transaction))
                .expect_err("should not get processing fee");

            let proposers = drive
                .get_epoch_proposers(&epoch, 1, Some(&transaction))
                .expect("should get proposers");

            assert_eq!(proposers, vec!());
        }
    }

    mod add_mark_as_paid_operations {
        use crate::error;
        use crate::fee_pools::epochs::epoch_key_constants;

        #[test]
        fn test_values_are_deleted() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(0);

            let mut batch = super::GroveDbOpBatch::new();

            epoch.add_init_current_operations(1.0, 2, 3, &mut batch);

            // Apply init current
            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let mut batch = super::GroveDbOpBatch::new();

            epoch.add_mark_as_paid_operations(&mut batch);

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            match drive
                .grove
                .get(
                    epoch.get_path(),
                    epoch_key_constants::KEY_PROPOSERS.as_slice(),
                    Some(&transaction),
                )
                .unwrap()
            {
                Ok(_) => assert!(false, "should not be able to get proposers"),
                Err(e) => match e {
                    grovedb::Error::PathKeyNotFound(_) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }

            match drive.get_epoch_processing_credits_for_distribution(&epoch, Some(&transaction)) {
                Ok(_) => assert!(false, "should not be able to get processing fee"),
                Err(e) => match e {
                    error::Error::GroveDB(grovedb::Error::PathKeyNotFound(_)) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }

            match drive.get_epoch_storage_credits_for_distribution(&epoch, Some(&transaction)) {
                Ok(_) => assert!(false, "should not be able to get storage fee"),
                Err(e) => match e {
                    error::Error::GroveDB(grovedb::Error::PathKeyNotFound(_)) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }
    }

    mod update_proposer_block_count {
        #[test]
        fn test_value_is_set() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let pro_tx_hash: [u8; 32] = rand::random();
            let block_count = 42;

            let epoch = super::Epoch::new(0);

            let mut batch = super::GroveDbOpBatch::new();

            batch.push(epoch.init_proposers_tree_operation());

            batch.push(epoch.update_proposer_block_count_operation(&pro_tx_hash, block_count));

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let stored_block_count = drive
                .get_epochs_proposer_block_count(&epoch, &pro_tx_hash, Some(&transaction))
                .expect("should get proposer block count");

            assert_eq!(stored_block_count, block_count);
        }
    }

    #[test]
    fn test_update_start_time() {
        let drive = setup_drive_with_initial_state_structure();
        let transaction = drive.grove.start_transaction();

        let epoch_tree = super::Epoch::new(0);

        let start_time_ms: u64 = Utc::now().timestamp_millis() as u64;

        let mut batch = GroveDbOpBatch::new();

        batch.push(epoch_tree.update_start_time_operation(start_time_ms));

        drive
            .grove_apply_batch(batch, false, Some(&transaction))
            .expect("should apply batch");

        let actual_start_time_ms = drive
            .get_epoch_start_time(&epoch_tree, Some(&transaction))
            .expect("should get start time");

        assert_eq!(start_time_ms, actual_start_time_ms);
    }

    #[test]
    fn test_update_epoch_start_block_height() {
        let drive = setup_drive_with_initial_state_structure();
        let transaction = drive.grove.start_transaction();

        let epoch = Epoch::new(0);

        let start_block_height = 1;

        let op = epoch.update_start_block_height_operation(start_block_height);

        drive
            .grove_apply_operation(op, false, Some(&transaction))
            .expect("should apply batch");

        let actual_start_block_height = drive
            .get_epoch_start_block_height(&epoch, Some(&transaction))
            .expect("should get start block height");

        assert_eq!(start_block_height, actual_start_block_height);
    }

    mod update_epoch_processing_credits_for_distribution {
        use crate::error;

        #[test]
        fn test_error_if_epoch_tree_is_not_initiated() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(7000);

            let op = epoch.update_processing_credits_for_distribution_operation(42);

            match drive.grove_apply_operation(op, false, Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to update processing fee on uninit epochs pool"
                ),
                Err(e) => match e {
                    error::Error::GroveDB(grovedb::Error::PathKeyNotFound(_)) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_value_is_set() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(0);

            let processing_fee: u64 = 42;

            let op = epoch.update_processing_credits_for_distribution_operation(42);

            drive
                .grove_apply_operation(op, false, Some(&transaction))
                .expect("should apply batch");

            let stored_processing_fee = drive
                .get_epoch_processing_credits_for_distribution(&epoch, Some(&transaction))
                .expect("should get processing fee");

            assert_eq!(stored_processing_fee, processing_fee);
        }
    }

    mod update_epoch_storage_credits_for_distribution {
        use crate::error;

        #[test]
        fn test_error_if_epoch_tree_is_not_initiated() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(7000);

            let op = epoch.update_storage_credits_for_distribution_operation(42);

            match drive.grove_apply_operation(op, false, Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to update storage fee on uninit epochs pool"
                ),
                Err(e) => match e {
                    error::Error::GroveDB(grovedb::Error::PathKeyNotFound(_)) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_value_is_set() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(0);

            let storage_fee = 42;

            let op = epoch.update_storage_credits_for_distribution_operation(storage_fee);

            drive
                .grove_apply_operation(op, false, Some(&transaction))
                .expect("should apply batch");

            let stored_storage_fee = drive
                .get_epoch_storage_credits_for_distribution(&epoch, Some(&transaction))
                .expect("should get storage fee");

            assert_eq!(stored_storage_fee, storage_fee);
        }
    }

    mod delete_proposers_tree {
        use crate::fee_pools::epochs::epoch_key_constants::KEY_PROPOSERS;

        #[test]
        fn test_values_has_been_deleted() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(0);

            let mut batch = super::GroveDbOpBatch::new();

            batch.push(epoch.init_proposers_tree_operation());

            // Apply proposers tree
            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let mut batch = super::GroveDbOpBatch::new();

            batch.push(epoch.delete_proposers_tree_operation());

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            match drive
                .grove
                .get(
                    epoch.get_path(),
                    KEY_PROPOSERS.as_slice(),
                    Some(&transaction),
                )
                .unwrap()
            {
                Ok(_) => assert!(false, "expect tree not exists"),
                Err(e) => match e {
                    grovedb::Error::PathKeyNotFound(_) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }
    }

    mod delete_proposers {
        #[test]
        fn test_values_are_being_deleted() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(0);

            let mut batch = super::GroveDbOpBatch::new();

            batch.push(epoch.init_proposers_tree_operation());

            // Apply proposers tree
            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let pro_tx_hashes: Vec<[u8; 32]> = (0..10).map(|_| rand::random()).collect();

            let mut batch = super::GroveDbOpBatch::new();

            for pro_tx_hash in pro_tx_hashes.iter() {
                batch.push(epoch.update_proposer_block_count_operation(pro_tx_hash, 1));
            }

            // Apply proposers block count updates
            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let mut stored_proposers = drive
                .get_epoch_proposers(&epoch, 20, Some(&transaction))
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
                awaited_result.get(0).unwrap().0.clone(),
                awaited_result.get(1).unwrap().0.clone(),
            ];

            // remove items we deleted
            awaited_result.remove(0);
            awaited_result.remove(1);

            let mut batch = super::GroveDbOpBatch::new();

            epoch.add_delete_proposers_operations(deleted_pro_tx_hashes, &mut batch);

            // Apply proposers deletion
            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let stored_proposers = drive
                .get_epoch_proposers(&epoch, 20, Some(&transaction))
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
