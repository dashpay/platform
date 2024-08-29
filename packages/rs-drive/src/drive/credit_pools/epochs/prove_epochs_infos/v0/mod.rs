use crate::drive::Drive;

use crate::error::Error;
use dpp::block::epoch::EPOCH_KEY_OFFSET;

use dpp::ProtocolError;

use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};

use crate::drive::credit_pools::epochs::epoch_key_constants::{
    KEY_FEE_MULTIPLIER, KEY_PROTOCOL_VERSION, KEY_START_BLOCK_CORE_HEIGHT, KEY_START_BLOCK_HEIGHT,
    KEY_START_TIME,
};
use crate::drive::credit_pools::pools_vec_path;
use crate::error::query::QuerySyntaxError;
use crate::query::QueryItem;
use dpp::version::PlatformVersion;

impl Drive {
    pub(super) fn prove_epochs_infos_v0(
        &self,
        start_epoch_index: u16,
        count: u16,
        ascending: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        // TODO: We should avoid magic numbers. For now we are good since count refers to the number of epochs to fetch and 16383 is large enough.
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
            KEY_PROTOCOL_VERSION.to_vec(),
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
            // The multiplier must be equal to requested keys count
            SizedQuery::new(query, Some(count * 5), None),
        );

        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
