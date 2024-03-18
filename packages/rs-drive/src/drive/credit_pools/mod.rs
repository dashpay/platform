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

#[cfg(feature = "full")]
use crate::drive::batch::GroveDbOpBatch;
#[cfg(feature = "full")]
use crate::drive::Drive;
#[cfg(feature = "full")]
use crate::error::drive::DriveError;
#[cfg(feature = "full")]
use crate::error::Error;

#[cfg(feature = "full")]
use crate::fee_pools::epochs::epoch_key_constants::KEY_POOL_STORAGE_FEES;
#[cfg(feature = "full")]
use crate::fee_pools::epochs::paths::encode_epoch_index_key;
#[cfg(feature = "full")]
use crate::fee_pools::epochs::paths::EpochProposers;

#[cfg(feature = "full")]
use dpp::block::epoch::{Epoch, EpochIndex};
#[cfg(feature = "full")]
use dpp::fee::epoch::SignedCreditsPerEpoch;
#[cfg(feature = "full")]
use dpp::fee::SignedCredits;
#[cfg(feature = "full")]
use grovedb::query_result_type::QueryResultType;
#[cfg(feature = "full")]
use grovedb::{Element, PathQuery, Query, TransactionArg};
#[cfg(feature = "full")]
use itertools::Itertools;

#[cfg(feature = "full")]
/// Epochs module
pub mod epochs;

#[cfg(any(feature = "full", feature = "verify"))]
pub(crate) mod paths;

#[cfg(feature = "full")]
pub mod pending_epoch_refunds;

#[cfg(feature = "full")]
pub mod storage_fee_distribution_pool;
#[cfg(feature = "full")]
pub mod unpaid_epoch;

#[cfg(feature = "full")]
use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;

#[cfg(feature = "full")]
use crate::drive::fee::get_overflow_error;

#[cfg(any(feature = "full", feature = "verify"))]
pub use paths::*;

#[cfg(feature = "full")]
impl Drive {
    /// Adds GroveDB operations to update epoch storage fee pools with specified map of credits to epochs
    /// This method optimized to update sequence of epoch pools without gaps
    pub fn add_update_epoch_storage_fee_pools_sequence_operations(
        &self,
        batch: &mut GroveDbOpBatch,
        credits_per_epochs: SignedCreditsPerEpoch,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        if credits_per_epochs.is_empty() {
            return Ok(());
        }

        let min_epoch_index = credits_per_epochs.keys().min().ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution("can't find min epoch index"),
        ))?;
        let min_encoded_epoch_index = encode_epoch_index_key(min_epoch_index.to_owned())?.to_vec();

        let max_epoch_index = credits_per_epochs.keys().max().ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution("can't find max epoch index"),
        ))?;
        let max_encoded_epoch_index = encode_epoch_index_key(max_epoch_index.to_owned())?.to_vec();

        let credits_per_epochs_length = credits_per_epochs.len();

        if max_epoch_index - min_epoch_index + 1 != credits_per_epochs_length as EpochIndex {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "gaps in credits per epoch are not supported",
            )));
        }

        let mut storage_fee_pool_query = Query::new();
        storage_fee_pool_query.insert_key(KEY_POOL_STORAGE_FEES.to_vec());

        let mut epochs_query = Query::new();

        epochs_query.insert_range_inclusive(min_encoded_epoch_index..=max_encoded_epoch_index);
        epochs_query.set_subquery(storage_fee_pool_query);

        let (storage_fee_pools_result, _) = self
            .grove
            .query_raw(
                &PathQuery::new_unsized(pools_vec_path(), epochs_query),
                transaction.is_some(),
                true,
                QueryResultType::QueryElementResultType,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        let storage_fee_pools = storage_fee_pools_result.to_elements();

        let mut negative_credits_from_previous_epochs: SignedCredits = 0;

        for (i, (epoch_index, credits)) in credits_per_epochs
            .into_iter()
            .sorted_by_key(|x| x.0)
            .enumerate()
        {
            let existing_storage_fee: SignedCredits = match storage_fee_pools.get(i) {
                Some(Element::SumItem(storage_fee, _)) => *storage_fee,
                None => 0,
                Some(_) => {
                    return Err(Error::Drive(DriveError::UnexpectedElementType(
                        "epoch storage pools must be sum items",
                    )))
                }
            };

            let mut credits_to_update =
                existing_storage_fee.checked_add(credits).ok_or_else(|| {
                    get_overflow_error("can't add credits to existing epoch pool storage fee")
                })?;

            // Subtract negative credits from previous epochs
            if negative_credits_from_previous_epochs != 0 {
                credits_to_update += negative_credits_from_previous_epochs;
                negative_credits_from_previous_epochs = 0;
            }

            // Epoch storage fee pool can't be negative so we keep negative amount
            // for the future epochs
            if credits_to_update < 0 {
                negative_credits_from_previous_epochs += credits_to_update;
                credits_to_update = 0;
            }

            // If we still have negative credits for the last pool it's not good
            if negative_credits_from_previous_epochs != 0 && i == credits_per_epochs_length - 1 {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "epoch storage pool went bellow zero",
                )));
            }

            batch.add_insert(
                Epoch::new(epoch_index)
                    .expect("epoch index should not overflow")
                    .get_path_vec(),
                KEY_POOL_STORAGE_FEES.to_vec(),
                Element::new_sum_item(credits_to_update),
            );
        }

        Ok(())
    }
}

#[cfg(feature = "full")]
#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;

    mod add_update_epoch_storage_fee_pools_operations {
        use super::*;
        use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
        use crate::fee_pools::epochs::operations_factory::EpochOperations;
        use dpp::block::epoch::EpochIndex;
        use dpp::fee::epoch::GENESIS_EPOCH_INDEX;
        use dpp::fee::Credits;
        use dpp::version::PlatformVersion;
        use grovedb::batch::Op;

        #[test]
        fn should_do_nothing_if_credits_per_epoch_are_empty() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let credits_per_epoch = SignedCreditsPerEpoch::default();

            let mut batch = GroveDbOpBatch::new();

            drive
                .add_update_epoch_storage_fee_pools_sequence_operations(
                    &mut batch,
                    credits_per_epoch,
                    Some(&transaction),
                )
                .expect("should update epoch storage pools");

            assert_eq!(batch.len(), 0);
        }

        #[test]
        fn should_update_epoch_storage_fee_pools() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            const TO_EPOCH_INDEX: EpochIndex = 10;

            let platform_version = PlatformVersion::first();

            // Store initial epoch storage pool values
            let operations = (GENESIS_EPOCH_INDEX..TO_EPOCH_INDEX)
                .enumerate()
                .map(|(i, epoch_index)| {
                    let credits = 10 - i as Credits;

                    let epoch = Epoch::new(epoch_index).unwrap();

                    epoch.update_storage_fee_pool_operation(credits)
                })
                .collect::<Result<_, _>>()
                .expect("should add operations");

            let batch = GroveDbOpBatch::from_operations(operations);

            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let credits_to_epochs: SignedCreditsPerEpoch = (GENESIS_EPOCH_INDEX..TO_EPOCH_INDEX)
                .enumerate()
                .map(|(credits, epoch_index)| (epoch_index, credits as SignedCredits))
                .collect();

            let mut batch = GroveDbOpBatch::new();

            drive
                .add_update_epoch_storage_fee_pools_sequence_operations(
                    &mut batch,
                    credits_to_epochs,
                    Some(&transaction),
                )
                .expect("should update epoch storage pools");

            assert_eq!(batch.len(), TO_EPOCH_INDEX as usize);

            for (i, operation) in batch.into_iter().enumerate() {
                assert_eq!(operation.key.get_key(), KEY_POOL_STORAGE_FEES);

                assert_eq!(
                    operation.path.to_path(),
                    Epoch::new(i as EpochIndex).unwrap().get_path_vec()
                );

                let Op::Insert {
                    element: Element::SumItem(credits, _),
                } = operation.op
                else {
                    panic!("invalid operation");
                };

                assert_eq!(credits, 10);
            }
        }

        #[test]
        fn should_subtract_negative_credits_from_future_epochs() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::latest();

            const TO_EPOCH_INDEX: EpochIndex = 10;

            // Store initial epoch storage pool values
            let operations = (GENESIS_EPOCH_INDEX..TO_EPOCH_INDEX)
                .enumerate()
                .map(|(i, epoch_index)| {
                    let credits = 10 - i as Credits;

                    let epoch = Epoch::new(epoch_index).unwrap();

                    epoch.update_storage_fee_pool_operation(credits)
                })
                .collect::<Result<_, _>>()
                .expect("should add operations");

            let batch = GroveDbOpBatch::from_operations(operations);

            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let mut credits_to_epochs: SignedCreditsPerEpoch = (GENESIS_EPOCH_INDEX
                ..TO_EPOCH_INDEX)
                .enumerate()
                .map(|(credits, epoch_index)| (epoch_index, credits as SignedCredits))
                .collect();

            // Add negative credits to update
            credits_to_epochs.insert(GENESIS_EPOCH_INDEX, -15);

            let mut batch = GroveDbOpBatch::new();

            drive
                .add_update_epoch_storage_fee_pools_sequence_operations(
                    &mut batch,
                    credits_to_epochs,
                    Some(&transaction),
                )
                .expect("should update epoch storage pools");

            assert_eq!(batch.len(), TO_EPOCH_INDEX as usize);

            let updated_credits: Vec<_> = batch
                .into_iter()
                .map(|operation| {
                    let Op::Insert {
                        element: Element::SumItem(credits, _),
                    } = operation.op
                    else {
                        panic!("invalid operation");
                    };

                    credits
                })
                .collect();

            let expected_credits = [0, 5, 10, 10, 10, 10, 10, 10, 10, 10];

            assert_eq!(updated_credits, expected_credits);
        }
    }
}
