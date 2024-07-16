use crate::drive::credit_pools::epochs::start_block::StartBlockInfo;
use crate::drive::credit_pools::paths::pools_vec_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::EpochIndex;

use crate::drive::credit_pools::epochs;
use crate::drive::credit_pools::epochs::epoch_key_constants::{
    KEY_START_BLOCK_CORE_HEIGHT, KEY_START_BLOCK_HEIGHT,
};
use grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Returns the index and start block platform and core heights of the first epoch between
    /// the two given.
    pub(super) fn get_first_epoch_start_block_info_between_epochs_v0(
        &self,
        from_epoch_index: EpochIndex,
        to_epoch_index: EpochIndex,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<StartBlockInfo>, Error> {
        let mut start_block_height_query = Query::new();
        start_block_height_query.insert_key(KEY_START_BLOCK_HEIGHT.to_vec());
        start_block_height_query.insert_key(KEY_START_BLOCK_CORE_HEIGHT.to_vec());

        let mut epochs_query = Query::new();

        let from_epoch_key = epochs::paths::encode_epoch_index_key(from_epoch_index)?.to_vec();
        let current_epoch_key = epochs::paths::encode_epoch_index_key(to_epoch_index)?.to_vec();

        epochs_query.insert_range_after_to_inclusive(from_epoch_key..=current_epoch_key);

        epochs_query.set_subquery(start_block_height_query);

        let sized_query = SizedQuery::new(epochs_query, Some(2), None);

        let path_query = PathQuery::new(pools_vec_path(), sized_query);

        let (result_items, _) = self
            .grove
            .query_raw(
                &path_query,
                transaction.is_some(),
                false, //set to false on purpose
                true,
                QueryPathKeyElementTrioResultType,
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if result_items.is_empty() {
            return Ok(None);
        }

        let mut path_key_elements = result_items.to_path_key_elements().into_iter();

        let (_, key, element) = path_key_elements.next().unwrap();

        if key != KEY_START_BLOCK_CORE_HEIGHT.to_vec() {
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "start block core height should exist".to_string(),
            )));
        }

        let Element::Item(item, _) = element else {
            return Err(Error::Drive(DriveError::UnexpectedElementType(
                "start block core height must be an item",
            )));
        };

        let next_start_block_core_height =
            u32::from_be_bytes(item.as_slice().try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedSerialization(String::from(
                    "start block core height must be u32",
                )))
            })?);

        let (path, key, element) = path_key_elements.next().unwrap();

        if key != KEY_START_BLOCK_HEIGHT.to_vec() {
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "start block height should exist".to_string(),
            )));
        }

        let Element::Item(item, _) = element else {
            return Err(Error::Drive(DriveError::UnexpectedElementType(
                "start block must be an item",
            )));
        };

        let next_start_block_height =
            u64::from_be_bytes(item.as_slice().try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedSerialization(String::from(
                    "start block height must be u64",
                )))
            })?);

        let epoch_key = path
            .last()
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "epoch pool shouldn't have empty path",
            )))?;

        let epoch_index = epochs::paths::decode_epoch_index_key(epoch_key.as_slice())?;

        Ok(Some(StartBlockInfo {
            epoch_index,
            start_block_height: next_start_block_height,
            start_block_core_height: next_start_block_core_height,
        }))
    }
}
