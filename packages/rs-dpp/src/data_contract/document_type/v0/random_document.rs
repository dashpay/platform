//! Random Documents.
//!
//! This module defines the CreateRandomDocument trait and its functions, which
//! create various types of random documents.
//!

use crate::data_contract::document_type::property_names::{CREATED_AT, UPDATED_AT};
use crate::data_contract::document_type::random_document::{
    CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
};
use crate::data_contract::document_type::v0::DocumentTypeV0;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::document::property_names::{
    CREATED_AT_BLOCK_HEIGHT, CREATED_AT_CORE_BLOCK_HEIGHT, UPDATED_AT_BLOCK_HEIGHT,
    UPDATED_AT_CORE_BLOCK_HEIGHT,
};
use crate::document::{Document, DocumentV0};
use crate::identity::accessors::IdentityGettersV0;
use crate::identity::Identity;
use crate::prelude::{BlockHeight, CoreBlockHeight, TimestampMillis};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Bytes32, Identifier};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

impl CreateRandomDocument for DocumentTypeV0 {
    /// Creates a random Document using a seed if given, otherwise entropy.
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

    /// Creates a document with a random id, owner id, and properties using StdRng.
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

    /// Creates `count` Documents with random data using a seed if given, otherwise entropy.
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

    /// Creates `count` Documents with random data using the random number generator given.
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

    /// Creates a document with a random id, owner id, and properties using StdRng.
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

    /// Creates a document with a given owner id and entropy, and properties using StdRng.
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
            &self.data_contract_id,
            &owner_id,
            self.name.as_str(),
            entropy.as_slice(),
        );
        // dbg!("gen", hex::encode(id), hex::encode(&self.data_contract_id), hex::encode(&owner_id), self.name.as_str(), hex::encode(entropy.as_slice()));
        let properties = self
            .properties
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

        let revision = if self.documents_mutable {
            Some(1)
        } else {
            None
        };

        let created_at = if self.required_fields.contains(CREATED_AT) {
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

        let updated_at = if self.required_fields.contains(UPDATED_AT) {
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

        let created_at_block_height = if self.required_fields.contains(CREATED_AT_BLOCK_HEIGHT) {
            if block_height.is_some() {
                block_height
            } else {
                Some(0)
            }
        } else {
            None
        };

        let updated_at_block_height = if self.required_fields.contains(UPDATED_AT_BLOCK_HEIGHT) {
            if block_height.is_some() {
                block_height
            } else {
                Some(0)
            }
        } else {
            None
        };

        let created_at_core_block_height =
            if self.required_fields.contains(CREATED_AT_CORE_BLOCK_HEIGHT) {
                if core_block_height.is_some() {
                    core_block_height
                } else {
                    Some(0)
                }
            } else {
                None
            };

        let updated_at_core_block_height =
            if self.required_fields.contains(UPDATED_AT_CORE_BLOCK_HEIGHT) {
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
                created_at_block_height,
                updated_at_block_height,
                created_at_core_block_height,
                updated_at_core_block_height,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentTypeV0::random_document_with_params".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Creates `count` Documents with random data using the random number generator given.
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
        let mut vec = vec![];
        for _i in 0..count {
            let identity_num = rng.gen_range(0..identities.len());
            let identity = identities.get(identity_num).unwrap().clone();
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
