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
        let results = match self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok((results, _)) => results,
            // An elected contract stored before the counts existed has no tree: no counts. The
            // cost of the lookup is billed all the same.
            Err(error) if is_missing_counts_tree(&error) => return Ok(BTreeMap::new()),
            Err(error) => return Err(error),
        };
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
    ) -> Result<Option<u32>, Error> {
        let path = contract_moderation_action_counts_path(contract_id.as_slice());
        // The plain read tells a member without a count (the key is missing) from a contract
        // without the tree (its parent is), at the cost of the one read the optional read
        // would make: GroveDB's optional read finds nothing either way.
        let element = match self.grove_get_raw(
            (&path).into(),
            identity_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(element) => element,
            Err(Error::GroveDB(error)) if matches!(*error, grovedb::Error::PathKeyNotFound(_)) => {
                return Ok(Some(0))
            }
            // An elected contract stored before the counts existed has no tree to count in.
            // The cost of the lookup is billed all the same.
            Err(error) if is_missing_counts_tree(&error) => return Ok(None),
            Err(error) => return Err(error),
        };
        let malformed = |description: String| {
            Error::Drive(DriveError::CorruptedDriveState(format!(
                "moderation action count of {} on contract {} is malformed: {}",
                identity_id, contract_id, description
            )))
        };
        match element {
            Some(Element::Item(value, _)) => decode_moderation_action_count(&value)
                .map(Some)
                .map_err(malformed),
            Some(_) => Err(malformed("not an item".to_string())),
            None => Ok(Some(0)),
        }
    }
}

/// Whether `error` is GroveDB not finding the counts tree of a contract: one stored, elected
/// already, before protocol version 14 counted actions (a development network's), which
/// `insert_contract_moderation_trees` did not give one.
fn is_missing_counts_tree(error: &Error) -> bool {
    matches!(
        error,
        Error::GroveDB(error) if matches!(
            **error,
            grovedb::Error::PathParentLayerNotFound(_)
                | grovedb::Error::PathNotFound(_)
                | grovedb::Error::InvalidParentLayerPath(_)
        )
    )
}
