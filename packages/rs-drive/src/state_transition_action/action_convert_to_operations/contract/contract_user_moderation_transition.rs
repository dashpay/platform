use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::contract::contract_user_moderation::v0::{
    ContractDocumentDeletionContext, ContractDocumentRestorationContext,
    ContractUserModerationTransitionActionV0, ContractWarningContext,
};
use crate::state_transition_action::contract::contract_user_moderation::ContractUserModerationTransitionAction;
use crate::util::batch::DriveOperation::{
    ContractModerationOperation, DocumentOperation, IdentityOperation,
};
use crate::util::batch::{
    ContractModerationOperationType, DocumentOperationType, DriveOperation, IdentityOperationType,
};
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::epoch::Epoch;
use dpp::data_contract::config::moderation::{ContractDocumentRemoval, ContractWarning};
use dpp::document::DocumentV0Getters;
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationAction;
use dpp::version::PlatformVersion;
use std::borrow::Cow;

impl DriveHighLevelOperationConverter for ContractUserModerationTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .contract_user_moderation_transition
        {
            0 => {
                let ContractUserModerationTransitionAction::V0(
                    ContractUserModerationTransitionActionV0 {
                        moderator_id,
                        data_contract_id: contract_id,
                        identity_contract_nonce,
                        action,
                        target_is_suspended,
                        warning,
                        document_deletion,
                        document_restoration,
                        moderation_action_count,
                        ..
                    },
                ) = self;

                let mut operations = vec![IdentityOperation(
                    IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: moderator_id.to_buffer(),
                        contract_id: contract_id.to_buffer(),
                        nonce: identity_contract_nonce,
                    },
                )];

                match action {
                    ContractUserModerationAction::Ban {
                        identity_id,
                        reason,
                    } => {
                        // A ban supersedes a suspension, lapsed or not.
                        if target_is_suspended {
                            operations.push(ContractModerationOperation(
                                ContractModerationOperationType::RemoveSuspension {
                                    contract_id,
                                    identity_id,
                                },
                            ));
                        }
                        operations.push(ContractModerationOperation(
                            ContractModerationOperationType::AddBan {
                                contract_id,
                                identity_id,
                                reason,
                                moderator_id,
                            },
                        ));
                    }
                    ContractUserModerationAction::Unban { identity_id } => {
                        operations.push(ContractModerationOperation(
                            ContractModerationOperationType::RemoveBan {
                                contract_id,
                                identity_id,
                            },
                        ));
                    }
                    ContractUserModerationAction::Suspend {
                        identity_id,
                        until,
                        reason,
                    } => {
                        operations.push(ContractModerationOperation(
                            ContractModerationOperationType::AddSuspension {
                                contract_id,
                                identity_id,
                                until,
                                reason,
                                replaces_existing: target_is_suspended,
                                moderator_id,
                            },
                        ));
                    }
                    ContractUserModerationAction::Unsuspend { identity_id } => {
                        operations.push(ContractModerationOperation(
                            ContractModerationOperationType::RemoveSuspension {
                                contract_id,
                                identity_id,
                            },
                        ));
                    }
                    ContractUserModerationAction::Warn {
                        identity_id,
                        reason,
                    } => {
                        let ContractWarningContext {
                            existing_warnings,
                            warned_at,
                        } = warning.ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "a warn action must carry what its validation read",
                        )))?;
                        // The entry is rewritten whole: the warnings it held, then this one.
                        let replaces_existing = !existing_warnings.is_empty();
                        let mut warnings = existing_warnings;
                        warnings.push(ContractWarning { warned_at, reason });
                        operations.push(ContractModerationOperation(
                            ContractModerationOperationType::AddWarning {
                                contract_id,
                                identity_id,
                                warnings,
                                replaces_existing,
                                moderator_id,
                            },
                        ));
                    }
                    ContractUserModerationAction::ClearWarnings { identity_id } => {
                        operations.push(ContractModerationOperation(
                            ContractModerationOperationType::RemoveWarnings {
                                contract_id,
                                identity_id,
                            },
                        ));
                    }
                    ContractUserModerationAction::DeleteDocument {
                        document_type_name,
                        document_id,
                        reason,
                    } => {
                        let ContractDocumentDeletionContext {
                            data_contract_fetch_info,
                            document_owner_id,
                            removed_at,
                            document_hash,
                            replaces_restored_record,
                        } =
                            document_deletion
                                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "a document deletion action must carry what its validation read",
                            )))?;
                        // The deletion of the document, which keeps every index and aggregate
                        // of its type right and does not ask `canBeDeleted` (that is the
                        // owner's rule, not the moderators'), then its record: a fresh one, or
                        // in place of the restored one a document deleted before carries. The
                        // marker makes the batch refund nobody: the document's owner forfeits
                        // the storage fee.
                        operations.push(DocumentOperation(
                            DocumentOperationType::DeleteDocumentByModerator {
                                document_id,
                                contract_info: DataContractInfo::DataContractFetchInfo(
                                    data_contract_fetch_info,
                                ),
                                document_type_info: DocumentTypeInfo::DocumentTypeName(
                                    document_type_name.clone(),
                                ),
                            },
                        ));
                        operations.push(ContractModerationOperation(
                            ContractModerationOperationType::AddDocumentRemoval {
                                contract_id,
                                document_type_name,
                                document_id,
                                removal: ContractDocumentRemoval {
                                    document_owner_id,
                                    moderator_id,
                                    reason,
                                    removed_at,
                                    document_hash,
                                    restoration: None,
                                },
                                replaces_existing: replaces_restored_record,
                                moderator_id,
                            },
                        ));
                        operations.push(ContractModerationOperation(
                            ContractModerationOperationType::ForfeitStorageRefunds,
                        ));
                    }
                    ContractUserModerationAction::RestoreDocument {
                        document_type_name, ..
                    } => {
                        let ContractDocumentRestorationContext {
                            data_contract_fetch_info,
                            document,
                            removal,
                        } = document_restoration.ok_or(Error::Drive(
                            DriveError::CorruptedCodeExecution(
                                "a document restore action must carry what its validation read",
                            ),
                        ))?;
                        let document_id = document.id();
                        let owner_id = document.owner_id();
                        // The document goes back the way a create puts it in, every index and
                        // aggregate of its type included. Its storage flags name its owner, as
                        // they did before the deletion: the moderator pays for the bytes, and
                        // the refund of a later deletion is the owner's, as it always was. Then
                        // the record, marked restored in place: two operations on one key
                        // would fail the batch, so it is replaced rather than deleted and
                        // written again, and it is never deleted.
                        let storage_flags =
                            StorageFlags::new_single_epoch(epoch.index, Some(owner_id.to_buffer()));
                        operations.push(DocumentOperation(DocumentOperationType::AddDocument {
                            owned_document_info: OwnedDocumentInfo {
                                document_info: DocumentOwnedInfo((
                                    document,
                                    Some(Cow::Owned(storage_flags)),
                                )),
                                owner_id: Some(owner_id.to_buffer()),
                            },
                            contract_info: DataContractInfo::DataContractFetchInfo(
                                data_contract_fetch_info,
                            ),
                            document_type_info: DocumentTypeInfo::DocumentTypeName(
                                document_type_name.clone(),
                            ),
                            override_document: false,
                        }));
                        operations.push(ContractModerationOperation(
                            ContractModerationOperationType::AddDocumentRemoval {
                                contract_id,
                                document_type_name,
                                document_id,
                                removal,
                                replaces_existing: true,
                                moderator_id,
                            },
                        ));
                    }
                }

                // A member of an elected contract's seated team signed an action that counts
                // toward its share of the moderators pot.
                if let Some(count) = moderation_action_count {
                    operations.push(ContractModerationOperation(
                        ContractModerationOperationType::SetActionCount {
                            contract_id,
                            identity_id: moderator_id,
                            count,
                        },
                    ));
                }

                Ok(operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "ContractUserModerationTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::config::moderation::ContractModerationReason;
    use dpp::platform_value::Identifier;

    fn action(
        action: ContractUserModerationAction,
        target_is_suspended: bool,
    ) -> ContractUserModerationTransitionAction {
        ContractUserModerationTransitionAction::V0(ContractUserModerationTransitionActionV0 {
            moderator_id: Identifier::from([0xAA; 32]),
            data_contract_id: Identifier::from([0xBB; 32]),
            identity_contract_nonce: 4,
            action,
            target_is_suspended,
            warning: None,
            document_deletion: None,
            document_restoration: None,
            moderation_action_count: None,
            user_fee_increase: 0,
        })
    }

    #[test]
    fn should_rewrite_the_warning_list_entry_with_the_new_warning_last() {
        let platform_version = PlatformVersion::latest();
        let epoch = Epoch::new(0).expect("epoch");
        let target = Identifier::from([0xCC; 32]);
        let earlier = ContractWarning {
            warned_at: 5,
            reason: ContractModerationReason::from_text("first strike"),
        };

        let mut warn = action(
            ContractUserModerationAction::Warn {
                identity_id: target,
                reason: ContractModerationReason::from_text("second strike"),
            },
            false,
        );
        let ContractUserModerationTransitionAction::V0(v0) = &mut warn;
        v0.warning = Some(ContractWarningContext {
            existing_warnings: vec![earlier.clone()],
            warned_at: 9,
        });
        let ops = warn
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("operations");
        assert_eq!(ops.len(), 2);
        assert!(matches!(
            &ops[1],
            ContractModerationOperation(ContractModerationOperationType::AddWarning {
                identity_id,
                warnings,
                replaces_existing: true,
                ..
            }) if *identity_id == target
                && warnings.len() == 2
                && warnings[0] == earlier
                && warnings[1].warned_at == 9
                && warnings[1].reason.text == "second strike"
        ));

        // A warn that lost what its validation read can not write a whole entry.
        let orphan = action(
            ContractUserModerationAction::Warn {
                identity_id: target,
                reason: ContractModerationReason::from_text("spam"),
            },
            false,
        );
        assert!(orphan
            .into_high_level_drive_operations(&epoch, platform_version)
            .is_err());

        let ops = action(
            ContractUserModerationAction::ClearWarnings {
                identity_id: target,
            },
            false,
        )
        .into_high_level_drive_operations(&epoch, platform_version)
        .expect("operations");
        assert!(matches!(
            &ops[1],
            ContractModerationOperation(ContractModerationOperationType::RemoveWarnings { .. })
        ));
    }

    #[test]
    fn should_bump_the_contract_nonce_then_edit_the_list() {
        let platform_version = PlatformVersion::latest();
        let epoch = Epoch::new(0).expect("epoch");
        let target = Identifier::from([0xCC; 32]);

        let ops = action(
            ContractUserModerationAction::Ban {
                identity_id: target,
                reason: ContractModerationReason::from_text("spam"),
            },
            false,
        )
        .into_high_level_drive_operations(&epoch, platform_version)
        .expect("operations");
        assert_eq!(ops.len(), 2);
        assert!(matches!(
            &ops[0],
            IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce { nonce: 4, .. })
        ));
        assert!(matches!(
            &ops[1],
            ContractModerationOperation(ContractModerationOperationType::AddBan {
                identity_id,
                reason,
                ..
            }) if *identity_id == target && reason.text == "spam"
        ));
    }

    #[test]
    fn should_remove_a_suspension_when_banning_a_suspended_identity() {
        let platform_version = PlatformVersion::latest();
        let epoch = Epoch::new(0).expect("epoch");
        let target = Identifier::from([0xCC; 32]);

        let ops = action(
            ContractUserModerationAction::Ban {
                identity_id: target,
                reason: ContractModerationReason::from_text("spam"),
            },
            true,
        )
        .into_high_level_drive_operations(&epoch, platform_version)
        .expect("operations");
        assert_eq!(ops.len(), 3);
        assert!(matches!(
            &ops[1],
            ContractModerationOperation(ContractModerationOperationType::RemoveSuspension { .. })
        ));
        assert!(matches!(
            &ops[2],
            ContractModerationOperation(ContractModerationOperationType::AddBan { .. })
        ));
    }

    #[test]
    fn should_write_the_moderation_action_count_of_a_seated_team_member_last() {
        let platform_version = PlatformVersion::latest();
        let epoch = Epoch::new(0).expect("epoch");
        let target = Identifier::from([0xCC; 32]);

        let ops = action(
            ContractUserModerationAction::Ban {
                identity_id: target,
                reason: ContractModerationReason::from_text("spam"),
            },
            false,
        )
        .with_moderation_action_count(3)
        .into_high_level_drive_operations(&epoch, platform_version)
        .expect("operations");
        assert_eq!(ops.len(), 3);
        assert!(matches!(
            &ops[2],
            ContractModerationOperation(ContractModerationOperationType::SetActionCount {
                identity_id,
                count: 3,
                ..
            }) if *identity_id == Identifier::from([0xAA; 32])
        ));
    }

    #[test]
    fn should_replace_an_existing_suspension() {
        let platform_version = PlatformVersion::latest();
        let epoch = Epoch::new(0).expect("epoch");
        let target = Identifier::from([0xCC; 32]);

        let ops = action(
            ContractUserModerationAction::Suspend {
                identity_id: target,
                until: 99,
                reason: ContractModerationReason::from_text("again"),
            },
            true,
        )
        .into_high_level_drive_operations(&epoch, platform_version)
        .expect("operations");
        assert_eq!(ops.len(), 2);
        assert!(matches!(
            &ops[1],
            ContractModerationOperation(ContractModerationOperationType::AddSuspension {
                until: 99,
                reason,
                replaces_existing: true,
                ..
            }) if reason.text == "again"
        ));
    }
}
