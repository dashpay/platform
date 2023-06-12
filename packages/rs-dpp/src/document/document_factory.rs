use anyhow::Context;
use chrono::Utc;
use std::collections::{BTreeMap, HashSet};

use itertools::Itertools;

use platform_value::btreemap_extensions::BTreeValueMapReplacementPathHelper;
use platform_value::{Bytes32, ReplacementType, Value};

use serde::{Deserialize, Serialize};

use crate::consensus::basic::document::InvalidDocumentTypeError;
use crate::document::extended_document::ExtendedDocument;

use crate::consensus::basic::decode::SerializedObjectParsingError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::document::document_transition::INITIAL_REVISION;
use crate::document::{document_transition, Document, DocumentV0};
use crate::identity::TimestampMillis;
use crate::serialization_traits::PlatformDeserializable;
use crate::util::entropy_generator::{DefaultEntropyGenerator, EntropyGenerator};
use crate::version::LATEST_PLATFORM_VERSION;
use crate::{
    data_contract::{errors::DataContractError, DataContract},
    prelude::Identifier,
    state_repository::StateRepositoryLike,
    ProtocolError,
};

use super::{
    document_validator::DocumentValidator, errors::DocumentError,
    fetch_and_validate_data_contract::DataContractFetcherAndValidator,
    generate_document_id::generate_document_id, DocumentsBatchTransition,
};

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
pub struct DocumentFactory<ST> {
    protocol_version: u32,
    document_validator: DocumentValidator,
    data_contract_fetcher_and_validator: DataContractFetcherAndValidator<ST>,
    entropy_generator: Box<dyn EntropyGenerator>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FactoryOptions {
    #[serde(default)]
    pub skip_validation: bool,
    #[serde(default)]
    pub action: Action,
}

impl<ST> DocumentFactory<ST>
where
    ST: StateRepositoryLike,
{
    pub fn new(
        protocol_version: u32,
        validate_document: DocumentValidator,
        data_contract_fetcher_and_validator: DataContractFetcherAndValidator<ST>,
    ) -> Self {
        DocumentFactory {
            protocol_version,
            document_validator: validate_document,
            data_contract_fetcher_and_validator,
            entropy_generator: Box::new(DefaultEntropyGenerator),
        }
    }

    pub fn new_with_entropy_generator(
        protocol_version: u32,
        validate_document: DocumentValidator,
        data_contract_fetcher_and_validator: DataContractFetcherAndValidator<ST>,
        entropy_generator: Box<dyn EntropyGenerator>,
    ) -> Self {
        DocumentFactory {
            protocol_version,
            document_validator: validate_document,
            data_contract_fetcher_and_validator,
            entropy_generator,
        }
    }

    pub fn create_extended_document_for_state_transition(
        &self,
        data_contract: DataContract,
        owner_id: Identifier,
        document_type_name: String,
        data: Value,
    ) -> Result<ExtendedDocument, ProtocolError> {
        if !data_contract.is_document_defined(&document_type_name) {
            return Err(DataContractError::InvalidDocumentTypeError(
                InvalidDocumentTypeError::new(document_type_name, data_contract.id),
            )
            .into());
        }

        let document_entropy = self.entropy_generator.generate()?;

        let document_id = generate_document_id(
            &data_contract.id,
            &owner_id,
            &document_type_name,
            &document_entropy,
        );

        let document_type = data_contract.document_type_for_name(document_type_name.as_str())?;
        let revision = if document_type.documents_mutable {
            Some(INITIAL_REVISION)
        } else {
            None
        };

        let contains_created_at = document_type.required_fields.contains(PROPERTY_CREATED_AT);
        let contains_updated_at = document_type.required_fields.contains(PROPERTY_UPDATED_AT);

        let (created_at, updated_at) = if contains_created_at || contains_updated_at {
            //we want only one call to get current time
            let now = Utc::now().timestamp_millis() as TimestampMillis;
            let created_at = if contains_created_at { Some(now) } else { None };

            let updated_at = if contains_updated_at { Some(now) } else { None };
            (created_at, updated_at)
        } else {
            (None, None)
        };

        let mut document = DocumentV0 {
            id: document_id,
            owner_id,
            properties: data
                .into_btree_string_map()
                .map_err(ProtocolError::ValueError)?,
            revision,
            created_at,
            updated_at,
        };

        let (identifiers, _): (HashSet<_>, HashSet<_>) =
            data_contract.get_identifiers_and_binary_paths_owned(document_type_name.as_str())?;

        document
            .properties
            .replace_at_paths(identifiers, ReplacementType::Identifier)?;

        // let json_value = document.to_json_with_identifiers_using_bytes()?;
        // let validation_result =
        //     self.document_validator
        //         .validate(&json_value, &data_contract, document_type)?;

        let extended_document = ExtendedDocument {
            feature_version: LATEST_PLATFORM_VERSION
                .extended_document
                .default_current_version,
            document_type_name,
            data_contract_id: data_contract.id,
            document,
            data_contract,
            metadata: None,
            entropy: Bytes32::new(document_entropy),
        };

        // if !validation_result.is_valid() {
        //     return Err(ProtocolError::Document(Box::new(
        //         DocumentError::InvalidDocumentError {
        //             errors: validation_result.errors,
        //             raw_document: json_value,
        //         },
        //     )));
        // }

        Ok(extended_document)
    }

    pub fn create_state_transition(
        &self,
        documents_iter: impl IntoIterator<Item = (Action, Vec<ExtendedDocument>)>,
    ) -> Result<DocumentsBatchTransition, ProtocolError> {
        let mut raw_documents_transitions: Vec<Value> = vec![];
        let mut data_contracts: Vec<DataContract> = vec![];
        let documents: Vec<(Action, Vec<ExtendedDocument>)> = documents_iter.into_iter().collect();
        let flattened_documents_iter = documents.iter().flat_map(|(_, v)| v);

        if Self::is_empty(flattened_documents_iter.clone()) {
            return Err(DocumentError::NoDocumentsSuppliedError.into());
        }

        let is_the_same = Self::is_ownership_the_same(
            flattened_documents_iter
                .clone()
                .map(|extended_document| &extended_document.document.owner_id),
        );
        if !is_the_same {
            return Err(DocumentError::MismatchOwnerIdsError {
                documents: documents.into_iter().flat_map(|(_, v)| v).collect(),
            }
            .into());
        }

        let owner_id = flattened_documents_iter
            .clone()
            .next()
            .unwrap()
            .owner_id()
            .to_owned();
        for (action, documents) in documents {
            data_contracts.extend(documents.iter().map(|d| d.data_contract().clone()));

            let raw_transitions = match action {
                Action::Create => Self::raw_document_create_transitions(documents)?,
                Action::Delete => Self::raw_document_delete_transitions(documents)?,
                Action::Replace => Self::raw_document_replace_transitions(documents)?,
            };

            raw_documents_transitions.extend(raw_transitions);
        }

        if raw_documents_transitions.is_empty() {
            return Err(DocumentError::NoDocumentsSuppliedError.into());
        }

        let raw_batch_transition = BTreeMap::from([
            (
                PROPERTY_FEATURE_VERSION.to_string(),
                Value::U16(LATEST_PLATFORM_VERSION.document.default_current_version),
            ),
            (
                PROPERTY_OWNER_ID.to_string(),
                Value::Identifier(owner_id.to_buffer()),
            ),
            (
                PROPERTY_TRANSITIONS.to_string(),
                Value::Array(raw_documents_transitions),
            ),
        ]);

        DocumentsBatchTransition::from_value_map(raw_batch_transition, data_contracts)
    }

    pub fn create_extended_from_document_buffer(
        &self,
        buffer: &[u8],
        document_type: &str,
        data_contract: &DataContract,
    ) -> Result<ExtendedDocument, ProtocolError> {
        let document_type = data_contract.document_types.get(document_type).ok_or(
            ProtocolError::DataContractError(DataContractError::DocumentTypeNotFound(
                "document type was not found in the data contract",
            )),
        )?;

        let document = Document::from_bytes(buffer, document_type)?;

        Ok(ExtendedDocument {
            protocol_version: data_contract.protocol_version,
            document_type_name: document_type.name.clone(),
            data_contract_id: data_contract.id,
            document,
            data_contract: data_contract.clone(),
            metadata: None,
            entropy: Bytes32::default(),
        })
    }

    pub async fn create_from_buffer(
        &self,
        buffer: impl AsRef<[u8]>,
        options: FactoryOptions,
    ) -> Result<ExtendedDocument, ProtocolError> {
        let document = <ExtendedDocument as PlatformDeserializable>::deserialize(buffer.as_ref())
            .map_err(|e| {
            ConsensusError::BasicError(BasicError::SerializedObjectParsingError(
                SerializedObjectParsingError::new(format!("Decode protocol entity: {:#?}", e)),
            ))
        })?;
        self.create_from_object(document.to_value()?, options).await
    }

    pub async fn create_from_object(
        &self,
        raw_document: Value,
        options: FactoryOptions,
    ) -> Result<ExtendedDocument, ProtocolError> {
        let data_contract = self
            .validate_data_contract_for_extended_document(&raw_document, options)
            .await?;

        ExtendedDocument::from_untrusted_platform_value(raw_document, data_contract)
    }

    async fn validate_data_contract_for_extended_document(
        &self,
        raw_document: &Value,
        options: FactoryOptions,
    ) -> Result<DataContract, ProtocolError> {
        let result = self
            .data_contract_fetcher_and_validator
            .validate_extended(raw_document)
            .await?;

        if !result.is_valid() {
            return Err(ProtocolError::Document(Box::new(
                DocumentError::InvalidDocumentError {
                    errors: result.errors,
                    raw_document: raw_document.clone(),
                },
            )));
        }
        let data_contract = result
            .into_data()
            .context("Validator didn't return Data Contract. This shouldn't happen")?;

        if !options.skip_validation {
            let result = self
                .document_validator
                .validate_extended(raw_document, &data_contract)?;
            if !result.is_valid() {
                return Err(ProtocolError::Document(Box::new(
                    DocumentError::InvalidDocumentError {
                        errors: result.errors,
                        raw_document: raw_document.clone(),
                    },
                )));
            }
        }

        Ok(data_contract)
    }

    fn raw_document_create_transitions(
        documents: Vec<ExtendedDocument>,
    ) -> Result<Vec<Value>, ProtocolError> {
        let mut raw_transitions = vec![];
        for document in documents {
            if document.needs_revision()? {
                let Some(revision) = document.revision() else {
                    return Err(DocumentError::RevisionAbsentError {
                        document: Box::new(document),
                    }.into());
                };
                if revision != &INITIAL_REVISION {
                    return Err(DocumentError::InvalidInitialRevisionError {
                        document: Box::new(document),
                    }
                    .into());
                }
            }
            let mut map = document.to_map_value()?;

            map.retain(|key, _| {
                !key.starts_with('$') || DOCUMENT_CREATE_KEYS_TO_STAY.contains(&key.as_str())
            });
            map.insert(PROPERTY_ACTION.to_string(), Value::U8(Action::Create as u8));
            map.insert(
                PROPERTY_ENTROPY.to_string(),
                Value::Bytes(document.entropy.to_vec()),
            );
            raw_transitions.push(map.into());
        }

        Ok(raw_transitions)
    }

    fn raw_document_replace_transitions(
        documents: Vec<ExtendedDocument>,
    ) -> Result<Vec<Value>, ProtocolError> {
        let mut raw_transitions = vec![];
        for document in documents {
            if !document.can_be_modified()? {
                return Err(DocumentError::TryingToReplaceImmutableDocument {
                    document: Box::new(document),
                }
                .into());
            }
            let Some(document_revision) = document.revision() else {
                return Err(DocumentError::RevisionAbsentError {
                    document: Box::new(document),
                }.into());
            };
            let mut map = document.to_map_value()?;

            map.retain(|key, _| {
                !key.starts_with('$') || DOCUMENT_REPLACE_KEYS_TO_STAY.contains(&key.as_str())
            });
            map.insert(
                PROPERTY_ACTION.to_string(),
                Value::U8(Action::Replace as u8),
            );
            let new_revision = document_revision + 1;
            map.insert(PROPERTY_REVISION.to_string(), Value::U64(new_revision));

            // If document have an originally set `updatedAt`
            // we should update it then
            let contains_updated_at = document
                .document_type()?
                .required_fields
                .contains(PROPERTY_UPDATED_AT);

            if contains_updated_at {
                let now = Utc::now().timestamp_millis() as TimestampMillis;
                map.insert(PROPERTY_UPDATED_AT.to_string(), Value::U64(now));
            }

            raw_transitions.push(map.into());
        }
        Ok(raw_transitions)
    }

    fn raw_document_delete_transitions(
        documents: Vec<ExtendedDocument>,
    ) -> Result<Vec<Value>, ProtocolError> {
        Ok(documents
            .into_iter()
            .map(|document| {
                let mut map: BTreeMap<String, Value> = BTreeMap::new();
                map.insert(PROPERTY_ACTION.to_string(), Value::U8(Action::Delete as u8));
                map.insert(PROPERTY_ID.to_string(), document.document().id.into());
                map.insert(
                    PROPERTY_TYPE.to_string(),
                    Value::Text(document.document_type_name().clone()),
                );
                map.insert(
                    PROPERTY_DATA_CONTRACT_ID.to_string(),
                    document.data_contract_id().into(),
                );
                map.into()
            })
            .collect())
    }

    fn is_empty<T>(data: impl IntoIterator<Item = T>) -> bool {
        data.into_iter().next().is_none()
    }

    fn is_ownership_the_same<'a>(ids: impl IntoIterator<Item = &'a Identifier>) -> bool {
        ids.into_iter().all_equal()
    }
}

#[cfg(test)]
mod test {
    use platform_value::btreemap_extensions::BTreeValueMapHelper;
    use platform_value::platform_value;
    use platform_value::string_encoding::Encoding;
    use std::sync::Arc;

    use crate::tests::fixtures::get_extended_documents_fixture;
    use crate::{
        assert_error_contains,
        state_repository::MockStateRepositoryLike,
        tests::{
            fixtures::{get_data_contract_fixture, get_document_validator_fixture},
            utils::generate_random_identifier_struct,
        },
    };

    use super::*;

    #[test]
    fn document_with_type_and_data() {
        let mut data_contract = get_data_contract_fixture(None).data_contract;
        let document_type = "niceDocument";

        let factory = DocumentFactory::new(
            1,
            get_document_validator_fixture(),
            DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new())),
        );
        let name = "Cutie";
        let contract_id = Identifier::from_string(
            "FQco85WbwNgb5ix8QQAH6wurMcgEC5ENSCv5ixG9cj12",
            Encoding::Base58,
        )
        .unwrap();
        let owner_id = Identifier::from_string(
            "5zcXZpTLWFwZjKjq3ME5KVavtZa9YUaZESVzrndehBhq",
            Encoding::Base58,
        )
        .unwrap();

        data_contract.id = contract_id;

        let document = factory
            .create_extended_document_for_state_transition(
                data_contract,
                owner_id,
                document_type.to_string(),
                platform_value!({ "name": name }),
            )
            .expect("document creation shouldn't fail");
        assert_eq!(document_type, document.document_type_name);
        assert_eq!(
            name,
            document
                .properties()
                .get_str("name")
                .expect("property 'name' should exist")
        );
        assert_eq!(contract_id, document.data_contract_id);
        assert_eq!(owner_id, document.owner_id());
        assert_eq!(
            document_transition::INITIAL_REVISION,
            *document.revision().unwrap()
        );
        assert!(!document.id().to_string(Encoding::Base58).is_empty());
        assert!(document.created_at().is_some());
    }

    #[test]
    fn create_state_transition_no_documents() {
        let factory = DocumentFactory::new(
            1,
            get_document_validator_fixture(),
            DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new())),
        );

        let result = factory.create_state_transition(vec![]);
        assert_error_contains!(result, "No documents were supplied to state transition")
    }

    #[test]
    fn create_transition_mismatch_user_id() {
        let data_contract = get_data_contract_fixture(None).data_contract;
        let mut documents = get_extended_documents_fixture(data_contract).unwrap();

        let factory = DocumentFactory::new(
            1,
            get_document_validator_fixture(),
            DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new())),
        );

        documents[0].document.owner_id = generate_random_identifier_struct();

        let result = factory.create_state_transition(vec![(Action::Create, documents)]);
        assert_error_contains!(result, "Documents have mixed owner ids")
    }

    #[test]
    fn create_transition_invalid_initial_revision() {
        let data_contract = get_data_contract_fixture(None).data_contract;
        let mut documents = get_extended_documents_fixture(data_contract).unwrap();
        documents[0].document.revision = Some(3);

        let factory = DocumentFactory::new(
            1,
            get_document_validator_fixture(),
            DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new())),
        );
        let result = factory.create_state_transition(vec![(Action::Create, documents)]);
        assert_error_contains!(result, "Invalid Document initial revision '3'")
    }

    #[test]
    fn create_transitions_with_passed_documents() {
        let data_contract = get_data_contract_fixture(None).data_contract;
        let documents = get_extended_documents_fixture(data_contract).unwrap();
        let factory = DocumentFactory::new(
            1,
            get_document_validator_fixture(),
            DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new())),
        );

        let new_document = documents[0].clone();
        let batch_transition = factory
            .create_state_transition(vec![
                (Action::Create, documents),
                (Action::Replace, vec![new_document]),
            ])
            .expect("state transitions should be created");
        assert_eq!(11, batch_transition.transitions.len());
        assert_eq!(
            10,
            batch_transition
                .transitions
                .iter()
                .filter(|t| t.as_transition_create().is_some())
                .count()
        );
        assert_eq!(
            1,
            batch_transition
                .transitions
                .iter()
                .filter(|t| t.as_transition_replace().is_some())
                .count()
        )
    }
}
