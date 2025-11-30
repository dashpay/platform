use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
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
        /// The platform address
        address: PlatformAddress,
        /// The nonce for the address
        nonce: AddressNonce,
        /// The balance value to set
        balance: Credits,
    },
    /// Adds a balance for a given address in the AddressBalances tree.
    /// This operation adds the balance for the address with the given nonce, that nonce is not changed.
    AddBalanceToAddress {
        /// The platform address
        address: PlatformAddress,
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
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            AddressFundsOperationType::SetBalanceToAddress {
                address,
                nonce,
                balance,
            } => {
                let mut drive_operations = vec![];
                drive.set_balance_to_address(
                    address,
                    nonce,
                    balance,
                    &mut drive_operations,
                    platform_version,
                )?;
                Ok(drive_operations)
            }
            AddressFundsOperationType::AddBalanceToAddress {
                address,
                balance_to_add,
            } => {
                let mut drive_operations = vec![];
                drive.add_balance_to_address(
                    address,
                    balance_to_add,
                    &mut drive_operations,
                    transaction,
                    platform_version,
                )?;
                Ok(drive_operations)
            }
        }
    }
}
