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
use crate::error::fee::FeeError;
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

        if let Element::Item(item, _) = element {
            Ok(u64::from_be_bytes(item.as_slice().try_into().map_err(
                |_| {
                    Error::Fee(FeeError::CorruptedProposerBlockCountItemLength(
                        "epochs proposer block count item have an invalid length",
                    ))
                },
            )?))
        } else {
            Err(Error::Fee(FeeError::CorruptedProposerBlockCountNotItem(
                "epochs proposer block count must be an item",
            )))
        }
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
            Err(err) => match err {
                grovedb::Error::PathNotFound(_) => Ok(true),
                _ => Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "internal grovedb error",
                ))),
            },
        }
    }

    /// Returns a list of the Epoch's block proposers
    pub fn get_epoch_proposers(
        &self,
        epoch_tree: &Epoch,
        limit: u16,
        transaction: TransactionArg,
    ) -> Result<Vec<(Vec<u8>, u64)>, Error> {
        let path_as_vec: Vec<Vec<u8>> = epoch_tree
            .get_proposers_path()
            .iter()
            .map(|slice| slice.to_vec())
            .collect();

        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(path_as_vec, SizedQuery::new(query, Some(limit), None));

        let key_elements = self
            .grove
            .query_raw(&path_query, QueryKeyElementPairResultType, transaction)
            .unwrap()
            .map_err(Error::GroveDB)?
            .0
            .to_key_elements();

        let result = key_elements
            .into_iter()
            .map(|(pro_tx_hash, element)| {
                if let Element::Item(item, _) = element {
                    let block_count =
                        u64::from_be_bytes(item.as_slice().try_into().map_err(|_| {
                            Error::Fee(FeeError::CorruptedProposerBlockCountItemLength(
                                "epochs proposer block count item have an invalid length",
                            ))
                        })?);

                    Ok((pro_tx_hash, block_count))
                } else {
                    Err(Error::Fee(FeeError::CorruptedProposerBlockCountNotItem(
                        "epochs proposer block count must be an item",
                    )))
                }
            })
            .collect::<Result<Vec<(Vec<u8>, u64)>, Error>>()?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use grovedb::Element;

    use crate::error::{self, fee::FeeError};

    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;
    use crate::drive::batch::GroveDbOpBatch;
    use crate::fee_pools::epochs::Epoch;

    mod get_epochs_proposer_block_count {

        #[test]
        fn test_error_if_value_has_invalid_length() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let pro_tx_hash: [u8; 32] = rand::random();

            let epoch = super::Epoch::new(0);

            let mut batch = super::GroveDbOpBatch::new();

            batch.push(epoch.init_proposers_tree_operation());

            batch.add_insert(
                epoch.get_proposers_vec_path(),
                pro_tx_hash.to_vec(),
                super::Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
            );

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            match drive.get_epochs_proposer_block_count(&epoch, &pro_tx_hash, Some(&transaction)) {
                Ok(_) => assert!(false, "should not be able to decode stored value"),
                Err(e) => match e {
                    super::error::Error::Fee(
                        super::FeeError::CorruptedProposerBlockCountItemLength(_),
                    ) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_epoch_tree_is_not_initiated() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let pro_tx_hash: [u8; 32] = rand::random();

            let epoch = super::Epoch::new(7000);

            match drive.get_epochs_proposer_block_count(&epoch, &pro_tx_hash, Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to get proposer block count on uninit epochs pool"
                ),
                Err(e) => match e {
                    super::error::Error::GroveDB(grovedb::Error::PathNotFound(_)) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }
    }

    mod is_epochs_proposers_tree_empty {
        #[test]
        fn test_check_if_empty() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(0);

            let result = drive
                .is_epochs_proposers_tree_empty(&epoch, Some(&transaction))
                .expect("should check if tree is empty");

            assert_eq!(result, true);
        }
    }

    mod get_epoch_proposers {
        #[test]
        fn test_value() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let pro_tx_hash: [u8; 32] = rand::random();
            let block_count = 42;

            let epoch = super::Epoch::new(0);

            let mut batch = super::GroveDbOpBatch::new();

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
