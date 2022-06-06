use std::{collections::HashMap, sync::Arc};

use crate::{
    consensus::basic::BasicError,
    data_contract::{
        enrich_data_contract_with_base_schema::enrich_data_contract_with_base_schema,
        enrich_data_contract_with_base_schema::PREFIX_BYTE_0, DataContract,
    },
    util::{json_value::JsonValueExt, string_encoding::Encoding},
    validation::{JsonSchemaValidator, ValidationResult},
    version::ProtocolVersionValidator,
    Convertible, ProtocolError,
};
use anyhow::anyhow;
use lazy_static::lazy_static;

use serde_json::Value as JsonValue;

const PROPERTY_PROTOCOL_VERSION: &str = "$protocolVersion";
const PROPERTY_DOCUMENT_TYPE: &str = "$type";

lazy_static! {
    static ref BASE_DOCUMENT_SCHEMA: JsonValue =
        serde_json::from_str(include_str!("../../schema/document/documentBase.json")).unwrap();
}

pub struct DocumentValidator {
    json_schema_validator: Arc<JsonSchemaValidator>,
    protocol_version_validator: Arc<ProtocolVersionValidator>,
}

impl DocumentValidator {
    pub fn new(
        json_schema_validator: Arc<JsonSchemaValidator>,
        protocol_version_validator: Arc<ProtocolVersionValidator>,
    ) -> Self {
        Self {
            json_schema_validator,
            protocol_version_validator,
        }
    }

    pub fn validate(
        &self,
        raw_document: &JsonValue,
        data_contract: &DataContract,
    ) -> Result<ValidationResult, ProtocolError> {
        let mut result = ValidationResult::default();

        let maybe_document_type = raw_document.get(PROPERTY_DOCUMENT_TYPE);
        if maybe_document_type.is_none() {
            result.add_error(BasicError::MissingDocumentTypeError);
            return Ok(result);
        }

        let enriched_data_contract = enrich_data_contract_with_base_schema(
            data_contract,
            &BASE_DOCUMENT_SCHEMA,
            PREFIX_BYTE_0,
            &[],
        )?;

        let document_type = maybe_document_type.unwrap().as_str().ok_or_else(|| {
            anyhow!(
                "the document type '{:?}' cannot be converted into the string",
                maybe_document_type
            )
        })?;
        let document_schema_ref = enriched_data_contract.get_document_schema_ref(document_type);

        let mut additional_schemas = HashMap::new();
        additional_schemas.insert(
            enriched_data_contract.id.to_string(Encoding::Base58),
            enriched_data_contract.to_json()?,
        );

        let json_schema_validation_result = self.json_schema_validator.validate(raw_document)?;
        result.merge(json_schema_validation_result);

        if !result.is_valid() {
            return Ok(result);
        }

        let protocol_version = raw_document.get_u64(PROPERTY_PROTOCOL_VERSION)? as u32;
        result.merge(self.protocol_version_validator.validate(protocol_version)?);

        Ok(result)
    }
}
