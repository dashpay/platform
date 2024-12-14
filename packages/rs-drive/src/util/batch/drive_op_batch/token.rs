use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use crate::util::batch::IdentityOperationType;
use crate::util::object_size_info::DataContractInfo;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

/// Operations on Documents
#[derive(Clone, Debug)]
pub enum TokenOperationType {
    /// Burns token from the account issuing the .
    TokenBurn {
        /// The token id
        token_id: Identifier,
        /// The identity to burn from
        identity_balance_holder_id: Identifier,
        /// The amount to burn
        burn_amount: TokenAmount,
    },
    /// Adds a document to a contract matching the desired info.
    TokenMint {
        /// The token id
        token_id: Identifier,
        /// The identity to burn from
        identity_balance_holder_id: Identifier,
        /// The amount to issue
        mint_amount: TokenAmount,
    },
    /// Adds a document to a contract matching the desired info.
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
                    &mut None,
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
            } => {
                let token_id_bytes: [u8; 32] = token_id.to_buffer();
                let identity_id_bytes: [u8; 32] = identity_balance_holder_id.to_buffer();
                let batch_operations = drive.token_mint_operations(
                    token_id_bytes,
                    identity_id_bytes,
                    mint_amount,
                    &mut None,
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
                    &mut None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
        }
    }
}
