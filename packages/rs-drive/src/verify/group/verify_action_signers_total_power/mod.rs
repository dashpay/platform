mod v0;

use crate::drive::Drive;
use dpp::data_contract::group::GroupSumPower;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the total signing power for a given group action using a Merkle proof.
    ///
    /// This function checks the integrity and validity of signer entries in the group action's signer sum tree
    /// by processing a GroveDB proof. It computes the total `GroupSumPower` of all signers recorded for the action.
    ///
    /// The method dispatches to a versioned implementation (currently only version 0 is supported).
    ///
    /// # Parameters
    /// - `proof`: A byte slice containing a GroveDB cryptographic proof (e.g., a query proof).
    /// - `contract_id`: The identifier of the data contract defining the group.
    /// - `group_contract_position`: The position/index of the group in the contract.
    /// - `action_status`: Whether the group action is currently `Active` or `Closed`.
    /// - `action_id`: The unique identifier of the group action being verified.
    /// - `verify_subset_of_proof`: If `true`, verifies only the relevant part of the proof for optimization. If `false`, verifies the full structure.
    /// - `platform_version`: A reference to the current `PlatformVersion`, used to determine the correct versioned implementation.
    ///
    /// # Returns
    /// A `Result` with:
    /// - `Ok((root_hash, total_power))` — where:
    ///   - `root_hash`: The Merkle root hash verified during the proof process.
    ///   - `total_power`: The combined `GroupSumPower` of all verified signers.
    /// - `Err(Error)` — if the version is unknown or the proof is invalid.
    ///
    /// # Errors
    /// - [`Error::Drive(DriveError::UnknownVersionMismatch)`] if the method version is unsupported.
    /// - Other variants of [`Error`] may occur if the proof cannot be parsed or verified.
    ///
    /// # Versioning
    /// - Dispatches to `verify_action_signers_total_power_v0` when `platform_version.drive.methods.verify.group.verify_action_signers_total_power == 0`.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn verify_action_signer_and_total_power(
        proof: &[u8],
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: Option<GroupActionStatus>,
        action_id: Identifier,
        action_signer_id: Identifier,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, GroupActionStatus, GroupSumPower), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .group
            .verify_action_signers_total_power
        {
            0 => Self::verify_action_signers_total_power_v0(
                proof,
                contract_id,
                group_contract_position,
                action_status,
                action_id,
                action_signer_id,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_action_signers_total_power".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
