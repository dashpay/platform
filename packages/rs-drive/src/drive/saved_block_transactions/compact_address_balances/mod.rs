mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Compacts address balance changes from recent blocks, including the current block,
    /// into a single compacted entry.
    ///
    /// This function drains all entries from the address balances tree, merges them
    /// with the provided current block's address balances, and stores the result in
    /// the compacted address balances tree with a (start_block, end_block) key.
    ///
    /// # Arguments
    /// * `current_address_balances` - The current block's address balance changes to include
    /// * `current_block_height` - The height of the current block
    /// * `transaction` - Optional database transaction
    /// * `platform_version` - The platform version
    ///
    /// # Returns
    /// * `Ok((start, end))` - The block range that was compacted
    /// * `Err` - An error occurred
    pub fn compact_address_balances_with_current_block(
        &self,
        current_address_balances: &BTreeMap<PlatformAddress, CreditOperation>,
        current_block_height: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(u64, u64), Error> {
        match platform_version
            .drive
            .methods
            .saved_block_transactions
            .compact_address_balances
        {
            0 => self.compact_address_balances_with_current_block_v0(
                current_address_balances,
                current_block_height,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "compact_address_balances_with_current_block".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
