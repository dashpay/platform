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
pub enum TokenOperationType<'a> {
    /// Adds a document to a contract matching the desired info.
    TokenBurn {
        /// Data Contract info to potentially be resolved if needed
        contract_info: DataContractInfo<'a>,
        /// Token position in the contract, is 0 if there is only one token
        token_position: u16,
        /// The token id
        token_id: Identifier,
        /// The amount to burn
        burn_amount: TokenAmount,
    },
    /// Adds a document to a contract matching the desired info.
    TokenIssuance {
        /// Data Contract info to potentially be resolved if needed
        contract_info: DataContractInfo<'a>,
        /// Token position in the contract, is 0 if there is only one token
        token_position: u16,
        /// The token id
        token_id: Identifier,
        /// The amount to issue
        issuance_amount: TokenAmount,
    },
    /// Adds a document to a contract matching the desired info.
    TokenTransfer {
        /// Data Contract info to potentially be resolved if needed
        contract_info: DataContractInfo<'a>,
        /// Token position in the contract, is 0 if there is only one token
        token_position: u16,
        /// The token id
        token_id: Identifier,
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
                contract_info,
                token_position,
                token_id,
                burn_amount,
            } => {}
            TokenOperationType::TokenIssuance {
                contract_info,
                token_position,
                token_id,
                issuance_amount,
            } => {}
            TokenOperationType::TokenTransfer {
                contract_info,
                token_position,
                token_id,
                recipient_id,
                amount,
            } => {}
        }
    }
}
