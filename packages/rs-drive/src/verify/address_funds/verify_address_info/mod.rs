mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the proof of a single address's balance and nonce information.
    ///
    /// This method validates and extracts a specific address's balance and nonce based on the provided proof.
    /// It ensures the integrity and authenticity of the data associated with the specified address.
    /// The method supports multiple versions for backward compatibility and forwards the verification logic
    /// to the appropriate versioned implementation.
    ///
    /// # Arguments
    /// - `proof`: A byte slice containing the cryptographic proof for the address information.
    /// - `address`: The platform address to verify.
    /// - `verify_subset_of_proof`: A boolean flag indicating whether to verify only a subset of the proof (useful for optimizations).
    /// - `platform_version`: A reference to the platform version, used to determine the appropriate versioned implementation.
    ///
    /// # Returns
    /// - `Ok((RootHash, Option<(AddressNonce, Credits)>))`: On success, returns a tuple containing:
    ///   - `RootHash`: The root hash of the Merkle tree, confirming the proof's validity.
    ///   - `Option<(AddressNonce, Credits)>`: The verified address balance and nonce if it exists, or `None` if the address is absent.
    /// - `Err(Error)`: If verification fails, returns an [`Error`] indicating the cause of failure.
    ///
    /// # Errors
    /// - [`Error::Proof`]: If the proof is invalid, corrupted, or contains unexpected data structures.
    /// - [`Error::Drive(DriveError::UnknownVersionMismatch)`]: If the method is called with an unsupported platform version.
    /// - [`Error::GroveDB`]: If the data deserialization or conversion fails during proof verification.
    pub fn verify_address_info(
        proof: &[u8],
        address: &PlatformAddress,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<(AddressNonce, Credits)>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .address_funds
            .verify_address_info
        {
            0 => Self::verify_address_info_v0(
                proof,
                address,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_address_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::PlatformAddress;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_verify_address_info_unknown_version_mismatch() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .address_funds
            .verify_address_info = 255;

        let address = PlatformAddress::P2pkh([0u8; 20]);

        let result = Drive::verify_address_info(&[], &address, false, &platform_version);

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
