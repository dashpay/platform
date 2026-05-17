use crate::verify::RootHash;

use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::SingleDocumentDriveQuery;

use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl SingleDocumentDriveQuery {
    /// Verifies the proof of a document while keeping it serialized.
    ///
    /// `is_subset` indicates if the function should verify a subset of a larger proof.
    ///
    /// # Parameters
    ///
    /// - `is_subset`: A boolean indicating whether to verify a subset of a larger proof.
    /// - `proof`: A byte slice representing the proof to be verified.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Option<Vec<u8>>`. The `Option<Vec<u8>>`
    /// represents the serialized document if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb verification fails.
    /// - The elements returned are not items, the proof is incorrect.
    #[inline(always)]
    pub(super) fn verify_proof_keep_serialized_v0(
        &self,
        is_subset: bool,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Vec<u8>>), Error> {
        let path_query = self.construct_path_query(platform_version)?;
        let (root_hash, mut proved_key_values) = if is_subset {
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
            return Err(Error::Proof(ProofError::CorruptedProof(format!(
                "we should always get back one element, we got {}",
                proved_key_values.len()
            ))));
        }

        let element = proved_key_values.remove(0).2;

        let serialized = element
            .map(|element| element.into_item_bytes().map_err(Error::from))
            .transpose()?;

        Ok((root_hash, serialized))
    }
}

#[cfg(test)]
mod tests {
    use crate::query::{SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contracts::SystemDataContract;
    use dpp::document::DocumentV0Getters;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_single_document_keep_serialized() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("expected to load DPNS contract");

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("preorder")
            .expect("expected preorder document type");

        // Insert a document
        let document = document_type
            .random_document(Some(77), platform_version)
            .expect("expected a random document");

        let doc_id = document.id();

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert document");

        // Build single document query
        let single_query = SingleDocumentDriveQuery {
            contract_id: contract.id().to_buffer(),
            document_type_name: "preorder".to_string(),
            document_type_keeps_history: false,
            document_id: doc_id.to_buffer(),
            block_time_ms: None,
            contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
        };

        // Generate proof
        let path_query = single_query
            .construct_path_query(platform_version)
            .expect("expected to construct path query");
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("expected to get proof");

        // Verify proof keeping serialized
        let (_root_hash, serialized) = single_query
            .verify_proof_keep_serialized(false, proof.as_slice(), platform_version)
            .expect("expected proof verification to succeed");

        assert!(serialized.is_some(), "expected serialized document bytes");
        assert!(
            !serialized.unwrap().is_empty(),
            "serialized document should not be empty"
        );
    }

    #[test]
    fn should_prove_and_verify_absent_single_document_keep_serialized() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("expected to load DPNS contract");

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to apply contract");

        let _document_type = contract
            .document_type_for_name("preorder")
            .expect("expected preorder document type");

        // No document inserted
        let single_query = SingleDocumentDriveQuery {
            contract_id: contract.id().to_buffer(),
            document_type_name: "preorder".to_string(),
            document_type_keeps_history: false,
            document_id: [2u8; 32],
            block_time_ms: None,
            contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
        };

        let path_query = single_query
            .construct_path_query(platform_version)
            .expect("expected to construct path query");
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("expected to get proof");

        let (_root_hash, serialized) = single_query
            .verify_proof_keep_serialized(false, proof.as_slice(), platform_version)
            .expect("expected proof verification to succeed");

        assert!(serialized.is_none(), "expected no serialized document");
    }
}
