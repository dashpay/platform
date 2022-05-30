use crate::{
    data_contract::DataContract,
    mocks,
    prelude::Identifier,
    util::entropy_generator,
    util::{json_schema::JsonSchemaExt, json_value::JsonValueExt},
    ProtocolError,
};
use chrono::Utc;
use serde_json::{json, Value as JsonValue};

use super::{
    document_transition::{self, Action},
    generate_document_id::generate_document_id,
    Document, DocumentsBatchTransition,
};

const PROPERTY_PROTOCOL_VERSION: &str = "protocolVersion";
const PROPERTY_ENTROPY: &str = "$entropy";
const PROPERTY_ACTION: &str = "$action";
const PROPERTY_OWNER_ID: &str = "ownerId";
const PROPERTY_TYPE: &str = "$type";
const PROPERTY_ID: &str = "$id";
const PROPERTY_DATA_CONTRACT_ID: &str = "$dataContractId";
const PROPERTY_REVISION: &str = "$revision";
const PROPERTY_CREATED_AT: &str = "$createdAt";
const PROPERTY_UPDATED_AT: &str = "$updatedAt";

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
    document_validator: mocks::DocumentValidator,
    fetch_and_validate_data_contract: mocks::FetchAndValidateDataContract,
}

impl DocumentFactory {
    pub fn new(
        protocol_version: u32,
        validate_document: mocks::DocumentValidator,
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
        data_contract: &DataContract,
        owner_id: Identifier,
        document_type: String,
        data: JsonValue,
    ) -> Result<Document, ProtocolError> {
        if !data_contract.is_document_defined(&document_type) {
            return Err(ProtocolError::InvalidDocumentTypeError {
                document_type,
                data_contract: data_contract.to_owned(),
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

        let mut document = Document {
            protocol_version: self.protocol_version,
            id: document_id,
            document_type,
            data_contract_id: data_contract.id.to_owned(),
            owner_id,
            revision: document_transition::INITIAL_REVISION,
            data,
            ..Default::default()
        };

        let creation_time = Utc::now().timestamp_millis();
        if document_required_fields.contains(&PROPERTY_CREATED_AT) {
            document.created_at = Some(creation_time)
        }

        if document_required_fields.contains(&PROPERTY_UPDATED_AT) {
            document.updated_at = Some(creation_time)
        }

        let validation_result = self
            .document_validator
            .validate_document(&document, data_contract);

        if !validation_result.is_valid() {
            return Err(ProtocolError::InvalidDocumentError {
                errors: validation_result.errors,
                document,
            });
        }

        document.entropy = Some(document_entropy);
        Ok(document)
    }

    pub fn create_state_transition(
        &self,
        documents: impl IntoIterator<Item = (Action, Vec<Document>)>,
    ) -> Result<DocumentsBatchTransition, ProtocolError> {
        let documents_iter = documents.into_iter();

        let mut raw_documents_transitions: Vec<JsonValue> = vec![];
        let mut data_contracts: Vec<DataContract> = vec![];

        for (action, documents) in documents_iter {
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
            "protocolVersion": self.protocol_version,
            "ownerId" : raw_documents_transitions[0].get_bytes("ownerId")?,
            "transitions": raw_documents_transitions,
        });

        DocumentsBatchTransition::from_raw_object(raw_batch_transition, data_contracts)
    }

    fn raw_document_create_transitions(
        documents: Vec<Document>,
    ) -> Result<Vec<JsonValue>, ProtocolError> {
        let mut raw_transitions = vec![];
        for document in documents {
            let mut raw_document = document.to_object(false)?;
            if let Some(map) = raw_document.as_object_mut() {
                map.retain(|key, _| {
                    key.starts_with('$') && !DOCUMENT_CREATE_KEYS_TO_STAY.contains(&key.as_ref())
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
                    key.starts_with('$') && !DOCUMENT_REPLACE_KEYS_TO_STAY.contains(&key.as_ref())
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
}
