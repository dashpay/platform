use crate::drive::batch::{DriveOperation, GroveDbOpBatch};
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::credits::Creditable;
use dpp::fee::epoch::CreditsPerEpoch;
use crate::fee::get_overflow_error;
use crate::fee_pools::epochs_root_tree_key_constants::KEY_PENDING_EPOCH_REFUNDS;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, TransactionArg};
use crate::drive::credit_pools::pending_epoch_refunds::pending_epoch_refunds_path_vec;

impl Drive {

    /// Adds operations to delete pending epoch refunds except epochs from provided collection
    pub(super) fn add_delete_pending_epoch_refunds_except_specified_operations_v0(
        &self,
        batch: &mut GroveDbOpBatch,
        refunds_per_epoch: &CreditsPerEpoch,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        // TODO: Replace with key iterator
        let mut query = Query::new();

        query.insert_all();

        let (query_result, _) = self
            .grove
            .query_raw(
                &PathQuery::new_unsized(pending_epoch_refunds_path_vec(), query),
                transaction.is_some(),
                QueryResultType::QueryKeyElementPairResultType,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        for (epoch_index_key, _) in query_result.to_key_elements() {
            let epoch_index =
                u16::from_be_bytes(epoch_index_key.as_slice().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "pending updates epoch index for must be u16",
                    ))
                })?);

            if refunds_per_epoch.contains_key(&epoch_index) {
                continue;
            }

            batch.add_delete(pending_epoch_refunds_path_vec(), epoch_index_key);
        }

        Ok(())
    }
}


#[cfg(test)]
mod tests {

        use dpp::block::extended_block_info::BlockInfo;
        use grovedb::batch::Op;
    use dpp::version::drive_versions::DriveVersion;
    use crate::drive::batch::GroveDbOpBatch;
    use crate::drive::credit_pools::pending_epoch_refunds::pending_epoch_refunds_path_vec;
    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;

    #[test]
        fn should_add_delete_operations_v0() {
            let drive = setup_drive_with_initial_state_structure();
        let drive_version = DriveVersion::default();

            let transaction = drive.grove.start_transaction();

            // Store initial set of pending refunds

            let initial_pending_refunds =
                CreditsPerEpoch::from_iter([(1, 15), (3, 25), (7, 95), (9, 100), (12, 120)]);

            let mut batch = vec![];

            add_update_pending_epoch_refunds_operations(&mut batch, initial_pending_refunds)
                .expect("should update pending epoch updates");

            drive
                .apply_drive_operations(batch, true, &BlockInfo::default(), Some(&transaction), &drive_version)
                .expect("should apply batch");

            // Delete existing pending refunds except specified epochs

            let new_pending_refunds = CreditsPerEpoch::from_iter([(1, 15), (3, 25)]);

            let mut batch = GroveDbOpBatch::new();

            drive
                .add_delete_pending_epoch_refunds_except_specified_operations(
                    &mut batch,
                    &new_pending_refunds,
                    Some(&transaction),
                    &drive_version,
                )
                .expect("should fetch and merge pending updates");

            let expected_pending_refunds =
                CreditsPerEpoch::from_iter([(7, 95), (9, 100), (12, 120)]);

            assert_eq!(batch.len(), expected_pending_refunds.len());

            for operation in batch.into_iter() {
                assert!(matches!(operation.op, Op::Delete));

                assert_eq!(operation.path.to_path(), pending_epoch_refunds_path_vec());

                let epoch_index_key = operation.key.get_key();
                let epoch_index = u16::from_be_bytes(
                    epoch_index_key
                        .try_into()
                        .expect("should convert to u16 bytes"),
                );

                assert!(expected_pending_refunds.contains_key(&epoch_index));
            }
        }
    }
