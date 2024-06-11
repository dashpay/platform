mod v0;

use crate::drive::verify::RootHash;
use crate::error::drive::DriveError;
use dpp::prelude::TimestampMillis;
use std::collections::BTreeMap;

use crate::error::Error;

use crate::query::VotePollsByEndDateDriveQuery;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::VotePoll;

impl VotePollsByEndDateDriveQuery {
    /// Verifies the serialized proof for a vote poll based on the end date.
    ///
    /// This function analyzes a byte slice which contains the serialized proof, performs verification, and returns
    /// the results, which include the root hash of the proof and a structured map of the contests.
    ///
    /// # Arguments
    ///
    /// * `proof` - A byte slice of the serialized proof that needs to be verified.
    /// * `platform_version` - The version of the platform which defines how the proof should be verified.
    ///
    /// # Returns
    ///
    /// A `Result` which is either:
    /// * `Ok((RootHash, BTreeMap<TimestampMillis, Vec<ContestedDocumentResourceVotePoll>))` if the proof is verified successfully.
    /// * `Err(Error)` if there is a failure in proof verification or during the deserialization of documents.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// 1. Proof verification fails due to invalid data or signature issues.
    /// 2. Deserialization error occurs due to malformed data or incompatible types in the document(s).
    ///
    pub fn verify_vote_polls_by_end_date_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, BTreeMap<TimestampMillis, Vec<VotePoll>>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_vote_polls_by_end_date_proof
        {
            0 => self.verify_vote_polls_by_end_date_proof_v0(proof),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_vote_polls_by_end_date_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
