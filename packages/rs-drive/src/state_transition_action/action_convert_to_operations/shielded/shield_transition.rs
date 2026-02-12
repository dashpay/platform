use super::{append_note_commitments, insert_encrypted_notes, update_balance_and_record_anchor};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::shield::ShieldTransitionAction;
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for ShieldTransitionAction {
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
            .shield_transition
        {
            0 => match self {
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
                    append_note_commitments(&mut ops, &v0.note_commitments);

                    // 3. Insert encrypted notes with auto-incremented keys in count tree
                    insert_encrypted_notes(&mut ops, &v0.note_commitments, &v0.encrypted_notes);

                    // 4. Update total balance and record anchor
                    let new_total_balance = v0
                        .current_total_balance
                        .checked_add(v0.shield_amount)
                        .ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance overflow when adding shield amount"
                                    .to_string(),
                            ))
                        })?;
                    update_balance_and_record_anchor(&mut ops, new_total_balance);

                    Ok(ops)
                }
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "ShieldTransitionAction::into_high_level_drive_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
