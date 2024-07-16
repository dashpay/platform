use crate::drive::credit_pools::pools_path;
use crate::drive::Drive;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Checks if an Epoch tree exists. Returns a bool.
    /// Does not need to be versioned as it is very simple
    pub fn has_epoch_tree_exists(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        self.grove
            .has_raw(
                &pools_path(),
                &epoch_tree.key,
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::GroveDB)
    }
}

#[cfg(test)]
mod tests {

    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    mod has_epoch_tree_exists {
        use super::*;
        use dpp::block::epoch::Epoch;

        use dpp::fee::epoch::GENESIS_EPOCH_INDEX;
        use platform_version::version::PlatformVersion;

        #[test]
        fn test_return_true_if_tree_exists() {
            let platform_version = PlatformVersion::latest();
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch_tree = Epoch::new(GENESIS_EPOCH_INDEX).unwrap();

            let is_exist = drive
                .has_epoch_tree_exists(&epoch_tree, Some(&transaction), platform_version)
                .expect("should check epoch tree existence");

            assert!(is_exist);
        }

        #[test]
        fn test_return_false_if_tree_doesnt_exist() {
            let platform_version = PlatformVersion::latest();
            // default will be 40 epochs per era
            // 50 eras
            // = 2000
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch_tree = Epoch::new(2000 + 1).unwrap();

            let is_exist = drive
                .has_epoch_tree_exists(&epoch_tree, Some(&transaction), platform_version)
                .expect("should check epoch tree existence");

            assert!(!is_exist);
        }
    }
}
