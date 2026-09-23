use crate::error::Error;
use crate::query::yes_no_vote_poll_state_query::{
    YesNoVotePollState, YesNoVotePollStateDriveQuery,
};
use crate::verify::RootHash;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl YesNoVotePollStateDriveQuery {
    #[inline(always)]
    pub(super) fn verify_yes_no_vote_poll_state_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, YesNoVotePollState), Error> {
        let path_query = self.construct_path_query()?;
        let (root_hash, proved_key_values) = GroveDb::verify_query_with_absence_proof(
            proof,
            &path_query,
            &platform_version.drive.grove_version,
        )?;
        let key_elements = proved_key_values
            .into_iter()
            .filter_map(|(_, key, maybe_element)| maybe_element.map(|element| (key, element)));
        let state = Self::state_from_key_elements(key_elements)?;
        Ok((root_hash, state))
    }
}
