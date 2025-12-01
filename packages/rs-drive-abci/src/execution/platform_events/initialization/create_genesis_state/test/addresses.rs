use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::util::batch::drive_op_batch::{AddressFundsOperationType, DriveOperation};

const PLATFORM_ADDRESS_1: PlatformAddress = PlatformAddress::P2pkh([10; 20]);
const PLATFORM_ADDRESS_2: PlatformAddress = PlatformAddress::P2sh([11; 20]);
const PLATFORM_ADDRESS_1_NONCE: AddressNonce = 5;
const PLATFORM_ADDRESS_2_NONCE: AddressNonce = 7;
const PLATFORM_ADDRESS_1_BALANCE: Credits = 1_000_000;
const PLATFORM_ADDRESS_2_BALANCE: Credits = 2_000_000;

impl<C> Platform<C> {
    /// Create deterministic address balances for SDK tests.
    pub(super) fn create_data_for_addresses(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let operations = vec![
            DriveOperation::AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address: PLATFORM_ADDRESS_1,
                nonce: PLATFORM_ADDRESS_1_NONCE,
                balance: PLATFORM_ADDRESS_1_BALANCE,
            }),
            DriveOperation::AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address: PLATFORM_ADDRESS_2,
                nonce: PLATFORM_ADDRESS_2_NONCE,
                balance: PLATFORM_ADDRESS_2_BALANCE,
            }),
        ];

        self.drive.apply_drive_operations(
            operations,
            true,
            block_info,
            transaction,
            platform_version,
            None,
        )?;

        Ok(())
    }
}
