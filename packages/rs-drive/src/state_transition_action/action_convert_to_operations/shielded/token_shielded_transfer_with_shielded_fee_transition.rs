use super::{insert_notes, insert_nullifiers, update_balance};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::token_shielded_transfer_with_shielded_fee::TokenShieldedTransferWithShieldedFeeTransitionAction;
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation;
use crate::util::batch::DriveOperation::TokenOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for TokenShieldedTransferWithShieldedFeeTransitionAction {
    /// The token pool side is one token operation; the credit pool side spends the fee
    /// bundle's nullifiers, appends its change notes and debits the credit pool by everything
    /// that left it (the fee, plus the price for a purchase). No identity nonce is touched:
    /// the spent nullifiers are the replay protection.
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
            .token_shielded_transfer_with_shielded_fee_transition
        {
            0 => match self {
                TokenShieldedTransferWithShieldedFeeTransitionAction::V0(v0) => {
                    let mut ops: Vec<DriveOperation<'a>> = Vec::new();
                    ops.push(TokenOperation(TokenOperationType::TokenShieldedTransfer {
                        token_id: v0.token_id,
                        nullifiers: v0.token_notes.iter().map(|note| note.nullifier).collect(),
                        notes: v0.token_notes,
                    }));
                    let credits_leaving = v0.fee_amount;

                    insert_nullifiers(&mut ops, &v0.fee_notes);
                    insert_notes(&mut ops, &v0.fee_notes);
                    let new_total_balance = v0
                        .current_credit_pool_balance
                        .checked_sub(credits_leaving)
                        .ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance underflow when paying a token pool transition"
                                    .to_string(),
                            ))
                        })?;
                    update_balance(&mut ops, new_total_balance);
                    Ok(ops)
                }
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "TokenShieldedTransferWithShieldedFeeTransitionAction::into_high_level_drive_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
