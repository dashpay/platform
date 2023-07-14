mod v0;

use crate::drive::batch::GroveDbOpBatch;
use crate::drive::grove_operations::BatchDeleteApplyType::StatefulBatchDelete;
use crate::drive::grove_operations::BatchInsertApplyType;
use crate::drive::object_size_info::PathKeyElementInfo;
use std::collections::BTreeMap;

use crate::drive::protocol_upgrade::{desired_version_for_validators_path, versions_counter_path};
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::error::Error::GroveDB;
use crate::fee::op::LowLevelDriveOperation;
use crate::query::QueryItem;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::drive_versions::DriveVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, TransactionArg};
use integer_encoding::VarInt;
use nohash_hasher::IntMap;
use std::ops::RangeFull;

impl Drive {
    /// Removes the proposed app versions for a list of validators.
    ///
    /// # Arguments
    ///
    /// * `validator_pro_tx_hashes` - A vector of ProTx hashes representing the validators
    ///                                whose proposed app versions should be removed.
    /// * `transaction` - A transaction argument to interact with the underlying storage.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<[u8; 32]>, Error>` - Returns the pro_tx_hashes of validators that were removed,
    ///                             or an error if an issue was encountered.
    ///
    /// # Errors
    ///
    /// This function may return an error if any of the following conditions are met:
    ///
    /// * There is an issue interacting with the underlying storage.
    /// * The cache state is corrupted.
    pub fn remove_validators_proposed_app_versions<I>(
        &self,
        validator_pro_tx_hashes: I,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<[u8; 32]>, Error>
    where
        I: IntoIterator<Item = [u8; 32]>,
    {
        match drive_version
            .methods
            .protocol_upgrade
            .remove_validators_proposed_app_versions
        {
            0 => self.remove_validators_proposed_app_versions_v0(
                validator_pro_tx_hashes,
                transaction,
                &self.drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_validators_proposed_app_versions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Removes the proposed app versions for a list of validators.
    ///
    /// # Arguments
    ///
    /// * `validator_pro_tx_hashes` - An into iterator generic of ProTx hashes representing the validators
    ///                                whose proposed app versions should be removed.
    /// * `transaction` - A transaction argument to interact with the underlying storage.
    /// * `drive_operations` - A mutable reference to a vector of low-level drive operations
    ///                        that will be populated with the required changes.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<[u8; 32]>, Error>` - Returns the pro_tx_hashes of validators that were removed,
    ///                             or an error if an issue was encountered.
    ///
    /// # Errors
    ///
    /// This function may return an error if any of the following conditions are met:
    ///
    /// * There is an issue interacting with the underlying storage.
    /// * The cache state is corrupted.
    pub(crate) fn remove_validators_proposed_app_versions_operations<I>(
        &self,
        validator_pro_tx_hashes: I,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Vec<[u8; 32]>, Error>
    where
        I: IntoIterator<Item = [u8; 32]>,
    {
        match drive_version
            .methods
            .protocol_upgrade
            .remove_validators_proposed_app_versions
        {
            0 => self.remove_validators_proposed_app_versions_operations_v0(
                validator_pro_tx_hashes,
                transaction,
                drive_operations,
                &self.drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_validators_proposed_app_versions_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
