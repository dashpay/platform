use grovedb::{Element, TransactionArg};
use rust_decimal::Decimal;

use crate::drive::object_size_info::PathKeyElementInfo;
use crate::drive::storage::batch::Batch;
use crate::{
    error::{fee::FeeError, Error},
    fee::pools::epoch::epoch_pool::EpochPool,
};

use super::constants;

impl<'e> EpochPool<'e> {
    pub fn get_storage_fee(&self, transaction: TransactionArg) -> Result<Decimal, Error> {
        let element = self
            .drive
            .grove
            .get(
                self.get_path(),
                constants::KEY_STORAGE_FEE.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if let Element::Item(item, _) = element {
            Ok(Decimal::deserialize(item.try_into().map_err(|_| {
                Error::Fee(FeeError::CorruptedStorageFeeInvalidItemLength(
                    "epoch storage fee item have an invalid length",
                ))
            })?))
        } else {
            Err(Error::Fee(FeeError::CorruptedStorageFeeNotItem(
                "epoch storage fee must be an item",
            )))
        }
    }

    pub fn get_processing_fee(&self, transaction: TransactionArg) -> Result<u64, Error> {
        let element = self
            .drive
            .grove
            .get(
                self.get_path(),
                constants::KEY_PROCESSING_FEE.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if let Element::Item(item, _) = element {
            Ok(u64::from_le_bytes(item.as_slice().try_into().map_err(
                |_| {
                    Error::Fee(FeeError::CorruptedProcessingFeeInvalidItemLength(
                        "epoch processing fee is not u64",
                    ))
                },
            )?))
        } else {
            Err(Error::Fee(FeeError::CorruptedProcessingFeeNotItem(
                "epoch processing fee must be an item",
            )))
        }
    }

    pub fn get_fee_multiplier(&self, transaction: TransactionArg) -> Result<u64, Error> {
        let element = self
            .drive
            .grove
            .get(
                self.get_path(),
                constants::KEY_FEE_MULTIPLIER.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if let Element::Item(item, _) = element {
            Ok(u64::from_le_bytes(item.as_slice().try_into().map_err(
                |_| {
                    Error::Fee(FeeError::CorruptedMultiplierInvalidItemLength(
                        "epoch multiplier item have an invalid length",
                    ))
                },
            )?))
        } else {
            Err(Error::Fee(FeeError::CorruptedMultiplierNotItem(
                "epoch multiplier must be an item",
            )))
        }
    }

    pub fn add_update_fee_multiplier_operations(
        &self,
        batch: &mut Batch,
        multiplier: u64,
    ) -> Result<(), Error> {
        batch.insert(PathKeyElementInfo::PathFixedSizeKeyElement((
            self.get_path(),
            constants::KEY_FEE_MULTIPLIER.as_slice(),
            Element::Item(multiplier.to_le_bytes().to_vec(), None),
        )))
    }

    pub fn add_update_processing_fee_operations(
        &self,
        batch: &mut Batch,
        processing_fee: u64,
    ) -> Result<(), Error> {
        batch.insert(PathKeyElementInfo::PathFixedSizeKeyElement((
            self.get_path(),
            constants::KEY_PROCESSING_FEE.as_slice(),
            Element::Item(processing_fee.to_le_bytes().to_vec(), None),
        )))
    }

    pub fn add_delete_processing_fee_operations(
        &self,
        batch: &mut Batch,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        batch.delete(
            self.get_path(),
            constants::KEY_PROCESSING_FEE.as_slice(),
            false,
            transaction,
        )
    }

    pub fn add_update_storage_fee_operations(
        &self,
        batch: &mut Batch,
        storage_fee: Decimal,
    ) -> Result<(), Error> {
        batch.insert(PathKeyElementInfo::PathFixedSizeKeyElement((
            self.get_path(),
            constants::KEY_STORAGE_FEE.as_slice(),
            Element::Item(storage_fee.serialize().to_vec(), None),
        )))?;

        Ok(())
    }

    pub fn add_delete_storage_fee_operations(
        &self,
        batch: &mut Batch,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        batch.delete(
            self.get_path(),
            constants::KEY_STORAGE_FEE.as_slice(),
            false,
            transaction,
        )
    }

    pub fn get_total_fees(&self, transaction: TransactionArg) -> Result<Decimal, Error> {
        let storage_fee = self.get_storage_fee(transaction)?;

        let processing_fee = self.get_processing_fee(transaction)?;

        let processing_fee = Decimal::from(processing_fee);

        Ok(storage_fee + processing_fee)
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::storage::batch::Batch;
    use grovedb::Element;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    use crate::{
        error::{self, fee::FeeError},
        fee::pools::{
            epoch::{constants, epoch_pool::EpochPool},
            tests::helpers::setup::{setup_drive, setup_fee_pools},
        },
    };

    mod update_storage_fee {
        #[test]
        fn test_error_if_epoch_pool_is_not_initiated() {
            let drive = super::setup_drive();
            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch = super::EpochPool::new(7000, &drive);

            let mut batch = super::Batch::new(&drive);

            epoch
                .add_update_storage_fee_operations(&mut batch, super::dec!(42.0))
                .expect("should update storage fee");

            match drive.apply_batch(batch, false, Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to update storage fee on uninit epoch pool"
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
            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch = super::EpochPool::new(0, &drive);

            let storage_fee = super::dec!(42.0);

            let mut batch = super::Batch::new(&drive);

            epoch
                .add_update_storage_fee_operations(&mut batch, storage_fee)
                .expect("should update storage fee");

            drive
                .apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let stored_storage_fee = epoch
                .get_storage_fee(Some(&transaction))
                .expect("should get storage fee");

            assert_eq!(stored_storage_fee, storage_fee);
        }
    }

    mod get_storage_fee {
        #[test]
        fn test_error_if_epoch_pool_is_not_initiated() {
            let drive = super::setup_drive();
            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch = super::EpochPool::new(7000, &drive);

            match epoch.get_storage_fee(Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to get storage fee on uninit epoch pool"
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
            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch = super::EpochPool::new(0, &drive);

            drive
                .grove
                .insert(
                    epoch.get_path(),
                    super::constants::KEY_STORAGE_FEE.as_slice(),
                    super::Element::Item(f64::MAX.to_le_bytes().to_vec(), None),
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            match epoch.get_storage_fee(Some(&transaction)) {
                Ok(_) => assert!(false, "should not be able to decode stored value"),
                Err(e) => match e {
                    super::error::Error::Fee(
                        super::FeeError::CorruptedStorageFeeInvalidItemLength(_),
                    ) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }
    }

    mod update_processing_fee {
        #[test]
        fn test_error_if_epoch_pool_is_not_initiated() {
            let drive = super::setup_drive();
            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch = super::EpochPool::new(7000, &drive);

            let mut batch = super::Batch::new(&drive);

            epoch
                .add_update_processing_fee_operations(&mut batch, 42)
                .expect("should update processing fee");

            match drive.apply_batch(batch, false, Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to update processing fee on uninit epoch pool"
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
            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch = super::EpochPool::new(0, &drive);

            let processing_fee: u64 = 42;

            let mut batch = super::Batch::new(&drive);

            epoch
                .add_update_processing_fee_operations(&mut batch, processing_fee)
                .expect("should update processing fee");

            drive
                .apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let stored_processing_fee = epoch
                .get_processing_fee(Some(&transaction))
                .expect("should get processing fee");

            assert_eq!(stored_processing_fee, processing_fee);
        }
    }

    mod get_processing_fee {
        #[test]
        fn test_error_if_value_has_invalid_length() {
            let drive = super::setup_drive();
            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch = super::EpochPool::new(0, &drive);

            drive
                .grove
                .insert(
                    epoch.get_path(),
                    super::constants::KEY_PROCESSING_FEE.as_slice(),
                    super::Element::Item(u128::MAX.to_le_bytes().to_vec(), None),
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            match epoch.get_processing_fee(Some(&transaction)) {
                Ok(_) => assert!(false, "should not be able to decode stored value"),
                Err(e) => match e {
                    super::error::Error::Fee(
                        super::FeeError::CorruptedProcessingFeeInvalidItemLength(_),
                    ) => {
                        assert!(true)
                    }
                    _ => assert!(false, "ivalid error type"),
                },
            }
        }
    }

    #[test]
    fn test_get_total_fees() {
        let drive = setup_drive();
        let (transaction, _) = setup_fee_pools(&drive, None);

        let processing_fee: u64 = 42;
        let storage_fee = dec!(1000);

        let epoch = EpochPool::new(0, &drive);

        let mut batch = super::Batch::new(&drive);

        epoch
            .add_update_processing_fee_operations(&mut batch, processing_fee)
            .expect("should update processing fee");

        epoch
            .add_update_storage_fee_operations(&mut batch, storage_fee)
            .expect("should update storage fee");

        drive
            .apply_batch(batch, false, Some(&transaction))
            .expect("should apply batch");

        let combined_fee = epoch
            .get_total_fees(Some(&transaction))
            .expect("should get combined fee");

        let processing_fee = Decimal::from(processing_fee);

        assert_eq!(combined_fee, processing_fee + storage_fee);
    }

    mod fee_multiplier {
        #[test]
        fn test_error_if_epoch_pool_is_not_initiated() {
            let drive = super::setup_drive();
            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch = super::EpochPool::new(7000, &drive);

            match epoch.get_fee_multiplier(Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to get multiplier on uninit epoch pool"
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
            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch = super::EpochPool::new(0, &drive);

            drive
                .grove
                .insert(
                    epoch.get_path(),
                    super::constants::KEY_FEE_MULTIPLIER.as_slice(),
                    super::Element::Item(u128::MAX.to_le_bytes().to_vec(), None),
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            match epoch.get_fee_multiplier(Some(&transaction)) {
                Ok(_) => assert!(false, "should not be able to decode stored value"),
                Err(e) => match e {
                    super::error::Error::Fee(
                        super::FeeError::CorruptedMultiplierInvalidItemLength(_),
                    ) => {
                        assert!(true)
                    }
                    _ => assert!(false, "ivalid error type"),
                },
            }
        }

        #[test]
        fn test_value_is_set() {
            let drive = super::setup_drive();
            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch = super::EpochPool::new(0, &drive);

            let multiplier = 42;

            let mut batch = super::Batch::new(&drive);

            epoch
                .add_init_empty_operations(&mut batch)
                .expect("should init empty pool");

            epoch
                .add_init_current_operations(&mut batch, multiplier, 1, 1)
                .expect("should init current");

            drive
                .apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let stored_multiplier = epoch
                .get_fee_multiplier(Some(&transaction))
                .expect("should get multiplier");

            assert_eq!(stored_multiplier, multiplier);
        }
    }

    mod overflow {
        use std::str::FromStr;

        #[test]
        fn test_u64_fee_conversion() {
            let processing_fee = u64::MAX;

            let decimal = super::Decimal::from_str(processing_fee.to_string().as_str())
                .expect("should convert u64::MAX to Decimal");

            let converted_to_u64: u64 = decimal
                .try_into()
                .expect("should convert Decimal back to u64::MAX");

            assert_eq!(processing_fee, converted_to_u64);
        }
    }
}
