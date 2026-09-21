//! Verification of the proof of an identity contender vote poll's state
mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::identity_contender_vote_poll_state_query::{
    IdentityContenderVotePollState, IdentityContenderVotePollStateQuery,
};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl IdentityContenderVotePollStateQuery {
    /// Verifies a proof of the state of an identity contender vote poll and returns the state
    /// the proof holds: an empty state, with no stored info, for a poll that never opened.
    pub fn verify_identity_contender_vote_poll_state_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, IdentityContenderVotePollState), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_identity_contender_vote_poll_state_proof
        {
            0 => self.verify_identity_contender_vote_poll_state_proof_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_identity_contender_vote_poll_state_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identifier::Identifier;

    #[test]
    fn should_refuse_an_unknown_version() {
        let query = IdentityContenderVotePollStateQuery {
            vote_poll_id: Identifier::new([1; 32]),
            limit: None,
            start_at: None,
        };
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_identity_contender_vote_poll_state_proof = 255;
        let result = query.verify_identity_contender_vote_poll_state_proof(&[], &platform_version);
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::UnknownVersionMismatch {
                received: 255,
                ..
            }))
        ));
    }
}
