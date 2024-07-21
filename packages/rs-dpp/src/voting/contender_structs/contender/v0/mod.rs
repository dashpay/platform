use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use crate::document::Document;
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

/// Represents a contender in the contested document vote poll.
///
/// This struct holds the identity ID of the contender, the serialized document,
/// and the vote tally.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct ContenderV0 {
    /// The identity ID of the contender.
    pub identity_id: Identifier,
    /// The document associated with the contender.
    pub document: Option<Document>,
    /// The vote tally for the contender.
    pub vote_tally: Option<u32>,
}

/// Represents a contender in the contested document vote poll.
/// This is for internal use where the document is in serialized form
///
/// This struct holds the identity ID of the contender, the serialized document,
/// and the vote tally.
#[derive(Debug, PartialEq, Eq, Clone, Default, Encode, Decode)]
pub struct ContenderWithSerializedDocumentV0 {
    /// The identity ID of the contender.
    pub identity_id: Identifier,
    /// The serialized document associated with the contender.
    pub serialized_document: Option<Vec<u8>>,
    /// The vote tally for the contender.
    pub vote_tally: Option<u32>,
}

impl ContenderV0 {
    pub fn try_into_contender_with_serialized_document(
        self,
        document_type_ref: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<ContenderWithSerializedDocumentV0, ProtocolError> {
        let ContenderV0 {
            identity_id,
            document,
            vote_tally,
        } = self;

        Ok(ContenderWithSerializedDocumentV0 {
            identity_id,
            serialized_document: document
                .map(|document| document.serialize(document_type_ref, platform_version))
                .transpose()?,
            vote_tally,
        })
    }

    pub fn try_to_contender_with_serialized_document(
        &self,
        document_type_ref: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<ContenderWithSerializedDocumentV0, ProtocolError> {
        let ContenderV0 {
            identity_id,
            document,
            vote_tally,
        } = self;

        Ok(ContenderWithSerializedDocumentV0 {
            identity_id: *identity_id,
            serialized_document: document
                .as_ref()
                .map(|document| document.serialize(document_type_ref, platform_version))
                .transpose()?,
            vote_tally: *vote_tally,
        })
    }
}

impl ContenderWithSerializedDocumentV0 {
    pub fn try_into_contender(
        self,
        document_type_ref: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<ContenderV0, ProtocolError> {
        let ContenderWithSerializedDocumentV0 {
            identity_id,
            serialized_document,
            vote_tally,
        } = self;

        Ok(ContenderV0 {
            identity_id,
            document: serialized_document
                .map(|document| {
                    Document::from_bytes(document.as_slice(), document_type_ref, platform_version)
                })
                .transpose()?,
            vote_tally,
        })
    }

    pub fn try_to_contender(
        &self,
        document_type_ref: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<ContenderV0, ProtocolError> {
        let ContenderWithSerializedDocumentV0 {
            identity_id,
            serialized_document,
            vote_tally,
        } = self;

        Ok(ContenderV0 {
            identity_id: *identity_id,
            document: serialized_document
                .as_ref()
                .map(|document| {
                    Document::from_bytes(document.as_slice(), document_type_ref, platform_version)
                })
                .transpose()?,
            vote_tally: *vote_tally,
        })
    }
}
