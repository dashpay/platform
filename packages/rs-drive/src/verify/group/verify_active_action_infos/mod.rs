mod v0;

use crate::drive::Drive;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::prelude::StartAtIncluded;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the proof of action information within a contract.
    ///
    /// This method validates and extracts action information stored in a contract based on the provided proof.
    /// It uses the proof to confirm the integrity and authenticity of the action data. The method supports
    /// different versions for backward compatibility and forwards the verification logic to the appropriate versioned implementation.
    ///
    /// # Type Parameters
    /// - `T`: The output container type that implements `FromIterator`. This is used to collect the verified action information
    ///   as pairs of [`Identifier`] and [`GroupAction`].
    ///
    /// # Arguments
    /// - `proof`: A byte slice containing the cryptographic proof for the active_action information.
    /// - `contract_id`: The identifier of the contract whose active_action information is being verified.
    /// - `start_active_action_contract_position`: An optional starting position for the active_action query, combined with a [`StartAtIncluded`] flag
    ///   to indicate whether the start position is inclusive.
    /// - `limit`: An optional limit on the number of active_actions to verify.
    /// - `verify_subset_of_proof`: A boolean flag indicating whether to verify only a subset of the proof (useful for optimizations).
    /// - `platform_version`: A reference to the platform version, used to determine the appropriate versioned implementation.
    ///
    /// # Returns
    /// - `Ok((RootHash, T))`: On success, returns a tuple containing:
    ///   - `RootHash`: The root hash of the Merkle tree, confirming the proof's validity.
    ///   - `T`: A collection of verified active_action information as pairs of [`GroupContractPosition`] and [`Group`].
    ///
    /// # Errors
    /// - [`Error::Proof`]: If the proof is invalid, corrupted, or contains unexpected data structures.
    /// - [`Error::Drive(DriveError::UnknownVersionMismatch)`]: If the method is called with an unsupported platform version.
    /// - Any other errors propagated from the versioned implementation.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_action_infos_in_contract<T: FromIterator<(Identifier, GroupAction)>>(
        proof: &[u8],
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        start_action_id: Option<(Identifier, StartAtIncluded)>,
        limit: Option<u16>,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .group
            .verify_action_infos
        {
            0 => Self::verify_action_infos_in_contract_v0(
                proof,
                contract_id,
                group_contract_position,
                action_status,
                start_action_id,
                limit,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_action_infos_in_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
