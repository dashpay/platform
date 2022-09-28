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

//! Unpaid Epoch.
//!

use crate::drive::fee_pools::pools_path;
use crate::drive::Drive;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee_pools::epochs_root_tree_key_constants::KEY_UNPAID_EPOCH_INDEX;
use grovedb::{Element, TransactionArg};

impl Drive {
    /// Returns the index of the unpaid Epoch.
    pub fn get_unpaid_epoch_index(&self, transaction: TransactionArg) -> Result<u16, Error> {
        let element = self
            .grove
            .get(pools_path(), KEY_UNPAID_EPOCH_INDEX, transaction)
            .unwrap()
            .map_err(Error::GroveDB)?;

        if let Element::Item(item, _) = element {
            Ok(u16::from_be_bytes(item.as_slice().try_into().map_err(
                |_| {
                    Error::Fee(FeeError::CorruptedUnpaidEpochIndexItemLength(
                        "item have an invalid length",
                    ))
                },
            )?))
        } else {
            Err(Error::Fee(FeeError::CorruptedUnpaidEpochIndexNotItem(
                "must be an item",
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    mod get_unpaid_epoch_index {
        use crate::common::helpers::setup::{
            setup_drive, setup_drive_with_initial_state_structure,
        };
        use crate::drive::fee_pools::pools_path;
        use crate::error;
        use crate::error::fee::FeeError;
        use crate::fee_pools::epochs_root_tree_key_constants::KEY_UNPAID_EPOCH_INDEX;
        use grovedb::Element;

        #[test]
        fn test_error_if_fee_pools_tree_is_not_initiated() {
            let drive = setup_drive(None);
            let transaction = drive.grove.start_transaction();

            match drive.get_unpaid_epoch_index(Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to get unpaid epoch if fee pools tree is not initialized"
                ),
                Err(e) => match e {
                    error::Error::GroveDB(grovedb::Error::PathNotFound(_)) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_element_has_invalid_type() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            drive
                .grove
                .insert(
                    pools_path(),
                    KEY_UNPAID_EPOCH_INDEX.as_slice(),
                    Element::empty_tree(),
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            match drive.get_unpaid_epoch_index(Some(&transaction)) {
                Ok(_) => assert!(false, "must be an error"),
                Err(e) => match e {
                    error::Error::Fee(FeeError::CorruptedUnpaidEpochIndexNotItem(_)) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_value_has_invalid_length() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            drive
                .grove
                .insert(
                    pools_path(),
                    KEY_UNPAID_EPOCH_INDEX.as_slice(),
                    Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            match drive.get_unpaid_epoch_index(Some(&transaction)) {
                Ok(_) => assert!(false, "must be an error"),
                Err(e) => match e {
                    error::Error::Fee(FeeError::CorruptedUnpaidEpochIndexItemLength(_)) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }
    }
}
