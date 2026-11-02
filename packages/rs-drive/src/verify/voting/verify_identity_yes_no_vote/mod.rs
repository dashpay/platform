mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::masternode_yes_no_vote_query::{
    MasternodeYesNoVote, MasternodeYesNoVoteDriveQuery,
};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl MasternodeYesNoVoteDriveQuery {
    /// Verifies a proof of a masternode's answer on a yes/no poll, by the poll's id, by the
    /// method version the platform version selects (`verify.voting.verify_identity_yes_no_vote`,
    /// not active before protocol version 15).
    ///
    /// # Parameters
    /// - `proof`: the proof.
    /// - `verify_subset_of_proof`: whether the proof may hold more than this answer.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// The root hash and the proved answer, `None` when the masternode has not voted on the
    /// poll (or the poll finished and its votes were removed).
    pub fn verify_identity_yes_no_vote(
        &self,
        proof: &[u8],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<MasternodeYesNoVote>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_identity_yes_no_vote
        {
            Some(0) => {
                self.verify_identity_yes_no_vote_v0(proof, verify_subset_of_proof, platform_version)
            }
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_identity_yes_no_vote".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "verify_identity_yes_no_vote".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
