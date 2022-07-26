use crate::drive::fee_pools::pools_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;
use grovedb::TransactionArg;

pub mod constants;
pub mod credit_distribution_pools;
pub mod proposers;
pub mod start_block;
pub mod start_time;

impl Drive {
    pub fn is_epoch_tree_exists(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
    ) -> Result<bool, Error> {
        self.grove
            .has_raw(pools_path(), &epoch_tree.key, transaction)
            .unwrap()
            .map_err(Error::GroveDB)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};

    use crate::drive::batch::GroveDbOpBatch;
    use crate::error;
    use crate::fee_pools::epochs::epoch_key_constants;
    use crate::fee_pools::epochs::Epoch;

    mod is_epoch_tree_exists {
        use crate::common::helpers::setup::setup_drive_with_initial_state_structure;
        use crate::drive::fee_pools::epochs::constants::{
            GENESIS_EPOCH_INDEX, PERPETUAL_STORAGE_EPOCHS,
        };
        use crate::fee_pools::epochs::Epoch;

        #[test]
        fn test_return_true_if_tree_exists() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch_tree = Epoch::new(GENESIS_EPOCH_INDEX);

            let is_exist = drive
                .is_epoch_tree_exists(&epoch_tree, Some(&transaction))
                .expect("should check epoch tree existence");

            assert!(is_exist);
        }

        #[test]
        fn test_return_false_if_tree_doesnt_exist() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch_tree = Epoch::new(PERPETUAL_STORAGE_EPOCHS + 1);

            let is_exist = drive
                .is_epoch_tree_exists(&epoch_tree, Some(&transaction))
                .expect("should check epoch tree existence");

            assert!(!is_exist);
        }
    }
}
