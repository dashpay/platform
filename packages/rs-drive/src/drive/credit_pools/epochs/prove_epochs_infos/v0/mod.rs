use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::{Epoch, EpochIndex, EPOCH_KEY_OFFSET};
use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0;
use dpp::ProtocolError;
use grovedb::query_result_type::QueryResultElement;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
use std::collections::BTreeMap;

use crate::drive::credit_pools::pools_vec_path;
use crate::error::query::QuerySyntaxError;
use crate::fee_pools::epochs::epoch_key_constants::{
    KEY_FEE_MULTIPLIER, KEY_START_BLOCK_CORE_HEIGHT, KEY_START_BLOCK_HEIGHT, KEY_START_TIME,
};
use crate::query::QueryItem;
use dpp::version::PlatformVersion;

impl Drive {
    pub(super) fn prove_epochs_infos_v0(
        &self,
        start_epoch_index: u16,
        count: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
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
        let mut query = Query::new_single_query_item(QueryItem::RangeFrom(
            index_with_offset.to_be_bytes().to_vec()..,
        ));
        query.set_subquery(subquery);
        let path_query = PathQuery::new(
            pools_vec_path(),
            SizedQuery::new(query, Some(count * 4), None),
        );

        self.grove_get_proved_path_query(
            &path_query,
            false,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
