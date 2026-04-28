use crate::consensus::basic::document::InvalidDocumentTypeError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::DataContract;
use crate::document::errors::DocumentError;
use crate::document::{Document, DocumentV0Getters, DocumentV0Setters, INITIAL_REVISION};
use chrono::Utc;
use std::collections::BTreeMap;

use crate::util::entropy_generator::{DefaultEntropyGenerator, EntropyGenerator};
use crate::version::PlatformVersion;
use crate::ProtocolError;

use platform_value::{Bytes32, Identifier, Value};

use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
#[cfg(feature = "extended-document")]
use crate::document::{
    extended_document::v0::ExtendedDocumentV0,
    ExtendedDocument, serialization_traits::DocumentPlatformConversionMethodsV0,
};
use crate::prelude::{BlockHeight, CoreBlockHeight, TimestampMillis};
#[cfg(feature = "state-transitions")]
use crate::state_transition::batch_transition::{
    batched_transition::{
        document_transition_action_type::DocumentTransitionActionType, DocumentCreateTransition,
        DocumentDeleteTransition, DocumentReplaceTransition,
    },
    BatchTransition, BatchTransitionV0,
};
use itertools::Itertools;
#[cfg(feature = "state-transitions")]
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_transition::DocumentTransition;
use crate::tokens::token_payment_info::TokenPaymentInfo;

/// Factory for creating documents
pub struct SpecializedDocumentFactoryV0 {
    protocol_version: u32,
    pub(super) data_contract: DataContract,
    entropy_generator: Box<dyn EntropyGenerator>,
}

impl SpecializedDocumentFactoryV0 {
    pub fn new(protocol_version: u32, data_contract: DataContract) -> Self {
        SpecializedDocumentFactoryV0 {
            protocol_version,
            data_contract,
            entropy_generator: Box::new(DefaultEntropyGenerator),
        }
    }

    pub fn new_with_entropy_generator(
        protocol_version: u32,
        data_contract: DataContract,
        entropy_generator: Box<dyn EntropyGenerator>,
    ) -> Self {
        SpecializedDocumentFactoryV0 {
            protocol_version,
            data_contract,
            entropy_generator,
        }
    }

    pub fn create_document(
        &self,
        data_contract: &DataContract,
        owner_id: Identifier,
        block_time: BlockHeight,
        core_block_height: CoreBlockHeight,
        document_type_name: String,
        data: Value,
    ) -> Result<Document, ProtocolError> {
        let platform_version = PlatformVersion::get(self.protocol_version)?;
        if !data_contract.has_document_type_for_name(&document_type_name) {
            return Err(DataContractError::InvalidDocumentTypeError(
                InvalidDocumentTypeError::new(document_type_name, data_contract.id()),
            )
            .into());
        }

        let document_entropy = self.entropy_generator.generate()?;

        let document_type = data_contract.document_type_for_name(document_type_name.as_str())?;

        document_type.create_document_from_data(
            data,
            owner_id,
            block_time,
            core_block_height,
            document_entropy,
            platform_version,
        )
    }
    pub fn create_document_without_time_based_properties(
        &self,
        owner_id: Identifier,
        document_type_name: String,
        data: Value,
    ) -> Result<Document, ProtocolError> {
        let platform_version = PlatformVersion::get(self.protocol_version)?;
        if !self
            .data_contract
            .has_document_type_for_name(&document_type_name)
        {
            return Err(DataContractError::InvalidDocumentTypeError(
                InvalidDocumentTypeError::new(document_type_name, self.data_contract.id()),
            )
            .into());
        }

        let document_entropy = self.entropy_generator.generate()?;

        let document_type = self
            .data_contract
            .document_type_for_name(document_type_name.as_str())?;

        document_type.create_document_from_data(
            data,
            owner_id,
            0,
            0,
            document_entropy,
            platform_version,
        )
    }
    #[cfg(feature = "extended-document")]
    pub fn create_extended_document(
        &self,
        owner_id: Identifier,
        document_type_name: String,
        data: Value,
    ) -> Result<ExtendedDocument, ProtocolError> {
        let platform_version = PlatformVersion::get(self.protocol_version)?;
        if !self
            .data_contract
            .has_document_type_for_name(&document_type_name)
        {
            return Err(DataContractError::InvalidDocumentTypeError(
                InvalidDocumentTypeError::new(document_type_name, self.data_contract.id()),
            )
            .into());
        }

        let document_entropy = self.entropy_generator.generate()?;

        let document_type = self
            .data_contract
            .document_type_for_name(document_type_name.as_str())?;

        let document = document_type.create_document_from_data(
            data,
            owner_id,
            0,
            0,
            document_entropy,
            platform_version,
        )?;

        let extended_document = match platform_version
            .dpp
            .document_versions
            .extended_document_structure_version
        {
            0 => Ok(ExtendedDocumentV0 {
                document_type_name,
                data_contract_id: self.data_contract.id(),
                document,
                data_contract: self.data_contract.clone(),
                metadata: None,
                entropy: Bytes32::new(document_entropy),
                token_payment_info: None,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentFactory::create_extended_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }?;

        Ok(extended_document)
    }
    #[cfg(feature = "state-transitions")]
    pub fn create_state_transition<'a>(
        &self,
        documents_iter: impl IntoIterator<
            Item = (
                DocumentTransitionActionType,
                Vec<(
                    Document,
                    DocumentTypeRef<'a>,
                    Bytes32,
                    Option<TokenPaymentInfo>,
                )>,
            ),
        >,
        nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>, //IdentityID/ContractID -> nonce
    ) -> Result<BatchTransition, ProtocolError> {
        let platform_version = PlatformVersion::get(self.protocol_version)?;
        // TODO: Use struct
        #[allow(clippy::type_complexity)]
        let documents: Vec<(
            DocumentTransitionActionType,
            Vec<(Document, DocumentTypeRef, Bytes32, Option<TokenPaymentInfo>)>,
        )> = documents_iter.into_iter().collect();
        let mut flattened_documents_iter = documents.iter().flat_map(|(_, v)| v).peekable();

        let Some((first_document, _, _, _)) = flattened_documents_iter.peek() else {
            return Err(DocumentError::NoDocumentsSuppliedError.into());
        };

        let owner_id = first_document.owner_id();

        let is_the_same_owner =
            flattened_documents_iter.all(|(document, _, _, _)| document.owner_id() == owner_id);
        if !is_the_same_owner {
            return Err(DocumentError::MismatchOwnerIdsError {
                documents: documents
                    .into_iter()
                    .flat_map(|(_, v)| {
                        v.into_iter()
                            .map(|(document, _, _, _)| document)
                            .collect::<Vec<_>>()
                    })
                    .collect(),
            }
            .into());
        }

        let transitions: Vec<_> = documents
            .into_iter()
            .map(|(action, documents)| match action {
                DocumentTransitionActionType::Create => {
                    Self::document_create_transitions(documents, nonce_counter, platform_version)
                }
                DocumentTransitionActionType::Delete => Self::document_delete_transitions(
                    documents
                        .into_iter()
                        .map(|(document, document_type, _, token_payment_info)| {
                            (document, document_type, token_payment_info)
                        })
                        .collect(),
                    nonce_counter,
                    platform_version,
                ),
                DocumentTransitionActionType::Replace => Self::document_replace_transitions(
                    documents
                        .into_iter()
                        .map(|(document, document_type, _, token_payment_info)| {
                            (document, document_type, token_payment_info)
                        })
                        .collect(),
                    nonce_counter,
                    platform_version,
                ),
                _ => Err(ProtocolError::InvalidStateTransitionType(
                    "action type not accounted for".to_string(),
                )),
            })
            .collect::<Result<Vec<_>, ProtocolError>>()?
            .into_iter()
            .flatten()
            .collect();

        if transitions.is_empty() {
            return Err(DocumentError::NoDocumentsSuppliedError.into());
        }

        Ok(BatchTransitionV0 {
            owner_id,
            transitions,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into())
    }

    #[cfg(feature = "extended-document")]
    pub fn create_extended_from_document_buffer(
        &self,
        buffer: &[u8],
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<ExtendedDocument, ProtocolError> {
        let document_type = self
            .data_contract
            .document_type_for_name(document_type_name)?;

        let document = Document::from_bytes(buffer, document_type, platform_version)?;

        match platform_version
            .dpp
            .document_versions
            .extended_document_structure_version
        {
            0 => Ok(ExtendedDocumentV0 {
                document_type_name: document_type_name.to_string(),
                data_contract_id: self.data_contract.id(),
                document,
                data_contract: self.data_contract.clone(),
                metadata: None,
                entropy: Bytes32::default(),
                token_payment_info: None,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentFactory::create_extended_from_document_buffer".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
    //
    // pub fn create_from_buffer(
    //     &self,
    //     buffer: impl AsRef<[u8]>,
    // ) -> Result<ExtendedDocument, ProtocolError> {
    //     let document = <ExtendedDocument as PlatformDeserializable>::deserialize(buffer.as_ref())
    //         .map_err(|e| {
    //             ConsensusError::BasicError(BasicError::SerializedObjectParsingError(
    //                 SerializedObjectParsingError::new(format!("Decode protocol entity: {:#?}", e)),
    //             ))
    //         })?;
    //     self.create_from_object(document.to_value()?).await
    // }
    //
    // pub fn create_from_object(
    //     &self,
    //     raw_document: Value,
    // ) -> Result<ExtendedDocument, ProtocolError> {
    //     ExtendedDocument::from_untrusted_platform_value(raw_document, data_contract)
    // }
    // //
    // // async fn validate_data_contract_for_extended_document(
    // //     &self,
    // //     raw_document: &Value,
    // //     options: FactoryOptions,
    // // ) -> Result<DataContract, ProtocolError> {
    // //     let result = self
    // //         .data_contract_fetcher_and_validator
    // //         .validate_extended(raw_document)
    // //         .await?;
    // //
    // //     if !result.is_valid() {
    // //         return Err(ProtocolError::Document(Box::new(
    // //             DocumentError::InvalidDocumentError {
    // //                 errors: result.errors,
    // //                 raw_document: raw_document.clone(),
    // //             },
    // //         )));
    // //     }
    // //     let data_contract = result
    // //         .into_data()
    // //         .context("Validator didn't return Data Contract. This shouldn't happen")?;
    // //
    // //     if !options.skip_validation {
    // //         let result = self
    // //             .document_validator
    // //             .validate_extended(raw_document, &data_contract)?;
    // //         if !result.is_valid() {
    // //             return Err(ProtocolError::Document(Box::new(
    // //                 DocumentError::InvalidDocumentError {
    // //                     errors: result.errors,
    // //                     raw_document: raw_document.clone(),
    // //                 },
    // //             )));
    // //         }
    // //     }
    // //
    // //     Ok(data_contract)
    // // }
    //
    #[cfg(feature = "state-transitions")]
    fn document_create_transitions(
        documents: Vec<(Document, DocumentTypeRef, Bytes32, Option<TokenPaymentInfo>)>,
        nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>, //IdentityID/ContractID -> nonce
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DocumentTransition>, ProtocolError> {
        documents
            .into_iter()
            .map(|(document, document_type, entropy, token_payment_info)| {
                if document_type.documents_mutable() {
                    //we need to have revisions
                    let Some(revision) = document.revision() else {
                        return Err(DocumentError::RevisionAbsentError {
                            document: Box::new(document),
                        }
                        .into());
                    };
                    if revision != INITIAL_REVISION {
                        return Err(DocumentError::InvalidInitialRevisionError {
                            document: Box::new(document),
                        }
                        .into());
                    }
                }
                let nonce = nonce_counter
                    .entry((document.owner_id(), document_type.data_contract_id()))
                    .or_default();

                let transition = DocumentCreateTransition::from_document(
                    document,
                    document_type,
                    entropy.to_buffer(),
                    token_payment_info,
                    *nonce,
                    platform_version,
                    None,
                    None,
                )?;

                *nonce += 1;

                Ok(transition.into())
            })
            .collect()
    }

    #[cfg(feature = "state-transitions")]
    fn document_replace_transitions(
        documents: Vec<(Document, DocumentTypeRef, Option<TokenPaymentInfo>)>,
        nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>, //IdentityID/ContractID -> nonce
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DocumentTransition>, ProtocolError> {
        documents
            .into_iter()
            .map(|(mut document, document_type, token_payment_info)| {
                if !document_type.documents_mutable() {
                    return Err(DocumentError::TryingToReplaceImmutableDocument {
                        document: Box::new(document),
                    }
                    .into());
                }
                if document.revision().is_none() {
                    return Err(DocumentError::RevisionAbsentError {
                        document: Box::new(document),
                    }
                    .into());
                };

                document.set_revision(document.revision().map(|revision| revision + 1));
                document.set_updated_at(Some(Utc::now().timestamp_millis() as TimestampMillis));

                let nonce = nonce_counter
                    .entry((document.owner_id(), document_type.data_contract_id()))
                    .or_default();

                let transition = DocumentReplaceTransition::from_document(
                    document,
                    document_type,
                    token_payment_info,
                    *nonce,
                    platform_version,
                    None,
                    None,
                )?;

                *nonce += 1;

                Ok(transition.into())
            })
            .collect()
        // let mut raw_transitions = vec![];
        // for (document, document_type) in documents {
        //     if !document_type.documents_mutable() {
        //         return Err(DocumentError::TryingToReplaceImmutableDocument {
        //             document: Box::new(document),
        //         }
        //         .into());
        //     }
        //     let Some(document_revision) = document.revision() else {
        //         return Err(DocumentError::RevisionAbsentError {
        //             document: Box::new(document),
        //         }.into());
        //     };
        //     let mut map = document.to_map_value()?;
        //
        //     map.retain(|key, _| {
        //         !key.starts_with('$') || DOCUMENT_REPLACE_KEYS_TO_STAY.contains(&key.as_str())
        //     });
        //     map.insert(
        //         PROPERTY_ACTION.to_string(),
        //         Value::U8(DocumentTransitionActionType::Replace as u8),
        //     );
        //     let new_revision = document_revision + 1;
        //     map.insert(PROPERTY_REVISION.to_string(), Value::U64(new_revision));
        //
        //     // If document have an originally set `updatedAt`
        //     // we should update it then
        //     let contains_updated_at = document_type
        //         .required_fields()
        //         .contains(PROPERTY_UPDATED_AT);
        //
        //     if contains_updated_at {
        //         let now = Utc::now().timestamp_millis() as TimestampMillis;
        //         map.insert(PROPERTY_UPDATED_AT.to_string(), Value::U64(now));
        //     }
        //
        //     raw_transitions.push(map.into());
        // }
        // Ok(raw_transitions)
    }

    #[cfg(feature = "state-transitions")]
    fn document_delete_transitions(
        documents: Vec<(Document, DocumentTypeRef, Option<TokenPaymentInfo>)>,
        nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>, //IdentityID/ContractID -> nonce
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DocumentTransition>, ProtocolError> {
        documents
            .into_iter()
            .map(|(document, document_type, token_payment_info)| {
                if !document_type.documents_can_be_deleted() {
                    return Err(DocumentError::TryingToDeleteIndelibleDocument {
                        document: Box::new(document),
                    }
                    .into());
                }
                let Some(_document_revision) = document.revision() else {
                    return Err(DocumentError::RevisionAbsentError {
                        document: Box::new(document),
                    }
                    .into());
                };
                let nonce = nonce_counter
                    .entry((document.owner_id(), document_type.data_contract_id()))
                    .or_default();
                let transition = DocumentDeleteTransition::from_document(
                    document,
                    document_type,
                    token_payment_info,
                    *nonce,
                    platform_version,
                    None,
                    None,
                )?;

                *nonce += 1;

                Ok(transition.into())
            })
            .collect()
    }

    fn is_ownership_the_same<'a>(ids: impl IntoIterator<Item = &'a Identifier>) -> bool {
        ids.into_iter().all_equal()
    }
}

#[cfg(test)]
#[allow(clippy::type_complexity)]
mod tests {
    use super::*;
    use crate::document::DocumentV0Getters;
    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::util::entropy_generator::EntropyGenerator;
    use platform_value::platform_value;

    /// A deterministic entropy generator for tests
    struct TestEntropyGenerator;

    impl EntropyGenerator for TestEntropyGenerator {
        fn generate(&self) -> anyhow::Result<[u8; 32]> {
            Ok([1u8; 32])
        }
    }

    fn setup_factory() -> (SpecializedDocumentFactoryV0, DataContract) {
        let platform_version = PlatformVersion::latest();
        let created = get_data_contract_fixture(None, 0, platform_version.protocol_version);
        let data_contract = created.data_contract_owned();
        let factory = SpecializedDocumentFactoryV0::new_with_entropy_generator(
            platform_version.protocol_version,
            data_contract.clone(),
            Box::new(TestEntropyGenerator),
        );
        (factory, data_contract)
    }

    #[test]
    fn new_creates_factory_with_default_entropy() {
        let platform_version = PlatformVersion::latest();
        let created = get_data_contract_fixture(None, 0, platform_version.protocol_version);
        let data_contract = created.data_contract_owned();
        let factory =
            SpecializedDocumentFactoryV0::new(platform_version.protocol_version, data_contract);
        // Just verify it was created without panic
        assert_eq!(factory.protocol_version, platform_version.protocol_version);
    }

    #[test]
    fn create_document_with_valid_type_succeeds() {
        let (factory, data_contract) = setup_factory();
        let owner_id = Identifier::from([10u8; 32]);

        let data = platform_value!({
            "firstName": "John",
            "lastName": "Doe",
        });

        let result = factory.create_document(
            &data_contract,
            owner_id,
            100,
            50,
            "indexedDocument".to_string(),
            data,
        );

        let doc = result.expect("should create document");
        assert_eq!(doc.owner_id(), owner_id);
    }

    #[test]
    fn create_document_with_invalid_type_fails() {
        let (factory, data_contract) = setup_factory();
        let owner_id = Identifier::from([10u8; 32]);

        let result = factory.create_document(
            &data_contract,
            owner_id,
            100,
            50,
            "nonExistentDocType".to_string(),
            Value::Null,
        );

        assert!(result.is_err());
    }

    #[test]
    fn create_document_without_time_based_properties_succeeds() {
        let (factory, _) = setup_factory();
        let owner_id = Identifier::from([20u8; 32]);

        let data = platform_value!({
            "name": "test",
        });

        let result = factory.create_document_without_time_based_properties(
            owner_id,
            "noTimeDocument".to_string(),
            data,
        );

        let doc = result.expect("should create document without time properties");
        assert_eq!(doc.owner_id(), owner_id);
    }

    #[test]
    fn create_document_without_time_based_properties_invalid_type_fails() {
        let (factory, _) = setup_factory();
        let owner_id = Identifier::from([20u8; 32]);

        let result = factory.create_document_without_time_based_properties(
            owner_id,
            "nonExistentDocType".to_string(),
            Value::Null,
        );

        assert!(result.is_err());
    }

    #[cfg(feature = "extended-document")]
    #[test]
    fn create_extended_document_succeeds() {
        let (factory, _) = setup_factory();
        let owner_id = Identifier::from([30u8; 32]);

        let data = platform_value!({
            "name": "test",
        });

        let result = factory.create_extended_document(owner_id, "noTimeDocument".to_string(), data);

        assert!(result.is_ok());
    }

    #[cfg(feature = "extended-document")]
    #[test]
    fn create_extended_document_invalid_type_fails() {
        let (factory, _) = setup_factory();
        let owner_id = Identifier::from([30u8; 32]);

        let result = factory.create_extended_document(
            owner_id,
            "nonExistentDocType".to_string(),
            Value::Null,
        );

        assert!(result.is_err());
    }

    #[test]
    fn is_ownership_the_same_with_same_ids() {
        let id = Identifier::from([1u8; 32]);
        assert!(SpecializedDocumentFactoryV0::is_ownership_the_same([
            &id, &id, &id
        ]));
    }

    #[test]
    fn is_ownership_the_same_with_different_ids() {
        let id1 = Identifier::from([1u8; 32]);
        let id2 = Identifier::from([2u8; 32]);
        assert!(!SpecializedDocumentFactoryV0::is_ownership_the_same([
            &id1, &id2
        ]));
    }

    #[test]
    fn is_ownership_the_same_with_single_id() {
        let id = Identifier::from([1u8; 32]);
        assert!(SpecializedDocumentFactoryV0::is_ownership_the_same([&id]));
    }

    #[test]
    fn is_ownership_the_same_with_empty_iter() {
        let ids: Vec<&Identifier> = vec![];
        assert!(SpecializedDocumentFactoryV0::is_ownership_the_same(ids));
    }

    // ----- Extended coverage -----

    #[test]
    fn new_with_invalid_protocol_version_still_constructs() {
        // `new` does not validate version; errors surface only during creation.
        let platform_version = PlatformVersion::latest();
        let created = get_data_contract_fixture(None, 0, platform_version.protocol_version);
        let factory = SpecializedDocumentFactoryV0::new(u32::MAX, created.data_contract_owned());
        assert_eq!(factory.protocol_version, u32::MAX);
    }

    #[test]
    fn create_document_bad_protocol_version_returns_error() {
        let platform_version = PlatformVersion::latest();
        let created = get_data_contract_fixture(None, 0, platform_version.protocol_version);
        let data_contract = created.data_contract_owned();
        let factory = SpecializedDocumentFactoryV0::new_with_entropy_generator(
            u32::MAX,
            data_contract.clone(),
            Box::new(TestEntropyGenerator),
        );

        let result = factory.create_document(
            &data_contract,
            Identifier::from([1u8; 32]),
            0,
            0,
            "noTimeDocument".to_string(),
            Value::Null,
        );
        assert!(result.is_err());
    }

    #[test]
    fn create_document_without_time_bad_protocol_version_returns_error() {
        let platform_version = PlatformVersion::latest();
        let created = get_data_contract_fixture(None, 0, platform_version.protocol_version);
        let factory = SpecializedDocumentFactoryV0::new_with_entropy_generator(
            u32::MAX,
            created.data_contract_owned(),
            Box::new(TestEntropyGenerator),
        );

        let result = factory.create_document_without_time_based_properties(
            Identifier::from([1u8; 32]),
            "noTimeDocument".to_string(),
            Value::Null,
        );
        assert!(result.is_err());
    }

    #[test]
    fn create_document_uses_entropy_generator_for_deterministic_id() {
        // Two documents for the same type with the same owner+entropy should get identical IDs.
        let (factory, _) = setup_factory();
        let owner_id = Identifier::from([42u8; 32]);

        let d1 = factory
            .create_document_without_time_based_properties(
                owner_id,
                "noTimeDocument".to_string(),
                platform_value!({ "name": "a" }),
            )
            .unwrap();
        let d2 = factory
            .create_document_without_time_based_properties(
                owner_id,
                "noTimeDocument".to_string(),
                platform_value!({ "name": "b" }),
            )
            .unwrap();

        // Identifier is derived from (contract_id, owner_id, type_name, entropy); all match → same.
        assert_eq!(d1.id(), d2.id());
    }

    #[test]
    fn create_document_uses_given_time_based_properties() {
        let (factory, data_contract) = setup_factory();
        let owner_id = Identifier::from([50u8; 32]);
        let block_time = 12345u64;
        let core_block_height = 67u32;

        // indexedDocument requires createdAt/updatedAt to be set. Provide explicit values.
        let data = platform_value!({
            "firstName": "Alice",
            "lastName": "Liddell",
        });

        let doc = factory
            .create_document(
                &data_contract,
                owner_id,
                block_time,
                core_block_height,
                "indexedDocument".to_string(),
                data,
            )
            .unwrap();

        assert_eq!(doc.owner_id(), owner_id);
        // the doc should at least have a non-empty id
        assert_ne!(doc.id().as_slice(), &[0u8; 32][..]);
    }

    // ----- State transition tests -----

    #[cfg(feature = "state-transitions")]
    mod state_transition_tests {
        use super::*;
        use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
        use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;
        use crate::state_transition::StateTransitionOwned;

        fn build_document(
            factory: &SpecializedDocumentFactoryV0,
            owner: Identifier,
            type_name: &str,
        ) -> Document {
            factory
                .create_document_without_time_based_properties(
                    owner,
                    type_name.to_string(),
                    platform_value!({ "name": "foo" }),
                )
                .expect("document should be created")
        }

        #[test]
        fn create_state_transition_create_action_populates_owner_and_nonce() {
            let (factory, data_contract) = setup_factory();
            let owner_id = Identifier::from([1u8; 32]);
            let doc = build_document(&factory, owner_id, "noTimeDocument");
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();

            let mut nonce_counter: BTreeMap<(Identifier, Identifier), u64> = BTreeMap::new();
            let entries = vec![(
                DocumentTransitionActionType::Create,
                vec![(doc, doc_type, Bytes32::new([2u8; 32]), None)],
            )];

            let batch = factory
                .create_state_transition(entries, &mut nonce_counter)
                .expect("batch transition should be created");

            assert_eq!(batch.owner_id(), owner_id);
            assert_eq!(batch.transitions_len(), 1);
            // nonce started at 0 and incremented to 1
            let key = (owner_id, data_contract.id());
            assert_eq!(*nonce_counter.get(&key).unwrap(), 1);
        }

        #[test]
        fn create_state_transition_no_documents_returns_error() {
            let (factory, _) = setup_factory();
            let mut nonce_counter: BTreeMap<(Identifier, Identifier), u64> = BTreeMap::new();

            // empty outer iter
            let empty: Vec<(
                DocumentTransitionActionType,
                Vec<(Document, DocumentTypeRef, Bytes32, Option<TokenPaymentInfo>)>,
            )> = vec![];
            let result = factory.create_state_transition(empty, &mut nonce_counter);
            assert!(
                matches!(
                    result,
                    Err(ProtocolError::Document(e)) if matches!(*e, DocumentError::NoDocumentsSuppliedError)
                ),
                "expected NoDocumentsSuppliedError"
            );
        }

        #[test]
        fn create_state_transition_mismatched_owners_returns_error() {
            let (factory, data_contract) = setup_factory();
            let owner_a = Identifier::from([1u8; 32]);
            let owner_b = Identifier::from([2u8; 32]);
            let doc_a = build_document(&factory, owner_a, "noTimeDocument");
            let doc_b = build_document(&factory, owner_b, "noTimeDocument");
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();

            let mut nonce_counter = BTreeMap::new();
            let entries = vec![(
                DocumentTransitionActionType::Create,
                vec![
                    (doc_a, doc_type, Bytes32::new([1u8; 32]), None),
                    (doc_b, doc_type, Bytes32::new([2u8; 32]), None),
                ],
            )];

            let result = factory.create_state_transition(entries, &mut nonce_counter);
            assert!(
                matches!(
                    result,
                    Err(ProtocolError::Document(e))
                        if matches!(*e, DocumentError::MismatchOwnerIdsError { .. })
                ),
                "expected MismatchOwnerIdsError"
            );
        }

        #[test]
        fn create_state_transition_replace_on_mutable_document_increments_revision() {
            let (factory, data_contract) = setup_factory();
            let owner_id = Identifier::from([3u8; 32]);
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();
            let doc = build_document(&factory, owner_id, "noTimeDocument");
            // noTimeDocument is mutable by default → revision is Some(1)
            assert_eq!(doc.revision(), Some(INITIAL_REVISION));

            let mut nonce_counter = BTreeMap::new();
            let entries = vec![(
                DocumentTransitionActionType::Replace,
                vec![(doc, doc_type, Bytes32::default(), None)],
            )];

            let batch = factory
                .create_state_transition(entries, &mut nonce_counter)
                .expect("replace transition should be built");
            assert_eq!(batch.transitions_len(), 1);
            assert_eq!(batch.owner_id(), owner_id);
            let key = (owner_id, data_contract.id());
            assert_eq!(*nonce_counter.get(&key).unwrap(), 1);
        }

        #[test]
        fn create_state_transition_replace_without_revision_returns_error() {
            let (factory, data_contract) = setup_factory();
            let owner_id = Identifier::from([4u8; 32]);
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();
            let mut doc = build_document(&factory, owner_id, "noTimeDocument");
            // Remove revision to trigger RevisionAbsentError.
            doc.set_revision(None);

            let mut nonce_counter = BTreeMap::new();
            let entries = vec![(
                DocumentTransitionActionType::Replace,
                vec![(doc, doc_type, Bytes32::default(), None)],
            )];
            let result = factory.create_state_transition(entries, &mut nonce_counter);
            assert!(
                matches!(
                    result,
                    Err(ProtocolError::Document(e))
                        if matches!(*e, DocumentError::RevisionAbsentError { .. })
                ),
                "expected RevisionAbsentError"
            );
        }

        #[test]
        fn create_state_transition_create_with_wrong_initial_revision_returns_error() {
            let (factory, data_contract) = setup_factory();
            let owner_id = Identifier::from([5u8; 32]);
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();
            let mut doc = build_document(&factory, owner_id, "noTimeDocument");
            // Invalid initial revision: must be INITIAL_REVISION (1).
            doc.set_revision(Some(42));

            let mut nonce_counter = BTreeMap::new();
            let entries = vec![(
                DocumentTransitionActionType::Create,
                vec![(doc, doc_type, Bytes32::default(), None)],
            )];
            let result = factory.create_state_transition(entries, &mut nonce_counter);
            assert!(
                matches!(
                    result,
                    Err(ProtocolError::Document(e))
                        if matches!(*e, DocumentError::InvalidInitialRevisionError { .. })
                ),
                "expected InvalidInitialRevisionError"
            );
        }

        #[test]
        fn create_state_transition_create_without_revision_on_mutable_returns_error() {
            let (factory, data_contract) = setup_factory();
            let owner_id = Identifier::from([6u8; 32]);
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();
            let mut doc = build_document(&factory, owner_id, "noTimeDocument");
            // For mutable documents, revision is required.
            doc.set_revision(None);

            let mut nonce_counter = BTreeMap::new();
            let entries = vec![(
                DocumentTransitionActionType::Create,
                vec![(doc, doc_type, Bytes32::default(), None)],
            )];
            let result = factory.create_state_transition(entries, &mut nonce_counter);
            assert!(
                matches!(
                    result,
                    Err(ProtocolError::Document(e))
                        if matches!(*e, DocumentError::RevisionAbsentError { .. })
                ),
                "expected RevisionAbsentError"
            );
        }

        #[test]
        fn create_state_transition_delete_with_mutable_doc() {
            let (factory, data_contract) = setup_factory();
            let owner_id = Identifier::from([7u8; 32]);
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();
            let doc = build_document(&factory, owner_id, "noTimeDocument");

            let mut nonce_counter = BTreeMap::new();
            let entries = vec![(
                DocumentTransitionActionType::Delete,
                vec![(doc, doc_type, Bytes32::default(), None)],
            )];
            let batch = factory
                .create_state_transition(entries, &mut nonce_counter)
                .expect("delete transition should be built");
            assert_eq!(batch.transitions_len(), 1);
            let key = (owner_id, data_contract.id());
            assert_eq!(*nonce_counter.get(&key).unwrap(), 1);
        }

        #[test]
        fn create_state_transition_delete_without_revision_returns_error() {
            let (factory, data_contract) = setup_factory();
            let owner_id = Identifier::from([8u8; 32]);
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();
            let mut doc = build_document(&factory, owner_id, "noTimeDocument");
            doc.set_revision(None);

            let mut nonce_counter = BTreeMap::new();
            let entries = vec![(
                DocumentTransitionActionType::Delete,
                vec![(doc, doc_type, Bytes32::default(), None)],
            )];
            let result = factory.create_state_transition(entries, &mut nonce_counter);
            assert!(
                matches!(
                    result,
                    Err(ProtocolError::Document(e))
                        if matches!(*e, DocumentError::RevisionAbsentError { .. })
                ),
                "expected RevisionAbsentError for delete without revision"
            );
        }

        #[test]
        fn create_state_transition_nonces_increment_per_document() {
            let (factory, data_contract) = setup_factory();
            let owner_id = Identifier::from([9u8; 32]);
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();
            let d1 = build_document(&factory, owner_id, "noTimeDocument");
            // produce a second doc with distinct id via different entropy/type path —
            // the entropy comes from the factory, but owner is same; we reuse same doc type
            // but set distinct revision-preserving id manually not needed, because nonce counter
            // is keyed by (owner, contract).
            let mut d2 = build_document(&factory, owner_id, "noTimeDocument");
            // ensure distinct id so the two can legitimately coexist
            d2.set_id(Identifier::from([0xEEu8; 32]));

            let mut nonce_counter = BTreeMap::new();
            // pre-seed a nonce so we can assert the post-value is base + 2
            nonce_counter.insert((owner_id, data_contract.id()), 10);

            let entries = vec![(
                DocumentTransitionActionType::Create,
                vec![
                    (d1, doc_type, Bytes32::new([1u8; 32]), None),
                    (d2, doc_type, Bytes32::new([2u8; 32]), None),
                ],
            )];
            let _ = factory
                .create_state_transition(entries, &mut nonce_counter)
                .expect("transition should build");

            assert_eq!(
                *nonce_counter.get(&(owner_id, data_contract.id())).unwrap(),
                12
            );
        }
    }

    #[cfg(feature = "extended-document")]
    mod extended_document_tests {
        use super::*;
        use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;

        #[test]
        fn create_extended_from_document_buffer_roundtrips() {
            let (factory, data_contract) = setup_factory();
            let owner_id = Identifier::from([77u8; 32]);

            let doc = factory
                .create_document_without_time_based_properties(
                    owner_id,
                    "noTimeDocument".to_string(),
                    platform_value!({ "name": "bob" }),
                )
                .expect("doc should be created");
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();

            let platform_version = PlatformVersion::latest();
            let bytes = doc
                .serialize(doc_type, &data_contract, platform_version)
                .expect("serialize");

            let extended = factory
                .create_extended_from_document_buffer(
                    bytes.as_slice(),
                    "noTimeDocument",
                    platform_version,
                )
                .expect("extended doc should deserialize");

            assert_eq!(extended.data_contract_id(), data_contract.id());
            assert_eq!(extended.document_type_name(), "noTimeDocument");
        }

        #[test]
        fn create_extended_from_document_buffer_invalid_type_fails() {
            let (factory, _) = setup_factory();
            let platform_version = PlatformVersion::latest();
            let result = factory.create_extended_from_document_buffer(
                &[0u8; 8],
                "doesNotExist",
                platform_version,
            );
            assert!(result.is_err());
        }

        #[test]
        fn create_extended_from_document_buffer_bad_bytes_fails() {
            let (factory, _) = setup_factory();
            let platform_version = PlatformVersion::latest();
            let result = factory.create_extended_from_document_buffer(
                &[0xFFu8; 4],
                "noTimeDocument",
                platform_version,
            );
            assert!(result.is_err());
        }
    }
}
