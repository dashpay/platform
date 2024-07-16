use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::{EpochIndex, EPOCH_KEY_OFFSET};
use dpp::util::deserializer::ProtocolVersion;
use dpp::ProtocolError;
use grovedb::query_result_type::{QueryResultElement, QueryResultType};
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
use std::collections::BTreeMap;

use crate::drive::credit_pools::epochs::epoch_key_constants::KEY_PROTOCOL_VERSION;
use crate::drive::credit_pools::pools_vec_path;
use crate::error::query::QuerySyntaxError;
use crate::query::QueryItem;
use dpp::version::PlatformVersion;

impl Drive {
    pub(super) fn get_epochs_protocol_versions_v0(
        &self,
        start_epoch_index: u16,
        count: Option<u16>,
        ascending: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<EpochIndex, ProtocolVersion>, Error> {
        if let Some(count) = count {
            // TODO: We should avoid magic numbers. For now we are good since count refers to the number of epochs to fetch and 16383 is large enough.
            if count > 16383 {
                return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                    "get_epochs_protocol_versions_v0 count too high {}",
                    count
                ))));
            }
        }
        let index_with_offset = start_epoch_index
            .checked_add(EPOCH_KEY_OFFSET)
            .ok_or(ProtocolError::Overflow("stored epoch index too high"))?;
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
        query.set_subquery_path(vec![KEY_PROTOCOL_VERSION.to_vec()]);
        let path_query = PathQuery::new(
            pools_vec_path(),
            // The multiplier must be equal to requested keys count
            SizedQuery::new(query, count, None),
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

        let mut map: BTreeMap<EpochIndex, ProtocolVersion> = BTreeMap::default();

        for result_item in results.elements.into_iter() {
            if let QueryResultElement::PathKeyElementTrioResultItem((
                mut path,
                _,
                protocol_version_element,
            )) = result_item
            {
                let Some(epoch_index_vec) = path.pop() else {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                        "path must be bigger than empty".to_string(),
                    )));
                };

                let epoch_index_bytes: [u8; 32] =
                    epoch_index_vec.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "extended epoch info: item has an invalid length".to_string(),
                        ))
                    })?;
                let epoch_index =
                    EpochIndex::from_be_bytes([epoch_index_bytes[0], epoch_index_bytes[1]])
                        .checked_sub(EPOCH_KEY_OFFSET)
                        .ok_or(Error::Drive(DriveError::CorruptedSerialization(
                            "epoch bytes on disk too small, should be over epoch key offset"
                                .to_string(),
                        )))?;

                let Element::Item(encoded_protocol_version, _) = protocol_version_element else {
                    return Err(Error::Drive(DriveError::UnexpectedElementType(
                        "protocol version must be an item",
                    )));
                };

                let protocol_version_bytes: [u8; 4] = encoded_protocol_version
                    .as_slice()
                    .try_into()
                    .map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "protocol version must be 4 bytes for a u32".to_string(),
                        ))
                    })?;

                let protocol_version = u32::from_be_bytes(protocol_version_bytes);

                map.insert(epoch_index, protocol_version);
            }
        }
        Ok(map)
    }
}
