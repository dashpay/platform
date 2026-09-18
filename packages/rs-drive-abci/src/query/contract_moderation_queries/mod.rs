//! Contract moderation queries: one identity's status on a moderated contract, and one page of
//! a contract's banlist or suspension list.

mod contract_moderation_entries;
mod contract_moderation_status;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dapi_grpc::platform::v0::ContractModerationList as ContractModerationListProto;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

/// Parses a 32 byte identifier out of a request field, naming the field in the error.
pub(super) fn identifier_from_request(
    bytes: Vec<u8>,
    field: &str,
) -> Result<Identifier, QueryError> {
    Identifier::from_bytes(&bytes).map_err(|_| {
        QueryError::InvalidArgument(format!(
            "{field} must be a valid identifier (32 bytes long)"
        ))
    })
}

/// The list a request names, or the error when the number is not one.
pub(super) fn list_from_request(
    list: i32,
    field: &str,
) -> Result<ContractModerationList, QueryError> {
    match ContractModerationListProto::try_from(list) {
        Ok(ContractModerationListProto::Banlist) => Ok(ContractModerationList::Banlist),
        Ok(ContractModerationListProto::Suspensions) => Ok(ContractModerationList::Suspensions),
        Err(_) => Err(QueryError::InvalidArgument(format!(
            "{field} {list} is not a moderation list"
        ))),
    }
}

impl<C> Platform<C> {
    /// The moderation lists the contract keeps, or a query error when the contract does not
    /// exist or keeps no list. A list the contract does not keep has no tree, so a query over
    /// it is refused here rather than failing in GroveDB.
    pub(super) fn kept_moderation_lists(
        &self,
        contract_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Result<Vec<ContractModerationList>, QueryError>, Error> {
        let Some(contract_fetch_info) = self.drive.get_contract_with_fetch_info(
            contract_id.to_buffer(),
            false,
            None,
            platform_version,
        )?
        else {
            return Ok(Err(QueryError::NotFound(format!(
                "contract {} not found",
                contract_id
            ))));
        };
        let Some(moderation) = contract_fetch_info.contract.config().moderation() else {
            return Ok(Err(QueryError::InvalidArgument(format!(
                "contract {} is not moderated",
                contract_id
            ))));
        };
        let _ = contract_fetch_info.contract.owner_id();
        Ok(Ok(moderation.lists().collect()))
    }
}
