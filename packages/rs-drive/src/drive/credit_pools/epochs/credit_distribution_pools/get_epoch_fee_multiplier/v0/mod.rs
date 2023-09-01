use grovedb::{Element, TransactionArg};

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::epoch::Epoch;

use crate::fee_pools::epochs::epoch_key_constants;
use crate::fee_pools::epochs::paths::EpochProposers;

impl Drive {
    /// Gets the Fee Multiplier for the Epoch.
    pub(super) fn get_epoch_fee_multiplier_v0(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
    ) -> Result<f64, Error> {
        let element = self
            .grove
            .get(
                &epoch_tree.get_path(),
                epoch_key_constants::KEY_FEE_MULTIPLIER.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        let Element::Item(encoded_multiplier, _) = element else {
            return Err(Error::Drive(DriveError::UnexpectedElementType(
                "epochs multiplier must be an item",
            )));
        };

        Ok(f64::from_be_bytes(
            encoded_multiplier.as_slice().try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedSerialization(
                    "epochs multiplier must be f64",
                ))
            })?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::drive::batch::GroveDbOpBatch;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::fee_pools::epochs::epoch_key_constants;
    use crate::fee_pools::epochs::operations_factory::EpochOperations;
    use crate::fee_pools::epochs::paths::EpochProposers;
    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::epoch::Epoch;
    use dpp::version::drive_versions::DriveVersion;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    #[test]
    fn test_error_if_epoch_tree_is_not_initiated_v0() {
        let drive = setup_drive_with_initial_state_structure();
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let epoch = Epoch::new(7000).unwrap();

        let result = drive.get_epoch_fee_multiplier(&epoch, Some(&transaction), platform_version);

        assert!(matches!(
            result,
            Err(Error::GroveDB(grovedb::Error::PathParentLayerNotFound(_)))
        ));
    }

    #[test]
    fn test_error_if_value_has_invalid_length_v0() {
        let drive = setup_drive_with_initial_state_structure();
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let epoch = Epoch::new(0).unwrap();

        drive
            .grove
            .insert(
                &epoch.get_path(),
                epoch_key_constants::KEY_FEE_MULTIPLIER.as_slice(),
                Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                None,
                Some(&transaction),
            )
            .unwrap()
            .expect("should insert invalid data");

        let result = drive.get_epoch_fee_multiplier(&epoch, Some(&transaction), platform_version);

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedSerialization(_)))
        ));
    }

    #[test]
    fn test_value_is_set_v0() {
        let drive = setup_drive_with_initial_state_structure();
        let drive_version = DriveVersion::latest();
        let transaction = drive.grove.start_transaction();

        let epoch = Epoch::new(0).unwrap();

        let multiplier = 42.0;

        let mut batch = GroveDbOpBatch::new();

        epoch
            .add_init_empty_operations(&mut batch)
            .expect("should add empty epoch operations");

        epoch.add_init_current_operations(multiplier, 1, 1, 1, &mut batch);

        drive
            .grove_apply_batch(batch, false, Some(&transaction), &drive_version)
            .expect("should apply batch");

        let stored_multiplier = drive
            .get_epoch_fee_multiplier_v0(&epoch, Some(&transaction))
            .expect("should get multiplier");

        assert_eq!(stored_multiplier, multiplier);
    }
}
