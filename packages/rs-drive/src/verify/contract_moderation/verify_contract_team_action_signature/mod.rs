mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof of one member's approval of one of a contract's team actions, wherever
    /// it is, and returns whether the action is still active or closed (it ran). What proves a
    /// team action's proposal or approval executed: an approval stays where it is written until
    /// its action closes, and then moves with it, so the proof holds while the approval stands.
    /// An approval deleted because its member left the team no longer proves.
    ///
    /// # Parameters
    ///
    /// * `proof`: The proof, as `prove_state_transition` produced it for the transition.
    /// * `contract_id`: The contract whose seated team votes on the action.
    /// * `action_id`: The action.
    /// * `signer_id`: The member whose approval the proof must hold.
    /// * `verify_subset_of_proof`: Whether the proof may prove more than this query (verified as
    ///   a subset).
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((RootHash, GroupActionStatus))` with the proof's root hash and where the approval
    ///   is.
    /// * `Err(Error)` when the method version is unknown, the proof fails verification, or it
    ///   holds the approval nowhere, or in both places.
    pub fn verify_contract_team_action_signature(
        proof: &[u8],
        contract_id: Identifier,
        action_id: Identifier,
        signer_id: Identifier,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, GroupActionStatus), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract_moderation
            .verify_contract_team_action_signature
        {
            0 => Self::verify_contract_team_action_signature_v0(
                proof,
                contract_id,
                action_id,
                signer_id,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contract_team_action_signature".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
