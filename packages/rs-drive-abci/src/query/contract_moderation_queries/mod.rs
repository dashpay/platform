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
        // Zero is what a proto3 client sends when it leaves the field out: not a list.
        Ok(ContractModerationListProto::Unspecified) | Err(_) => Err(QueryError::InvalidArgument(
            format!("{field} {list} is not a moderation list"),
        )),
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
        Ok(Ok(moderation.lists().collect()))
    }
}

#[cfg(test)]
pub(super) mod tests {
    use crate::query::tests::store_data_contract;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
    use dpp::data_contract::DataContract;
    use dpp::identifier::Identifier;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;

    pub const BANLIST: i32 = 1;
    pub const SUSPENSIONS: i32 = 2;

    /// Stores a contract that keeps the lists asked for (none: an unmoderated contract).
    pub fn store_contract(
        platform: &TempPlatform<MockCoreRPCLike>,
        banlist: bool,
        suspensions: bool,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let mut contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let moderation = (banlist || suspensions).then_some(ContractModerationConfig {
            banlist,
            suspensions,
            moderators: ContractModerators::ContractOwner,
        });
        contract.set_config(contract.config().clone().with_moderation(moderation));
        store_data_contract(platform, &contract, platform_version);
        contract
    }

    pub fn ban(
        platform: &TempPlatform<MockCoreRPCLike>,
        contract: &DataContract,
        target: Identifier,
        platform_version: &PlatformVersion,
    ) {
        platform
            .drive
            .add_contract_ban(
                contract.id(),
                target,
                contract.owner_id(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to ban");
    }

    pub fn suspend(
        platform: &TempPlatform<MockCoreRPCLike>,
        contract: &DataContract,
        target: Identifier,
        until: u64,
        platform_version: &PlatformVersion,
    ) {
        platform
            .drive
            .add_contract_suspension(
                contract.id(),
                target,
                until,
                false,
                contract.owner_id(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to suspend");
    }

    #[test]
    fn should_keep_the_list_numbers_of_the_wire_enum() {
        use super::{list_from_request, ContractModerationList};
        assert_eq!(
            list_from_request(BANLIST, "list").unwrap(),
            ContractModerationList::Banlist
        );
        assert_eq!(
            list_from_request(SUSPENSIONS, "list").unwrap(),
            ContractModerationList::Suspensions
        );
        assert!(list_from_request(7, "list").is_err());
        // The proto3 default, an omitted field, is refused rather than read as the banlist.
        assert!(list_from_request(0, "list").is_err());
    }
}
