use crate::drive::document::paths::{contract_document_type_path, contract_document_type_path_vec};
use crate::drive::document::private_document_store::{
    private_document_entries_page_limit, private_document_of, PRIVATE_DOCUMENT_STORE_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;

impl Drive {
    pub(super) fn prove_private_document_entries_v0(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        start_position: u64,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        private_document_of(document_type)?;
        // Committed state, which the proof is of too
        let chunk_power = self.committed_private_document_store_chunk_power(
            contract_document_type_path(&contract_id, document_type.name()),
            document_type.name(),
            None,
            platform_version,
        )?;
        let limit = private_document_entries_page_limit(chunk_power, start_position)?;
        // GroveDB's position range proof, which `GroveDb::verify_bulk_position_range_proof`
        // checks
        self.grove
            .prove_bulk_position_range(
                contract_document_type_path_vec(&contract_id, document_type.name()),
                &[PRIVATE_DOCUMENT_STORE_KEY],
                start_position,
                limit,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::from)
    }
}
