mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof of who approved one of a contract's team actions, active or closed as
    /// `status` says, and returns the members the proof holds, in identity id order; none when
    /// there is no such action.
    ///
    /// # Parameters
    ///
    /// * `proof`: The proof, as `prove_contract_team_action_signers` produced it.
    /// * `contract_id`: The contract whose seated team votes on the action.
    /// * `status`: Whether the action is active or closed.
    /// * `action_id`: The action.
    /// * `verify_subset_of_proof`: Whether the proof may prove more than this query (verified as
    ///   a subset).
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((RootHash, Vec<Identifier>))` with the proof's root hash and the proven approvals.
    /// * `Err(Error)` when the method version is unknown, the proof fails verification, or a
    ///   proven approval is malformed.
    pub fn verify_contract_team_action_signers(
        proof: &[u8],
        contract_id: Identifier,
        status: GroupActionStatus,
        action_id: Identifier,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Identifier>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract_moderation
            .verify_contract_team_action_signers
        {
            0 => Self::verify_contract_team_action_signers_v0(
                proof,
                contract_id,
                status,
                action_id,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contract_team_action_signers".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
