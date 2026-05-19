use super::{insert_notes, insert_nullifiers, update_balance};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::shielded_withdrawal::ShieldedWithdrawalTransitionAction;
use crate::util::batch::drive_op_batch::SystemOperationType;
use crate::util::batch::{DocumentOperationType, DriveOperation};
use crate::util::object_size_info::{DocumentInfo, OwnedDocumentInfo};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for ShieldedWithdrawalTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .shielded_withdrawal_transition
        {
            0 => match self {
                ShieldedWithdrawalTransitionAction::V0(v0) => {
                    let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                    // 1. Insert nullifiers (prevent double-spend)
                    insert_nullifiers(&mut ops, &v0.notes);

                    // 2. Insert change notes into CommitmentTree
                    insert_notes(&mut ops, &v0.notes);

                    // 3. Update total balance: subtract withdrawal amount + fee (both leave the pool)
                    let total_deduction =
                        v0.amount.checked_add(v0.fee_amount).ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "overflow when adding shielded_withdrawal amount and fee"
                                    .to_string(),
                            ))
                        })?;
                    let new_total_balance =
                        v0.current_total_balance
                            .checked_sub(total_deduction)
                            .ok_or_else(|| {
                                Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance underflow when subtracting shielded_withdrawal amount and fee"
                                    .to_string(),
                            ))
                            })?;
                    update_balance(&mut ops, new_total_balance);

                    // 4. Add withdrawal document
                    ops.push(DriveOperation::DocumentOperation(
                        DocumentOperationType::AddWithdrawalDocument {
                            owned_document_info: OwnedDocumentInfo {
                                document_info: DocumentInfo::DocumentOwnedInfo((
                                    v0.prepared_withdrawal_document,
                                    None,
                                )),
                                owner_id: None,
                            },
                        },
                    ));

                    // 5. Remove credits from system (they leave the system to Core)
                    ops.push(DriveOperation::SystemOperation(
                        SystemOperationType::RemoveFromSystemCredits { amount: v0.amount },
                    ));

                    Ok(ops)
                }
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "ShieldedWithdrawalTransitionAction::into_high_level_drive_operations"
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
    use crate::state_transition_action::shielded::shielded_withdrawal::v0::ShieldedWithdrawalTransitionActionV0;
    use crate::state_transition_action::shielded::ShieldedActionNote;
    use crate::util::batch::drive_op_batch::ShieldedPoolOperationType;
    use dpp::block::epoch::Epoch;
    use dpp::document::{Document, DocumentV0, DocumentV0Getters};
    use dpp::identity::core_script::CoreScript;
    use dpp::platform_value::Identifier;
    use dpp::version::PlatformVersion;
    use dpp::withdrawal::Pooling;

    fn make_note() -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [0x11; 32],
            cmx: [0x22; 32],
            encrypted_note: vec![1, 2, 3],
        }
    }

    fn make_document() -> Document {
        Document::V0(DocumentV0 {
            id: Identifier::from([0x11; 32]),
            owner_id: Identifier::from([0x22; 32]),
            properties: Default::default(),
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        })
    }

    fn make_action() -> ShieldedWithdrawalTransitionAction {
        ShieldedWithdrawalTransitionAction::V0(ShieldedWithdrawalTransitionActionV0 {
            amount: 3000,
            notes: vec![make_note()],
            anchor: [0xAA; 32],
            core_fee_per_byte: 1,
            pooling: Pooling::Never,
            output_script: CoreScript::from_bytes(vec![0x76, 0xA9]),
            fee_amount: 500,
            current_total_balance: 10000,
            prepared_withdrawal_document: make_document(),
        })
    }

    #[test]
    fn test_produces_nullifiers_notes_balance_withdrawal_doc_and_system_credits() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // InsertNullifiers + InsertNote (1) + UpdateTotalBalance
        // + AddWithdrawalDocument + RemoveFromSystemCredits
        assert_eq!(ops.len(), 5);

        // Verify InsertNullifiers carries the correct nullifier from our note
        match &ops[0] {
            DriveOperation::ShieldedPoolOperation(
                ShieldedPoolOperationType::InsertNullifiers { nullifiers },
            ) => {
                assert_eq!(nullifiers.len(), 1);
                assert_eq!(nullifiers[0], [0x11; 32]);
            }
            other => panic!("expected InsertNullifiers, got {:?}", other),
        }

        // Verify InsertNote carries nullifier, cmx, and encrypted_note from our note
        match &ops[1] {
            DriveOperation::ShieldedPoolOperation(ShieldedPoolOperationType::InsertNote {
                nullifier,
                cmx,
                encrypted_note,
            }) => {
                assert_eq!(*nullifier, [0x11; 32]);
                assert_eq!(*cmx, [0x22; 32]);
                assert_eq!(*encrypted_note, vec![1, 2, 3]);
            }
            other => panic!("expected InsertNote, got {:?}", other),
        }

        // Verify UpdateTotalBalance = 10000 - 3000 - 500 = 6500
        match &ops[2] {
            DriveOperation::ShieldedPoolOperation(
                ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
            ) => {
                assert_eq!(*new_total_balance, 6500);
            }
            other => panic!("expected UpdateTotalBalance, got {:?}", other),
        }

        // Verify AddWithdrawalDocument contains our document
        match &ops[3] {
            DriveOperation::DocumentOperation(DocumentOperationType::AddWithdrawalDocument {
                owned_document_info,
            }) => {
                assert!(
                    matches!(&owned_document_info.document_info, DocumentInfo::DocumentOwnedInfo((doc, None)) if doc.id() == Identifier::from([0x11; 32]))
                );
                assert_eq!(owned_document_info.owner_id, None);
            }
            other => panic!("expected AddWithdrawalDocument, got {:?}", other),
        }

        // Verify RemoveFromSystemCredits amount = 3000
        match &ops[4] {
            DriveOperation::SystemOperation(SystemOperationType::RemoveFromSystemCredits {
                amount,
            }) => {
                assert_eq!(*amount, 3000);
            }
            other => panic!("expected RemoveFromSystemCredits, got {:?}", other),
        }
    }

    #[test]
    fn test_balance_decreases_by_amount_plus_fee() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // Find the UpdateTotalBalance op
        let balance_op = ops.iter().find(|op| {
            matches!(
                op,
                DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::UpdateTotalBalance { .. }
                )
            )
        });
        match balance_op.unwrap() {
            DriveOperation::ShieldedPoolOperation(
                ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
            ) => {
                assert_eq!(*new_total_balance, 6500); // 10000 - 3000 - 500
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_has_withdrawal_document() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        let doc_op = ops.iter().find(|op| {
            matches!(
                op,
                DriveOperation::DocumentOperation(
                    DocumentOperationType::AddWithdrawalDocument { .. }
                )
            )
        });
        assert!(doc_op.is_some(), "expected AddWithdrawalDocument operation");

        // Verify the withdrawal document carries the correct id and owner_id
        match doc_op.unwrap() {
            DriveOperation::DocumentOperation(DocumentOperationType::AddWithdrawalDocument {
                owned_document_info,
            }) => {
                match &owned_document_info.document_info {
                    DocumentInfo::DocumentOwnedInfo((doc, storage_flags)) => {
                        assert_eq!(doc.id(), Identifier::from([0x11; 32]));
                        assert_eq!(doc.owner_id(), Identifier::from([0x22; 32]));
                        assert!(storage_flags.is_none());
                    }
                    other => panic!("expected DocumentOwnedInfo, got {:?}", other),
                }
                assert_eq!(owned_document_info.owner_id, None);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_removes_from_system_credits() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match ops.last().unwrap() {
            DriveOperation::SystemOperation(SystemOperationType::RemoveFromSystemCredits {
                amount,
            }) => {
                assert_eq!(*amount, 3000);
            }
            other => panic!("expected RemoveFromSystemCredits, got {:?}", other),
        }
    }

    #[test]
    fn test_underflow_returns_error() {
        let action = ShieldedWithdrawalTransitionAction::V0(ShieldedWithdrawalTransitionActionV0 {
            amount: 5000,
            notes: vec![],
            anchor: [0x00; 32],
            core_fee_per_byte: 1,
            pooling: Pooling::Never,
            output_script: CoreScript::from_bytes(vec![]),
            fee_amount: 6000,
            current_total_balance: 10000, // 5000 + 6000 > 10000
            prepared_withdrawal_document: make_document(),
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let result = action.into_high_level_drive_operations(&epoch, platform_version);
        assert!(result.is_err());
    }
}
