use crate::drive::document::paths::contract_documents_keeping_history_primary_key_path_for_document_id;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::document_type::DocumentPropertyType;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;
use std::collections::BTreeMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn verify_document_history_v0(
        proof: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        document_type: DocumentTypeRef,
        document_id: [u8; 32],
        start_at_ms: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<BTreeMap<u64, Document>>), Error> {
        let path_query = Self::fetch_document_history_query(
            contract_id,
            document_type_name,
            document_id,
            start_at_ms,
            limit,
            offset,
            platform_version,
        )?;
        let expected_path = contract_documents_keeping_history_primary_key_path_for_document_id(
            contract_id.as_slice(),
            document_type_name,
            document_id.as_slice(),
        )
        .into_iter()
        .map(|segment| segment.to_vec())
        .collect::<Vec<_>>();

        let (root_hash, mut proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        let mut documents: BTreeMap<u64, Document> = BTreeMap::new();
        for (path, key, maybe_element) in proved_key_values.drain(..) {
            if path != expected_path {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct path for the historical document"
                        .to_string(),
                )));
            }

            let date = DocumentPropertyType::decode_date_timestamp(&key).ok_or(Error::Drive(
                DriveError::CorruptedDocumentPath("document history key is not a valid timestamp"),
            ))?;

            // All historical revisions are decoded with the current `document_type`,
            // even though older revisions were written under an earlier contract
            // version. This is sound because data contract updates are constrained to
            // be backwards compatible: the schema-compatibility validator forbids
            // removing, reordering, or retyping existing properties and only permits
            // appending new (optional) properties at higher `$position` values
            // (see rs-json-schema-compatibility-validator and
            // DocumentType::validate_update). Documents serialize their properties in
            // `$position` order, and the deserializer skips trailing properties that
            // are absent from older bytes. So an old revision decodes correctly and
            // merely omits properties that were added after it was written — which is
            // accurate, since those properties did not exist in that revision.
            let maybe_document = maybe_element
                .map(|element| {
                    element
                        .into_item_bytes()
                        .map_err(Error::from)
                        .and_then(|bytes| {
                            Document::from_bytes(&bytes, document_type, platform_version)
                                .map_err(Error::from)
                        })
                })
                .transpose()?;

            if let Some(document) = maybe_document {
                documents.insert(date, document);
            } else {
                return Err(Error::Drive(DriveError::CorruptedDocumentPath(
                    "expected a document at this path",
                )));
            }
        }

        Ok((root_hash, Some(documents)))
    }
}
