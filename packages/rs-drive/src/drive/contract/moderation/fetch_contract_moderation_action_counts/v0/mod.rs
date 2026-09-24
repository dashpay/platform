use crate::drive::contract::moderation::types::decode_moderation_action_count;
use crate::drive::contract::paths::{
    contract_moderation_action_counts_path, contract_moderation_action_counts_path_vec,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::{Query, QueryItem};
use crate::util::grove_operations::DirectQueryType;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, SizedQuery, TransactionArg};
use std::collections::BTreeMap;
use std::ops::RangeFull;

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_contract_moderation_action_counts_add_to_operations_v0(
        &self,
        contract_id: Identifier,
        limit: u16,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, u32>, Error> {
        let mut query = Query::new_with_direction(true);
        query.insert_item(QueryItem::RangeFull(RangeFull));
        let path_query = PathQuery {
            path: contract_moderation_action_counts_path_vec(contract_id.as_slice()),
            query: SizedQuery {
                query,
                limit: Some(limit),
                offset: None,
            },
        };
        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            drive_operations,
            &platform_version.drive,
        )?;
        results
            .to_key_elements()
            .into_iter()
            .map(|(key, element)| {
                let malformed = |description: String| {
                    Error::Drive(DriveError::CorruptedDriveState(format!(
                        "moderation action count of contract {} is malformed: {}",
                        contract_id, description
                    )))
                };
                let identity_id = Identifier::from_bytes(&key)
                    .map_err(|_| malformed(format!("key {:?} is not an identity id", key)))?;
                let Element::Item(value, _) = element else {
                    return Err(malformed("not an item".to_string()));
                };
                Ok((
                    identity_id,
                    decode_moderation_action_count(&value).map_err(malformed)?,
                ))
            })
            .collect()
    }

    #[inline(always)]
    pub(super) fn fetch_contract_moderation_action_count_add_to_operations_v0(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<u32, Error> {
        let path = contract_moderation_action_counts_path(contract_id.as_slice());
        self.grove_get_raw_optional_item(
            (&path).into(),
            identity_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?
        .map(|value| {
            decode_moderation_action_count(&value).map_err(|description| {
                Error::Drive(DriveError::CorruptedDriveState(format!(
                    "moderation action count of {} on contract {} is malformed: {}",
                    identity_id, contract_id, description
                )))
            })
        })
        .transpose()
        .map(Option::unwrap_or_default)
    }
}
