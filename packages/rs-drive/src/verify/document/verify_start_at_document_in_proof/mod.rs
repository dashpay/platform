mod v0;

use crate::error::drive::DriveError;
use crate::verify::RootHash;

use crate::error::Error;
use crate::query::DriveDocumentQuery;
use dpp::document::Document;
use dpp::version::PlatformVersion;

impl DriveDocumentQuery<'_> {
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
    pub fn verify_start_at_document_in_proof(
        &self,
        proof: &[u8],
        is_proof_subset: bool,
        document_id: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Document>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document
            .verify_start_at_document_in_proof
        {
            0 => self.verify_start_at_document_in_proof_v0(
                proof,
                is_proof_subset,
                document_id,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_start_at_document_in_proof".to_string(),
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
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contracts::SystemDataContract;
    use dpp::system_data_contracts::load_system_data_contract;

    #[test]
    fn test_document_verify_start_at_document_in_proof_unknown_version() {
        let platform_version = PlatformVersion::latest();
        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("expected to load DPNS contract");
        let document_type = contract
            .document_type_for_name("domain")
            .expect("expected domain document type");

        let mut platform_version = platform_version.clone();
        platform_version
            .drive
            .methods
            .verify
            .document
            .verify_start_at_document_in_proof = 255;

        let query = DriveDocumentQuery {
            contract: &contract,
            document_type,
            internal_clauses: Default::default(),
            offset: None,
            limit: None,
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
            resolved_time_ranges: vec![],
        };

        let result =
            query.verify_start_at_document_in_proof(&[], false, [0u8; 32], &platform_version);

        assert!(
            matches!(result, Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                if method == "verify_start_at_document_in_proof" && known_versions == vec![0] && received == 255
            )
        );
    }
}
