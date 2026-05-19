//! Verification of the vote poll proofs

mod v0;

use crate::error::drive::DriveError;
use crate::verify::RootHash;

use crate::error::Error;

use crate::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQueryExecutionResult, ResolvedContestedDocumentVotePollDriveQuery,
};
use dpp::version::PlatformVersion;

impl ResolvedContestedDocumentVotePollDriveQuery<'_> {
    /// Verifies a proof for the vote poll vote state proof.
    ///
    /// This function takes a byte slice representing the serialized proof, verifies it, and returns a tuple consisting of the root hash
    /// and a vector of deserialized contenders.
    ///
    /// # Arguments
    ///
    /// * `proof` - A byte slice representing the proof to be verified.
    /// * `platform_version` - The platform version against which to verify the proof.
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// * A tuple with the root hash and a vector of deserialized `Document`s if the proof is valid.
    /// * An `Error` variant, in case the proof verification fails or a deserialization error occurs.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` variant if:
    /// 1. The proof verification fails.
    /// 2. A deserialization error occurs when parsing the serialized document(s).
    pub fn verify_vote_poll_vote_state_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, ContestedDocumentVotePollDriveQueryExecutionResult), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_vote_poll_vote_state_proof
        {
            0 => self.verify_vote_poll_vote_state_proof_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_vote_poll_vote_state_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed;
    use crate::error::drive::DriveError;
    use crate::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQueryResultType;
    use crate::util::object_size_info::DataContractResolvedInfo;
    use dpp::tests::json_document::json_document_to_contract;
    use std::sync::Arc;

    #[test]
    fn test_verify_vote_poll_vote_state_proof_unknown_version() {
        let platform_version = PlatformVersion::latest();
        let data_contract = json_document_to_contract(
            "tests/supporting_files/contract/dpns/dpns-contract.json",
            false,
            platform_version,
        )
        .expect("expected to create a data contract");

        let query = ResolvedContestedDocumentVotePollDriveQuery {
            vote_poll: ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
                contract: DataContractResolvedInfo::ArcDataContract(Arc::new(data_contract)),
                document_type_name: String::new(),
                index_name: String::new(),
                index_values: vec![],
            },
            result_type: ContestedDocumentVotePollDriveQueryResultType::Documents,
            offset: None,
            limit: None,
            start_at: None,
            allow_include_locked_and_abstaining_vote_tally: false,
        };

        let mut platform_version = platform_version.clone();
        platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_vote_poll_vote_state_proof = 255;

        let result = query.verify_vote_poll_vote_state_proof(&[], &platform_version);

        assert!(
            matches!(result, Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                if method == "verify_vote_poll_vote_state_proof" && known_versions == vec![0] && received == 255
            )
        );
    }
}
