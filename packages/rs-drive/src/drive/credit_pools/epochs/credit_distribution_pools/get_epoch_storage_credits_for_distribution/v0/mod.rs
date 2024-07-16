use dpp::balances::credits::Creditable;
use grovedb::{Element, TransactionArg};

use crate::drive::credit_pools::epochs::epoch_key_constants;
use crate::drive::credit_pools::epochs::paths::EpochProposers;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Gets the amount of storage credits to be distributed for the Epoch.
    pub(super) fn get_epoch_storage_credits_for_distribution_v0(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        let element = self
            .grove
            .get(
                &epoch_tree.get_path(),
                epoch_key_constants::KEY_POOL_STORAGE_FEES.as_slice(),
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        let Element::SumItem(item, _) = element else {
            return Err(Error::Drive(DriveError::UnexpectedElementType(
                "epochs storage fee must be an item",
            )));
        };

        Ok(item.to_unsigned())
    }
}

#[cfg(test)]
mod tests {

    use crate::error::drive::DriveError;
    use crate::error::Error;

    use crate::drive::credit_pools::epochs::epochs_root_tree_key_constants::KEY_STORAGE_FEE_POOL;
    use crate::drive::credit_pools::epochs::paths::EpochProposers;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    #[test]
    fn test_error_if_epoch_tree_is_not_initiated_v0() {
        let drive = setup_drive_with_initial_state_structure();
        let transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let epoch = Epoch::new(7000).unwrap();

        let result = drive.get_epoch_storage_credits_for_distribution(
            &epoch,
            Some(&transaction),
            platform_version,
        );

        assert!(matches!(
            result,
            Err(Error::GroveDB(grovedb::Error::PathParentLayerNotFound(_)))
        ));
    }

    #[test]
    fn test_error_if_value_has_invalid_length_v0() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure();
        let transaction = drive.grove.start_transaction();

        let epoch = Epoch::new(0).unwrap();

        drive
            .grove
            .insert(
                &epoch.get_path(),
                KEY_STORAGE_FEE_POOL.as_slice(),
                Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                None,
                Some(&transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("should insert invalid data");

        let result = drive.get_epoch_storage_credits_for_distribution_v0(
            &epoch,
            Some(&transaction),
            platform_version,
        );

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::UnexpectedElementType(_)))
        ));
    }
}
