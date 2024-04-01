use crate::drive::credit_pools::paths::pools_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee_pools::epochs_root_tree_key_constants::KEY_UNPAID_EPOCH_INDEX;
use dpp::block::epoch::EpochIndex;

use grovedb::{Element, TransactionArg};

impl Drive {
    /// Returns the index of the unpaid Epoch.
    #[inline(always)]
    pub(super) fn get_unpaid_epoch_index_v0(
        &self,
        transaction: TransactionArg,
    ) -> Result<EpochIndex, Error> {
        let element = self
            .grove
            .get(&pools_path(), KEY_UNPAID_EPOCH_INDEX, transaction)
            .unwrap()
            .map_err(Error::GroveDB)?;

        let Element::Item(encoded_epoch_index, _) = element else {
            return Err(Error::Drive(DriveError::UnexpectedElementType(
                "must be an item",
            )));
        };

        let epoch_index =
            EpochIndex::from_be_bytes(encoded_epoch_index.as_slice().try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedSerialization(
                    "item has an invalid length".to_string(),
                ))
            })?);

        Ok(epoch_index)
    }
}

#[cfg(feature = "full")]
#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};

    mod get_unpaid_epoch_index {
        use super::*;

        #[test]
        fn test_error_if_fee_pools_tree_is_not_initiated() {
            let drive = setup_drive(None);
            let transaction = drive.grove.start_transaction();

            let result = drive.get_unpaid_epoch_index_v0(Some(&transaction));

            assert!(matches!(
                result,
                Err(Error::GroveDB(grovedb::Error::PathParentLayerNotFound(_)))
            ));
        }

        #[test]
        fn test_error_if_element_has_invalid_type() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            // We need to first delete the item, because you can not replace an item with a tree
            drive
                .grove
                .delete(
                    &pools_path(),
                    KEY_UNPAID_EPOCH_INDEX.as_slice(),
                    None,
                    Some(&transaction),
                )
                .unwrap()
                .expect("should delete old item");

            drive
                .grove
                .insert(
                    &pools_path(),
                    KEY_UNPAID_EPOCH_INDEX.as_slice(),
                    Element::empty_tree(),
                    None,
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            let result = drive.get_unpaid_epoch_index_v0(Some(&transaction));

            assert!(matches!(
                result,
                Err(Error::Drive(DriveError::UnexpectedElementType(_)))
            ));
        }

        #[test]
        fn test_error_if_value_has_invalid_length() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            drive
                .grove
                .insert(
                    &pools_path(),
                    KEY_UNPAID_EPOCH_INDEX.as_slice(),
                    Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                    None,
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            let result = drive.get_unpaid_epoch_index_v0(Some(&transaction));

            assert!(matches!(
                result,
                Err(Error::Drive(DriveError::CorruptedSerialization(_)))
            ));
        }
    }
}
