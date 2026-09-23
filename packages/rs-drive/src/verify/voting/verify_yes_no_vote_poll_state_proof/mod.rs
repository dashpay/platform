mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::yes_no_vote_poll_state_query::{
    YesNoVotePollState, YesNoVotePollStateDriveQuery,
};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl YesNoVotePollStateDriveQuery {
    /// Verifies a proof of the state of a yes/no vote poll: its stored info and its tallies.
    pub fn verify_yes_no_vote_poll_state_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, YesNoVotePollState), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_yes_no_vote_poll_state_proof
        {
            0 => self.verify_yes_no_vote_poll_state_proof_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_yes_no_vote_poll_state_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
