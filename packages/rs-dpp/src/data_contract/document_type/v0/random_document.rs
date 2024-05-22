//! Random Documents.
//!
//! This module defines the CreateRandomDocument trait and its functions, which
//! create various types of random documents.
//!
//!

#[cfg(feature = "documents-faker")]
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "documents-faker")]
use platform_value::Value;
use platform_value::{Bytes32, Identifier};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::random_document::{
    CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
};
use crate::data_contract::document_type::v0::DocumentTypeV0;
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

impl CreateRandomDocument for DocumentTypeV0 {
    /// Create random documents using json-schema-faker-rs
    #[cfg(feature = "documents-faker")]
    fn random_documents_faker(
        &self,
        owner_id: Identifier,
        entropy: &Bytes32,
        count: u32,
        platform_version: &PlatformVersion,
        substitutions: &BTreeMap<&str, Value>,
    ) -> Result<Vec<Document>, ProtocolError> {
        use anyhow::Context;

        use crate::document::{
            extended_document_property_names::FEATURE_VERSION,
            property_names::{ID, OWNER_ID, REVISION},
            serialization_traits::DocumentPlatformValueMethodsV0,
        };

        let json_schema = &self.schema.clone().try_into()?;
        let json_documents = json_schema_faker::generate(json_schema, count as u16)
            .context("cannot generate a random document with json-schema-faker-rs")?;

        let fix_document = |mut document: platform_value::Value| {
            let id = Document::generate_document_id_v0(
                &self.data_contract_id,
                &owner_id,
                self.name.as_str(),
                entropy.as_slice(),
            );
            let now = SystemTime::now();
            let duration_since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
            let time_ms = duration_since_epoch.as_millis() as u64;

            if self.documents_mutable {
                document.as_map_mut().into_iter().for_each(|d| {
                    d.push((REVISION.into(), 1.into()));
                });
            }
            document.as_map_mut().into_iter().for_each(|d| {
                d.push((ID.into(), id.into()));
            });
            document.as_map_mut().into_iter().for_each(|d| {
                d.push((OWNER_ID.into(), owner_id.into()));
            });
            if self.required_fields.contains(FEATURE_VERSION) {
                document.as_map_mut().into_iter().for_each(|d| {
                    d.push((FEATURE_VERSION.into(), "0".into()));
                });
            }
            if self.required_fields.contains(CREATED_AT) {
                document.as_map_mut().into_iter().for_each(|d| {
                    d.push((CREATED_AT.into(), time_ms.into()));
                });
            }
            if self.required_fields.contains(UPDATED_AT) {
                document.as_map_mut().into_iter().for_each(|d| {
                    d.push((UPDATED_AT.into(), time_ms.into()));
                });
            }

            document
        };

        json_documents
            .into_iter()
            .map(|d| {
                let p_value: Value = d.into();
                let fixed_value = fix_document(p_value);

                // TODO: tl;dr use PlatformDeserialize instead of Deserialize for Documents
                //
                // `properties` is a `BTreeMap` with `platform_value::Value` as values, since
                // `Document::from_platform_value` does deserialization through Serde's data model
                // it losts some information like distinction between `Value::Bytes` and `Value::Bytes32`;
                // The solution here is to let deserialize a `Document`, but put `properties` unprocessed
                // since they were `platform_value::Value` and will be the same type again and no deserialization
                // is needed, especially that lossy kind.
                let mut properties = fixed_value
                    .to_map_ref()
                    .ok()
                    .and_then(|m| Value::map_into_btree_string_map(m.clone()).ok())
                    .unwrap_or_default();
                let mut document = Document::from_platform_value(fixed_value, platform_version);
                if let Ok(Document::V0(d)) = document.as_mut() {
                    // This moves stored properties back to the document so it could skip unnecessary
                    // and wrong deserialization part
                    d.properties.iter_mut().for_each(|(k, v)| {
                        substitutions
                            .get(k.as_str())
                            .cloned()
                            .or(properties.remove(k))
                            .into_iter()
                            .for_each(|prop| {
                                // TODO: schema and internal DocumentType representations are incompatible
                                // Properties are tweaked though, because the only integer type supported by
                                // DPP is i64, while `platform_value::Value` distincts them, and json schema is
                                // even more permissive; however, we want our proofs to work and proofs use the
                                // DPP model.
                                *v = match prop {
                                    Value::U64(x) => Value::I64(x as i64),
                                    Value::U32(x) => Value::I64(x as i64),
                                    Value::I32(x) => Value::I64(x as i64),
                                    x => x,
                                };
                            })
                    });
                }
                document
            })
            .collect()
    }

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

        let revision = if self.requires_revision() {
            Some(INITIAL_REVISION)
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
                transferred_at: None,
                created_at_block_height,
                updated_at_block_height,
                transferred_at_block_height: None,
                created_at_core_block_height,
                updated_at_core_block_height,
                transferred_at_core_block_height: None,
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
