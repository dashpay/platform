pub mod v0;

use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::serialization::{PlatformDeserializable, PlatformSerializable};
use crate::voting::contender_structs::contender::v0::ContenderV0;
use crate::voting::contender_structs::ContenderWithSerializedDocumentV0;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

/// Represents a contender in the contested document vote poll.
///
/// This struct holds the identity ID of the contender, the serialized document,
/// and the vote tally.
#[derive(Debug, PartialEq, Clone, From)]
pub enum Contender {
    /// V0
    V0(ContenderV0),
}

/// Represents a contender in the contested document vote poll.
/// This is for internal use where the document is in serialized form
///
/// This struct holds the identity ID of the contender, the serialized document,
/// and the vote tally.
#[derive(
    Debug, PartialEq, Eq, Clone, From, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[platform_serialize(unversioned)]
pub enum ContenderWithSerializedDocument {
    /// V0
    V0(ContenderWithSerializedDocumentV0),
}

impl Contender {
    pub fn identity_id(&self) -> Identifier {
        match self {
            Contender::V0(v0) => v0.identity_id,
        }
    }

    pub fn identity_id_ref(&self) -> &Identifier {
        match self {
            Contender::V0(v0) => &v0.identity_id,
        }
    }

    pub fn document(&self) -> &Option<Document> {
        match self {
            Contender::V0(v0) => &v0.document,
        }
    }

    pub fn take_document(&mut self) -> Option<Document> {
        match self {
            Contender::V0(v0) => v0.document.take(),
        }
    }

    pub fn vote_tally(&self) -> Option<u32> {
        match self {
            Contender::V0(v0) => v0.vote_tally,
        }
    }
}

impl ContenderWithSerializedDocument {
    pub fn identity_id(&self) -> Identifier {
        match self {
            ContenderWithSerializedDocument::V0(v0) => v0.identity_id,
        }
    }

    pub fn identity_id_ref(&self) -> &Identifier {
        match self {
            ContenderWithSerializedDocument::V0(v0) => &v0.identity_id,
        }
    }

    pub fn serialized_document(&self) -> &Option<Vec<u8>> {
        match self {
            ContenderWithSerializedDocument::V0(v0) => &v0.serialized_document,
        }
    }

    pub fn take_serialized_document(&mut self) -> Option<Vec<u8>> {
        match self {
            ContenderWithSerializedDocument::V0(v0) => v0.serialized_document.take(),
        }
    }

    pub fn vote_tally(&self) -> Option<u32> {
        match self {
            ContenderWithSerializedDocument::V0(v0) => v0.vote_tally,
        }
    }
}

impl ContenderWithSerializedDocument {
    pub fn try_into_contender(
        self,
        document_type_ref: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Contender, ProtocolError> {
        match self {
            ContenderWithSerializedDocument::V0(v0) => Ok(v0
                .try_into_contender(document_type_ref, platform_version)?
                .into()),
        }
    }

    pub fn try_to_contender(
        &self,
        document_type_ref: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Contender, ProtocolError> {
        match self {
            ContenderWithSerializedDocument::V0(v0) => Ok(v0
                .try_to_contender(document_type_ref, platform_version)?
                .into()),
        }
    }
}

impl Contender {
    pub fn try_into_contender_with_serialized_document(
        self,
        document_type_ref: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<ContenderWithSerializedDocument, ProtocolError> {
        match self {
            Contender::V0(v0) => Ok(v0
                .try_into_contender_with_serialized_document(document_type_ref, platform_version)?
                .into()),
        }
    }

    pub fn try_to_contender_with_serialized_document(
        &self,
        document_type_ref: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<ContenderWithSerializedDocument, ProtocolError> {
        match self {
            Contender::V0(v0) => Ok(v0
                .try_to_contender_with_serialized_document(document_type_ref, platform_version)?
                .into()),
        }
    }

    pub fn serialize(
        &self,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        self.try_to_contender_with_serialized_document(document_type, platform_version)?
            .serialize_to_bytes()
    }

    pub fn serialize_consume(
        self,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        self.try_into_contender_with_serialized_document(document_type, platform_version)?
            .serialize_to_bytes()
    }

    pub fn from_bytes(
        serialized_contender: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let serialized_contender =
            ContenderWithSerializedDocument::deserialize_from_bytes(serialized_contender)?;
        serialized_contender.try_into_contender(document_type, platform_version)
    }
}
