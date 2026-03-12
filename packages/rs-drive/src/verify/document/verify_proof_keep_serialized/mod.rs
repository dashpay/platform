mod v0;

use crate::error::drive::DriveError;
use crate::verify::RootHash;

use crate::error::Error;
use crate::query::DriveDocumentQuery;

use dpp::version::PlatformVersion;

impl DriveDocumentQuery<'_> {
    /// Verifies the given proof and returns the root hash of the GroveDB tree and a vector
    /// of serialized documents if the verification is successful.
    ///
    /// # Arguments
    /// * `proof` - A byte slice representing the proof to be verified.
    /// * `platform_version` - The platform version against which to verify the proof.
    ///
    /// # Returns
    /// * On success, returns a tuple containing the root hash of the GroveDB tree and a vector of serialized documents.
    /// * On failure, returns an Error.
    ///
    /// # Errors
    /// This function will return an Error if:
    /// 1. The start at document is not present in proof and it is expected to be.
    /// 2. The path query fails to verify against the given proof.
    /// 3. Converting the element into bytes fails.
    pub fn verify_proof_keep_serialized(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Vec<u8>>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document
            .verify_proof_keep_serialized
        {
            0 => self.verify_proof_keep_serialized_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_proof_keep_serialized".to_string(),
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
    fn test_document_verify_proof_keep_serialized_unknown_version() {
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
            .verify_proof_keep_serialized = 255;

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
        };

        let result = query.verify_proof_keep_serialized(&[], &platform_version);

        assert!(
            matches!(result, Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                if method == "verify_proof_keep_serialized" && known_versions == vec![0] && received == 255
            )
        );
    }
}
