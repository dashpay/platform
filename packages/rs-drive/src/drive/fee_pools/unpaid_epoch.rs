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
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::epoch::EpochIndex;
use crate::fee_pools::epochs_root_tree_key_constants::KEY_UNPAID_EPOCH_INDEX;
use grovedb::{Element, TransactionArg};

impl Drive {
    /// Returns the index of the unpaid Epoch.
    pub fn get_unpaid_epoch_index(&self, transaction: TransactionArg) -> Result<EpochIndex, Error> {
        let element = self
            .grove
            .get(pools_path(), KEY_UNPAID_EPOCH_INDEX, transaction)
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
                    "item have an invalid length",
                ))
            })?);

        Ok(epoch_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::common::helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};

    mod get_unpaid_epoch_index {
        use super::*;

        #[test]
        fn test_error_if_fee_pools_tree_is_not_initiated() {
            let drive = setup_drive(None);
            let transaction = drive.grove.start_transaction();

            let result = drive.get_unpaid_epoch_index(Some(&transaction));

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
                    pools_path(),
                    KEY_UNPAID_EPOCH_INDEX.as_slice(),
                    None,
                    Some(&transaction),
                )
                .unwrap()
                .expect("should delete old item");

            drive
                .grove
                .insert(
                    pools_path(),
                    KEY_UNPAID_EPOCH_INDEX.as_slice(),
                    Element::empty_tree(),
                    None,
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            let result = drive.get_unpaid_epoch_index(Some(&transaction));

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
                    pools_path(),
                    KEY_UNPAID_EPOCH_INDEX.as_slice(),
                    Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                    None,
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            let result = drive.get_unpaid_epoch_index(Some(&transaction));

            assert!(matches!(
                result,
                Err(Error::Drive(DriveError::CorruptedSerialization(_)))
            ));
        }
    }
}
