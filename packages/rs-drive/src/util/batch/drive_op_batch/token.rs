use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::tokens::status::TokenStatus;
use dpp::tokens::token_event::TokenEvent;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

/// Operations on Tokens
#[derive(Clone, Debug)]
pub enum TokenOperationType {
    /// Burns token from the account issuing the action.
    TokenBurn {
        /// The token id
        token_id: Identifier,
        /// The identity to burn from
        identity_balance_holder_id: Identifier,
        /// The amount to burn
        burn_amount: TokenAmount,
    },
    /// Mints tokens
    TokenMint {
        /// The token id
        token_id: Identifier,
        /// The identity to burn from
        identity_balance_holder_id: Identifier,
        /// The amount to issue
        mint_amount: TokenAmount,
        /// Should we allow this to be the first ever mint
        allow_first_mint: bool,
    },
    /// Performs a token transfer
    TokenTransfer {
        /// The token id
        token_id: Identifier,
        /// The token id
        sender_id: Identifier,
        /// The recipient of the transfer
        recipient_id: Identifier,
        /// The amount to transfer
        amount: TokenAmount,
    },
    /// Freezes an identity's token balance so money can no longer be sent out.
    TokenFreeze {
        /// The token id
        token_id: Identifier,
        /// The frozen identity id
        frozen_identity_id: Identifier,
    },
    /// Unfreezes an identity's token balance so money can be sent out again.
    TokenUnfreeze {
        /// The token id
        token_id: Identifier,
        /// The frozen identity id
        frozen_identity_id: Identifier,
    },
    /// Sets the status of the token.
    TokenSetStatus {
        /// The token id
        token_id: Identifier,
        /// The status
        status: TokenStatus,
    },
    /// Adds a historical document explaining a token action.
    TokenHistory {
        /// The token id
        token_id: Identifier,
        /// The identity making the event
        owner_id: Identifier,
        /// The nonce
        nonce: IdentityNonce,
        /// The token event
        event: TokenEvent,
    },
}

impl DriveLowLevelOperationConverter for TokenOperationType {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            TokenOperationType::TokenBurn {
                token_id,
                identity_balance_holder_id,
                burn_amount,
            } => {
                let token_id_bytes: [u8; 32] = token_id.to_buffer();
                let identity_id_bytes: [u8; 32] = identity_balance_holder_id.to_buffer();
                let batch_operations = drive.token_burn_operations(
                    token_id_bytes,
                    identity_id_bytes,
                    burn_amount,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenMint {
                token_id,
                identity_balance_holder_id,
                mint_amount,
                allow_first_mint,
            } => {
                println!(
                    "minting in {} token to id {} ({})",
                    token_id, identity_balance_holder_id, mint_amount
                );
                let token_id_bytes: [u8; 32] = token_id.to_buffer();
                let identity_id_bytes: [u8; 32] = identity_balance_holder_id.to_buffer();
                let batch_operations = drive.token_mint_operations(
                    token_id_bytes,
                    identity_id_bytes,
                    mint_amount,
                    allow_first_mint,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenTransfer {
                token_id,
                sender_id,
                recipient_id,
                amount,
            } => {
                let token_id_bytes: [u8; 32] = token_id.to_buffer();
                let sender_id_bytes: [u8; 32] = sender_id.to_buffer();
                let recipient_id_bytes: [u8; 32] = recipient_id.to_buffer();

                let batch_operations = drive.token_transfer_operations(
                    token_id_bytes,
                    sender_id_bytes,
                    recipient_id_bytes,
                    amount,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenHistory {
                token_id,
                owner_id,
                nonce,
                event,
            } => {
                let batch_operations = drive.add_token_transaction_history_operations(
                    token_id,
                    owner_id,
                    nonce,
                    event,
                    block_info,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenFreeze {
                token_id,
                frozen_identity_id,
            } => {
                let batch_operations = drive.token_freeze_operations(
                    token_id,
                    frozen_identity_id,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenUnfreeze {
                token_id,
                frozen_identity_id,
            } => {
                let batch_operations = drive.token_unfreeze_operations(
                    token_id,
                    frozen_identity_id,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenSetStatus { token_id, status } => {
                let batch_operations = drive.token_apply_status_operations(
                    token_id.to_buffer(),
                    status,
                    estimated_costs_only_with_layer_info,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
        }
    }
}
