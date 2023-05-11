use crate::drive::verify::RootHash;

use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::SingleDocumentDriveQuery;
use dpp::data_contract::document_type::DocumentType;
use dpp::document::Document;

use grovedb::GroveDb;

impl SingleDocumentDriveQuery {
    /// Verifies the document
    pub fn verify_proof_keep_serialized(
        &self,
        is_subset: bool,
        proof: &[u8],
    ) -> Result<(RootHash, Option<Vec<u8>>), Error> {
        let path_query = self.construct_path_query();
        let (root_hash, mut proved_key_values) = if is_subset {
            GroveDb::verify_subset_query(proof, &path_query)?
        } else {
            GroveDb::verify_query(proof, &path_query)?
        };

        // todo remove this (it shouldn't be needed)
        if proved_key_values.is_empty() {
            return Ok((root_hash, None));
        }

        if proved_key_values.len() != 1 {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "we should always get back one element",
            )));
        }

        let element = proved_key_values.remove(0).2;

        let serialized = element
            .map(|element| element.into_item_bytes().map_err(Error::GroveDB))
            .transpose()?;

        Ok((root_hash, serialized))
    }

    /// Verifies the proof of a single document query
    pub fn verify_proof(
        &self,
        is_subset: bool,
        proof: &[u8],
        document_type: &DocumentType,
    ) -> Result<(RootHash, Option<Document>), Error> {
        self.verify_proof_keep_serialized(is_subset, proof)
            .map(|(root_hash, serialized)| {
                let document = serialized
                    .map(|serialized| {
                        Document::from_bytes(serialized.as_slice(), document_type)
                            .map_err(Error::Protocol)
                    })
                    .transpose()?;
                Ok((root_hash, document))
            })?
    }
}
