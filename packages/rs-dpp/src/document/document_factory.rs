use crate::{
    data_contract::DataContract, mocks, prelude::Identifier, util::entropy_generator,
    util::json_schema::JsonSchemaExt, ProtocolError,
};
use chrono::Utc;
use serde_json::Value as JsonValue;

use super::{document_transition, generate_document_id::generate_document_id, Document};

pub const PROPERTY_CREATED_AT: &str = "$createdAt";
pub const PROPERTY_UPDATED_AT: &str = "$updatedAt";

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

        let mut document = Document {
            protocol_version: self.protocol_version,
            id: document_id,
            document_type,
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
            .validate_document(&document, &data_contract);

        if !validation_result.is_valid() {
            return Err(ProtocolError::InvalidDocumentError {
                errors: validation_result.errors,
                document,
            });
        }

        document.data_contract_id = data_contract.id;
        document.entropy = Some(document_entropy);

        Ok(document)
    }

    // TODO implement the rest of methods
}
