mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::BlockAwareCreditOperation;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

/// Result type for verified compacted address balance changes
/// Each entry is (start_block, end_block, address_balance_changes)
pub type VerifiedCompactedAddressBalanceChanges = Vec<(
    u64,
    u64,
    BTreeMap<PlatformAddress, BlockAwareCreditOperation>,
)>;

impl Drive {
    /// Verifies the proof of compacted address balance changes starting from a given block height.
    ///
    /// This method validates and extracts compacted address balance changes from the provided proof.
    /// Compacted entries represent merged data from multiple blocks.
    ///
    /// # Arguments
    /// - `proof`: A byte slice containing the cryptographic proof for the compacted address balance changes.
    /// - `start_block_height`: The block height to start verifying from.
    /// - `limit`: Optional maximum number of compacted entries to verify.
    /// - `platform_version`: A reference to the platform version.
    ///
    /// # Returns
    /// - `Ok((RootHash, VerifiedCompactedAddressBalanceChanges))`: On success, returns:
    ///   - `RootHash`: The root hash of the Merkle tree.
    ///   - `VerifiedCompactedAddressBalanceChanges`: Vector of (start_block, end_block, address_balance_changes) tuples.
    /// - `Err(Error)`: If verification fails.
    pub fn verify_compacted_address_balance_changes(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, VerifiedCompactedAddressBalanceChanges), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .address_funds
            .verify_compacted_address_balance_changes
        {
            0 => Self::verify_compacted_address_balance_changes_v0(
                proof,
                start_block_height,
                limit,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_compacted_address_balance_changes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
