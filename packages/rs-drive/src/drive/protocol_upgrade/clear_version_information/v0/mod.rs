use crate::drive::grove_operations::BatchDeleteApplyType::StatefulBatchDelete;

use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::drive::protocol_upgrade::{
    desired_version_for_validators_path, desired_version_for_validators_path_vec,
    versions_counter_path, versions_counter_path_vec,
};
use crate::drive::Drive;

use crate::error::Error;

use crate::fee::op::LowLevelDriveOperation;
use crate::query::QueryItem;

use dpp::version::drive_versions::DriveVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, TransactionArg};

use std::ops::RangeFull;

impl Drive {
    /// Clear all version information from the backing store, this is done on epoch change in
    /// execution logic
    pub(super) fn clear_version_information_v0(
        &self,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        self.clear_version_information_operations_v0(
            transaction,
            &mut batch_operations,
            drive_version,
        )?;
        let grove_db_operations =
            LowLevelDriveOperation::grovedb_operations_batch(&batch_operations);
        if !grove_db_operations.is_empty() {
            self.apply_batch_grovedb_operations(
                None,
                transaction,
                grove_db_operations,
                &mut vec![],
                drive_version,
            )?;
        }
        Ok(())
    }

    /// Clear all version information from the backing store, this is done on epoch change in
    /// execution logic
    pub(in crate::drive::protocol_upgrade) fn clear_version_information_operations_v0(
        &self,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let path_query = PathQuery::new_unsized(
            versions_counter_path_vec(),
            Query::new_single_query_item(QueryItem::RangeFull(RangeFull)),
        );
        let results = self
            .grove_get_path_query(
                &path_query,
                transaction,
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                drive_version,
            )?
            .0;
        for key in results.to_keys() {
            self.batch_delete(
                (&versions_counter_path()).into(),
                key.as_slice(),
                StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: (Some((false, false))),
                },
                transaction,
                drive_operations,
                drive_version,
            )?;
        }

        let path_query = PathQuery::new_unsized(
            desired_version_for_validators_path_vec(),
            Query::new_single_query_item(QueryItem::RangeFull(RangeFull)),
        );
        let (results, _) = self.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            drive_version,
        )?;
        for (key, _) in results.to_key_elements() {
            self.batch_delete(
                (&desired_version_for_validators_path()).into(),
                key.as_slice(),
                StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: (Some((false, false))),
                },
                transaction,
                drive_operations,
                drive_version,
            )?;
        }
        Ok(())
    }
}
