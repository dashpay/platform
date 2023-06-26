mod v0;

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
use dpp::version::drive_versions::DriveVersion;
use crate::drive::protocol_upgrade::versions_counter_path_vec;


impl Drive {
    /// Fetch versions by count for the upgrade window
    ///
    /// # Arguments
    ///
    /// * `transaction` - A `TransactionArg` object representing the transaction.
    ///
    /// # Returns
    ///
    /// * `Result<IntMap<ProtocolVersion, u64>, Error>` - If successful, returns an `Ok(IntMap<ProtocolVersion, u64>)` which contains versions and their counters. If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the Drive version is unknown or any issue with the data reading process.
    pub fn fetch_versions_with_counter(
        &self,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<IntMap<ProtocolVersion, u64>, Error> {
        match drive_version.methods.protocol_upgrade.fetch_versions_with_counter {
            0 => self.fetch_versions_with_counter_v0(transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_versions_with_counter".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}