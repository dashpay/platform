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

//! Epoch Start Blocks
//!
//! This modules implements functions in Drive relevant to Epoch start blocks.
//!

use crate::drive::fee_pools::pools_vec_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::epoch::EpochIndex;
use crate::fee_pools::epochs::{paths, Epoch};
use grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};

use crate::fee_pools::epochs::epoch_key_constants::KEY_START_BLOCK_HEIGHT;

impl Drive {
    /// Returns the block height of the Epoch's start block
    pub fn get_epoch_start_block_height(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        let element = self
            .grove
            .get(
                epoch_tree.get_path(),
                KEY_START_BLOCK_HEIGHT.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        let Element::Item(encoded_start_block_height, _) = element else {
            return Err(Error::Drive(DriveError::UnexpectedElementType("start block height must be an item")));
        };

        let start_block_height = u64::from_be_bytes(
            encoded_start_block_height
                .as_slice()
                .try_into()
                .map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "start block height must be u64",
                    ))
                })?,
        );

        Ok(start_block_height)
    }

    /// Returns the index and start block height of the first epoch between the two given.
    pub fn get_first_epoch_start_block_height_between_epochs(
        &self,
        from_epoch_index: EpochIndex,
        to_epoch_index: EpochIndex,
        transaction: TransactionArg,
    ) -> Result<Option<(EpochIndex, u64)>, Error> {
        let mut query = Query::new();

        let from_epoch_key = paths::encode_epoch_index_key(from_epoch_index)?.to_vec();
        let current_epoch_key = paths::encode_epoch_index_key(to_epoch_index)?.to_vec();

        query.insert_range_after_to_inclusive(from_epoch_key..=current_epoch_key);

        query.set_subquery_key(KEY_START_BLOCK_HEIGHT.to_vec());

        let sized_query = SizedQuery::new(query, Some(1), None);

        let path_query = PathQuery::new(pools_vec_path(), sized_query);

        let (result_items, _) = self
            .grove
            .query_raw(&path_query, QueryPathKeyElementTrioResultType, transaction)
            .unwrap()
            .map_err(Error::GroveDB)?;

        if result_items.len() == 0 {
            return Ok(None);
        }

        let (path, _, element) = result_items.to_path_key_elements().remove(0);

        let Element::Item(item, _) = element else {
            return Err(Error::Drive(DriveError::UnexpectedElementType("start block must be an item")));
        };

        let next_start_block_height =
            u64::from_be_bytes(item.as_slice().try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedSerialization(
                    "start block height must be u64",
                ))
            })?);

        let epoch_key = path
            .last()
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "epoch pool shouldn't have empty path",
            )))?;

        let epoch_index = paths::decode_epoch_index_key(epoch_key.as_slice())?;

        Ok(Some((epoch_index, next_start_block_height)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

    mod get_epoch_start_block_height {
        use super::*;

        #[test]
        fn test_error_if_epoch_tree_is_not_initiated() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let non_initiated_epoch = Epoch::new(7000);

            let result =
                drive.get_epoch_start_block_height(&non_initiated_epoch, Some(&transaction));

            assert!(matches!(
                result,
                Err(Error::GroveDB(grovedb::Error::PathParentLayerNotFound(_)))
            ));
        }

        #[test]
        fn test_error_if_value_is_not_set() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = Epoch::new(0);

            let result = drive.get_epoch_start_block_height(&epoch, Some(&transaction));

            assert!(matches!(result, Err(Error::GroveDB(_))));
        }

        #[test]
        fn test_error_if_value_has_invalid_length() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = Epoch::new(0);

            drive
                .grove
                .insert(
                    epoch.get_path(),
                    KEY_START_BLOCK_HEIGHT.as_slice(),
                    Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                    None,
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            let result = drive.get_epoch_start_block_height(&epoch, Some(&transaction));

            assert!(matches!(
                result,
                Err(Error::Drive(DriveError::CorruptedSerialization(_)))
            ));
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
                    KEY_START_BLOCK_HEIGHT.as_slice(),
                    Element::empty_tree(),
                    None,
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            let result = drive.get_epoch_start_block_height(&epoch, Some(&transaction));

            assert!(matches!(
                result,
                Err(Error::Drive(DriveError::UnexpectedElementType(_)))
            ));
        }
    }

    mod get_first_epoch_start_block_height_between_epochs {
        use super::*;
        use crate::drive::batch::GroveDbOpBatch;

        #[test]
        fn test_next_block_height() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch_tree_0 = Epoch::new(0);
            let epoch_tree_1 = Epoch::new(1);

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_tree_0.update_start_block_height_operation(1));
            batch.push(epoch_tree_1.update_start_block_height_operation(2));

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let next_epoch_start_block_height_option = drive
                .get_first_epoch_start_block_height_between_epochs(0, 2, Some(&transaction))
                .expect("should find next start_block_height");

            assert!(matches!(next_epoch_start_block_height_option, Some((1, 2))));
        }

        #[test]
        fn test_none_if_there_are_no_start_block_heights() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let next_epoch_start_block_height = drive
                .get_first_epoch_start_block_height_between_epochs(0, 4, Some(&transaction))
                .expect("should find next start_block_height");

            assert!(next_epoch_start_block_height.is_none());
        }

        #[test]
        fn test_none_if_start_block_height_is_outside_of_specified_epoch_range() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch_tree_0 = Epoch::new(0);
            let epoch_tree_3 = Epoch::new(3);

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_tree_0.update_start_block_height_operation(1));
            batch.push(epoch_tree_3.update_start_block_height_operation(3));

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let next_epoch_start_block_height = drive
                .get_first_epoch_start_block_height_between_epochs(0, 2, Some(&transaction))
                .expect("should find next start_block_height");

            assert!(next_epoch_start_block_height.is_none());
        }

        #[test]
        fn test_start_block_height_in_two_epoch_in_case_of_gaps() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch_tree_0 = Epoch::new(0);
            let epoch_tree_3 = Epoch::new(3);

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_tree_0.update_start_block_height_operation(1));
            batch.push(epoch_tree_3.update_start_block_height_operation(2));

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let next_epoch_start_block_height = drive
                .get_first_epoch_start_block_height_between_epochs(0, 4, Some(&transaction))
                .expect("should find next start_block_height");

            assert!(matches!(next_epoch_start_block_height, Some((3, 2))));
        }
    }
}
