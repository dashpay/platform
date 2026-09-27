//! Contract moderation queries: one identity's status on a moderated contract, one page of a
//! contract's banlist, suspension list or warning list, and the fee pots a contract's
//! document action fees collect in.

mod contract_document_removals;
mod contract_fee_pots;
mod contract_moderation_entries;
mod contract_moderation_status;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dapi_grpc::platform::v0::ContractModerationDocument as ContractModerationDocumentProto;
use dapi_grpc::platform::v0::ContractModerationList as ContractModerationListProto;
use dapi_grpc::platform::v0::ContractModerationReason as ContractModerationReasonProto;
use dapi_grpc::platform::v0::ContractWarning as ContractWarningProto;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::{
    ContractModerationList, ContractModerationReason, ContractWarning,
};
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
        Ok(ContractModerationListProto::Warnings) => Ok(ContractModerationList::Warnings),
        // Zero is what a proto3 client sends when it leaves the field out: not a list.
        Ok(ContractModerationListProto::Unspecified) | Err(_) => Err(QueryError::InvalidArgument(
            format!("{field} {list} is not a moderation list"),
        )),
    }
}

/// The wire number of a list.
pub(super) fn list_to_request(list: ContractModerationList) -> i32 {
    match list {
        ContractModerationList::Banlist => ContractModerationListProto::Banlist as i32,
        ContractModerationList::Suspensions => ContractModerationListProto::Suspensions as i32,
        ContractModerationList::Warnings => ContractModerationListProto::Warnings as i32,
    }
}

/// A reason as the wire carries it.
pub(super) fn reason_to_response(
    reason: ContractModerationReason,
) -> ContractModerationReasonProto {
    ContractModerationReasonProto {
        code: reason.code.map(u32::from),
        text: reason.text,
        documents: reason
            .documents
            .into_iter()
            .map(|document| ContractModerationDocumentProto {
                document_type_name: document.document_type_name,
                document_id: document.document_id.to_vec(),
            })
            .collect(),
        reason_document_id: reason.reason_document_id.map(|id| id.to_vec()),
    }
}

/// Warnings as the wire carries them, oldest first as stored.
pub(super) fn warnings_to_response(warnings: Vec<ContractWarning>) -> Vec<ContractWarningProto> {
    warnings
        .into_iter()
        .map(|warning| ContractWarningProto {
            warned_at: warning.warned_at,
            reason: Some(reason_to_response(warning.reason)),
        })
        .collect()
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
    use dpp::data_contract::config::moderation::{
        ContractModerationConfig, ContractModerationList, ContractModerationReason,
        ContractModerators, ContractWarning,
    };
    use dpp::data_contract::DataContract;
    use dpp::identifier::Identifier;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;

    pub const BANLIST: i32 = 1;
    pub const SUSPENSIONS: i32 = 2;
    pub const WARNINGS: i32 = 3;
    /// The reason [`ban`] gives.
    pub const BAN_REASON: &str = "spam";
    /// The reason [`suspend`] gives, with a code.
    pub const SUSPENSION_REASON: &str = "flooding";
    pub const SUSPENSION_REASON_CODE: u16 = 7;
    /// The reason [`warn`] gives.
    pub const WARNING_REASON: &str = "first strike";

    /// Stores a contract that keeps the lists asked for (none: an unmoderated contract).
    pub fn store_contract(
        platform: &TempPlatform<MockCoreRPCLike>,
        banlist: bool,
        suspensions: bool,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        store_contract_keeping(platform, banlist, suspensions, false, platform_version)
    }

    /// Stores a contract that keeps the lists asked for, the warning list included (none: an
    /// unmoderated contract).
    pub fn store_contract_keeping(
        platform: &TempPlatform<MockCoreRPCLike>,
        banlist: bool,
        suspensions: bool,
        warnings: bool,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let mut contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let moderation = (banlist || suspensions || warnings).then_some(ContractModerationConfig {
            banlist,
            suspensions,
            warnings,
            moderators: ContractModerators::ContractOwner,
        });
        contract.set_config(contract.config().clone().with_moderation(moderation));
        store_data_contract(platform, &contract, platform_version);
        contract
    }

    /// Warns `target` at block time `warned_at`, on top of the warnings it carries.
    pub fn warn(
        platform: &TempPlatform<MockCoreRPCLike>,
        contract: &DataContract,
        target: Identifier,
        warned_at: u64,
        platform_version: &PlatformVersion,
    ) {
        let mut warnings = platform
            .drive
            .fetch_contract_moderation_status(
                contract.id(),
                target,
                &[ContractModerationList::Warnings],
                None,
                platform_version,
            )
            .expect("expected to read the warnings")
            .warnings;
        let replaces_existing = !warnings.is_empty();
        warnings.push(ContractWarning {
            warned_at,
            reason: ContractModerationReason::from_text(WARNING_REASON),
        });
        platform
            .drive
            .add_contract_warning(
                contract.id(),
                target,
                &warnings,
                replaces_existing,
                contract.owner_id(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to warn");
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
                &ContractModerationReason::from_text(BAN_REASON),
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
                &ContractModerationReason {
                    code: Some(SUSPENSION_REASON_CODE),
                    text: SUSPENSION_REASON.to_string(),
                    documents: vec![],
                    reason_document_id: None,
                },
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
        assert_eq!(
            list_from_request(WARNINGS, "list").unwrap(),
            ContractModerationList::Warnings
        );
        assert!(list_from_request(7, "list").is_err());
        // The proto3 default, an omitted field, is refused rather than read as the banlist.
        assert!(list_from_request(0, "list").is_err());
    }
}
