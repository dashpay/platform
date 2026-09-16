use super::{insert_notes, insert_nullifiers, update_balance};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::token_purchase_from_shielded_pool::TokenPurchaseFromShieldedPoolTransitionAction;
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation;
use crate::util::batch::DriveOperation::{IdentityOperation, TokenOperation};
use crate::util::batch::IdentityOperationType;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for TokenPurchaseFromShieldedPoolTransitionAction {
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
            .token_purchase_from_shielded_pool_transition
        {
            0 => match self {
                TokenPurchaseFromShieldedPoolTransitionAction::V0(v0) => {
                    let mut ops: Vec<DriveOperation<'a>> = Vec::new();
                    ops.push(TokenOperation(TokenOperationType::TokenMintToPool {
                        token_id: v0.token_id,
                        amount: v0.token_count,
                        allow_first_mint: v0.allow_first_mint,
                        notes: v0.token_notes,
                    }));
                    // The price leaves the credit pool into the contract owner's balance; the
                    // fee leaves it to the fee pools. Both are right-hand-side terms of the
                    // credit conservation equation, so no system-credit adjustment.
                    if v0.total_agreed_price > 0 {
                        ops.push(IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                            identity_id: v0.contract_owner_id.to_buffer(),
                            added_balance: v0.total_agreed_price,
                        }));
                    }
                    let credits_leaving = v0
                        .total_agreed_price
                        .checked_add(v0.fee_amount)
                        .ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "token purchase price plus fee overflows".to_string(),
                            ))
                        })?;

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
                method: "TokenPurchaseFromShieldedPoolTransitionAction::into_high_level_drive_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
