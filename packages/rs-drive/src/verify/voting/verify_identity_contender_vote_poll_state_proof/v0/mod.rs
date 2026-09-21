use crate::error::Error;
use crate::query::identity_contender_vote_poll_state_query::{
    IdentityContenderVotePollState, IdentityContenderVotePollStateQuery,
};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl IdentityContenderVotePollStateQuery {
    #[inline(always)]
    pub(super) fn verify_identity_contender_vote_poll_state_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, IdentityContenderVotePollState), Error> {
        let path_query = self.construct_path_query();
        let (root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;
        let state = self.state_from_elements(proved_key_values, false)?;
        Ok((root_hash, state))
    }
}
