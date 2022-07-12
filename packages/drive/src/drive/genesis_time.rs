use crate::drive::object_size_info::PathKeyElementInfo;
use crate::drive::storage::batch::Batch;
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::{Element, TransactionArg};
use std::array::TryFromSliceError;

const KEY_GENESIS_TIME: &[u8; 1] = b"g";

impl Drive {
    pub fn get_genesis_time(&self, transaction: TransactionArg) -> Result<i64, Error> {
        let element = self
            .grove
            .get(
                [Into::<&[u8; 1]>::into(RootTree::SpentAssetLockTransactions).as_slice()],
                KEY_GENESIS_TIME.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if let Element::Item(item, _) = element {
            let genesis_time = i64::from_le_bytes(item.as_slice().try_into().map_err(
                |e: TryFromSliceError| {
                    Error::Drive(DriveError::CorruptedGenesisTimeInvalidItemLength(
                        e.to_string(),
                    ))
                },
            )?);

            Ok(genesis_time)
        } else {
            Err(Error::Drive(DriveError::CorruptedGenesisTimeNotItem()))
        }
    }

    pub fn add_update_genesis_time_operations(
        &self,
        batch: &mut Batch,
        genesis_time: i64,
    ) -> Result<(), Error> {
        batch.insert(PathKeyElementInfo::PathFixedSizeKeyElement((
            [Into::<&[u8; 1]>::into(RootTree::SpentAssetLockTransactions)],
            KEY_GENESIS_TIME.as_slice(),
            Element::Item(genesis_time.to_le_bytes().to_vec(), None),
        )))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::genesis_time::KEY_GENESIS_TIME;
    use crate::drive::RootTree;
    use crate::error;
    use crate::fee::pools::tests::helpers::setup::setup_drive;
    use grovedb::Element;

    mod get_genesis_time {

        #[test]
        fn test_error_if_fee_pools_is_not_initiated() {
            let drive = super::setup_drive();

            match drive.get_genesis_time(None) {
                Ok(_) => assert!(
                    false,
                    "should not be able to get genesis time on uninit fee pools"
                ),
                Err(e) => match e {
                    super::error::Error::GroveDB(grovedb::Error::PathNotFound(_)) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_value_has_invalid_length() {
            let drive = super::setup_drive();

            drive
                .create_initial_state_structure(None)
                .expect("expected to create root tree successfully");

            drive
                .grove
                .insert(
                    [
                        Into::<&[u8; 1]>::into(super::RootTree::SpentAssetLockTransactions)
                            .as_slice(),
                    ],
                    super::KEY_GENESIS_TIME.as_slice(),
                    super::Element::Item(u128::MAX.to_le_bytes().to_vec(), None),
                    None,
                )
                .unwrap()
                .expect("should insert invalid data");

            match drive.get_genesis_time(None) {
                Ok(_) => assert!(false, "should not be able to decode stored value"),
                Err(e) => match e {
                    super::error::Error::Drive(
                        super::error::drive::DriveError::CorruptedGenesisTimeInvalidItemLength(_),
                    ) => {
                        assert!(true)
                    }
                    _ => assert!(false, "ivalid error type"),
                },
            }
        }
    }

    mod update_genesis_time {
        use crate::drive::storage::batch::Batch;

        #[test]
        fn test_error_if_fee_pools_is_not_initiated() {
            let drive = super::setup_drive();

            let genesis_time: i64 = 1655396517902;

            let mut batch = Batch::new(&drive);

            drive
                .add_update_genesis_time_operations(&mut batch, genesis_time)
                .expect("should update genesis time");

            match drive.apply_batch(batch, false, None) {
                Ok(_) => assert!(
                    false,
                    "should not be able to update genesis time on uninit fee pools"
                ),
                Err(e) => match e {
                    super::error::Error::GroveDB(grovedb::Error::PathKeyNotFound(_)) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_value_is_set() {
            let drive = super::setup_drive();

            drive
                .create_initial_state_structure(None)
                .expect("expected to create root tree successfully");

            let genesis_time: i64 = 1655396517902;

            let mut batch = Batch::new(&drive);

            drive
                .add_update_genesis_time_operations(&mut batch, genesis_time)
                .expect("should update genesis time");

            drive
                .apply_batch(batch, false, None)
                .expect("should apply batch");

            let stored_genesis_time = drive
                .get_genesis_time(None)
                .expect("should get genesis time");

            assert_eq!(stored_genesis_time, genesis_time);
        }
    }
}
