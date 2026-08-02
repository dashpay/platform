mod v0;
mod v1;

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

/// Proof envelope for compacted address balance changes.
///
/// The predecessor proof independently authenticates which range, if any,
/// contains the requested height. The forward proof can then be verified
/// against a query derived only from that authenticated result.
#[derive(Debug, bincode::Encode, bincode::Decode)]
pub(crate) struct CompactedAddressBalanceProof {
    pub(crate) predecessor_proof: Vec<u8>,
    pub(crate) forward_proof: Vec<u8>,
}

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
    ///
    /// # Wire format versioning
    /// Feature version 0 decodes the legacy single GroveDB proof; feature
    /// version 1 decodes the two-proof [`CompactedAddressBalanceProof`]
    /// envelope. The server's
    /// `prove_compacted_address_balance_changes` dispatches its encoder on
    /// the same protocol version, so both sides switch formats together at
    /// the version boundary.
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
            1 => Self::verify_compacted_address_balance_changes_v1(
                proof,
                start_block_height,
                limit,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_compacted_address_balance_changes".to_string(),
                known_versions: vec![0, 1],
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
    fn test_verify_compacted_address_balance_changes_unknown_version_mismatch() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .address_funds
            .verify_compacted_address_balance_changes = 255;

        let result =
            Drive::verify_compacted_address_balance_changes(&[], 0, None, &platform_version);

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
