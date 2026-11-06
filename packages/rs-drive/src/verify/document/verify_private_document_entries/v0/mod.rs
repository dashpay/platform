use crate::drive::document::paths::contract_document_type_path_vec;
use crate::drive::document::private_document_store::{
    private_document_entries_page_limit, private_document_of, private_document_stored_entry_size,
    PrivateDocumentEntries, PrivateDocumentEntry, PRIVATE_DOCUMENT_STORE_KEY,
};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::{Element, GroveDb, PathQuery};

impl Drive {
    pub(super) fn verify_private_document_entries_v0(
        proof: &[u8],
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        start_position: u64,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, PrivateDocumentEntries), Error> {
        let private_document = private_document_of(document_type)?;
        let stored_entry_size = private_document_stored_entry_size(private_document)?;
        let grove_version = &platform_version.drive.grove_version;
        let type_path = contract_document_type_path_vec(&contract_id, document_type.name());
        // The page is settled by the chunk power the store committed, so read the store element
        // first: the proof binds its bytes to the parent tree, and through it to the root hash
        // the page derives too.
        let (element_root_hash, elements) = GroveDb::verify_subset_query(
            proof,
            &PathQuery::new_single_key(type_path.clone(), vec![PRIVATE_DOCUMENT_STORE_KEY]),
            grove_version,
        )?;
        let chunk_power = match elements.as_slice() {
            [(_, key, Some(element))] if key.as_slice() == [PRIVATE_DOCUMENT_STORE_KEY] => {
                match element.underlying() {
                    Element::PrivateDocumentStore(_, entry_size, chunk_power, _)
                        if *entry_size == stored_entry_size =>
                    {
                        Some(*chunk_power)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
        .ok_or_else(|| {
            Error::Proof(ProofError::CorruptedProof(format!(
                "the proof does not bind the store of \"{}\", of entries taking {} bytes",
                document_type.name(),
                stored_entry_size
            )))
        })?;
        let limit = private_document_entries_page_limit(chunk_power, start_position)?;
        // Verifies the canonical range query the prover used, requires the page to be complete
        // up to the store's count, and reads that count from the store element the same proof
        // binds to the root hash.
        let (root_hash, page) = GroveDb::verify_bulk_position_range_proof(
            proof,
            type_path,
            &[PRIVATE_DOCUMENT_STORE_KEY],
            start_position,
            limit,
            grove_version,
        )?;
        if root_hash != element_root_hash {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "the store element and the page derive different root hashes".to_string(),
            )));
        }
        let entries =
            page.entries
                .into_iter()
                .map(|(position, stored)| {
                    let stored_size = stored.len();
                    PrivateDocumentEntry::from_stored(position, stored, private_document)
                        .ok_or_else(|| {
                            Error::Proof(ProofError::CorruptedProof(format!(
                                "the proved entry at position {position} of the store of \"{}\" \
                                 takes {stored_size} bytes, not its type's header and entry size",
                                document_type.name(),
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
