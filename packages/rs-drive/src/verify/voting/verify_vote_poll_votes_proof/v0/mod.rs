use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::platform_value;
use grovedb::GroveDb;

use crate::error::Error;

use crate::query::vote_poll_contestant_votes_query::ResolvedContestedDocumentVotePollVotesDriveQuery;
use dpp::version::PlatformVersion;

impl<'a> ResolvedContestedDocumentVotePollVotesDriveQuery<'a> {
    /// Verifies a proof for a collection of documents.
    ///
    /// This function takes a slice of bytes `proof` containing a serialized proof,
    /// verifies it, and returns a tuple consisting of the root hash and a vector of deserialized documents.
    ///
    /// # Arguments
    ///
    /// * `proof` - A byte slice representing the proof to be verified.
    /// * `drive_version` - The current active drive version
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// * A tuple with the root hash and a vector of deserialized `Document`s, if the proof is valid.
    /// * An `Error` variant, in case the proof verification fails or deserialization error occurs.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` variant if:
    /// 1. The proof verification fails.
    /// 2. There is a deserialization error when parsing the serialized document(s) into `Document` struct(s).
    #[inline(always)]
    pub(crate) fn verify_vote_poll_votes_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Identifier>), Error> {
        let path_query = self.construct_path_query(platform_version)?;
        let (root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;
        let voters = proved_key_values
            .into_iter()
            .map(|(_, voter_id, _)| Identifier::try_from(voter_id))
            .collect::<Result<Vec<Identifier>, platform_value::Error>>()?;

        Ok((root_hash, voters))
    }
}
