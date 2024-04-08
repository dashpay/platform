use crate::drive::grove_operations::BatchDeleteApplyType::StatefulBatchDelete;

use crate::drive::object_size_info::PathKeyElementInfo;
use std::collections::BTreeMap;

use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::drive::protocol_upgrade::{desired_version_for_validators_path, versions_counter_path};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::error::Error::GroveDB;
use crate::fee::op::LowLevelDriveOperation;

use dpp::util::deserializer::ProtocolVersion;
use dpp::version::drive_versions::DriveVersion;

use grovedb::{Element, TransactionArg};
use integer_encoding::VarInt;

impl Drive {
    /// Removes the proposed app versions for a list of validators.
    ///
    /// This function iterates through the provided list of validator ProTx hashes and
    /// attempts to remove their proposed app versions. It also updates the version counter
    /// for each distinct version found in the removed validator proposals.
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
    pub(super) fn remove_validators_proposed_app_versions_v0<I>(
        &self,
        validator_pro_tx_hashes: I,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<[u8; 32]>, Error>
    where
        I: IntoIterator<Item = [u8; 32]>,
    {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        let inserted = self.remove_validators_proposed_app_versions_operations_v0(
            validator_pro_tx_hashes,
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
        Ok(inserted)
    }

    /// Removes the proposed app versions for a list of validators.
    ///
    /// This function iterates through the provided list of validator ProTx hashes and
    /// attempts to remove their proposed app versions. It also updates the version counter
    /// for each distinct version found in the removed validator proposals.
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
    pub(super) fn remove_validators_proposed_app_versions_operations_v0<I>(
        &self,
        validator_pro_tx_hashes: I,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Vec<[u8; 32]>, Error>
    where
        I: IntoIterator<Item = [u8; 32]>,
    {
        let mut version_counter = self.cache.protocol_versions_counter.write();

        version_counter.load_if_needed(self, transaction, drive_version)?;

        let path = desired_version_for_validators_path();

        let mut removed_pro_tx_hashes = Vec::new();
        let mut previous_versions_removals: BTreeMap<ProtocolVersion, u64> = BTreeMap::new();

        for validator_pro_tx_hash in validator_pro_tx_hashes {
            let removed_element = self.batch_remove_raw(
                (&path).into(),
                validator_pro_tx_hash.as_slice(),
                StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some((false, false)),
                },
                transaction,
                drive_operations,
                drive_version,
            )?;

            if let Some(removed_element) = removed_element {
                removed_pro_tx_hashes.push(validator_pro_tx_hash);

                let previous_version_bytes = removed_element.as_item_bytes().map_err(GroveDB)?;
                let previous_version = ProtocolVersion::decode_var(previous_version_bytes)
                    .ok_or(Error::Drive(DriveError::CorruptedElementType(
                        "encoded value could not be decoded",
                    )))
                    .map(|(value, _)| value)?;

                let entry = previous_versions_removals
                    .entry(previous_version)
                    .or_insert(0);
                *entry += 1;
            }
        }

        for (previous_version, change) in previous_versions_removals {
            let previous_count = version_counter
                .get(&previous_version)
                .map_err(|error| {
                    DriveError::CorruptedCacheState(format!(
                        "{error}. we should never face with blocked global cache when we get previous count because version counter trees must be empty at this point"
                    ))
                })?
                .ok_or(Error::Drive(DriveError::CorruptedCacheState(
                    "trying to lower the count of a version from cache that is not found"
                        .to_string(),
                )))?;
            let removed_count = previous_count.checked_sub(change).ok_or(Error::Drive(DriveError::CorruptedDriveState(
                "trying to lower the count of a version from cache that would result in a negative value"
                    .to_string(),
            )))?;

            version_counter.set_block_cache_version_count(previous_version, removed_count);

            let previous_version_bytes = previous_version.encode_var_vec();
            self.batch_insert(
                PathKeyElementInfo::PathFixedSizeKeyRefElement((
                    versions_counter_path(),
                    &previous_version_bytes,
                    Element::new_item(removed_count.encode_var_vec()),
                )),
                drive_operations,
                drive_version,
            )?;
        }

        Ok(removed_pro_tx_hashes)
    }
}
