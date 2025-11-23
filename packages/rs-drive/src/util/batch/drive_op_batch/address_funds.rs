use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::identity::{KeyOfType, KeyOfTypeWithNonce};
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

/// Operations on Address Funds
#[derive(Clone, Debug)]
pub enum AddressFundsOperationType {
    /// Sets a balance for a given address in the AddressBalances tree.
    /// This operation directly sets (or overwrites) the balance for the address with the given nonce.
    SetBalanceToAddress {
        /// The key (containing key type and key data) with its associated nonce
        key_of_type_with_nonce: KeyOfTypeWithNonce,
        /// The balance value to set
        balance: Credits,
    },
    /// Adds a balance for a given address in the AddressBalances tree.
    /// This operation adds the balance for the address with the given nonce, that nonce is not changed.
    AddBalanceToAddress {
        /// The key (containing key type and key data)
        key_of_type: KeyOfType,
        /// The balance value to add
        balance_to_add: Credits,
    },
}

impl DriveLowLevelOperationConverter for AddressFundsOperationType {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        _estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _block_info: &BlockInfo,
        _transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            AddressFundsOperationType::SetBalanceToAddress {
                key_of_type_with_nonce,
                balance,
            } => {
                let mut drive_operations = vec![];
                drive.set_balance_to_address(
                    key_of_type_with_nonce,
                    balance,
                    &mut drive_operations,
                    platform_version,
                )?;
                Ok(drive_operations)
            }
            AddressFundsOperationType::AddBalanceToAddress {
                key_of_type,
                balance,
            } => {
                let mut drive_operations = vec![];
                drive.add_balance_to_address(
                    key_of_type,
                    balance,
                    &mut drive_operations,
                    platform_version,
                )?;
                Ok(drive_operations)
            }
        }
    }
}
