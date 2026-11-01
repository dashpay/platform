use crate::drive::document::paths::contract_document_type_path_vec;
use crate::drive::document::private_document_store::{
    private_document_entries_page_limit, private_document_of, PrivateDocumentEntries,
    PrivateDocumentEntry, PRIVATE_DOCUMENT_STORE_KEY,
};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl Drive {
    pub(super) fn verify_private_document_entries_v0(
        proof: &[u8],
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        start_position: u64,
        count: u32,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, PrivateDocumentEntries), Error> {
        let private_document = private_document_of(document_type)?;
        let limit = private_document_entries_page_limit(
            private_document,
            start_position,
            count,
            platform_version,
        )?;
        // Verifies the canonical range query the prover used, requires the page to be complete
        // up to the store's count, and reads that count from the store element the same proof
        // binds to the root hash.
        let (root_hash, page) = GroveDb::verify_bulk_position_range_proof(
            proof,
            contract_document_type_path_vec(&contract_id, document_type.name()),
            &[PRIVATE_DOCUMENT_STORE_KEY],
            start_position,
            limit,
            &platform_version.drive.grove_version,
        )?;
        let entries =
            page.entries
                .into_iter()
                .map(|(position, stored)| {
                    PrivateDocumentEntry::from_stored(position, &stored, private_document)
                        .ok_or_else(|| {
                            Error::Proof(ProofError::CorruptedProof(format!(
                                "the proved entry at position {position} of the store of \"{}\" \
                             takes {} bytes, not its type's header and entry size",
                                document_type.name(),
                                stored.len()
                            )))
                        })
                })
                .collect::<Result<Vec<_>, Error>>()?;
        Ok((
            root_hash,
            PrivateDocumentEntries {
                entries,
                total_count: page.total_count,
            },
        ))
    }
}
