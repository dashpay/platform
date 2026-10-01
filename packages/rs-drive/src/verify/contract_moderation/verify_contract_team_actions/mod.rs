mod v0;

use crate::drive::contract::moderation::types::{
    ContractTeamActionEntry, ContractTeamActionsQuery,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof of one page of a contract's team actions, active or closed as the query
    /// says, and returns the actions the proof holds, in action id order.
    ///
    /// # Parameters
    ///
    /// * `proof`: The proof, as `prove_contract_team_actions` produced it.
    /// * `contract_id`: The contract whose seated team votes on the actions.
    /// * `query`: The status, the start and the limit the proof was built for.
    /// * `verify_subset_of_proof`: Whether the proof may prove more than this query (verified as
    ///   a subset).
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((RootHash, Vec<ContractTeamActionEntry>))` with the proof's root hash and the
    ///   proven actions, in action id order.
    /// * `Err(Error)` when the method version is unknown, the proof fails verification, or a
    ///   proven action is malformed.
    pub fn verify_contract_team_actions(
        proof: &[u8],
        contract_id: Identifier,
        query: &ContractTeamActionsQuery,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<ContractTeamActionEntry>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract_moderation
            .verify_contract_team_actions
        {
            0 => Self::verify_contract_team_actions_v0(
                proof,
                contract_id,
                query,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contract_team_actions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
