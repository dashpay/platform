use crate::drive::contract::moderation::types::{
    ContractModerationEntriesQuery, ContractModerationEntry,
};
use crate::drive::contract::paths::{contract_moderation_list_key, contract_other_path};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_contract_moderation_entries_v0(
        &self,
        contract_id: Identifier,
        query: &ContractModerationEntriesQuery,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<ContractModerationEntry>, Error> {
        Self::check_contract_moderation_entries_limit(query.limit, platform_version)?;

        // A path query over a missing list tree is an error in GroveDB, so check first. The
        // check itself reads under the contract's other tree, which a contract id nobody has
        // (or a contract stored before the tree existed) does not have either: that reads as
        // no list, like the list's own absence.
        let exists = match self.grove_has_raw(
            (&contract_other_path(contract_id.as_slice())).into(),
            contract_moderation_list_key(query.list),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        ) {
            Ok(exists) => exists,
            Err(Error::GroveDB(error))
                if matches!(
                    *error,
                    grovedb::Error::PathParentLayerNotFound(_) | grovedb::Error::PathNotFound(_)
                ) =>
            {
                false
            }
            Err(error) => return Err(error),
        };
        if !exists {
            return Ok(vec![]);
        }

        let path_query = Self::contract_moderation_entries_query(contract_id.to_buffer(), query);
        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        results
            .to_key_elements()
            .into_iter()
            .map(|(key, element)| {
                ContractModerationEntry::from_key_element(query.list, &key, &element).map_err(
                    |description| {
                        Error::Drive(DriveError::CorruptedDriveState(format!(
                            "contract {} {} is malformed: {}",
                            contract_id, query.list, description
                        )))
                    },
                )
            })
            .collect()
    }
}
