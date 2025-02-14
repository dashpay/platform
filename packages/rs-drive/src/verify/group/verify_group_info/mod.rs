mod v0;

use crate::drive::Drive;
use dpp::data_contract::group::Group;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the proof of a single group's information within a contract.
    ///
    /// This method validates and extracts a specific group's information stored in a contract based on the provided proof.
    /// It ensures the integrity and authenticity of the data associated with the specified group position in the contract.
    /// The method supports multiple versions for backward compatibility and forwards the verification logic
    /// to the appropriate versioned implementation.
    ///
    /// # Arguments
    /// - `proof`: A byte slice containing the cryptographic proof for the group information.
    /// - `contract_id`: The identifier of the contract containing the group information.
    /// - `group_contract_position`: The position of the group within the contract to verify.
    /// - `verify_subset_of_proof`: A boolean flag indicating whether to verify only a subset of the proof (useful for optimizations).
    /// - `platform_version`: A reference to the platform version, used to determine the appropriate versioned implementation.
    ///
    /// # Returns
    /// - `Ok((RootHash, Option<Group>))`: On success, returns a tuple containing:
    ///   - `RootHash`: The root hash of the Merkle tree, confirming the proof's validity.
    ///   - `Option<Group>`: The verified group information if it exists, or `None` if the group is absent.
    /// - `Err(Error)`: If verification fails, returns an [`Error`] indicating the cause of failure.
    ///
    /// # Errors
    /// - [`Error::Proof`]: If the proof is invalid, corrupted, or contains unexpected data structures.
    /// - [`Error::Drive(DriveError::UnknownVersionMismatch)`]: If the method is called with an unsupported platform version.
    /// - [`Error::GroveDB`]: If the data deserialization or conversion fails during proof verification.
    pub fn verify_group_info(
        proof: &[u8],
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Group>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .group
            .verify_group_info
        {
            0 => Self::verify_group_info_v0(
                proof,
                contract_id,
                group_contract_position,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_group_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
