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
use crate::document::document_methods::DocumentMethodsV0;
#[cfg(feature = "extended-document")]
use crate::document::extended_document::v0::ExtendedDocumentV0;
use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
#[cfg(feature = "extended-document")]
use crate::document::ExtendedDocument;
use crate::prelude::{BlockHeight, CoreBlockHeight, TimestampMillis};
#[cfg(feature = "state-transitions")]
use crate::state_transition::documents_batch_transition::{
    document_transition::{
        action_type::DocumentTransitionActionType, DocumentCreateTransition,
        DocumentDeleteTransition, DocumentReplaceTransition, DocumentTransition,
    },
    DocumentsBatchTransition, DocumentsBatchTransitionV0,
};
use itertools::Itertools;

const PROPERTY_FEATURE_VERSION: &str = "$version";
const PROPERTY_ENTROPY: &str = "$entropy";
const PROPERTY_ACTION: &str = "$action";
const PROPERTY_OWNER_ID: &str = "ownerId";
const PROPERTY_DOCUMENT_OWNER_ID: &str = "$ownerId";
const PROPERTY_TYPE: &str = "$type";
const PROPERTY_ID: &str = "$id";
const PROPERTY_TRANSITIONS: &str = "transitions";
const PROPERTY_DATA_CONTRACT_ID: &str = "$dataContractId";
const PROPERTY_REVISION: &str = "$revision";
const PROPERTY_CREATED_AT: &str = "$createdAt";
const PROPERTY_UPDATED_AT: &str = "$updatedAt";
const PROPERTY_DOCUMENT_TYPE: &str = "$type";

const DOCUMENT_CREATE_KEYS_TO_STAY: [&str; 5] = [
    PROPERTY_ID,
    PROPERTY_TYPE,
    PROPERTY_DATA_CONTRACT_ID,
    PROPERTY_CREATED_AT,
    PROPERTY_UPDATED_AT,
];

const DOCUMENT_REPLACE_KEYS_TO_STAY: [&str; 5] = [
    PROPERTY_ID,
    PROPERTY_TYPE,
    PROPERTY_DATA_CONTRACT_ID,
    PROPERTY_REVISION,
    PROPERTY_UPDATED_AT,
];

/// Factory for creating documents
pub struct DocumentFactoryV0 {
    protocol_version: u32,
    entropy_generator: Box<dyn EntropyGenerator>,
}

impl DocumentFactoryV0 {
    pub fn new(protocol_version: u32) -> Self {
        DocumentFactoryV0 {
            protocol_version,
            entropy_generator: Box::new(DefaultEntropyGenerator),
        }
    }

    pub fn new_with_entropy_generator(
        protocol_version: u32,
        entropy_generator: Box<dyn EntropyGenerator>,
    ) -> Self {
        DocumentFactoryV0 {
            protocol_version,
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
        data_contract: &DataContract,
        owner_id: Identifier,
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
            0,
            0,
            document_entropy,
            platform_version,
        )
    }

    #[cfg(feature = "extended-document")]
    pub fn create_extended_document(
        &self,
        data_contract: &DataContract,
        owner_id: Identifier,
        document_type_name: String,
        data: Value,
    ) -> Result<ExtendedDocument, ProtocolError> {
        let platform_version = PlatformVersion::get(self.protocol_version)?;
        if !data_contract.has_document_type_for_name(&document_type_name) {
            return Err(DataContractError::InvalidDocumentTypeError(
                InvalidDocumentTypeError::new(document_type_name, data_contract.id()),
            )
            .into());
        }

        let document_entropy = self.entropy_generator.generate()?;

        let document_type = data_contract.document_type_for_name(document_type_name.as_str())?;

        // Extended documents are client side, so we don't need to fill in their timestamp properties
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
                data_contract_id: data_contract.id(),
                document,
                data_contract: data_contract.clone(),
                metadata: None,
                entropy: Bytes32::new(document_entropy),
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
                Vec<(Document, DocumentTypeRef<'a>, Bytes32)>,
            ),
        >,
        nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>, //IdentityID/ContractID -> nonce
    ) -> Result<DocumentsBatchTransition, ProtocolError> {
        let platform_version = PlatformVersion::get(self.protocol_version)?;
        let documents: Vec<(
            DocumentTransitionActionType,
            Vec<(Document, DocumentTypeRef, Bytes32)>,
        )> = documents_iter.into_iter().collect();
        let mut flattened_documents_iter = documents.iter().flat_map(|(_, v)| v).peekable();

        let Some((first_document, _, _)) = flattened_documents_iter.peek() else {
            return Err(DocumentError::NoDocumentsSuppliedError.into());
        };

        let owner_id = first_document.owner_id();

        let is_the_same_owner =
            flattened_documents_iter.all(|(document, _, _)| document.owner_id() == owner_id);
        if !is_the_same_owner {
            return Err(DocumentError::MismatchOwnerIdsError {
                documents: documents
                    .into_iter()
                    .flat_map(|(_, v)| {
                        v.into_iter()
                            .map(|(document, _, _)| document)
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
                        .map(|(document, document_type, _)| (document, document_type))
                        .collect(),
                    nonce_counter,
                    platform_version,
                ),
                DocumentTransitionActionType::Replace => Self::document_replace_transitions(
                    documents
                        .into_iter()
                        .map(|(document, document_type, _)| (document, document_type))
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

        Ok(DocumentsBatchTransitionV0 {
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
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<ExtendedDocument, ProtocolError> {
        let document_type = data_contract.document_type_for_name(document_type_name)?;

        let document = Document::from_bytes(buffer, document_type, platform_version)?;

        match platform_version
            .dpp
            .document_versions
            .extended_document_structure_version
        {
            0 => Ok(ExtendedDocumentV0 {
                document_type_name: document_type_name.to_string(),
                data_contract_id: data_contract.id(),
                document,
                data_contract: data_contract.clone(),
                metadata: None,
                entropy: Bytes32::default(),
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
        documents: Vec<(Document, DocumentTypeRef, Bytes32)>,
        nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>, //IdentityID/ContractID -> nonce
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DocumentTransition>, ProtocolError> {
        documents
            .into_iter()
            .map(|(document, document_type, entropy)| {
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
        documents: Vec<(Document, DocumentTypeRef)>,
        nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>, //IdentityID/ContractID -> nonce
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DocumentTransition>, ProtocolError> {
        documents
            .into_iter()
            .map(|(mut document, document_type)| {
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

                document.increment_revision()?;
                document.set_updated_at(Some(Utc::now().timestamp_millis() as TimestampMillis));

                let nonce = nonce_counter
                    .entry((document.owner_id(), document_type.data_contract_id()))
                    .or_default();

                let transition = DocumentReplaceTransition::from_document(
                    document,
                    document_type,
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
        documents: Vec<(Document, DocumentTypeRef)>,
        nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>, //IdentityID/ContractID -> nonce
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DocumentTransition>, ProtocolError> {
        documents
            .into_iter()
            .map(|(document, document_type)| {
                if !document_type.documents_mutable() {
                    return Err(DocumentError::TryingToDeleteImmutableDocument {
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
