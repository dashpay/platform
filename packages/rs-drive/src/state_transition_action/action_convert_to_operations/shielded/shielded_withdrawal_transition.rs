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

                    // 3. Update total balance: the pool decreases by the full `amount`
                    //    (= unshielding_amount). Of that, `amount - fee_amount` leaves the
                    //    platform to Core (see RemoveFromSystemCredits below) and
                    //    `fee_amount` stays in-platform, flowing to the fee pools at block
                    //    finalization, so credits are conserved.
                    let new_total_balance =
                        v0.current_total_balance
                            .checked_sub(v0.amount)
                            .ok_or_else(|| {
                                Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance underflow when subtracting shielded_withdrawal amount"
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

                    // 5. Remove credits from the system: only the NET amount
                    //    (`amount - fee_amount`) actually leaves the platform to Core. The
                    //    fee stays in-platform (moves from the shielded pool sum tree to the
                    //    fee pools sum tree), so the `total_credits_in_platform` counter must
                    //    only drop by the net. Validation guarantees `amount >= fee_amount`;
                    //    we still guard the subtraction defensively.
                    let net_withdrawal_amount =
                        v0.amount.checked_sub(v0.fee_amount).ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "shielded_withdrawal fee exceeds withdrawal amount".to_string(),
                            ))
                        })?;
                    ops.push(DriveOperation::SystemOperation(
                        SystemOperationType::RemoveFromSystemCredits {
                            amount: net_withdrawal_amount,
                        },
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
    use dpp::shielded::{compute_minimum_shielded_fee, SerializedAction};
    use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
    use dpp::state_transition::state_transitions::shielded::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
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

        // Verify UpdateTotalBalance = 10000 - 3000 (amount only) = 7000
        match &ops[2] {
            DriveOperation::ShieldedPoolOperation(
                ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
            ) => {
                assert_eq!(*new_total_balance, 7000);
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

        // Verify RemoveFromSystemCredits amount = NET (3000 - 500) = 2500
        match &ops[4] {
            DriveOperation::SystemOperation(SystemOperationType::RemoveFromSystemCredits {
                amount,
            }) => {
                assert_eq!(*amount, 2500);
            }
            other => panic!("expected RemoveFromSystemCredits, got {:?}", other),
        }
    }

    #[test]
    fn test_balance_decreases_by_amount_only() {
        // The pool decrements by exactly `amount` (= unshielding_amount); the fee is
        // carved out of that amount, not charged on top of it.
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
                assert_eq!(*new_total_balance, 7000); // 10000 - 3000 (amount only)
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
    fn test_removes_net_from_system_credits() {
        // Only the NET amount (amount - fee) leaves the platform to Core; the fee
        // stays in-platform and flows to the fee pools.
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
                assert_eq!(*amount, 2500); // 3000 amount - 500 fee
            }
            other => panic!("expected RemoveFromSystemCredits, got {:?}", other),
        }
    }

    /// A minimal serialized Orchard action (dummy bytes; the transformer only copies
    /// these fields into the action's notes — no proof verification happens here).
    fn make_serialized_action() -> SerializedAction {
        SerializedAction {
            nullifier: [0x11; 32],
            rk: [0x33; 32],
            cmx: [0x22; 32],
            encrypted_note: vec![1, 2, 3],
            cv_net: [0x44; 32],
            spend_auth_sig: [0x55; 64],
        }
    }

    /// Build a real single-action `ShieldedWithdrawalTransition` with the given gross amount.
    fn make_transition(unshielding_amount: u64) -> ShieldedWithdrawalTransition {
        ShieldedWithdrawalTransition::V0(ShieldedWithdrawalTransitionV0 {
            actions: vec![make_serialized_action()],
            unshielding_amount,
            anchor: [0xAA; 32],
            proof: vec![],
            binding_signature: [0u8; 64],
            core_fee_per_byte: 1,
            pooling: Pooling::Never,
            output_script: CoreScript::from_bytes(vec![0x76, 0xA9]),
        })
    }

    #[test]
    fn test_fee_amount_is_non_zero() {
        // Regression guard for the fee-bypass bug. Drive the action through the REAL
        // transformer (`try_from_transition`) using the REAL fee function — not a hardcoded
        // fixture — so the test fails if the fee is computed as zero or dropped on the way
        // into the action.
        let platform_version = PlatformVersion::latest();
        let fee = compute_minimum_shielded_fee(1, platform_version);
        assert!(fee > 0, "computed minimum shielded fee must be non-zero");

        // Net (= unshielding_amount - fee) must clear the dust floor for the transform to
        // succeed; pad comfortably above it.
        let transition = make_transition(fee + 1_000_000);

        let result = ShieldedWithdrawalTransitionAction::try_from_transition(
            &transition,
            10_000_000,
            0,
            fee,
            platform_version,
        );
        assert!(result.is_valid(), "errors: {:?}", result.errors);
        let action = result.into_data().expect("action");
        match action {
            ShieldedWithdrawalTransitionAction::V0(v0) => {
                assert_eq!(
                    v0.fee_amount, fee,
                    "the action must carry the computed fee, not drop it to zero"
                );
            }
        }
    }

    #[test]
    fn test_transform_rejects_net_below_min_withdrawal_amount() {
        use dpp::state_transition::state_transitions::address_funds::address_credit_withdrawal_transition::MIN_WITHDRAWAL_AMOUNT;
        // A gross amount that covers the fee but leaves a net below the Core dust floor must
        // be rejected by the transformer, not turned into a zero/dust withdrawal document.
        let platform_version = PlatformVersion::latest();
        let fee = compute_minimum_shielded_fee(1, platform_version);

        // net = MIN_WITHDRAWAL_AMOUNT - 1 (just under the floor)
        let transition = make_transition(fee + MIN_WITHDRAWAL_AMOUNT - 1);

        let result = ShieldedWithdrawalTransitionAction::try_from_transition(
            &transition,
            10_000_000,
            0,
            fee,
            platform_version,
        );
        assert!(
            !result.is_valid(),
            "transform must reject a sub-dust net withdrawal amount"
        );
    }

    #[test]
    fn test_conservation_pool_minus_amount_system_minus_net() {
        // Conservation at the operation level:
        //   - shielded pool sum tree:        -amount
        //   - RemoveFromSystemCredits (counter): -(amount - fee)
        // The fee (= amount - net) is reconciled into the fee pools sum tree at block
        // finalization, so both the sum-tree total and the counter ultimately drop by
        // exactly the net (amount - fee).
        let amount = 3000u64;
        let fee = 500u64;
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        let mut shielded_delta: i128 = 0;
        let mut system_credit_delta: i128 = 0;
        for op in &ops {
            match op {
                DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
                ) => shielded_delta = *new_total_balance as i128 - 10000i128,
                DriveOperation::SystemOperation(SystemOperationType::RemoveFromSystemCredits {
                    amount,
                }) => system_credit_delta = -(*amount as i128),
                _ => {}
            }
        }

        assert_eq!(shielded_delta, -(amount as i128));
        assert_eq!(system_credit_delta, -((amount - fee) as i128));
    }

    #[test]
    fn test_pool_underflow_returns_error() {
        // Pool has less than `amount`; pool decrement must error.
        let action = ShieldedWithdrawalTransitionAction::V0(ShieldedWithdrawalTransitionActionV0 {
            amount: 5000,
            notes: vec![],
            anchor: [0x00; 32],
            core_fee_per_byte: 1,
            pooling: Pooling::Never,
            output_script: CoreScript::from_bytes(vec![]),
            fee_amount: 500,
            current_total_balance: 4000, // 4000 < 5000 (amount)
            prepared_withdrawal_document: make_document(),
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let result = action.into_high_level_drive_operations(&epoch, platform_version);
        assert!(result.is_err());
    }

    #[test]
    fn test_fee_exceeds_amount_returns_error() {
        // Defensive guard: if fee somehow exceeds amount, the net-withdrawal
        // subtraction in RemoveFromSystemCredits must error rather than underflow.
        let action = ShieldedWithdrawalTransitionAction::V0(ShieldedWithdrawalTransitionActionV0 {
            amount: 100,
            notes: vec![],
            anchor: [0x00; 32],
            core_fee_per_byte: 1,
            pooling: Pooling::Never,
            output_script: CoreScript::from_bytes(vec![]),
            fee_amount: 500, // fee > amount
            current_total_balance: 10000,
            prepared_withdrawal_document: make_document(),
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let result = action.into_high_level_drive_operations(&epoch, platform_version);
        assert!(result.is_err());
    }
}
