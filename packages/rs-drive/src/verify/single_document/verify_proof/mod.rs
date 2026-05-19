mod v0;

use crate::verify::RootHash;

use crate::error::Error;
use crate::query::SingleDocumentDriveQuery;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::Document;

use crate::error::drive::DriveError;

use dpp::version::PlatformVersion;

impl SingleDocumentDriveQuery {
    /// Verifies the proof of a single document query.
    ///
    /// `is_subset` indicates if the function should verify a subset of a larger proof.
    ///
    /// # Parameters
    ///
    /// - `is_subset`: A boolean indicating whether to verify a subset of a larger proof.
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `document_type`: The type of the document being verified.
    /// - `platform_version`: The platform version against which to verify the proof.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Option<Document>`. The `Option<Document>`
    /// represents the deserialized document if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - An unknown or unsupported platform version is provided.
    /// - Any other error as documented in the specific versioned function.
    pub fn verify_proof(
        &self,
        is_subset: bool,
        proof: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Document>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .single_document
            .verify_proof
        {
            0 => self.verify_proof_v0(is_subset, proof, document_type, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "SingleDocumentDriveQuery::verify_proof".to_string(),
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
    use crate::query::SingleDocumentDriveQueryContestedStatus;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contracts::SystemDataContract;
    use dpp::system_data_contracts::load_system_data_contract;

    #[test]
    fn test_single_document_verify_proof_unknown_version() {
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
            .single_document
            .verify_proof = 255;

        let query = SingleDocumentDriveQuery {
            contract_id: [0u8; 32],
            document_type_name: "domain".to_string(),
            document_type_keeps_history: false,
            document_id: [0u8; 32],
            block_time_ms: None,
            contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
        };

        let result = query.verify_proof(false, &[], document_type, &platform_version);

        assert!(
            matches!(result, Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                if method == "SingleDocumentDriveQuery::verify_proof" && known_versions == vec![0] && received == 255
            )
        );
    }
}
