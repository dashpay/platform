use crate::drive::batch::GroveDbOpBatch;
use crate::drive::grove_operations::BatchDeleteApplyType::StatefulBatchDelete;
use crate::drive::grove_operations::BatchInsertApplyType;
use crate::drive::object_size_info::PathKeyElementInfo;
use crate::drive::system::{misc_path, misc_path_vec};
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::error::Error::GroveDB;
use crate::fee::op::DriveOperation;
use crate::query::QueryItem;
use dpp::util::deserializer::ProtocolVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, TransactionArg};
use integer_encoding::VarInt;
use nohash_hasher::IntMap;
use std::ops::RangeFull;

/// constant id for various versions counter
pub const VERSIONS_COUNTER: [u8; 1] = [0];
/// constant id for subtree containing the desired versions for each validator
pub const VALIDATOR_DESIRED_VERSIONS: [u8; 1] = [1];
/// constant id for value of the starting window
/// This is set to the genesis time
pub const WINDOW_START: [u8; 1] = [2];

/// Add operations for creating initial versioning state structure
pub fn add_initial_fork_update_structure_operations(batch: &mut GroveDbOpBatch) {
    batch.add_insert_empty_tree(
        vec![vec![RootTree::Versions as u8]],
        VERSIONS_COUNTER.to_vec(),
    );

    batch.add_insert_empty_tree(
        vec![vec![RootTree::Versions as u8]],
        VALIDATOR_DESIRED_VERSIONS.to_vec(),
    );
}

pub(crate) fn versions_counter_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Versions),
        VERSIONS_COUNTER.as_slice(),
    ]
}

fn versions_counter_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Versions as u8], VERSIONS_COUNTER.to_vec()]
}

pub(crate) fn desired_version_for_validators_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Versions),
        VALIDATOR_DESIRED_VERSIONS.as_slice(),
    ]
}

fn desired_version_for_validators_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Versions as u8],
        VALIDATOR_DESIRED_VERSIONS.to_vec(),
    ]
}

impl Drive {
    /// Clear all version information from the backing store, this is done on epoch change in
    /// execution logic
    pub fn clear_version_information(&self, transaction: TransactionArg) -> Result<(), Error> {
        let mut batch_operations: Vec<DriveOperation> = vec![];
        self.clear_version_information_operations(transaction, &mut batch_operations)?;
        let grove_db_operations = DriveOperation::grovedb_operations_batch(&batch_operations);
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
    pub fn change_to_new_version_and_clear_version_information(
        &self,
        current_version: ProtocolVersion,
        next_version: ProtocolVersion,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let mut batch_operations: Vec<DriveOperation> = vec![];
        self.clear_version_information_operations(transaction, &mut batch_operations)?;
        self.set_current_protocol_version_operations(
            current_version,
            transaction,
            &mut batch_operations,
        )?;
        self.set_next_protocol_version_operations(
            next_version,
            transaction,
            &mut batch_operations,
        )?;
        let grove_db_operations = DriveOperation::grovedb_operations_batch(&batch_operations);
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
    pub fn clear_version_information_operations(
        &self,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
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
                versions_counter_path(),
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
                desired_version_for_validators_path(),
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
    /// Fetch versions by count for the upgrade window
    pub fn fetch_versions_with_counter(
        &self,
        transaction: TransactionArg,
    ) -> Result<IntMap<ProtocolVersion, u64>, Error> {
        let mut version_counter = IntMap::<ProtocolVersion, u64>::default();
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
        for (version_bytes, _count_element) in results.to_key_elements() {
            let version = ProtocolVersion::decode_var(version_bytes.as_slice())
                .ok_or(Error::Drive(DriveError::CorruptedElementType(
                    "encoded value could not be decoded",
                )))
                .map(|(value, _)| value)?;
            let count = u64::decode_var(version_bytes.as_slice())
                .ok_or(Error::Drive(DriveError::CorruptedElementType(
                    "encoded value could not be decoded",
                )))
                .map(|(value, _)| value)?;
            version_counter.insert(version, count);
        }
        Ok(version_counter)
    }
    /// Update the validator proposed app version
    /// returns true if the value was changed, or is new
    /// returns false if it was not changed
    pub fn update_validator_proposed_app_version(
        &self,
        validator_pro_tx_hash: [u8; 32],
        version: ProtocolVersion,
        transaction: TransactionArg,
    ) -> Result<bool, Error> {
        let mut batch_operations: Vec<DriveOperation> = vec![];
        let inserted = self.update_validator_proposed_app_version_operations(
            validator_pro_tx_hash,
            version,
            transaction,
            &mut batch_operations,
        )?;

        let grove_db_operations = DriveOperation::grovedb_operations_batch(&batch_operations);
        if !grove_db_operations.is_empty() {
            self.apply_batch_grovedb_operations(
                None,
                transaction,
                grove_db_operations,
                &mut vec![],
            )?;
        }
        Ok(inserted)
    }
    /// Update the validator proposed app version
    /// returns true if the value was changed, or is new
    /// returns false if it was not changed
    pub(crate) fn update_validator_proposed_app_version_operations(
        &self,
        validator_pro_tx_hash: [u8; 32],
        version: ProtocolVersion,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<bool, Error> {
        let mut cache = self.cache.borrow_mut();
        let version_counter = cache
            .protocol_versions_counter
            .get_or_insert(self.fetch_versions_with_counter(transaction)?);

        let path = desired_version_for_validators_path();
        let version_bytes = version.encode_var_vec();
        let version_element = Element::new_item(version_bytes.clone());

        let (value_changed, previous_element) = self.batch_insert_if_changed_value(
            PathKeyElementInfo::PathFixedSizeKeyRefElement((
                path,
                validator_pro_tx_hash.as_slice(),
                version_element,
            )),
            BatchInsertApplyType::StatefulBatchInsert,
            transaction,
            drive_operations,
        )?;

        // if we will insert we need to add it to the version counter
        if value_changed {
            // if we had a different previous version we need to remove it from the version counter
            if let Some(previous_element) = previous_element {
                let previous_version_bytes = previous_element.as_item_bytes().map_err(GroveDB)?;
                let previous_version = ProtocolVersion::decode_var(previous_version_bytes)
                    .ok_or(Error::Drive(DriveError::CorruptedElementType(
                        "encoded value could not be decoded",
                    )))
                    .map(|(value, _)| value)?;
                //we should remove 1 from the previous version
                let previous_count =
                    version_counter
                        .get_mut(&previous_version)
                        .ok_or(Error::Drive(DriveError::CorruptedCacheState(
                            "trying to lower the count of a version from cache that is not found"
                                .to_string(),
                        )))?;
                if previous_count == &0 {
                    return Err(Error::Drive(DriveError::CorruptedCacheState(
                        "trying to lower the count of a version from cache that is already at 0"
                            .to_string(),
                    )));
                }
                *previous_count -= 1;
                self.batch_insert(
                    PathKeyElementInfo::PathFixedSizeKeyRefElement((
                        versions_counter_path(),
                        previous_version_bytes,
                        Element::new_item(previous_count.encode_var_vec()),
                    )),
                    drive_operations,
                )?;
            }

            let version_count = version_counter.entry(version).or_default();
            if version_count == &u64::MAX {
                return Err(Error::Drive(DriveError::CorruptedCacheState(
                    "trying to raise the count of a version from cache that is already at max"
                        .to_string(),
                )));
            }
            *version_count += 1;
            self.batch_insert(
                PathKeyElementInfo::PathFixedSizeKeyRefElement((
                    versions_counter_path(),
                    version_bytes.as_slice(),
                    Element::new_item(version_count.encode_var_vec()),
                )),
                drive_operations,
            )?;
        }

        Ok(value_changed)
    }
}
