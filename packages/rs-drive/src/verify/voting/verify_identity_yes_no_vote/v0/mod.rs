use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::masternode_yes_no_vote_query::{
    MasternodeYesNoVote, MasternodeYesNoVoteDriveQuery,
};
use crate::verify::bounded_decode::decode_yes_no_vote_reference;
use crate::verify::RootHash;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl MasternodeYesNoVoteDriveQuery {
    /// Version 0: the prover's own path query, one key, present or proved absent.
    #[inline(always)]
    pub(crate) fn verify_identity_yes_no_vote_v0(
        &self,
        proof: &[u8],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<MasternodeYesNoVote>), Error> {
        let path_query = self.construct_path_query();
        let (root_hash, mut proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };
        if proved_key_values.len() != 1 {
            return Err(Error::Proof(ProofError::TooManyElements(
                "expected one masternode yes/no vote",
            )));
        }
        let (_, _, maybe_element) = proved_key_values.remove(0);
        let maybe_vote = maybe_element
            .map(|element| {
                let bytes = element.into_item_bytes()?;
                let storage_form = decode_yes_no_vote_reference(&bytes)?;
                Ok::<MasternodeYesNoVote, Error>(MasternodeYesNoVote {
                    vote_choice: storage_form.vote_choice,
                    times_voted: storage_form.identity_vote_times,
                })
            })
            .transpose()?;
        Ok((root_hash, maybe_vote))
    }
}
