use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::{Epoch, EpochIndex, EPOCH_KEY_OFFSET};
use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::ProtocolError;
use grovedb::query_result_type::{QueryResultElement, QueryResultType};
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
use std::collections::BTreeMap;
use std::u64;

use crate::drive::credit_pools::pools_vec_path;
use crate::error::query::QuerySyntaxError;
use crate::fee_pools::epochs::epoch_key_constants::{
    KEY_FEE_MULTIPLIER, KEY_START_BLOCK_CORE_HEIGHT, KEY_START_BLOCK_HEIGHT, KEY_START_TIME,
};
use crate::query::QueryItem;
use dpp::version::PlatformVersion;

impl Drive {
    pub(super) fn get_epochs_infos_v0(
        &self,
        start_epoch_index: u16,
        count: u16,
        ascending: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<ExtendedEpochInfo>, Error> {
        if count > 16383 {
            return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                "get_epochs_infos_v0 count too high {}",
                count
            ))));
        }
        let index_with_offset = start_epoch_index
            .checked_add(EPOCH_KEY_OFFSET)
            .ok_or(ProtocolError::Overflow("stored epoch index too high"))?;
        let mut subquery = Query::new();
        subquery.insert_keys(vec![
            KEY_START_TIME.to_vec(),
            KEY_START_BLOCK_HEIGHT.to_vec(),
            KEY_START_BLOCK_CORE_HEIGHT.to_vec(),
            KEY_FEE_MULTIPLIER.to_vec(),
        ]);
        let mut query = if ascending {
            Query::new_single_query_item(QueryItem::RangeFrom(
                index_with_offset.to_be_bytes().to_vec()..,
            ))
        } else {
            Query::new_single_query_item(QueryItem::RangeToInclusive(
                ..=index_with_offset.to_be_bytes().to_vec(),
            ))
        };
        query.left_to_right = ascending;
        query.set_subquery(subquery);
        let path_query = PathQuery::new(
            pools_vec_path(),
            SizedQuery::new(query, Some(count * 4), None),
        );

        let results = self
            .grove_get_path_query(
                &path_query,
                transaction,
                QueryResultType::QueryPathKeyElementTrioResultType,
                &mut vec![],
                &platform_version.drive,
            )?
            .0;

        let results = results.elements.into_iter().fold(
            BTreeMap::<_, BTreeMap<_, _>>::new(),
            |mut acc, result_item| {
                if let QueryResultElement::PathKeyElementTrioResultItem((path, key, element)) =
                    result_item
                {
                    acc.entry(path)
                        .or_insert_with(BTreeMap::new)
                        .insert(key, element);
                }
                acc
            },
        );

        // Convert the BTreeMap entries to ExtendedEpochInfo
        let extended_epoch_infos = results
            .into_iter()
            .filter_map(|(path, inner_map)| {
                // Extract the epoch index from the path's last component
                // and adjust by subtracting the EPOCH_KEY_OFFSET
                let epoch_index_result: Result<EpochIndex, Error> = path
                    .last()
                    .ok_or(Error::Drive(DriveError::CorruptedSerialization(
                        "extended epoch info: path can not be empty".to_string(),
                    )))
                    .and_then(|epoch_index_vec| {
                        epoch_index_vec.as_slice().try_into().map_err(|_| {
                            Error::Drive(DriveError::CorruptedSerialization(
                                "extended epoch info: item has an invalid length".to_string(),
                            ))
                        })
                    })
                    .and_then(|epoch_index_bytes| {
                        EpochIndex::from_be_bytes(epoch_index_bytes)
                            .checked_sub(EPOCH_KEY_OFFSET)
                            .ok_or(Error::Drive(DriveError::CorruptedSerialization(
                                "epoch bytes on disk too small, should be over epoch key offset"
                                    .to_string(),
                            )))
                    });

                let epoch_index = match epoch_index_result {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e)),
                };

                let first_block_time_element = inner_map.get(&KEY_START_TIME.to_vec())?;

                let Element::Item(encoded_start_time, _) = first_block_time_element else {
                    return Some(Err(Error::Drive(DriveError::UnexpectedElementType(
                        "start time must be an item",
                    ))));
                };

                let first_block_time_bytes: [u8; 8] =
                    match encoded_start_time.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "extended epoch info: block time must be 8 bytes for a u64".to_string(),
                        ))
                    }) {
                        Ok(value) => value,
                        Err(e) => return Some(Err(e)),
                    };

                let first_block_time = u64::from_be_bytes(first_block_time_bytes);

                let first_block_height_element = inner_map.get(&KEY_START_BLOCK_HEIGHT.to_vec())?;

                let Element::Item(encoded_start_block_height, _) = first_block_height_element
                else {
                    return Some(Err(Error::Drive(DriveError::UnexpectedElementType(
                        "extended epoch info: start time must be an item",
                    ))));
                };

                let first_block_height_bytes: [u8; 8] = match encoded_start_block_height
                    .as_slice()
                    .try_into()
                    .map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "extended epoch info: block height must be 8 bytes for a u64"
                                .to_string(),
                        ))
                    }) {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e)),
                };

                let first_block_height = u64::from_be_bytes(first_block_height_bytes);

                let first_core_block_height_element =
                    inner_map.get(&KEY_START_BLOCK_CORE_HEIGHT.to_vec())?;

                let Element::Item(encoded_start_core_block_height, _) =
                    first_core_block_height_element
                else {
                    return Some(Err(Error::Drive(DriveError::UnexpectedElementType(
                        "start time must be an item",
                    ))));
                };

                let first_core_block_height_bytes: [u8; 4] = match encoded_start_core_block_height
                    .as_slice()
                    .try_into()
                    .map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "core block height must be 4 bytes for a u32".to_string(),
                        ))
                    }) {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e)),
                };

                let first_core_block_height = u32::from_be_bytes(first_core_block_height_bytes);

                let fee_multiplier_element = inner_map.get(&KEY_FEE_MULTIPLIER.to_vec())?;

                let Element::Item(encoded_multiplier, _) = fee_multiplier_element else {
                    return Some(Err(Error::Drive(DriveError::UnexpectedElementType(
                        "epochs multiplier must be an item",
                    ))));
                };

                let fee_multiplier_bytes: [u8; 8] =
                    match encoded_multiplier.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "fee multiplier must be 8 bytes for a f64".to_string(),
                        ))
                    }) {
                        Ok(value) => value,
                        Err(e) => return Some(Err(e)),
                    };

                let fee_multiplier = f64::from_be_bytes(fee_multiplier_bytes);

                // Construct the ExtendedEpochInfo
                Some(Ok(ExtendedEpochInfoV0 {
                    index: epoch_index,
                    first_block_time,
                    first_block_height,
                    first_core_block_height,
                    fee_multiplier,
                }
                .into()))
            })
            .collect::<Result<Vec<ExtendedEpochInfo>, Error>>()?;

        Ok(extended_epoch_infos)
    }
}
