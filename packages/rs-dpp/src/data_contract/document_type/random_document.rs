use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::document::Document;
use crate::identity::Identity;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_value::{Bytes32, Identifier};
use rand::prelude::StdRng;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Encode, Decode)]
pub enum DocumentFieldFillType {
    /// Do not fill a field if that field is not required
    DoNotFillIfNotRequired,
    /// Should fill a field even if that field is not required
    FillIfNotRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Encode, Decode)]
pub enum DocumentFieldFillSize {
    /// Fill to the min size allowed by the contract
    MinDocumentFillSize,
    /// Fill to the max size allowed by the contract
    MaxDocumentFillSize,
    /// Fill any size that is allowed by the contract
    AnyDocumentFillSize,
}

// TODO The factory is used in benchmark and tests. Probably it should be available under the test feature
/// Functions for creating various types of random documents.
pub trait CreateRandomDocument {
    /// Random documents with DoNotFillIfNotRequired and AnyDocumentFillSize
    fn random_documents(
        &self,
        count: u32,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, ProtocolError>;
    /// Random documents with rng
    fn random_documents_with_rng(
        &self,
        count: u32,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, ProtocolError>;
    /// Creates `count` Documents with random data using the random number generator given.
    fn random_documents_with_params(
        &self,
        count: u32,
        identities: &Vec<Identity>,
        time_ms: u64,
        document_field_fill_type: DocumentFieldFillType,
        document_field_fill_size: DocumentFieldFillSize,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Document, Identity, Bytes32)>, ProtocolError>;
    /// Random document
    fn random_document(
        &self,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError>;
    /// Creates a document with a random id, owner id, and properties using StdRng.
    fn random_document_with_identifier_and_entropy(
        &self,
        rng: &mut StdRng,
        owner_id: Identifier,
        entropy: Bytes32,
        document_field_fill_type: DocumentFieldFillType,
        document_field_fill_size: DocumentFieldFillSize,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError>;
    /// Random document with rng
    fn random_document_with_rng(
        &self,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError>;
    /// Creates a document with a random id, owner id, and properties using StdRng.
    fn random_document_with_params(
        &self,
        owner_id: Identifier,
        entropy: Bytes32,
        time_ms: u64,
        document_field_fill_type: DocumentFieldFillType,
        document_field_fill_size: DocumentFieldFillSize,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError>;
}

impl CreateRandomDocument for DocumentType {
    fn random_documents(
        &self,
        count: u32,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, ProtocolError> {
        match self {
            DocumentType::V0(v0) => v0.random_documents(count, seed, platform_version),
        }
    }

    fn random_documents_with_rng(
        &self,
        count: u32,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, ProtocolError> {
        match self {
            DocumentType::V0(v0) => v0.random_documents_with_rng(count, rng, platform_version),
        }
    }

    fn random_documents_with_params(
        &self,
        count: u32,
        identities: &Vec<Identity>,
        time_ms: u64,
        document_field_fill_type: DocumentFieldFillType,
        document_field_fill_size: DocumentFieldFillSize,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Document, Identity, Bytes32)>, ProtocolError> {
        match self {
            DocumentType::V0(v0) => v0.random_documents_with_params(
                count,
                identities,
                time_ms,
                document_field_fill_type,
                document_field_fill_size,
                rng,
                platform_version,
            ), // Add more cases as necessary for other variants
        }
    }

    fn random_document(
        &self,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentType::V0(v0) => v0.random_document(seed, platform_version),
        }
    }

    fn random_document_with_rng(
        &self,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentType::V0(v0) => v0.random_document_with_rng(rng, platform_version),
        }
    }

    fn random_document_with_params(
        &self,
        owner_id: Identifier,
        entropy: Bytes32,
        time_ms: u64,
        document_field_fill_type: DocumentFieldFillType,
        document_field_fill_size: DocumentFieldFillSize,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentType::V0(v0) => v0.random_document_with_params(
                owner_id,
                entropy,
                time_ms,
                document_field_fill_type,
                document_field_fill_size,
                rng,
                platform_version,
            ), // Add more cases as necessary for other variants
        }
    }
    fn random_document_with_identifier_and_entropy(
        &self,
        rng: &mut StdRng,
        owner_id: Identifier,
        entropy: Bytes32,
        document_field_fill_type: DocumentFieldFillType,
        document_field_fill_size: DocumentFieldFillSize,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentType::V0(v0) => v0.random_document_with_identifier_and_entropy(
                rng,
                owner_id,
                entropy,
                document_field_fill_type,
                document_field_fill_size,
                platform_version,
            ),
        }
    }
}

impl<'a> CreateRandomDocument for DocumentTypeRef<'a> {
    fn random_documents(
        &self,
        count: u32,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.random_documents(count, seed, platform_version),
        }
    }

    fn random_documents_with_rng(
        &self,
        count: u32,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.random_documents_with_rng(count, rng, platform_version),
        }
    }

    fn random_documents_with_params(
        &self,
        count: u32,
        identities: &Vec<Identity>,
        time_ms: u64,
        document_field_fill_type: DocumentFieldFillType,
        document_field_fill_size: DocumentFieldFillSize,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Document, Identity, Bytes32)>, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.random_documents_with_params(
                count,
                identities,
                time_ms,
                document_field_fill_type,
                document_field_fill_size,
                rng,
                platform_version,
            ), // Add more cases as necessary for other variants
        }
    }

    fn random_document(
        &self,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.random_document(seed, platform_version),
        }
    }

    fn random_document_with_rng(
        &self,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.random_document_with_rng(rng, platform_version),
        }
    }

    fn random_document_with_params(
        &self,
        owner_id: Identifier,
        entropy: Bytes32,
        time_ms: u64,
        document_field_fill_type: DocumentFieldFillType,
        document_field_fill_size: DocumentFieldFillSize,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.random_document_with_params(
                owner_id,
                entropy,
                time_ms,
                document_field_fill_type,
                document_field_fill_size,
                rng,
                platform_version,
            ), // Add more cases as necessary for other variants
        }
    }

    fn random_document_with_identifier_and_entropy(
        &self,
        rng: &mut StdRng,
        owner_id: Identifier,
        entropy: Bytes32,
        document_field_fill_type: DocumentFieldFillType,
        document_field_fill_size: DocumentFieldFillSize,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.random_document_with_identifier_and_entropy(
                rng,
                owner_id,
                entropy,
                document_field_fill_type,
                document_field_fill_size,
                platform_version,
            ),
        }
    }
}
