use crate::{
    data_contract::DataContract,
    mocks,
    prelude::Identifier,
    util::entropy_generator,
    util::{json_schema::JsonSchemaExt, json_value::JsonValueExt},
    ProtocolError,
};
use chrono::Utc;
use itertools::Itertools;
use serde_json::{json, Value as JsonValue};

use super::{
    document_transition::{self, Action},
    document_validator::DocumentValidator,
    generate_document_id::generate_document_id,
    Document, DocumentsBatchTransition,
};

const PROPERTY_DOCUMENT_PROTOCOL_VERSION: &str = "$protocolVersion";
const PROPERTY_PROTOCOL_VERSION: &str = "protocolVersion";
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
pub struct DocumentFactory {
    protocol_version: u32,
    document_validator: DocumentValidator,
    fetch_and_validate_data_contract: mocks::FetchAndValidateDataContract,
}

impl DocumentFactory {
    pub fn new(
        protocol_version: u32,
        validate_document: DocumentValidator,
        fetch_and_validate_data_contract: mocks::FetchAndValidateDataContract,
    ) -> Self {
        DocumentFactory {
            protocol_version,
            document_validator: validate_document,
            fetch_and_validate_data_contract,
        }
    }

    pub fn create(
        &self,
        data_contract: DataContract,
        owner_id: Identifier,
        document_type: String,
        data: JsonValue,
    ) -> Result<Document, ProtocolError> {
        if !data_contract.is_document_defined(&document_type) {
            return Err(ProtocolError::InvalidDocumentTypeError {
                document_type,
                data_contract,
            });
        }

        let document_entropy = entropy_generator::generate();

        let document_required_fields = data_contract
            .get_document_schema(&document_type)?
            .get_schema_required_fields()?;

        let document_id = generate_document_id(
            &data_contract.id,
            &owner_id,
            &document_type,
            &document_entropy,
        );

        let mut raw_document = json!({
            PROPERTY_DOCUMENT_PROTOCOL_VERSION: self.protocol_version,
            PROPERTY_ID: document_id.to_buffer(),
            PROPERTY_DOCUMENT_TYPE: document_type,
            PROPERTY_DATA_CONTRACT_ID: data_contract.id.to_buffer(),
            PROPERTY_DOCUMENT_OWNER_ID: owner_id.to_buffer(),
            PROPERTY_REVISION: document_transition::INITIAL_REVISION,
        });

        if let JsonValue::Object(ref mut raw_document_map) = raw_document {
            if let JsonValue::Object(data_map) = data {
                raw_document_map.extend(data_map)
            }
        }

        let creation_time = Utc::now().timestamp_millis();
        if document_required_fields.contains(&PROPERTY_CREATED_AT) {
            raw_document.insert(PROPERTY_CREATED_AT.to_string(), json!(Some(creation_time)))?;
        }

        if document_required_fields.contains(&PROPERTY_UPDATED_AT) {
            raw_document.insert(PROPERTY_UPDATED_AT.to_string(), json!(Some(creation_time)))?;
        }

        let validation_result = self
            .document_validator
            .validate(&raw_document, &data_contract)?;

        if !validation_result.is_valid() {
            return Err(ProtocolError::InvalidDocumentError {
                errors: validation_result.errors,
                raw_document,
            });
        }

        let mut document = Document::from_raw_document(raw_document, data_contract)?;
        document.entropy = document_entropy;

        Ok(document)
    }

    pub fn create_state_transition(
        &self,
        documents_iter: impl IntoIterator<Item = (Action, Vec<Document>)>,
    ) -> Result<DocumentsBatchTransition, ProtocolError> {
        let mut raw_documents_transitions: Vec<JsonValue> = vec![];
        let mut data_contracts: Vec<DataContract> = vec![];
        let documents: Vec<(Action, Vec<Document>)> = documents_iter.into_iter().collect();
        let flattened_documents = documents.iter().flat_map(|(_, v)| v);

        if Self::is_empty(flattened_documents.clone()) {
            return Err(ProtocolError::NoDocumentsSuppliedError);
        }

        let is_the_same =
            Self::is_ownership_the_same(flattened_documents.clone().map(|d| &d.owner_id));
        if !is_the_same {
            return Err(ProtocolError::MismatchOwnerIdsError {
                documents: documents.into_iter().flat_map(|(_, v)| v).collect(),
            });
        }

        let owner_id = flattened_documents
            .clone()
            .next()
            .unwrap()
            .owner_id
            .to_owned();
        for (action, documents) in documents {
            data_contracts.extend(documents.iter().map(|d| d.data_contract.clone()));

            let raw_transitions = match action {
                Action::Create => Self::raw_document_create_transitions(documents)?,
                Action::Delete => Self::raw_document_delete_transitions(documents)?,
                Action::Replace => Self::raw_document_replace_transitions(documents)?,
            };

            raw_documents_transitions.extend(raw_transitions);
        }

        if raw_documents_transitions.is_empty() {
            return Err(ProtocolError::NoDocumentsSuppliedError);
        }

        let raw_batch_transition = json!({
            PROPERTY_PROTOCOL_VERSION: self.protocol_version,
            PROPERTY_OWNER_ID : owner_id.to_buffer(),
            PROPERTY_TRANSITIONS: raw_documents_transitions,
        });

        DocumentsBatchTransition::from_raw_object(raw_batch_transition, data_contracts)
    }

    fn raw_document_create_transitions(
        documents: Vec<Document>,
    ) -> Result<Vec<JsonValue>, ProtocolError> {
        let mut raw_transitions = vec![];
        for document in documents {
            if document.revision != document_transition::INITIAL_REVISION {
                return Err(ProtocolError::InvalidInitialRevisionError { document });
            }
            let mut raw_document = document.to_object(false)?;

            if let Some(map) = raw_document.as_object_mut() {
                map.retain(|key, _| {
                    !key.starts_with('$') || DOCUMENT_CREATE_KEYS_TO_STAY.contains(&key.as_str())
                });
                map.insert(
                    PROPERTY_ACTION.to_string(),
                    serde_json::to_value(Action::Create)?,
                );
                map.insert(
                    PROPERTY_ENTROPY.to_string(),
                    serde_json::to_value(document.entropy)?,
                );
            }
            raw_transitions.push(raw_document);
        }

        Ok(raw_transitions)
    }

    fn raw_document_replace_transitions(
        documents: Vec<Document>,
    ) -> Result<Vec<JsonValue>, ProtocolError> {
        let mut raw_transitions = vec![];
        for document in documents {
            let document_revision = document.revision;
            let mut raw_document = document.to_object(false)?;

            if let Some(map) = raw_document.as_object_mut() {
                map.retain(|key, _| {
                    !key.starts_with('$') || DOCUMENT_REPLACE_KEYS_TO_STAY.contains(&key.as_str())
                });
                map.insert(
                    PROPERTY_ACTION.to_string(),
                    serde_json::to_value(Action::Replace)?,
                );
                let new_revision = document_revision + 1;
                map.insert(PROPERTY_REVISION.to_string(), json!(new_revision));

                // If document have an originally set `updatedAt`
                // we should update it then
                if let Some(update_at) = map.get_mut(PROPERTY_UPDATED_AT) {
                    *update_at = json!(Utc::now().timestamp_millis())
                }
            }

            raw_transitions.push(raw_document);
        }
        Ok(raw_transitions)
    }

    fn raw_document_delete_transitions(
        documents: Vec<Document>,
    ) -> Result<Vec<JsonValue>, ProtocolError> {
        Ok(documents
            .into_iter()
            .map(|document| {
                json!({
                PROPERTY_ACTION: Action::Delete,
                PROPERTY_ID: document.id.buffer,
                PROPERTY_TYPE: document.document_type,
                PROPERTY_DATA_CONTRACT_ID: document.data_contract_id.buffer})
            })
            .collect())
    }

    // TODO implement rest methods
    //   async createFromObject(rawDocument, options = {}) {
    //   async createFromBuffer(buffer, options = {}) {

    fn is_empty<T>(data: impl IntoIterator<Item = T>) -> bool {
        data.into_iter().next().is_none()
    }

    fn is_ownership_the_same<'a>(docs: impl IntoIterator<Item = &'a Identifier>) -> bool {
        docs.into_iter().all_equal()
    }
}

#[cfg(test)]
mod test {
    use crate::{
        assert_error_contains,
        tests::{
            fixtures::{
                get_data_contract_fixture, get_document_validator_fixture, get_documents_fixture,
            },
            utils::generate_random_identifier_struct,
        },
        util::string_encoding::Encoding,
    };

    use super::*;

    #[test]
    fn document_with_type_and_data() {
        let mut data_contract = get_data_contract_fixture(None);
        let document_type = "niceDocument";

        let factory = DocumentFactory::new(
            1,
            get_document_validator_fixture(),
            mocks::FetchAndValidateDataContract {},
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

        data_contract.id = contract_id.clone();

        let document = factory
            .create(
                data_contract,
                owner_id.clone(),
                document_type.to_string(),
                json!({ "name": name }),
            )
            .expect("document creation shouldn't fail");

        assert_eq!(document_type, document.document_type);
        assert_eq!(
            name,
            document.get("name").expect("property 'name' should exist")
        );
        assert_eq!(contract_id, document.data_contract_id);
        assert_eq!(owner_id, document.owner_id);
        assert_eq!(document_transition::INITIAL_REVISION, document.revision);
        assert!(!document.id.to_string(Encoding::Base58).is_empty());
        assert!(document.created_at.is_some());
    }

    #[test]
    fn create_state_transition_no_documents() {
        let factory = DocumentFactory::new(
            1,
            get_document_validator_fixture(),
            mocks::FetchAndValidateDataContract {},
        );

        let result = factory.create_state_transition(vec![]);
        assert_error_contains!(result, "No documents were supplied to state transition")
    }

    #[test]
    fn create_transition_mismatch_user_id() {
        let data_contract = get_data_contract_fixture(None);
        let mut documents = get_documents_fixture(data_contract).unwrap();

        let factory = DocumentFactory::new(
            1,
            get_document_validator_fixture(),
            mocks::FetchAndValidateDataContract {},
        );
        documents[0].owner_id = generate_random_identifier_struct();

        let result = factory.create_state_transition(vec![(Action::Create, documents)]);
        assert_error_contains!(result, "Documents have mixed owner ids")
    }

    #[test]
    fn create_transition_invalid_initial_revision() {
        let data_contract = get_data_contract_fixture(None);
        let mut documents = get_documents_fixture(data_contract).unwrap();
        documents[0].revision = 3;

        let factory = DocumentFactory::new(
            1,
            get_document_validator_fixture(),
            mocks::FetchAndValidateDataContract {},
        );
        let result = factory.create_state_transition(vec![(Action::Create, documents)]);
        assert_error_contains!(result, "Invalid Document initial revision 3")
    }

    #[test]
    fn create_transitions_with_passed_documents() {
        let data_contract = get_data_contract_fixture(None);
        let documents = get_documents_fixture(data_contract).unwrap();
        let factory = DocumentFactory::new(
            1,
            get_document_validator_fixture(),
            mocks::FetchAndValidateDataContract {},
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
