//! Contract moderation queries: one identity's status on a moderated contract, one page of a
//! contract's banlist, suspension list or warning list, the records of the documents its
//! moderators deleted, the actions its seated team votes on, how many moderation actions each
//! member of that team signed since the last payout, and the fee pots a contract's document
//! action fees collect in.

mod contract_document_removals;
mod contract_fee_pots;
mod contract_moderation_action_counts;
mod contract_moderation_entries;
mod contract_moderation_status;
mod contract_team_action_signers;
mod contract_team_actions;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dapi_grpc::platform::v0::get_contract_document_removals_response::{
    ContractDocumentRemoval as ContractDocumentRemovalProto,
    ContractDocumentRestoration as ContractDocumentRestorationProto,
};
use dapi_grpc::platform::v0::get_contract_team_actions_response::{
    contract_team_action, ContractTeamAction as ContractTeamActionProto,
    DeleteSettledDocument as DeleteSettledDocumentProto,
};
use dapi_grpc::platform::v0::ContractModerationDocument as ContractModerationDocumentProto;
use dapi_grpc::platform::v0::ContractModerationList as ContractModerationListProto;
use dapi_grpc::platform::v0::ContractModerationReason as ContractModerationReasonProto;
use dapi_grpc::platform::v0::ContractWarning as ContractWarningProto;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::ContractTeamActionEvent;
use dpp::data_contract::config::moderation::{
    ContractModerationList, ContractModerationReason, ContractWarning,
};
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::contract::moderation::types::{
    ContractDocumentRemovalEntry, ContractTeamActionEntry,
};
use drive::drive::contract::DataContractFetchInfo;
use std::sync::Arc;

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

/// A document removal record as the wire carries it: the one shape the removals query and a
/// join through a `moderatedDocument` reference answer with. The fields it keeps go out as the
/// record stores them, encoded as the document encoded its properties, for the reader to read
/// under the document's type as it reads documents.
pub(super) fn removal_entry_to_response(
    entry: ContractDocumentRemovalEntry,
) -> ContractDocumentRemovalProto {
    ContractDocumentRemovalProto {
        document_id: entry.document_id.to_vec(),
        document_owner_id: entry.removal.document_owner_id.to_vec(),
        moderator_id: entry.removal.moderator_id.to_vec(),
        removed_at: entry.removal.removed_at,
        reason: Some(reason_to_response(entry.removal.reason)),
        document_hash: entry.removal.document_hash.to_vec(),
        restoration: entry.removal.restoration.map(|restoration| {
            ContractDocumentRestorationProto {
                moderator_id: restoration.moderator_id.to_vec(),
                restored_at: restoration.restored_at,
            }
        }),
        kept_fields: entry.removal.kept_fields,
    }
}

/// A team action as the wire carries it.
pub(super) fn team_action_to_response(entry: ContractTeamActionEntry) -> ContractTeamActionProto {
    let ContractTeamActionEvent::DeleteSettledDocument {
        document_type_name,
        document_id,
        document_last_modified_at,
        document_revision,
        reason,
    } = entry.action.event;
    ContractTeamActionProto {
        action_id: entry.action_id.to_vec(),
        proposer_id: entry.action.proposer_id.to_vec(),
        proposed_at: entry.action.proposed_at,
        event: Some(contract_team_action::Event::DeleteSettledDocument(
            DeleteSettledDocumentProto {
                document_type_name,
                document_id: document_id.to_vec(),
                document_last_modified_at,
                document_revision,
                reason: Some(reason_to_response(reason)),
            },
        )),
        approval_count: entry.approval_count,
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
    /// The contract a moderation query names, or `NotFound` when there is none: the one read
    /// every moderation query's check makes before refusing a contract without the tree it
    /// reads.
    fn fetch_queried_contract(
        &self,
        contract_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Result<Arc<DataContractFetchInfo>, QueryError>, Error> {
        Ok(self
            .drive
            .get_contract_with_fetch_info(contract_id.to_buffer(), false, None, platform_version)?
            .ok_or_else(|| QueryError::NotFound(format!("contract {} not found", contract_id))))
    }

    /// The moderation lists the contract keeps, or a query error when the contract does not
    /// exist or keeps no list. A list the contract does not keep has no tree, so a query over
    /// it is refused here rather than failing in GroveDB.
    pub(super) fn kept_moderation_lists(
        &self,
        contract_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Result<Vec<ContractModerationList>, QueryError>, Error> {
        let contract_fetch_info =
            match self.fetch_queried_contract(contract_id, platform_version)? {
                Ok(contract_fetch_info) => contract_fetch_info,
                Err(error) => return Ok(Err(error)),
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

impl<C> Platform<C> {
    /// Nothing, or a query error when the contract does not exist or keeps no moderation action
    /// counts: only an elected contract, whose seated team shares the moderators pot by them,
    /// has a counts tree, and a proof over a tree that does not exist could not be built.
    pub(super) fn check_keeps_moderation_action_counts(
        &self,
        contract_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Result<(), QueryError>, Error> {
        let contract_fetch_info =
            match self.fetch_queried_contract(contract_id, platform_version)? {
                Ok(contract_fetch_info) => contract_fetch_info,
                Err(error) => return Ok(Err(error)),
            };
        let elected = contract_fetch_info
            .contract
            .config()
            .moderation()
            .is_some_and(|moderation| moderation.moderators.elected().is_some());
        if !elected {
            return Ok(Err(QueryError::InvalidArgument(format!(
                "contract {} keeps no moderation action counts: its moderators are not an \
                 elected team",
                contract_id
            ))));
        }
        // An elected contract stored before protocol version 14 counted actions (a development
        // network's) has no counts tree: no proof of it could be built, so it is refused with
        // or without one, rather than answered empty unproved and failing proved.
        if !self.drive.contract_keeps_moderation_action_counts(
            contract_id,
            None,
            platform_version,
        )? {
            return Ok(Err(QueryError::InvalidArgument(format!(
                "contract {} keeps no moderation action counts: it was stored before its \
                 team's actions were counted",
                contract_id
            ))));
        }
        Ok(Ok(()))
    }
}

impl<C> Platform<C> {
    /// Nothing, or a query error when the contract does not exist or keeps no team actions:
    /// only a contract with a document type that sets `moderatorAbilities.deleteSettled` has a
    /// team actions tree, and a proof over a tree that does not exist could not be built.
    pub(super) fn check_keeps_team_actions(
        &self,
        contract_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Result<(), QueryError>, Error> {
        let contract_fetch_info =
            match self.fetch_queried_contract(contract_id, platform_version)? {
                Ok(contract_fetch_info) => contract_fetch_info,
                Err(error) => return Ok(Err(error)),
            };
        if !contract_fetch_info.contract.keeps_team_actions() {
            return Ok(Err(QueryError::InvalidArgument(format!(
                "contract {} keeps no team actions: none of its document types lets a seated \
                 team delete its settled documents",
                contract_id
            ))));
        }
        Ok(Ok(()))
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
    use dpp::data_contract::config::moderation::{
        ContractTeamAction, ContractTeamActionEvent, ElectedModerators, InterimModerators,
        ModerationAbility, DEFAULT_ELECTION_WINDOW_SECONDS,
    };
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::data_contract::DataContract;
    use dpp::identifier::Identifier;
    use dpp::platform_value::platform_value;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use drive::drive::contract::moderation::types::ContractTeamActionWrite;
    use drive::drive::Drive;
    use drive::util::batch::{ContractModerationOperationType, DriveOperation};
    use std::collections::{BTreeMap, BTreeSet};

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

    const SETTLED_POST: &str = "post";

    /// A contract whose elected team deletes settled posts once its leader approves
    pub fn contract_with_settled_posts() -> DataContract {
        let platform_version = PlatformVersion::latest();
        let mut contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_config(contract.config().clone().with_moderation(Some(
            ContractModerationConfig {
                banlist: false,
                suspensions: false,
                moderators: ContractModerators::Elected(Box::new(ElectedModerators {
                    join_window: DEFAULT_ELECTION_WINDOW_SECONDS,
                    vote_window: DEFAULT_ELECTION_WINDOW_SECONDS,
                    challenge_cool_down: None,
                    election_delay: None,
                    max_added_moderators: 0,
                    moderated_document_types: BTreeMap::from([(
                        SETTLED_POST.to_string(),
                        BTreeSet::from([ModerationAbility::DeleteDocuments]),
                    )]),
                    interim: InterimModerators::ContractOwner,
                    owner_protected: false,
                })),
                warnings: false,
            },
        )));
        contract
            .set_document_schema(
                SETTLED_POST,
                platform_value!({
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "maxLength": 50, "position": 0 },
                    },
                    "required": ["$createdAt", "$updatedAt"],
                    "additionalProperties": false,
                    "moderatorAbilities": {
                        "delete": true,
                        "deleteWithin": 86400,
                        "deleteSettled": { "leader": true },
                    },
                }),
                true,
                &mut vec![],
                platform_version,
            )
            .expect("expected to add the post type");
        contract
    }

    /// The proposal of the deletion of post `seed`
    pub fn settled_deletion_proposal(seed: u8) -> ContractTeamAction {
        ContractTeamAction {
            proposer_id: Identifier::from([0x77; 32]),
            proposed_at: 1_000 + seed as u64,
            event: ContractTeamActionEvent::DeleteSettledDocument {
                document_type_name: SETTLED_POST.to_string(),
                document_id: Identifier::from([seed; 32]),
                document_last_modified_at: 10 + seed as u64,
                // Every third one is of a type whose documents carry no revision.
                document_revision: (!seed.is_multiple_of(3)).then_some(seed as u64),
                reason: ContractModerationReason {
                    code: Some(seed as u16),
                    text: "doxxing".to_string(),
                    documents: vec![],
                    reason_document_id: None,
                },
            },
        }
    }

    /// Proposes the deletion of post `seed` as action `seed`, closed at once when `closes`
    pub fn propose_settled_deletion(
        drive: &Drive,
        contract: &DataContract,
        seed: u8,
        closes: bool,
    ) {
        drive
            .apply_drive_operations(
                vec![DriveOperation::ContractModerationOperation(
                    ContractModerationOperationType::AddTeamActionSignature {
                        contract_id: contract.id(),
                        action_id: Identifier::from([seed; 32]),
                        signer_id: Identifier::from([0x77; 32]),
                        write: ContractTeamActionWrite::Propose {
                            action: settled_deletion_proposal(seed),
                            closes,
                        },
                    },
                )],
                true,
                &BlockInfo::default(),
                None,
                PlatformVersion::latest(),
                None,
            )
            .expect("expected to record the proposal");
    }
}
