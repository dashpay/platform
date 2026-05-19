mod v0;

use crate::error::drive::DriveError;
use crate::verify::RootHash;
use dpp::platform_value::Value;

use crate::error::Error;

use crate::query::vote_polls_by_document_type_query::ResolvedVotePollsByDocumentTypeQuery;
use dpp::version::PlatformVersion;

impl ResolvedVotePollsByDocumentTypeQuery<'_> {
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
    pub fn verify_contests_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Value>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_contests_proof
        {
            0 => self.verify_contests_proof_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contests_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::object_size_info::DataContractResolvedInfo;
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::version::PlatformVersion;
    use std::sync::Arc;

    #[test]
    fn test_verify_contests_proof_unknown_version() {
        let platform_version = PlatformVersion::latest();
        let data_contract = json_document_to_contract(
            "tests/supporting_files/contract/dpns/dpns-contract.json",
            false,
            platform_version,
        )
        .expect("expected to create a data contract");

        let doc_type_name = String::new();
        let index_name = String::new();
        let start_index_values = vec![];
        let end_index_values = vec![];
        let start_at_value = None;

        let query = ResolvedVotePollsByDocumentTypeQuery {
            contract: DataContractResolvedInfo::ArcDataContract(Arc::new(data_contract)),
            document_type_name: &doc_type_name,
            index_name: &index_name,
            start_index_values: &start_index_values,
            end_index_values: &end_index_values,
            start_at_value: &start_at_value,
            limit: None,
            order_ascending: true,
        };

        let mut platform_version = platform_version.clone();
        platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_contests_proof = 255;

        let result = query.verify_contests_proof(&[], &platform_version);

        assert!(
            matches!(result, Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                if method == "verify_contests_proof" && known_versions == vec![0] && received == 255
            )
        );
    }
}
