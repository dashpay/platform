mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::fee::Credits;
use dpp::identity::KeyOfType;
use dpp::prelude::KeyOfTypeNonce;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the proof of multiple addresses' balance and nonce information.
    ///
    /// This method validates and extracts balance and nonce information for multiple addresses based on the provided proof.
    /// It uses the proof to confirm the integrity and authenticity of the address data. The method supports
    /// different versions for backward compatibility and forwards the verification logic to the appropriate versioned implementation.
    ///
    /// # Type Parameters
    /// - `T`: The output container type that implements `FromIterator`. This is used to collect the verified address information
    ///   as pairs of [`KeyOfType`] and `Option<(KeyOfTypeNonce, Credits)>`.
    ///
    /// # Arguments
    /// - `proof`: A byte slice containing the cryptographic proof for the address information.
    /// - `keys_of_type`: An iterator over the addresses to verify.
    /// - `verify_subset_of_proof`: A boolean flag indicating whether to verify only a subset of the proof (useful for optimizations).
    /// - `platform_version`: A reference to the platform version, used to determine the appropriate versioned implementation.
    ///
    /// # Returns
    /// - `Ok((RootHash, T))`: On success, returns a tuple containing:
    ///   - `RootHash`: The root hash of the Merkle tree, confirming the proof's validity.
    ///   - `T`: A collection of verified address information as pairs of [`KeyOfType`] and `Option<(KeyOfTypeNonce, Credits)>`.
    /// - `Err(Error)`: If verification fails, returns an [`Error`] indicating the cause of failure.
    ///
    /// # Errors
    /// - [`Error::Proof`]: If the proof is invalid, corrupted, or contains unexpected data structures.
    /// - [`Error::Drive(DriveError::UnknownVersionMismatch)`]: If the method is called with an unsupported platform version.
    /// - Any other errors propagated from the versioned implementation.
    pub fn verify_addresses_infos<
        'a,
        I: IntoIterator<Item = &'a KeyOfType>,
        T: FromIterator<(KeyOfType, Option<(KeyOfTypeNonce, Credits)>)>,
    >(
        proof: &[u8],
        keys_of_type: I,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .address_funds
            .verify_addresses_infos
        {
            0 => Self::verify_addresses_infos_v0(
                proof,
                keys_of_type,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_addresses_infos".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
