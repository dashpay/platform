use crate::drive::fee_pools::pools_vec_path;
use crate::drive::Drive;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee_pools::epochs::{paths, Epoch};
use grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};

use crate::fee_pools::epochs::epoch_key_constants;
use crate::fee_pools::epochs::epoch_key_constants::KEY_START_BLOCK_HEIGHT;

impl Drive {
    pub fn get_epoch_start_block_height(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        let element = self
            .grove
            .get(
                epoch_tree.get_path(),
                epoch_key_constants::KEY_START_BLOCK_HEIGHT.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if let Element::Item(item, _) = element {
            Ok(u64::from_be_bytes(item.as_slice().try_into().map_err(
                |_| Error::Fee(FeeError::CorruptedStartBlockHeightItemLength()),
            )?))
        } else {
            Err(Error::Fee(FeeError::CorruptedStartBlockHeightNotItem()))
        }
    }

    pub fn get_first_epoch_start_block_height_between_epochs(
        &self,
        from_epoch_index: u16,
        to_epoch_index: u16,
        transaction: TransactionArg,
    ) -> Result<Option<(u16, u64)>, Error> {
        let mut start_block_height_query = Query::new();
        start_block_height_query.insert_key(KEY_START_BLOCK_HEIGHT.to_vec());

        let mut epochs_query = Query::new();

        let from_epoch_key = paths::encode_epoch_index_key(from_epoch_index)?.to_vec();
        let current_epoch_key = paths::encode_epoch_index_key(to_epoch_index)?.to_vec();

        epochs_query.insert_range_after_to_inclusive(from_epoch_key..=current_epoch_key);

        epochs_query.set_subquery(start_block_height_query);

        let sized_query = SizedQuery::new(epochs_query, Some(1), None);

        let path_query = PathQuery::new(pools_vec_path(), sized_query);

        let (result_items, _) = self
            .grove
            .query_raw(&path_query, QueryPathKeyElementTrioResultType, transaction)
            .unwrap()
            .map_err(Error::GroveDB)?;

        if result_items.elements.is_empty() {
            return Ok(None);
        }

        let first_result = &result_items.to_path_key_elements()[0];

        let (path, _, element) = first_result;

        let next_start_block_height = if let Element::Item(item, _) = element {
            u64::from_be_bytes(item.as_slice().try_into().map_err(|_| {
                Error::Fee(FeeError::CorruptedProposerBlockCountItemLength(
                    "item have an invalid length",
                ))
            })?)
        } else {
            return Err(Error::Fee(FeeError::CorruptedStartBlockHeightItemLength()));
        };

        let epoch_key = path
            .last()
            .ok_or(Error::Fee(FeeError::CorruptedStartBlockHeightItemLength()))?;

        let epoch_index = paths::decode_epoch_index_key(epoch_key.as_slice())?;

        Ok(Some((epoch_index, next_start_block_height)))
    }
}

#[cfg(test)]
mod tests {
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;
    use crate::fee_pools::epochs::epoch_key_constants;
    use grovedb::Element;

    use crate::error;
    use crate::error::fee::FeeError;

    use super::Epoch;

    mod get_epoch_start_block_height {
        use crate::fee_pools::epochs::Epoch;

        #[test]
        fn test_error_if_epoch_tree_is_not_initiated() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let non_initiated_epoch = Epoch::new(7000);

            match drive.get_epoch_start_block_height(&non_initiated_epoch, Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to get start block height on uninit epochs pool"
                ),
                Err(e) => match e {
                    super::error::Error::GroveDB(grovedb::Error::PathNotFound(_)) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_value_is_not_set() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(0);

            match drive.get_epoch_start_block_height(&epoch, Some(&transaction)) {
                Ok(_) => assert!(false, "must be an error"),
                Err(e) => match e {
                    super::error::Error::GroveDB(_) => assert!(true),
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
                    super::epoch_key_constants::KEY_START_BLOCK_HEIGHT.as_slice(),
                    super::Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            match drive.get_epoch_start_block_height(&epoch, Some(&transaction)) {
                Ok(_) => assert!(false, "should not be able to decode stored value"),
                Err(e) => match e {
                    super::error::Error::Fee(
                        super::FeeError::CorruptedStartBlockHeightItemLength(),
                    ) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_element_has_invalid_type() {
            let drive = super::setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch = super::Epoch::new(0);

            drive
                .grove
                .insert(
                    epoch.get_path(),
                    super::epoch_key_constants::KEY_START_BLOCK_HEIGHT.as_slice(),
                    super::Element::empty_tree(),
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            match drive.get_epoch_start_block_height(&epoch, Some(&transaction)) {
                Ok(_) => assert!(false, "should not be able to decode stored value"),
                Err(e) => match e {
                    super::error::Error::Fee(
                        super::FeeError::CorruptedStartBlockHeightNotItem(),
                    ) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }
    }

    mod get_first_epoch_start_block_height_between_epochs {
        use crate::common::helpers::identities::create_test_masternode_identities_and_add_them_as_epoch_block_proposers;
        use crate::common::helpers::setup::setup_drive_with_initial_state_structure;
        use crate::drive::batch::GroveDbOpBatch;
        use crate::fee_pools::epochs::Epoch;

        #[test]
        fn test_next_block_height() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let epoch_tree_0 = Epoch::new(0);
            let epoch_tree_1 = Epoch::new(1);

            let mut batch = GroveDbOpBatch::new();

            batch.push(epoch_tree_0.update_start_block_height_operation(1));
            batch.push(epoch_tree_1.update_start_block_height_operation(2));

            // Apply proposers tree
            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let next_epoch_start_block_height_option = drive
                .get_first_epoch_start_block_height_between_epochs(0, 2, Some(&transaction))
                .expect("should find next start_block_height");

            match next_epoch_start_block_height_option {
                None => assert!(false, "should find start_block_height"),
                Some((epoch_index, start_block_height)) => {
                    assert_eq!(epoch_index, 1);
                    assert_eq!(start_block_height, 2);
                }
            }
        }

        #[test]
        fn test_none_if_there_are_no_start_block_heights() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let next_epoch_start_block_height = drive
                .get_first_epoch_start_block_height_between_epochs(0, 4, Some(&transaction))
                .expect("should find next start_block_height");

            match next_epoch_start_block_height {
                None => assert!(true),
                Some(_) => assert!(false, "should not find any"),
            }
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

            // Apply proposers tree
            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let next_epoch_start_block_height = drive
                .get_first_epoch_start_block_height_between_epochs(0, 2, Some(&transaction))
                .expect("should find next start_block_height");

            match next_epoch_start_block_height {
                None => assert!(true),
                Some(_) => assert!(false, "should not find any"),
            }
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

            // Apply proposers tree
            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let next_epoch_start_block_height = drive
                .get_first_epoch_start_block_height_between_epochs(0, 4, Some(&transaction))
                .expect("should find next start_block_height");

            match next_epoch_start_block_height {
                None => assert!(false),
                Some((epoch_index, start_block_height)) => {
                    assert_eq!(epoch_index, 3);
                    assert_eq!(start_block_height, 2);
                }
            }
        }
    }
}
