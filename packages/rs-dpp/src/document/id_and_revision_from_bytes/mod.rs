mod v0;

use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::prelude::Revision;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

impl Document {
    /// Reads the id and the revision of a serialized document from the fields its
    /// serialization format writes ahead of the document's properties, without decoding the
    /// properties, so the result does not depend on how the document type's current schema
    /// encodes them.
    ///
    /// The migration of retained document histories on the first block of protocol version 15
    /// keys every retained revision by what this returns.
    ///
    /// # Parameters
    /// * `serialized_document`: The document's bytes, in any serialization format the method
    ///   version reads.
    /// * `document_type`: The document's type, whose configuration (not its properties) says
    ///   which header fields the bytes carry.
    /// * `platform_version`: Selects the method version.
    ///
    /// # Returns
    /// * `Ok((id, revision))`: the document's id, and its revision when its type carries one.
    /// * `Err(ProtocolError)` when the header cannot be read, its serialization format is
    ///   unknown, or the method version is unknown or not active.
    pub fn id_and_revision_from_bytes(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<(Identifier, Option<Revision>), ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .document_method_versions
            .id_and_revision_from_bytes
        {
            Some(0) => v0::id_and_revision_from_bytes_v0(serialized_document, document_type),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::id_and_revision_from_bytes".to_string(),
                known_versions: vec![0],
                received: version,
            }),
            None => Err(ProtocolError::UnknownVersionError(
                "Document::id_and_revision_from_bytes is not active before protocol version 15"
                    .to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use crate::document::{Document, DocumentV0Getters};
    use crate::tests::json_document::json_document_to_contract;
    use crate::ProtocolError;
    use platform_version::version::PlatformVersion;

    /// Protocol version 15 reads a serialized document's id and revision this way; protocol
    /// version 14, whose tables do not select the method, refuses to.
    #[test]
    fn should_read_the_header_from_protocol_version_15_only() {
        let latest = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/family/family-contract.json",
            false,
            latest,
        )
        .expect("the family contract");
        let document_type = contract
            .document_type_for_name("person")
            .expect("the person type");
        let document = document_type
            .random_document(Some(5), latest)
            .expect("a random person");
        let bytes = document
            .serialize(document_type, &contract, latest)
            .expect("serialized");

        assert_eq!(
            Document::id_and_revision_from_bytes(&bytes, document_type, latest)
                .expect("read at the latest protocol version"),
            (document.id(), document.revision())
        );
        let protocol_version_14 = PlatformVersion::get(14).expect("protocol version 14");
        assert!(matches!(
            Document::id_and_revision_from_bytes(&bytes, document_type, protocol_version_14),
            Err(ProtocolError::UnknownVersionError(_))
        ));
    }
}
