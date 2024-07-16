mod v0;

use crate::verify::RootHash;
use crate::error::drive::DriveError;

use crate::error::Error;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use crate::query::vote_poll_contestant_votes_query::ResolvedContestedDocumentVotePollVotesDriveQuery;
use crate::query::vote_poll_vote_state_query::Contender;

impl<'a> ResolvedContestedDocumentVotePollVotesDriveQuery<'a> {
    /// Verifies if a document exists at the beginning of a proof,
    /// and returns the root hash and the optionally found document.
    ///
    /// # Arguments
    ///
    /// * `proof` - A byte slice containing the proof data.
    /// * `is_proof_subset` - A boolean indicating whether the proof is a subset query or not.
    /// * `document_id` - A byte_32 array, representing the ID of the document to start at.
    /// * `platform_version` - The platform version against which to verify the proof.
    ///
    /// # Returns
    ///
    /// A `Result` with a tuple containing:
    /// * The root hash of the verified proof.
    /// * An `Option<Document>` containing the found document if available.
    ///
    /// # Errors
    ///
    /// This function returns an Error in the following cases:
    /// * If the proof is corrupted (wrong path, wrong key, etc.).
    /// * If the provided proof has an incorrect number of elements.
    pub fn verify_start_at_contender_in_proof(
        &self,
        proof: &[u8],
        is_proof_subset: bool,
        document_id: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Contender>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_start_at_contender_in_proof
        {
            0 => self.verify_start_at_contender_in_proof_v0(
                proof,
                is_proof_subset,
                document_id,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_start_at_contender_in_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
