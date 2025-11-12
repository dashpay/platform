use bincode::{Decode, Encode};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::document::property_names::{
    CREATED_AT, CREATED_AT_BLOCK_HEIGHT, CREATED_AT_CORE_BLOCK_HEIGHT, UPDATED_AT,
    UPDATED_AT_BLOCK_HEIGHT, UPDATED_AT_CORE_BLOCK_HEIGHT,
};
use crate::document::{Document, DocumentV0, INITIAL_REVISION};
use crate::identity::accessors::IdentityGettersV0;
use crate::identity::Identity;
use crate::prelude::{BlockHeight, CoreBlockHeight, TimestampMillis};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Bytes32, Identifier};
use rand::prelude::StdRng;
use rand::SeedableRng;

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
pub trait CreateRandomDocument: DocumentTypeV0Getters + DocumentTypeV0Methods {
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
    ) -> Result<Document, ProtocolError> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        self.random_document_with_rng(&mut rng, platform_version)
    }

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
    ) -> Result<Document, ProtocolError> {
        let owner_id = Identifier::random_with_rng(rng);
        let entropy = Bytes32::random_with_rng(rng);

        self.random_document_with_params(
            owner_id,
            entropy,
            None,
            None,
            None,
            DocumentFieldFillType::FillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            rng,
            platform_version,
        )
    }
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
    ) -> Result<Vec<Document>, ProtocolError> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        self.random_documents_with_rng(count, &mut rng, platform_version)
    }

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
    ) -> Result<Vec<Document>, ProtocolError> {
        let mut vec: Vec<Document> = vec![];
        for _i in 0..count {
            vec.push(self.random_document_with_rng(rng, platform_version)?);
        }
        Ok(vec)
    }

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
    ) -> Result<Document, ProtocolError> {
        self.random_document_with_params(
            owner_id,
            entropy,
            None,
            None,
            None,
            document_field_fill_type,
            document_field_fill_size,
            rng,
            platform_version,
        )
    }
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
    #[allow(clippy::too_many_arguments)]
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
        let id = Document::generate_document_id_v0(
            &self.data_contract_id(),
            &owner_id,
            self.name().as_str(),
            entropy.as_slice(),
        );
        // dbg!("gen", hex::encode(id), hex::encode(&self.data_contract_id), hex::encode(&owner_id), self.name.as_str(), hex::encode(entropy.as_slice()));
        let properties = self
            .properties()
            .iter()
            .filter_map(|(key, property)| {
                if property.required
                    || document_field_fill_type == DocumentFieldFillType::FillIfNotRequired
                {
                    let value = match document_field_fill_size {
                        DocumentFieldFillSize::MinDocumentFillSize => {
                            property.property_type.random_sub_filled_value(rng)
                        }
                        DocumentFieldFillSize::MaxDocumentFillSize => {
                            property.property_type.random_filled_value(rng)
                        }
                        DocumentFieldFillSize::AnyDocumentFillSize => {
                            property.property_type.random_value(rng)
                        }
                    };
                    Some((key.clone(), value))
                } else {
                    None
                }
            })
            .collect();

        let revision = if self.requires_revision() {
            Some(INITIAL_REVISION)
        } else {
            None
        };

        let created_at = if self.required_fields().contains(CREATED_AT) {
            if time_ms.is_some() {
                time_ms
            } else {
                let now = SystemTime::now();
                let duration_since_epoch =
                    now.duration_since(UNIX_EPOCH).expect("Time went backwards");
                let milliseconds = duration_since_epoch.as_millis() as u64;
                Some(milliseconds)
            }
        } else {
            None
        };

        let updated_at = if self.required_fields().contains(UPDATED_AT) {
            if time_ms.is_some() {
                time_ms
            } else if created_at.is_some() {
                created_at
            } else {
                let now = SystemTime::now();
                let duration_since_epoch =
                    now.duration_since(UNIX_EPOCH).expect("Time went backwards");
                let milliseconds = duration_since_epoch.as_millis() as u64;
                Some(milliseconds)
            }
        } else {
            None
        };

        let created_at_block_height = if self.required_fields().contains(CREATED_AT_BLOCK_HEIGHT) {
            if block_height.is_some() {
                block_height
            } else {
                Some(0)
            }
        } else {
            None
        };

        let updated_at_block_height = if self.required_fields().contains(UPDATED_AT_BLOCK_HEIGHT) {
            if block_height.is_some() {
                block_height
            } else {
                Some(0)
            }
        } else {
            None
        };

        let created_at_core_block_height = if self
            .required_fields()
            .contains(CREATED_AT_CORE_BLOCK_HEIGHT)
        {
            if core_block_height.is_some() {
                core_block_height
            } else {
                Some(0)
            }
        } else {
            None
        };

        let updated_at_core_block_height = if self
            .required_fields()
            .contains(UPDATED_AT_CORE_BLOCK_HEIGHT)
        {
            if core_block_height.is_some() {
                core_block_height
            } else {
                Some(0)
            }
        } else {
            None
        };

        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(DocumentV0 {
                id,
                properties,
                owner_id,
                revision,
                created_at,
                updated_at,
                transferred_at: None,
                created_at_block_height,
                updated_at_block_height,
                transferred_at_block_height: None,
                created_at_core_block_height,
                updated_at_core_block_height,
                transferred_at_core_block_height: None,
                creator_id: None,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentTypeV0::random_document_with_params".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

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
    #[allow(clippy::too_many_arguments)]
    fn random_documents_with_params<'i>(
        &self,
        count: u32,
        identities: &[&'i Identity],
        time_ms: Option<TimestampMillis>,
        block_height: Option<BlockHeight>,
        core_block_height: Option<CoreBlockHeight>,
        document_field_fill_type: DocumentFieldFillType,
        document_field_fill_size: DocumentFieldFillSize,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Document, &'i Identity, Bytes32)>, ProtocolError> {
        let mut vec = vec![];

        if identities.len() < count as usize {
            return Err(ProtocolError::CorruptedCodeExecution(format!(
                "not enough identities to create {count} documents"
            )));
        }

        for i in 0..count {
            let identity = identities[i as usize];
            let entropy = Bytes32::random_with_rng(rng);
            vec.push((
                self.random_document_with_params(
                    identity.id(),
                    entropy,
                    time_ms,
                    block_height,
                    core_block_height,
                    document_field_fill_type,
                    document_field_fill_size,
                    rng,
                    platform_version,
                )?,
                identity,
                entropy,
            ));
        }
        Ok(vec)
    }
}

impl CreateRandomDocument for DocumentType {}

impl CreateRandomDocument for DocumentTypeRef<'_> {}
