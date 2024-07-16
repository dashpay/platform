use crate::drive::credit_pools::pending_epoch_refunds::pending_epoch_refunds_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::batch::GroveDbOpBatch;

use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;

use dpp::fee::epoch::CreditsPerEpoch;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, TransactionArg};
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Adds operations to delete pending epoch refunds except epochs from provided collection
    pub(super) fn add_delete_pending_epoch_refunds_except_specified_operations_v0(
        &self,
        batch: &mut GroveDbOpBatch,
        refunds_per_epoch: &CreditsPerEpoch,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        // TODO: Replace with key iterator
        let mut query = Query::new();

        query.insert_all();

        let (query_result, _) = self
            .grove
            .query_raw(
                &PathQuery::new_unsized(pending_epoch_refunds_path_vec(), query),
                transaction.is_some(),
                true,
                true,
                QueryResultType::QueryKeyElementPairResultType,
                transaction,
                &drive_version.grove_version,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        for (epoch_index_key, _) in query_result.to_key_elements() {
            let epoch_index =
                u16::from_be_bytes(epoch_index_key.as_slice().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(String::from(
                        "pending updates epoch index for must be u16",
                    )))
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

    use crate::drive::credit_pools::pending_epoch_refunds::pending_epoch_refunds_path_vec;
    use crate::drive::Drive;
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::GroveDbOpBatch;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::fee::epoch::CreditsPerEpoch;

    use dpp::version::PlatformVersion;
    use grovedb::batch::Op;

    #[test]
    fn should_add_delete_operations_v0() {
        let drive = setup_drive_with_initial_state_structure();
        let platform_version = PlatformVersion::latest();

        let transaction = drive.grove.start_transaction();

        // Store initial set of pending refunds

        let initial_pending_refunds =
            CreditsPerEpoch::from_iter([(1, 15), (3, 25), (7, 95), (9, 100), (12, 120)]);

        let mut batch = vec![];

        Drive::add_update_pending_epoch_refunds_operations(
            &mut batch,
            initial_pending_refunds,
            &platform_version.drive,
        )
        .expect("should update pending epoch updates");

        drive
            .apply_drive_operations(
                batch,
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("should apply batch");

        // Delete existing pending refunds except specified epochs

        let new_pending_refunds = CreditsPerEpoch::from_iter([(1, 15), (3, 25)]);

        let mut batch = GroveDbOpBatch::new();

        drive
            .add_delete_pending_epoch_refunds_except_specified_operations(
                &mut batch,
                &new_pending_refunds,
                Some(&transaction),
                &platform_version.drive,
            )
            .expect("should fetch and merge pending updates");

        let expected_pending_refunds = CreditsPerEpoch::from_iter([(7, 95), (9, 100), (12, 120)]);

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
