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

//! Credit Distribution.
//!
//! This module implements functions in Drive to distribute fees for a given Epoch.
//!

use grovedb::{Element, TransactionArg};

use crate::drive::Drive;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;

use crate::fee_pools::epochs::epoch_key_constants;

impl Drive {
    /// Gets the amount of storage credits to be distributed for the Epoch.
    pub fn get_epoch_storage_credits_for_distribution(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        let element = self
            .grove
            .get(
                epoch_tree.get_path(),
                epoch_key_constants::KEY_POOL_STORAGE_FEES.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if let Element::Item(item, _) = element {
            Ok(u64::from_be_bytes(item.as_slice().try_into().map_err(
                |_| {
                    Error::Fee(FeeError::CorruptedStorageFeeInvalidItemLength(
                        "epochs storage fee is not u64",
                    ))
                },
            )?))
        } else {
            Err(Error::Fee(FeeError::CorruptedStorageFeeNotItem(
                "epochs storage fee must be an item",
            )))
        }
    }

    /// Gets the amount of processing fees to be distributed for the Epoch.
    pub fn get_epoch_processing_credits_for_distribution(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        let element = self
            .grove
            .get(
                epoch_tree.get_path(),
                epoch_key_constants::KEY_POOL_PROCESSING_FEES.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if let Element::Item(item, _) = element {
            Ok(u64::from_be_bytes(item.as_slice().try_into().map_err(
                |_| {
                    Error::Fee(FeeError::CorruptedProcessingFeeInvalidItemLength(
                        "epochs processing fee is not u64",
                    ))
                },
            )?))
        } else {
            Err(Error::Fee(FeeError::CorruptedProcessingFeeNotItem(
                "epochs processing fee must be an item",
            )))
        }
    }

    /// Gets the Fee Multiplier for the Epoch.
    pub(crate) fn get_epoch_fee_multiplier(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
    ) -> Result<f64, Error> {
        let element = self
            .grove
            .get(
                epoch_tree.get_path(),
                epoch_key_constants::KEY_FEE_MULTIPLIER.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if let Element::Item(item, _) = element {
            Ok(f64::from_be_bytes(item.as_slice().try_into().map_err(
                |_| {
                    Error::Fee(FeeError::CorruptedMultiplierInvalidItemLength(
                        "epochs multiplier item have an invalid length",
                    ))
                },
            )?))
        } else {
            Err(Error::Fee(FeeError::CorruptedMultiplierNotItem(
                "epochs multiplier must be an item",
            )))
        }
    }

    /// Gets the total credits to be distributed for the Epoch.
    pub fn get_epoch_total_credits_for_distribution(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        let storage_pool_credits =
            self.get_epoch_storage_credits_for_distribution(epoch_tree, transaction)?;

        let processing_pool_credits =
            self.get_epoch_processing_credits_for_distribution(epoch_tree, transaction)?;

        storage_pool_credits
            .checked_add(processing_pool_credits)
            .ok_or(Error::Fee(FeeError::Overflow(
                "overflow getting total credits for distribution",
            )))
    }
}

#[cfg(test)]
mod tests {
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;
    use crate::drive::batch::GroveDbOpBatch;
    use crate::error;
    use crate::error::fee::FeeError;
    use crate::fee_pools::epochs::epoch_key_constants;
    use crate::fee_pools::epochs::Epoch;
    use grovedb::Element;

    mod get_epoch_storage_credits_for_distribution {
        use crate::fee_pools::epochs_root_tree_key_constants::KEY_STORAGE_FEE_POOL;

        #[test]
        fn test_error_if_epoch_tree_is_not_initiated() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(7000);

            match drive.get_epoch_storage_credits_for_distribution(&epoch, Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to get storage fee on uninit epochs pool"
                ),
                Err(e) => match e {
                    super::error::Error::GroveDB(grovedb::Error::PathNotFound(_)) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_value_has_invalid_length() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(0);

            drive
                .grove
                .insert(
                    epoch.get_path(),
                    KEY_STORAGE_FEE_POOL.as_slice(),
                    super::Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            match drive.get_epoch_storage_credits_for_distribution(&epoch, Some(&transaction)) {
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

    mod get_epoch_processing_credits_for_distribution {
        #[test]
        fn test_error_if_value_has_invalid_length() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(0);

            drive
                .grove
                .insert(
                    epoch.get_path(),
                    super::epoch_key_constants::KEY_POOL_PROCESSING_FEES.as_slice(),
                    super::Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            match drive.get_epoch_processing_credits_for_distribution(&epoch, Some(&transaction)) {
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
    fn test_get_epoch_total_credits_for_distribution() {
        let drive = setup_drive_with_initial_state_structure();
        let transaction = drive.grove.start_transaction();

        let processing_fee: u64 = 42;
        let storage_fee: u64 = 1000;

        let epoch = Epoch::new(0);

        let mut batch = GroveDbOpBatch::new();

        batch.push(epoch.update_processing_credits_for_distribution_operation(processing_fee));

        batch.push(epoch.update_storage_credits_for_distribution_operation(storage_fee));

        drive
            .grove_apply_batch(batch, false, Some(&transaction))
            .expect("should apply batch");

        let retrieved_combined_fee = drive
            .get_epoch_total_credits_for_distribution(&epoch, Some(&transaction))
            .expect("should get combined fee");

        assert_eq!(retrieved_combined_fee, processing_fee + storage_fee);
    }

    mod fee_multiplier {
        #[test]
        fn test_error_if_epoch_tree_is_not_initiated() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(7000);

            match drive.get_epoch_fee_multiplier(&epoch, Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to get multiplier on uninit epochs pool"
                ),
                Err(e) => match e {
                    super::error::Error::GroveDB(grovedb::Error::PathNotFound(_)) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_value_has_invalid_length() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(0);

            drive
                .grove
                .insert(
                    epoch.get_path(),
                    super::epoch_key_constants::KEY_FEE_MULTIPLIER.as_slice(),
                    super::Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            match drive.get_epoch_fee_multiplier(&epoch, Some(&transaction)) {
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
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(0);

            let multiplier = 42.0;

            let mut batch = super::GroveDbOpBatch::new();

            epoch.add_init_empty_operations(&mut batch);

            epoch.add_init_current_operations(multiplier, 1, 1, &mut batch);

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let stored_multiplier = drive
                .get_epoch_fee_multiplier(&epoch, Some(&transaction))
                .expect("should get multiplier");

            assert_eq!(stored_multiplier, multiplier);
        }
    }
}
