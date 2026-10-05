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
///
/// A batch must write each address at most once. `AddBalanceToAddress` computes the new
/// balance from the one committed before its batch, `SetBalanceToAddress` overwrites it, and
/// GroveDB keeps only the last write of a key, so a second write to one address would replace
/// the first. Every batch writes each address once today: an address may not be both an input
/// and an output of a transition (`OutputAddressAlsoInputError`), inputs and outputs are maps,
/// and the fee of a transition paid from its inputs is applied in a batch of its own.
/// `DriveOperation::merge_balance_writes` does not merge these; a batch that needs two writes
/// to one address must fold them into one.
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
        estimated_costs_only_with_layer_info: &mut Option<
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
            } => drive.set_balance_to_address_operations(
                address,
                nonce,
                balance,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            AddressFundsOperationType::AddBalanceToAddress {
                address,
                balance_to_add,
            } => drive.add_balance_to_address_operations(
                address,
                balance_to_add,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
        }
    }
}
