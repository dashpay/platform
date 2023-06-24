use crate::drive::batch::GroveDbOpBatch;
use crate::drive::grove_operations::BatchDeleteApplyType::StatefulBatchDelete;
use crate::drive::grove_operations::BatchInsertApplyType;
use crate::drive::object_size_info::PathKeyElementInfo;
use std::collections::BTreeMap;

use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::error::Error::GroveDB;
use crate::fee::op::LowLevelDriveOperation;
use crate::query::QueryItem;
use dpp::util::deserializer::ProtocolVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, TransactionArg};
use integer_encoding::VarInt;
use nohash_hasher::IntMap;
use std::ops::RangeFull;
use dpp::version::PlatformVersion;
use crate::drive::protocol_upgrade::{desired_version_for_validators_path, desired_version_for_validators_path_vec, versions_counter_path, versions_counter_path_vec};


impl Drive {
    /// Clear all version information from the backing store, this is done on epoch change in
    /// execution logic
    pub(super) fn clear_version_information_v0(&self, transaction: TransactionArg, platform_version: &PlatformVersion) -> Result<(), Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        self.clear_version_information_operations_v0(transaction, &mut batch_operations, platform_version)?;
        let grove_db_operations =
            LowLevelDriveOperation::grovedb_operations_batch(&batch_operations);
        if !grove_db_operations.is_empty() {
            self.apply_batch_grovedb_operations(
                None,
                transaction,
                grove_db_operations,
                &mut vec![],
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
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let path_query = PathQuery::new_unsized(
            versions_counter_path_vec(),
            Query::new_single_query_item(QueryItem::RangeFull(RangeFull)),
        );
        let (results, _) = self.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
        )?;
        for (key, _) in results.to_key_elements() {
            self.batch_delete(
                (&versions_counter_path()).into(),
                key.as_slice(),
                StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: (Some((false, false))),
                },
                transaction,
                drive_operations,
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
            )?;
        }
        Ok(())
    }
}