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

//! Epoch Proposers.
//!
//! This module implements functions in Drive relevant to block proposers.
//!

use grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;

impl Drive {
    /// Returns the given proposer's block count
    pub fn get_epochs_proposer_block_count(
        &self,
        epoch: &Epoch,
        proposer_tx_hash: &[u8; 32],
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        let element = self
            .grove
            .get(epoch.get_proposers_path(), proposer_tx_hash, transaction)
            .unwrap()
            .map_err(Error::GroveDB)?;

        let Element::Item(encoded_proposer_block_count, _) = element else {
            return Err(Error::Drive(DriveError::UnexpectedElementType(
                "epochs proposer block count must be an item",
            )));
        };

        let proposer_block_count =
            u64::from_be_bytes(encoded_proposer_block_count.as_slice().try_into().map_err(
                |_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "epochs proposer block count item have an invalid length",
                    ))
                },
            )?);

        Ok(proposer_block_count)
    }

    /// Returns true if the Epoch's Proposers Tree is empty
    pub fn is_epochs_proposers_tree_empty(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
    ) -> Result<bool, Error> {
        match self
            .grove
            .is_empty_tree(epoch_tree.get_proposers_path(), transaction)
            .unwrap()
        {
            Ok(result) => Ok(result),
            Err(grovedb::Error::PathNotFound(_) | grovedb::Error::PathParentLayerNotFound(_)) => {
                Ok(true)
            }
            Err(_) => Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "internal grovedb error",
            ))),
        }
    }

    /// Returns a list of the Epoch's block proposers
    pub fn get_epoch_proposers(
        &self,
        epoch_tree: &Epoch,
        limit: u16,
        transaction: TransactionArg,
    ) -> Result<Vec<(Vec<u8>, u64)>, Error> {
        let path_as_vec = epoch_tree.get_proposers_path_vec();

        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(path_as_vec, SizedQuery::new(query, Some(limit), None));

        let key_elements = self
            .grove
            .query_raw(
                &path_query,
                transaction.is_some(),
                QueryKeyElementPairResultType,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?
            .0
            .to_key_elements();

        let proposers = key_elements
            .into_iter()
            .map(|(pro_tx_hash, element)| {
                let Element::Item(encoded_block_count, _) = element else {
                    return Err(Error::Drive(DriveError::UnexpectedElementType(
                        "epochs proposer block count must be an item",
                    )));
                };

                let block_count = u64::from_be_bytes(
                    encoded_block_count.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "epochs proposer block count must be u64",
                        ))
                    })?,
                );

                Ok((pro_tx_hash, block_count))
            })
            .collect::<Result<_, _>>()?;

        Ok(proposers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;
    use crate::drive::batch::GroveDbOpBatch;

    mod get_epochs_proposer_block_count {
        use super::*;

        #[test]
        fn test_error_if_value_has_invalid_length() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let pro_tx_hash: [u8; 32] = rand::random();

            let epoch = Epoch::new(0);

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch.init_proposers_tree_operation());

            batch.add_insert(
                epoch.get_proposers_path_vec(),
                pro_tx_hash.to_vec(),
                Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
            );

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let result =
                drive.get_epochs_proposer_block_count(&epoch, &pro_tx_hash, Some(&transaction));

            assert!(matches!(
                result,
                Err(Error::Drive(DriveError::CorruptedSerialization(_),))
            ));
        }

        #[test]
        fn test_error_if_epoch_tree_is_not_initiated() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let pro_tx_hash: [u8; 32] = rand::random();

            let epoch = Epoch::new(7000);

            let result =
                drive.get_epochs_proposer_block_count(&epoch, &pro_tx_hash, Some(&transaction));

            assert!(matches!(
                result,
                Err(Error::GroveDB(grovedb::Error::PathParentLayerNotFound(_)))
            ));
        }
    }

    mod is_epochs_proposers_tree_empty {
        use super::*;

        #[test]
        fn test_check_if_empty() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = Epoch::new(0);

            let result = drive
                .is_epochs_proposers_tree_empty(&epoch, Some(&transaction))
                .expect("should check if tree is empty");

            assert!(result);
        }
    }

    mod get_epoch_proposers {
        use super::*;

        #[test]
        fn test_value() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let pro_tx_hash: [u8; 32] = rand::random();
            let block_count = 42;

            let epoch = Epoch::new(0);

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch.init_proposers_tree_operation());

            batch.push(epoch.update_proposer_block_count_operation(&pro_tx_hash, block_count));

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let result = drive
                .get_epoch_proposers(&epoch, 100, Some(&transaction))
                .expect("should get proposers");

            assert_eq!(result, vec!((pro_tx_hash.to_vec(), block_count)));
        }
    }
}
