mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

/// Result type for verified address balance changes per block
pub type VerifiedAddressBalanceChangesPerBlock =
    Vec<(u64, BTreeMap<PlatformAddress, CreditOperation>)>;

impl Drive {
    /// Verifies the proof of recent address balance changes starting from a given block height.
    ///
    /// This method validates and extracts address balance changes from the provided proof.
    ///
    /// # Arguments
    /// - `proof`: A byte slice containing the cryptographic proof for the address balance changes.
    /// - `start_block_height`: The block height to start verifying from.
    /// - `limit`: Optional maximum number of blocks to verify.
    /// - `verify_subset_of_proof`: A boolean flag indicating whether to verify only a subset of the proof.
    /// - `platform_version`: A reference to the platform version.
    ///
    /// # Returns
    /// - `Ok((RootHash, VerifiedAddressBalanceChangesPerBlock))`: On success, returns:
    ///   - `RootHash`: The root hash of the Merkle tree.
    ///   - `VerifiedAddressBalanceChangesPerBlock`: Vector of (block_height, address_balance_changes) tuples.
    /// - `Err(Error)`: If verification fails.
    pub fn verify_recent_address_balance_changes(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, VerifiedAddressBalanceChangesPerBlock), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .address_funds
            .verify_recent_address_balance_changes
        {
            0 => Self::verify_recent_address_balance_changes_v0(
                proof,
                start_block_height,
                limit,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_recent_address_balance_changes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_verify_recent_address_balance_changes_unknown_version_mismatch() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .address_funds
            .verify_recent_address_balance_changes = 255;

        let result =
            Drive::verify_recent_address_balance_changes(&[], 0, None, false, &platform_version);

        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::UnknownVersionMismatch { .. }))
            ),
            "expected UnknownVersionMismatch, got {:?}",
            result,
        );
    }
}
