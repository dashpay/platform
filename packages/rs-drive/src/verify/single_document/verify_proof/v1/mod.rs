use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::SingleDocumentDriveQuery;
use crate::verify::RootHash;
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::version::PlatformVersion;

impl SingleDocumentDriveQuery {
    pub(super) fn verify_proof_v1(
        &self,
        is_subset: bool,
        proof: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Document>), Error> {
        if !self.is_lifecycle_temporal_query(platform_version)? {
            return self.verify_proof_v0(is_subset, proof, document_type, platform_version);
        }
        let (root, retained) =
            self.verify_lifecycle_temporal_proof(is_subset, proof, platform_version)?;
        let document = retained
            .map(|retained| {
                let document =
                    Document::from_bytes(&retained.bytes, document_type, platform_version)?;
                if document.id().as_slice() != self.document_id
                    || document_type.requires_revision()
                        && document.revision() != Some(retained.revision)
                {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "the returned document does not match its retained revision key"
                            .to_string(),
                    )));
                }
                Ok(document)
            })
            .transpose()?;
        Ok((root, document))
    }
}
