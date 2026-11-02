use crate::drive::contract::moderation::types::{
    document_removal_encoded_size, estimated_document_removal_kept_fields_size,
    ContractTeamActionWrite, CONTRACT_DOCUMENT_RESTORATION_SIZE,
};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::contract::contract_user_moderation::v0::{
    ContractDocumentChangeContext, ContractDocumentDeletionContext,
    ContractDocumentRemovalRecordContext, ContractDocumentRestorationContext,
    ContractTeamActionContext, ContractUserModerationTransitionActionV0, ContractWarningContext,
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
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::{
    ContractDocumentRemoval, ContractModerationReason, ContractTeamActionEvent, ContractWarning,
};
use dpp::document::DocumentV0Getters;
use dpp::identifier::Identifier;
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
                        document_change,
                        team_action,
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
                        let deletion =
                            document_deletion
                                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "a document deletion action must carry what its validation read",
                            )))?;
                        push_document_deletion_operations(
                            &mut operations,
                            contract_id,
                            moderator_id,
                            document_type_name,
                            document_id,
                            reason,
                            deletion,
                            platform_version,
                        )?;
                    }
                    ContractUserModerationAction::DeleteSettledDocument { .. }
                    | ContractUserModerationAction::ApproveTeamAction { .. } => {
                        let ContractTeamActionContext {
                            action_id,
                            write,
                            deletion,
                            approver_action_counts,
                        } = team_action.ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "a team action's proposal or approval must carry what its validation \
                             read",
                        )))?;
                        // The signature that meets the rule runs the action, deleting the
                        // document its event names as a moderator's deletion does, for its
                        // reason, then the approval is written, the action closed, and every
                        // approver counted. The deletion comes with a closing write and only
                        // with one.
                        let closing_action = match &write {
                            ContractTeamActionWrite::Propose {
                                action,
                                closes: true,
                            }
                            | ContractTeamActionWrite::Close { action, .. } => Some(action),
                            ContractTeamActionWrite::Propose { closes: false, .. }
                            | ContractTeamActionWrite::Approve { .. } => None,
                        };
                        match (closing_action, deletion) {
                            (Some(action), Some(context)) => {
                                let ContractTeamActionEvent::DeleteSettledDocument {
                                    document_type_name,
                                    document_id,
                                    reason,
                                    ..
                                } = &action.event;
                                push_document_deletion_operations(
                                    &mut operations,
                                    contract_id,
                                    moderator_id,
                                    document_type_name.clone(),
                                    *document_id,
                                    reason.clone(),
                                    context,
                                    platform_version,
                                )?;
                            }
                            (None, None) => {}
                            _ => {
                                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                                    "a team action's deletion must come with the signature that \
                                     closes the action, and only with it",
                                )))
                            }
                        }
                        operations.push(ContractModerationOperation(
                            ContractModerationOperationType::AddTeamActionSignature {
                                contract_id,
                                action_id,
                                signer_id: moderator_id,
                                write,
                            },
                        ));
                        for (identity_id, count) in approver_action_counts {
                            operations.push(ContractModerationOperation(
                                ContractModerationOperationType::SetActionCount {
                                    contract_id,
                                    identity_id,
                                    count,
                                },
                            ));
                        }
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
                        // The record replaced is the same one unmarked: the restore adds its
                        // restoration and nothing else
                        let replaced_record_size = removal_record_size(&removal)?
                            .saturating_sub(CONTRACT_DOCUMENT_RESTORATION_SIZE as u32);
                        let estimated_kept_fields_size =
                            estimated_document_removal_kept_fields_size(
                                data_contract_fetch_info
                                    .contract
                                    .document_type_for_name(&document_type_name)?,
                                platform_version,
                            )?;
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
                                removal: Box::new(removal),
                                replaced_record_size: Some(replaced_record_size),
                                estimated_kept_fields_size,
                                moderator_id,
                            },
                        ));
                    }
                    ContractUserModerationAction::ChangeDocumentFields {
                        document_type_name,
                        ..
                    } => {
                        let ContractDocumentChangeContext {
                            data_contract_fetch_info,
                            document,
                        } = document_change
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "a document field change action must carry what its validation built",
                        )))?;
                        // The document is updated the way a replace updates it, every index and
                        // aggregate of its type included. Its storage flags name its owner, as a
                        // replace's do: the moderator pays for the bytes the change adds, and a
                        // refund of the document's storage stays the owner's, as it always was.
                        let owner_id = document.owner_id();
                        let storage_flags =
                            StorageFlags::new_single_epoch(epoch.index, Some(owner_id.to_buffer()));
                        operations.push(DocumentOperation(DocumentOperationType::UpdateDocument {
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
                                document_type_name,
                            ),
                        }));
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

/// The operations of a moderator's deletion of a document: the document goes, keeping every
/// index and aggregate of its type right and not asking `canBeDeleted` (that is the owner's rule,
/// not the moderators'), then its record when the type keeps one, with the fields the type
/// keeps: a fresh one, or in place of the restored one a document deleted before carries. Unless
/// the type refunds the owner, the marker makes the batch refund nobody for the document's
/// storage: whoever paid for it forfeits the storage fee, while the moderation records the batch
/// rewrites refund as ever.
#[allow(clippy::too_many_arguments)]
fn push_document_deletion_operations(
    operations: &mut Vec<DriveOperation<'_>>,
    contract_id: Identifier,
    moderator_id: Identifier,
    document_type_name: String,
    document_id: Identifier,
    reason: ContractModerationReason,
    deletion: ContractDocumentDeletionContext,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    let ContractDocumentDeletionContext {
        data_contract_fetch_info,
        document_owner_id,
        record,
        refunds_owner,
    } = deletion;
    let estimated_kept_fields_size = estimated_document_removal_kept_fields_size(
        data_contract_fetch_info
            .contract
            .document_type_for_name(&document_type_name)?,
        platform_version,
    )?;
    operations.push(DocumentOperation(
        DocumentOperationType::ForceDeleteDocument {
            document_id,
            contract_info: DataContractInfo::DataContractFetchInfo(data_contract_fetch_info),
            document_type_info: DocumentTypeInfo::DocumentTypeName(document_type_name.clone()),
        },
    ));
    if let Some(ContractDocumentRemovalRecordContext {
        removed_at,
        document_hash,
        replaced_record,
        kept_fields,
    }) = record
    {
        let replaced_record_size = replaced_record
            .as_ref()
            .map(removal_record_size)
            .transpose()?;
        operations.push(ContractModerationOperation(
            ContractModerationOperationType::AddDocumentRemoval {
                contract_id,
                document_type_name,
                document_id,
                removal: Box::new(ContractDocumentRemoval {
                    document_owner_id,
                    moderator_id,
                    reason,
                    removed_at,
                    document_hash,
                    restoration: None,
                    kept_fields,
                }),
                replaced_record_size,
                estimated_kept_fields_size,
                moderator_id,
            },
        ));
    }
    if !refunds_owner {
        operations.push(ContractModerationOperation(
            ContractModerationOperationType::ForfeitStorageRefunds,
        ));
    }
    Ok(())
}

/// The stored size of `removal`, a record read back from state or about to be written, which
/// the document size limits keep far inside what a record can hold.
fn removal_record_size(removal: &ContractDocumentRemoval) -> Result<u32, Error> {
    document_removal_encoded_size(removal)
        .ok()
        .and_then(|size| u32::try_from(size).ok())
        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
            "a removal record exceeds what a record can hold",
        )))
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
            document_change: None,
            team_action: None,
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
    fn should_write_the_approval_alone_while_the_action_stays_active() {
        let platform_version = PlatformVersion::latest();
        let epoch = Epoch::new(0).expect("epoch");
        let action_id = Identifier::from([0xDD; 32]);
        let approval = || {
            action(
                ContractUserModerationAction::ApproveTeamAction { action_id },
                false,
            )
        };

        let mut pending = approval();
        let ContractUserModerationTransitionAction::V0(v0) = &mut pending;
        v0.team_action = Some(ContractTeamActionContext {
            action_id,
            write: ContractTeamActionWrite::Approve {
                dropped_signers: vec![],
            },
            deletion: None,
            approver_action_counts: vec![],
        });
        let ops = pending
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("operations");
        // The nonce, then the approval: no deletion and no count while the action stays active.
        assert_eq!(ops.len(), 2);
        assert!(matches!(
            &ops[1],
            ContractModerationOperation(ContractModerationOperationType::AddTeamActionSignature {
                action_id: id,
                signer_id,
                write: ContractTeamActionWrite::Approve { dropped_signers },
                ..
            }) if *id == action_id && dropped_signers.is_empty() && *signer_id == Identifier::from([0xAA; 32])
        ));

        // An approval that lost what its validation read can not write the approval.
        assert!(approval()
            .into_high_level_drive_operations(&epoch, platform_version)
            .is_err());
    }

    #[test]
    fn should_refuse_a_closing_write_without_the_deletion_it_runs() {
        let platform_version = PlatformVersion::latest();
        let epoch = Epoch::new(0).expect("epoch");
        let action_id = Identifier::from([0xDD; 32]);
        let proposed = dpp::data_contract::config::moderation::ContractTeamAction {
            proposer_id: Identifier::from([0xAA; 32]),
            proposed_at: 5,
            event: ContractTeamActionEvent::DeleteSettledDocument {
                document_type_name: "post".to_string(),
                document_id: Identifier::from([0xEE; 32]),
                document_last_modified_at: 1,
                document_revision: Some(1),
                reason: ContractModerationReason::from_text("doxxing"),
            },
        };
        // A write that closes the action runs its deletion: without one, the action would close
        // and prove as run while the document stays
        for write in [
            ContractTeamActionWrite::Propose {
                action: proposed.clone(),
                closes: true,
            },
            ContractTeamActionWrite::Close {
                action: proposed.clone(),
                earlier_signers: vec![],
                dropped_signers: vec![],
            },
        ] {
            let mut closing = action(
                ContractUserModerationAction::ApproveTeamAction { action_id },
                false,
            );
            let ContractUserModerationTransitionAction::V0(v0) = &mut closing;
            v0.team_action = Some(ContractTeamActionContext {
                action_id,
                write,
                deletion: None,
                approver_action_counts: vec![],
            });
            assert!(closing
                .into_high_level_drive_operations(&epoch, platform_version)
                .is_err());
        }
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
