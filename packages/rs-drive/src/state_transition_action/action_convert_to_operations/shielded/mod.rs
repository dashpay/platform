use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::shield::ShieldTransitionAction;
use crate::state_transition_action::shielded::shield_from_asset_lock::ShieldFromAssetLockTransitionAction;
use crate::state_transition_action::shielded::shielded_transfer::ShieldedTransferTransitionAction;
use crate::state_transition_action::shielded::shielded_withdrawal::ShieldedWithdrawalTransitionAction;
use crate::state_transition_action::shielded::unshield::UnshieldTransitionAction;
use crate::util::batch::drive_op_batch::finalize_task::DriveOperationFinalizeTask;
use crate::util::batch::drive_op_batch::{
    AddressFundsOperationType, ShieldedPoolOperationType, SystemOperationType,
};
use crate::util::batch::{DocumentOperationType, DriveOperation};
use crate::util::object_size_info::{DocumentInfo, OwnedDocumentInfo};
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use grovedb::Element;

/// Build the encrypted note items (cmx || encrypted_note) for auto-incremented insertion.
fn build_encrypted_note_items(
    note_commitments: &[[u8; 32]],
    encrypted_notes: &[Vec<u8>],
) -> Vec<Element> {
    note_commitments
        .iter()
        .zip(encrypted_notes.iter())
        .map(|(cmx, encrypted_note)| {
            let mut value = cmx.to_vec();
            value.extend_from_slice(encrypted_note);
            Element::new_item(value)
        })
        .collect()
}

impl DriveHighLevelOperationConverter for ShieldTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match self {
            ShieldTransitionAction::V0(v0) => {
                let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                // 1. Debit each input address: set remaining balance
                for (address, (nonce, remaining_balance)) in v0.inputs_with_remaining_balance {
                    ops.push(DriveOperation::AddressFundsOperation(
                        AddressFundsOperationType::SetBalanceToAddress {
                            address,
                            nonce,
                            balance: remaining_balance,
                        },
                    ));
                }

                // 2. Append each note commitment to the commitment tree
                for cmx in v0.note_commitments.iter() {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        ShieldedPoolOperationType::AppendNoteCommitment { cmx: *cmx },
                    ));
                }

                // 3. Insert encrypted notes with auto-incremented keys in count tree
                ops.push(DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::InsertEncryptedNotes {
                        items: build_encrypted_note_items(
                            &v0.note_commitments,
                            &v0.encrypted_notes,
                        ),
                    },
                ));

                // 4. Update total balance SumItem: current_total_balance + shield_amount
                let new_total_balance = v0
                    .current_total_balance
                    .checked_add(v0.shield_amount)
                    .ok_or_else(|| {
                        Error::Drive(crate::error::drive::DriveError::CorruptedDriveState(
                            "shielded pool total balance overflow when adding shield amount"
                                .to_string(),
                        ))
                    })?;
                ops.push(DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
                ));

                // Checkpoint the commitment tree and record anchor after batch is applied
                ops.push(DriveOperation::FinalizeOperation(
                    DriveOperationFinalizeTask::RecordShieldedAnchor,
                ));

                Ok(ops)
            }
        }
    }
}

impl DriveHighLevelOperationConverter for ShieldedTransferTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match self {
            ShieldedTransferTransitionAction::V0(v0) => {
                let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                // 1. Insert each nullifier (InsertOnly to prevent double-spend)
                for nullifier in v0.nullifiers.iter() {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        ShieldedPoolOperationType::InsertNullifier {
                            nullifier: *nullifier,
                        },
                    ));
                }

                // 2. Append each note commitment to the commitment tree
                for cmx in v0.note_commitments.iter() {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        ShieldedPoolOperationType::AppendNoteCommitment { cmx: *cmx },
                    ));
                }

                // 3. Insert encrypted notes with auto-incremented keys in count tree
                ops.push(DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::InsertEncryptedNotes {
                        items: build_encrypted_note_items(
                            &v0.note_commitments,
                            &v0.encrypted_notes,
                        ),
                    },
                ));

                // 4. Update total balance SumItem: subtract fee_amount
                let new_total_balance = v0
                    .current_total_balance
                    .checked_sub(v0.fee_amount)
                    .ok_or_else(|| {
                        Error::Drive(crate::error::drive::DriveError::CorruptedDriveState(
                            "shielded pool total balance underflow when subtracting fee_amount"
                                .to_string(),
                        ))
                    })?;
                ops.push(DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
                ));

                // Checkpoint the commitment tree and record anchor after batch is applied
                ops.push(DriveOperation::FinalizeOperation(
                    DriveOperationFinalizeTask::RecordShieldedAnchor,
                ));

                Ok(ops)
            }
        }
    }
}

impl DriveHighLevelOperationConverter for UnshieldTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match self {
            UnshieldTransitionAction::V0(v0) => {
                let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                // 1. Insert each nullifier (InsertOnly to prevent double-spend)
                for nullifier in v0.nullifiers.iter() {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        ShieldedPoolOperationType::InsertNullifier {
                            nullifier: *nullifier,
                        },
                    ));
                }

                // 2. Credit the output address with the unshielded amount
                ops.push(DriveOperation::AddressFundsOperation(
                    AddressFundsOperationType::AddBalanceToAddress {
                        address: v0.output_address,
                        balance_to_add: v0.amount,
                    },
                ));

                // 3. Append each note commitment (change outputs) to the commitment tree
                for cmx in v0.note_commitments.iter() {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        ShieldedPoolOperationType::AppendNoteCommitment { cmx: *cmx },
                    ));
                }

                // 4. Insert encrypted notes with auto-incremented keys in count tree
                ops.push(DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::InsertEncryptedNotes {
                        items: build_encrypted_note_items(
                            &v0.note_commitments,
                            &v0.encrypted_notes,
                        ),
                    },
                ));

                // 5. Update total balance SumItem: subtract the unshielded amount
                //    (fee is handled separately by the fee system)
                let new_total_balance = v0
                    .current_total_balance
                    .checked_sub(v0.amount)
                    .ok_or_else(|| Error::Drive(
                        crate::error::drive::DriveError::CorruptedDriveState(
                            "shielded pool total balance underflow when subtracting unshield amount".to_string(),
                        ),
                    ))?;
                ops.push(DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
                ));

                // Checkpoint the commitment tree and record anchor after batch is applied
                ops.push(DriveOperation::FinalizeOperation(
                    DriveOperationFinalizeTask::RecordShieldedAnchor,
                ));

                Ok(ops)
            }
        }
    }
}

impl DriveHighLevelOperationConverter for ShieldFromAssetLockTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match self {
            ShieldFromAssetLockTransitionAction::V0(v0) => {
                let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                // 1. Add credits to system from the asset lock
                ops.push(DriveOperation::SystemOperation(
                    SystemOperationType::AddToSystemCredits {
                        amount: v0.asset_lock_value_to_be_consumed,
                    },
                ));

                // 2. Record asset lock as consumed (prevent replay)
                let asset_lock_value = AssetLockValue::new(
                    v0.asset_lock_value_to_be_consumed,
                    vec![], // tx_out_script not needed for shielded
                    0,      // remaining_credit_value = 0 (fully consumed)
                    vec![], // no used tags for shielded
                    _platform_version,
                )
                .map_err(|e| Error::Protocol(Box::new(e)))?;
                ops.push(DriveOperation::SystemOperation(
                    SystemOperationType::AddUsedAssetLock {
                        asset_lock_outpoint: v0.asset_lock_outpoint.into(),
                        asset_lock_value,
                    },
                ));

                // 3. Append note commitments to commitment tree
                for cmx in v0.note_commitments.iter() {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        ShieldedPoolOperationType::AppendNoteCommitment { cmx: *cmx },
                    ));
                }

                // 4. Insert encrypted notes with auto-incremented keys in count tree
                ops.push(DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::InsertEncryptedNotes {
                        items: build_encrypted_note_items(
                            &v0.note_commitments,
                            &v0.encrypted_notes,
                        ),
                    },
                ));

                // 5. Update total balance: current_total_balance + shield_amount
                let new_total_balance = v0.current_total_balance.checked_add(v0.shield_amount)
                    .ok_or_else(|| Error::Drive(
                        crate::error::drive::DriveError::CorruptedDriveState(
                            "shielded pool total balance overflow when adding shield_from_asset_lock amount".to_string(),
                        ),
                    ))?;
                ops.push(DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
                ));

                // 6. Record anchor after batch is applied
                ops.push(DriveOperation::FinalizeOperation(
                    DriveOperationFinalizeTask::RecordShieldedAnchor,
                ));

                Ok(ops)
            }
        }
    }
}

impl DriveHighLevelOperationConverter for ShieldedWithdrawalTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match self {
            ShieldedWithdrawalTransitionAction::V0(v0) => {
                let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                // 1. Insert nullifiers (prevent double-spend)
                for nullifier in v0.nullifiers.iter() {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        ShieldedPoolOperationType::InsertNullifier {
                            nullifier: *nullifier,
                        },
                    ));
                }

                // 2. Append change note commitments to commitment tree
                for cmx in v0.note_commitments.iter() {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        ShieldedPoolOperationType::AppendNoteCommitment { cmx: *cmx },
                    ));
                }

                // 3. Insert encrypted change notes with auto-incremented keys in count tree
                ops.push(DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::InsertEncryptedNotes {
                        items: build_encrypted_note_items(
                            &v0.note_commitments,
                            &v0.encrypted_notes,
                        ),
                    },
                ));

                // 4. Update total balance: subtract withdrawal amount
                let new_total_balance = v0
                    .current_total_balance
                    .checked_sub(v0.amount)
                    .ok_or_else(|| Error::Drive(
                        crate::error::drive::DriveError::CorruptedDriveState(
                            "shielded pool total balance underflow when subtracting shielded_withdrawal amount".to_string(),
                        ),
                    ))?;
                ops.push(DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
                ));

                // 5. Add withdrawal document
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

                // 6. Remove credits from system (they leave the system to Core)
                ops.push(DriveOperation::SystemOperation(
                    SystemOperationType::RemoveFromSystemCredits { amount: v0.amount },
                ));

                // 7. Record anchor after batch is applied
                ops.push(DriveOperation::FinalizeOperation(
                    DriveOperationFinalizeTask::RecordShieldedAnchor,
                ));

                Ok(ops)
            }
        }
    }
}
