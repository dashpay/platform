mod v0;
mod v1;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

pub use v0::CompactedAddressBalanceChanges;

impl Drive {
    /// Fetches compacted address balance changes starting from a given block height.
    ///
    /// # Arguments
    /// * `start_block_height` - The block height to start fetching from
    /// * `limit` - Optional maximum number of compacted entries to return
    /// * `transaction` - Optional database transaction
    /// * `platform_version` - The platform version
    ///
    /// # Returns
    /// A vector of (start_block, end_block, address_balance_changes) tuples
    pub fn fetch_compacted_address_balance_changes(
        &self,
        start_block_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<CompactedAddressBalanceChanges, Error> {
        match platform_version
            .drive
            .methods
            .saved_block_transactions
            .fetch_address_balances
        {
            0 => self.fetch_compacted_address_balance_changes_v0(
                start_block_height,
                limit,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_compacted_address_balance_changes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Proves compacted address balance changes starting from a given block height.
    ///
    /// # Arguments
    /// * `start_block_height` - The block height to start from
    /// * `limit` - Optional maximum number of compacted entries to prove
    /// * `transaction` - Optional database transaction
    /// * `platform_version` - The platform version
    ///
    /// # Returns
    /// A proof whose wire format depends on the protocol version: feature
    /// version 0 returns a single GroveDB proof, feature version 1 returns
    /// the two-proof `CompactedAddressBalanceProof` bincode envelope. The
    /// client's `verify_compacted_address_balance_changes` dispatches its
    /// decoder on the same protocol version, so both sides switch formats
    /// together at the version boundary.
    pub fn prove_compacted_address_balance_changes(
        &self,
        start_block_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .saved_block_transactions
            .prove_compacted_address_balance_changes
        {
            0 => self.prove_compacted_address_balance_changes_v0(
                start_block_height,
                limit,
                transaction,
                platform_version,
            ),
            1 => self.prove_compacted_address_balance_changes_v1(
                start_block_height,
                limit,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_compacted_address_balance_changes".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
