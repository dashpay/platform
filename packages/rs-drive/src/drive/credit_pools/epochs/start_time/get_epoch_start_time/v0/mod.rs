use crate::drive::credit_pools::epochs::epoch_key_constants::KEY_START_TIME;
use crate::drive::credit_pools::epochs::paths::EpochProposers;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use grovedb::{Element, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Returns the start time of the given Epoch.
    pub(super) fn get_epoch_start_time_v0(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        let element = self
            .grove
            .get(
                &epoch_tree.get_path(),
                KEY_START_TIME.as_slice(),
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        let Element::Item(encoded_start_time, _) = element else {
            return Err(Error::Drive(DriveError::UnexpectedElementType(
                "start time must be an item",
            )));
        };

        let start_time =
            u64::from_be_bytes(encoded_start_time.as_slice().try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedSerialization(String::from(
                    "start time must be u64",
                )))
            })?);

        Ok(start_time)
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    use super::*;

    mod get_epoch_start_time {
        use super::*;
        use dpp::version::PlatformVersion;

        #[test]
        fn test_error_if_epoch_tree_is_not_initiated() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::latest();

            let non_initiated_epoch_tree = Epoch::new(7000).unwrap();

            let result = drive.get_epoch_start_time(
                &non_initiated_epoch_tree,
                Some(&transaction),
                platform_version,
            );

            assert!(matches!(
                result,
                Err(Error::GroveDB(grovedb::Error::PathParentLayerNotFound(_)))
            ));
        }

        #[test]
        fn test_error_if_value_is_not_set() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::latest();

            let epoch_tree = Epoch::new(0).unwrap();

            let result =
                drive.get_epoch_start_time(&epoch_tree, Some(&transaction), platform_version);

            assert!(matches!(result, Err(Error::GroveDB(_))));
        }

        #[test]
        fn test_error_if_element_has_invalid_type() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::latest();

            let epoch = Epoch::new(0).unwrap();

            drive
                .grove
                .insert(
                    &epoch.get_path(),
                    KEY_START_TIME.as_slice(),
                    Element::empty_tree(),
                    None,
                    Some(&transaction),
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .expect("should insert invalid data");

            let result = drive.get_epoch_start_time(&epoch, Some(&transaction), platform_version);

            assert!(matches!(
                result,
                Err(Error::Drive(DriveError::UnexpectedElementType(_)))
            ));
        }

        #[test]
        fn test_error_if_value_has_invalid_length() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::latest();

            let epoch_tree = Epoch::new(0).unwrap();

            drive
                .grove
                .insert(
                    &epoch_tree.get_path(),
                    KEY_START_TIME.as_slice(),
                    Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                    None,
                    Some(&transaction),
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .expect("should insert invalid data");

            let result =
                drive.get_epoch_start_time(&epoch_tree, Some(&transaction), platform_version);

            assert!(matches!(
                result,
                Err(Error::Drive(DriveError::CorruptedSerialization(_)))
            ))
        }
    }
}
