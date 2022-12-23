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

//! Epoch Start Times.
//!
//! This module implements functions in Drive relevant to Epoch start times.
//!

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;
use grovedb::{Element, TransactionArg};

use crate::fee_pools::epochs::epoch_key_constants::KEY_START_TIME;

impl Drive {
    /// Returns the start time of the given Epoch.
    pub fn get_epoch_start_time(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        let element = self
            .grove
            .get(
                epoch_tree.get_path(),
                KEY_START_TIME.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        let Element::Item(encoded_start_time, _) = element else {
            return Err(Error::Drive(DriveError::UnexpectedElementType("start time must be an item")))
        };

        let start_time =
            u64::from_be_bytes(encoded_start_time.as_slice().try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedSerialization("start time must be u64"))
            })?);

        Ok(start_time)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

    use super::*;

    mod get_epoch_start_time {
        use super::*;

        #[test]
        fn test_error_if_epoch_tree_is_not_initiated() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let non_initiated_epoch_tree = Epoch::new(7000);

            let result = drive.get_epoch_start_time(&non_initiated_epoch_tree, Some(&transaction));

            assert!(matches!(
                result,
                Err(Error::GroveDB(grovedb::Error::PathParentLayerNotFound(_)))
            ));
        }

        #[test]
        fn test_error_if_value_is_not_set() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch_tree = Epoch::new(0);

            let result = drive.get_epoch_start_time(&epoch_tree, Some(&transaction));

            assert!(matches!(result, Err(Error::GroveDB(_))));
        }

        #[test]
        fn test_error_if_element_has_invalid_type() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = Epoch::new(0);

            drive
                .grove
                .insert(
                    epoch.get_path(),
                    KEY_START_TIME.as_slice(),
                    Element::empty_tree(),
                    None,
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            let result = drive.get_epoch_start_time(&epoch, Some(&transaction));

            assert!(matches!(
                result,
                Err(Error::Drive(DriveError::UnexpectedElementType(_)))
            ));
        }

        #[test]
        fn test_error_if_value_has_invalid_length() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch_tree = Epoch::new(0);

            drive
                .grove
                .insert(
                    epoch_tree.get_path(),
                    KEY_START_TIME.as_slice(),
                    Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                    None,
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            let result = drive.get_epoch_start_time(&epoch_tree, Some(&transaction));

            assert!(matches!(
                result,
                Err(Error::Drive(DriveError::CorruptedSerialization(_)))
            ))
        }
    }
}
