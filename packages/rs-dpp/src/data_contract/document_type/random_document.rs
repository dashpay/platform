use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::document::Document;
use crate::identity::Identity;
use crate::prelude::{BlockHeight, CoreBlockHeight, TimestampMillis};
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
    /// Generates a single random document, employing default behavior for document field
    /// filling where fields that are not required will not be filled (`DoNotFillIfNotRequired`) and
    /// any fill size that is contractually allowed may be used (`AnyDocumentFillSize`).
    /// This method provides a straightforward way to create a document with random data, with an
    /// optional seed for deterministic randomness.
    ///
    /// # Parameters:
    /// - `seed`: An optional seed value for initializing the random number generator for deterministic outcomes.
    /// - `platform_version`: The version of the platform for which the document is being generated.
    ///
    /// # Returns:
    /// A `Result<Document, ProtocolError>`, which is `Ok` containing the document if successful, or an error
    /// if the operation fails.
    fn random_document(
        &self,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError>;

    /// Generates a single random document using a specified random number generator. Applies default behavior
    /// for filling document fields, not filling those that are not required and using any fill size allowed by
    /// the contract.
    ///
    /// # Parameters:
    /// - `rng`: A mutable reference to an `StdRng` random number generator.
    /// - `platform_version`: The version of the platform for which the document is being generated.
    ///
    /// # Returns:
    /// A `Result<Document, ProtocolError>`, which is `Ok` containing the document if successful, or an error
    /// if the operation fails.
    fn random_document_with_rng(
        &self,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError>;

    /// Generates a specified number of random documents, employing default behavior for document field
    /// filling where fields that are not required will not be filled (`DoNotFillIfNotRequired`) and
    /// any fill size that is contractually allowed may be used (`AnyDocumentFillSize`). This method
    /// is particularly useful for generating test documents or simulating documents in a variety of
    /// sizes and completeness.
    ///
    /// # Parameters:
    /// - `count`: The number of random documents to generate.
    /// - `seed`: An optional seed value for initializing the random number generator for deterministic outcomes.
    /// - `platform_version`: The version of the platform for which these documents are being generated.
    ///
    /// # Returns:
    /// A `Result<Vec<Document>, ProtocolError>`, which is `Ok` containing a vector of documents if successful,
    /// or an error if the operation fails.
    fn random_documents(
        &self,
        count: u32,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, ProtocolError>;

    /// Generates a specified number of random documents using a specific random number generator (`rng`).
    /// Default document field filling behavior is applied, where not required fields are not filled and any
    /// fill size allowed by the contract may be used. This allows for controlled randomness in the document
    /// generation process.
    ///
    /// # Parameters:
    /// - `count`: The number of random documents to generate.
    /// - `rng`: A mutable reference to an `StdRng` random number generator.
    /// - `platform_version`: The version of the platform for which these documents are being generated.
    ///
    /// # Returns:
    /// A `Result<Vec<Document>, ProtocolError>`, which is `Ok` containing a vector of documents if successful,
    /// or an error if the operation fails.
    fn random_documents_with_rng(
        &self,
        count: u32,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, ProtocolError>;

    /// Generates a single random document with a specified identifier and entropy values, using a given
    /// random number generator. Defaults to not filling non-required fields and allowing any fill size,
    /// unless specified otherwise through `document_field_fill_type` and `document_field_fill_size`.
    ///
    /// # Parameters:
    /// - `rng`: A mutable reference to an `StdRng` random number generator.
    /// - `owner_id`: The identifier for the owner of the document.
    /// - `entropy`: A Bytes32 value to influence the randomness.
    /// - `document_field_fill_type`: Specifies how document fields should be filled.
    /// - `document_field_fill_size`: Specifies the size of the content to fill document fields with.
    /// - `platform_version`: The version of the platform for which the document is being generated.
    ///
    /// # Returns:
    /// A `Result<Document, ProtocolError>`, which is `Ok` containing the document if successful, or an error
    /// if the operation fails.
    fn random_document_with_identifier_and_entropy(
        &self,
        rng: &mut StdRng,
        owner_id: Identifier,
        entropy: Bytes32,
        document_field_fill_type: DocumentFieldFillType,
        document_field_fill_size: DocumentFieldFillSize,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError>;

    /// Generates a single random document with specified parameters for customization, using a given
    /// random number generator. Defaults to not filling non-required fields and allowing any fill size
    /// unless explicitly specified through `document_field_fill_type` and `document_field_fill_size`.
    ///
    /// # Parameters:
    /// - `owner_id`: The identifier for the owner of the document.
    /// - `entropy`: A Bytes32 value to influence the randomness.
    /// - `time_ms`: An optional timestamp in milliseconds.
    /// - `block_height`: An optional block height to be used in created_at_block_height/updated_at_block_height. Will default to 0 if required but not provided.
    /// - `core_block_height`: An optional core block height to be used in created_at_core_block_height/updated_at_core_block_height. Will default to 0 if required but not provided.
    /// - `document_field_fill_type`: Specifies how document fields should be filled.
    /// - `document_field_fill_size`: Specifies the size of the content to fill document fields with.
    /// - `rng`: A mutable reference to an `StdRng` random number generator.
    /// - `platform_version`: The version of the platform for which the document is being generated.
    ///
    /// # Returns:
    /// A `Result<Document, ProtocolError>`, which is `Ok` containing the document if successful, or an error
    /// if the operation fails.
    fn random_document_with_params(
        &self,
        owner_id: Identifier,
        entropy: Bytes32,
        time_ms: Option<TimestampMillis>,
        block_height: Option<BlockHeight>,
        core_block_height: Option<CoreBlockHeight>,
        document_field_fill_type: DocumentFieldFillType,
        document_field_fill_size: DocumentFieldFillSize,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError>;

    /// Generates a specified number of random documents with additional parameters for customization, including
    /// identities, timestamps, block heights, and specific document field filling strategies.
    ///
    /// # Parameters:
    /// - `count`: The number of random documents to generate.
    /// - `identities`: An array of `Identity` objects to associate with the documents.
    /// - `time_ms`: An optional timestamp in milliseconds.
    /// - `block_height`: An optional block height to be used in created_at_block_height/updated_at_block_height. Will default to 0 if required but not provided.
    /// - `core_block_height`: An optional core block height to be used in created_at_core_block_height/updated_at_core_block_height. Will default to 0 if required but not provided.
    /// - `document_field_fill_type`: Specifies how document fields should be filled.
    /// - `document_field_fill_size`: Specifies the size of the content to fill document fields with.
    /// - `rng`: A mutable reference to an `StdRng` random number generator.
    /// - `platform_version`: The version of the platform for which these documents are being generated.
    ///
    /// # Returns:
    /// A `Result<Vec<(Document, Identity, Bytes32)>, ProtocolError>` which is `Ok` containing a vector of tuples
    /// if successful, each tuple consisting of a Document, its associated Identity, and a Bytes32 value, or an error
    /// if the operation fails.
    fn random_documents_with_params(
        &self,
        count: u32,
        identities: &[Identity],
        time_ms: Option<TimestampMillis>,
        block_height: Option<BlockHeight>,
        core_block_height: Option<CoreBlockHeight>,
        document_field_fill_type: DocumentFieldFillType,
        document_field_fill_size: DocumentFieldFillSize,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Document, Identity, Bytes32)>, ProtocolError>;
}

impl CreateRandomDocument for DocumentType {
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

    fn random_document_with_params(
        &self,
        owner_id: Identifier,
        entropy: Bytes32,
        time_ms: Option<TimestampMillis>,
        block_height: Option<BlockHeight>,
        core_block_height: Option<CoreBlockHeight>,
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
                block_height,
                core_block_height,
                document_field_fill_type,
                document_field_fill_size,
                rng,
                platform_version,
            ), // Add more cases as necessary for other variants
        }
    }
    fn random_documents_with_params(
        &self,
        count: u32,
        identities: &[Identity],
        time_ms: Option<TimestampMillis>,
        block_height: Option<BlockHeight>,
        core_block_height: Option<CoreBlockHeight>,
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
                block_height,
                core_block_height,
                document_field_fill_type,
                document_field_fill_size,
                rng,
                platform_version,
            ), // Add more cases as necessary for other variants
        }
    }
}

impl<'a> CreateRandomDocument for DocumentTypeRef<'a> {
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

    fn random_document_with_params(
        &self,
        owner_id: Identifier,
        entropy: Bytes32,
        time_ms: Option<TimestampMillis>,
        block_height: Option<BlockHeight>,
        core_block_height: Option<CoreBlockHeight>,
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
                block_height,
                core_block_height,
                document_field_fill_type,
                document_field_fill_size,
                rng,
                platform_version,
            ), // Add more cases as necessary for other variants
        }
    }

    fn random_documents_with_params(
        &self,
        count: u32,
        identities: &[Identity],
        time_ms: Option<TimestampMillis>,
        block_height: Option<BlockHeight>,
        core_block_height: Option<CoreBlockHeight>,
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
                block_height,
                core_block_height,
                document_field_fill_type,
                document_field_fill_size,
                rng,
                platform_version,
            ), // Add more cases as necessary for other variants
        }
    }
}
