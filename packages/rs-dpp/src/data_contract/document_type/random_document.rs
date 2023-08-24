use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::document::Document;
use crate::identity::Identity;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Bytes32, Identifier};
use rand::prelude::StdRng;
use std::time::{SystemTime, UNIX_EPOCH};

// TODO The factory is used in benchmark and tests. Probably it should be available under the test feature
/// Functions for creating various types of random documents.
pub trait CreateRandomDocument {
    /// Random documents
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
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError>;
    /// Random filled documents
    fn random_filled_documents(
        &self,
        count: u32,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, ProtocolError>;
    /// Random filled document
    fn random_filled_document(
        &self,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError>;
    /// Random filled document with rng
    fn random_filled_document_with_rng(
        &self,
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
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Document, Identity, Bytes32)>, ProtocolError> {
        match self {
            DocumentType::V0(v0) => {
                v0.random_documents_with_params(count, identities, time_ms, rng, platform_version)
            } // Add more cases as necessary for other variants
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
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentType::V0(v0) => {
                v0.random_document_with_params(owner_id, entropy, time_ms, rng, platform_version)
            } // Add more cases as necessary for other variants
        }
    }

    fn random_filled_documents(
        &self,
        count: u32,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, ProtocolError> {
        match self {
            DocumentType::V0(v0) => v0.random_filled_documents(count, seed, platform_version),
        }
    }

    fn random_filled_document(
        &self,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentType::V0(v0) => v0.random_filled_document(seed, platform_version),
        }
    }

    fn random_filled_document_with_rng(
        &self,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentType::V0(v0) => v0.random_filled_document_with_rng(rng, platform_version),
        }
    }

    fn random_document_with_identifier_and_entropy(
        &self,
        rng: &mut StdRng,
        owner_id: Identifier,
        entropy: Bytes32,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentType::V0(v0) => v0.random_document_with_identifier_and_entropy(
                rng,
                owner_id,
                entropy,
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
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Document, Identity, Bytes32)>, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => {
                v0.random_documents_with_params(count, identities, time_ms, rng, platform_version)
            } // Add more cases as necessary for other variants
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
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => {
                v0.random_document_with_params(owner_id, entropy, time_ms, rng, platform_version)
            } // Add more cases as necessary for other variants
        }
    }

    fn random_filled_documents(
        &self,
        count: u32,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.random_filled_documents(count, seed, platform_version),
        }
    }

    fn random_filled_document(
        &self,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.random_filled_document(seed, platform_version),
        }
    }

    fn random_filled_document_with_rng(
        &self,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.random_filled_document_with_rng(rng, platform_version),
        }
    }

    fn random_document_with_identifier_and_entropy(
        &self,
        rng: &mut StdRng,
        owner_id: Identifier,
        entropy: Bytes32,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.random_document_with_identifier_and_entropy(
                rng,
                owner_id,
                entropy,
                platform_version,
            ),
        }
    }
}
