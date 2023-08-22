use std::sync::Arc;

use anyhow::anyhow;
use lazy_static::lazy_static;
use platform_value::Value;
use serde_json::Value as JsonValue;

use crate::consensus::basic::document::{InvalidDocumentTypeError, MissingDocumentTypeError};
use crate::data_contract::document_type::DocumentType;

use crate::validation::SimpleConsensusValidationResult;
use crate::{
    consensus::basic::BasicError,
    data_contract::{enrich_with_base_schema::PREFIX_BYTE_0, DataContract},
    validation::JsonSchemaValidator,
    version::ProtocolVersionValidator,
    ProtocolError,
};

const PROPERTY_PROTOCOL_VERSION: &str = "$protocolVersion";
const PROPERTY_DOCUMENT_TYPE: &str = "$type";

lazy_static! {
    pub static ref BASE_DOCUMENT_SCHEMA: JsonValue =
        serde_json::from_str(include_str!("../../../schema/document/document-base.json")).unwrap();
}

lazy_static! {
    pub static ref EXTENDED_DOCUMENT_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "../../../schema/document/documentExtended.json"
    ))
    .unwrap();
}

#[derive(Clone)]
pub struct DocumentValidator {
    protocol_version_validator: Arc<ProtocolVersionValidator>,
}

impl DocumentValidator {
    pub fn new(protocol_version_validator: Arc<ProtocolVersionValidator>) -> Self {
        Self {
            protocol_version_validator,
        }
    }

    pub fn validate(
        &self,
        raw_document: &JsonValue,
        data_contract: &DataContract,
        document_type: &DocumentType,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut result = SimpleConsensusValidationResult::default();
        let enriched_data_contract =
            data_contract.enrich_with_base_schema(&BASE_DOCUMENT_SCHEMA, PREFIX_BYTE_0, &[])?;

        //todo: maybe we should validate on the document type instead as it already has all the
        //information needed
        let document_schema = enriched_data_contract
            .get_document_schema(document_type.name.as_str())?
            .to_owned();

        let json_schema_validator = if let Some(defs) = &data_contract.defs {
            JsonSchemaValidator::new_with_definitions(document_schema, defs.iter())
        } else {
            JsonSchemaValidator::new(document_schema)
        }
        .map_err(|e| anyhow!("unable to process the contract: {}", e))?;

        let json_schema_validation_result = json_schema_validator.validate(raw_document)?;
        result.merge(json_schema_validation_result);

        if !result.is_valid() {
            return Ok(result);
        }
        //todo: validate the version

        Ok(result)
    }

    pub fn validate_extended(
        &self,
        raw_document: &Value,
        data_contract: &DataContract,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut result = SimpleConsensusValidationResult::default();

        let Some(document_type_name) = raw_document.get_optional_str(PROPERTY_DOCUMENT_TYPE).map_err(ProtocolError::ValueError)? else {
            result.add_error(BasicError::MissingDocumentTypeError(MissingDocumentTypeError::new()));
            return Ok(result);
        };

        // check if there is a document type
        if !data_contract.has_document_type_for_name(document_type_name) {
            result.add_error(BasicError::InvalidDocumentTypeError(
                InvalidDocumentTypeError::new(
                    document_type_name.to_owned(),
                    data_contract.id.to_owned(),
                ),
            ));
            return Ok(result);
        }

        let enriched_data_contract =
            data_contract.enrich_with_base_schema(&EXTENDED_DOCUMENT_SCHEMA, PREFIX_BYTE_0, &[])?;
        let document_schema = enriched_data_contract
            .get_document_schema(document_type_name)?
            .to_owned();

        let json_schema_validator = if let Some(defs) = &data_contract.defs {
            JsonSchemaValidator::new_with_definitions(document_schema, defs.iter())
        } else {
            JsonSchemaValidator::new(document_schema)
        }
        .map_err(|e| anyhow!("unable to process the contract: {}", e))?;

        let json_value = raw_document
            .try_to_validating_json()
            .map_err(ProtocolError::ValueError)?;
        let json_schema_validation_result = json_schema_validator.validate(&json_value)?;
        result.merge(json_schema_validation_result);

        if !result.is_valid() {
            return Ok(result);
        }

        let protocol_version = raw_document
            .get_integer(PROPERTY_PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?;
        result.merge(self.protocol_version_validator.validate(protocol_version)?);

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use platform_value::Value;
    use std::sync::Arc;
    use test_case::test_case;

    use crate::errors::consensus::codes::ErrorWithCode;
    use crate::tests::fixtures::get_extended_documents_fixture;
    use crate::tests::utils::json_schema_error;
    use crate::validation::SimpleConsensusValidationResult;
    use crate::{
        consensus::{basic::json_schema_error::JsonSchemaError, ConsensusError},
        data_contract::DataContract,
        tests::fixtures::get_data_contract_fixture,
        version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
    };

    use super::DocumentValidator;

    struct TestData {
        data_contract: DataContract,
        raw_document: Value,
        document_validator: DocumentValidator,
    }

    fn get_test_data() -> TestData {
        let data_contract = get_data_contract_fixture(None).data_contract;
        let documents = get_extended_documents_fixture(data_contract.clone()).unwrap();
        let raw_document = documents
            .iter()
            .map(|d| d.to_value())
            .next()
            .expect("at least one Document should be present")
            .expect("Document should be converted to Object");

        let protocol_version_validator = ProtocolVersionValidator::new(
            LATEST_VERSION,
            LATEST_VERSION,
            COMPATIBILITY_MAP.clone(),
        );

        let document_validator = DocumentValidator::new(Arc::new(protocol_version_validator));

        TestData {
            data_contract,
            raw_document,
            document_validator,
        }
    }
    #[test_case("$protocolVersion")]
    #[test_case("$revision")]
    #[test_case("$id")]
    #[test_case("$dataContractId")]
    #[test_case("$ownerId")]
    fn property_should_exist(property_name: &str) {
        let TestData {
            mut raw_document,
            document_validator,
            data_contract,
        } = get_test_data();
        raw_document
            .remove(property_name)
            .unwrap_or_else(|_| panic!("the {} should exist and be removed", property_name));

        let result = document_validator
            .validate_extended(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        assert_eq!(schema_error.keyword(), "required");
        assert_eq!(schema_error.property_name(), property_name);
    }

    #[test_case("$id")]
    #[test_case("$dataContractId")]
    #[test_case("$ownerId")]
    fn should_be_byte_array(property_name: &str) {
        let TestData {
            mut raw_document,
            document_validator,
            data_contract,
        } = get_test_data();

        raw_document
            .insert(
                String::from(property_name),
                Value::Text("string".to_string()),
            )
            .unwrap();

        let result = document_validator
            .validate_extended(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        let param_type = schema_error
            .params()
            .get_str("type")
            .expect("should has type param");

        assert_eq!(schema_error.keyword(), "type");
        assert_eq!(param_type, "array");

        assert_eq!(format!("/{}", property_name), schema_error.instance_path());
        assert_eq!(
            format!("/properties/{}/type", property_name),
            schema_error.schema_path()
        );
    }

    #[test_case("$id")]
    #[test_case("$dataContractId")]
    #[test_case("$ownerId")]
    fn id_should_be_byte_array_at_least_32_bytes_long(property_name: &str) {
        let TestData {
            mut raw_document,
            document_validator,
            data_contract,
        } = get_test_data();

        let too_short_id = [0u8; 31];
        raw_document
            .insert(
                String::from(property_name),
                Value::Bytes(too_short_id.to_vec()),
            )
            .unwrap();

        let result = document_validator
            .validate_extended(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        let min_items: u32 = schema_error
            .params()
            .get_integer("minItems")
            .expect("should get limit");

        assert_eq!(min_items, 32);

        assert_eq!(
            format!("/{}", property_name),
            schema_error.instance_path().to_string()
        );
        assert_eq!(
            format!("/properties/{}/minItems", property_name),
            schema_error.schema_path().to_string()
        );
    }

    #[test_case("$id")]
    #[test_case("$dataContractId")]
    #[test_case("$ownerId")]
    fn id_should_be_byte_array_no_more_32_bytes_long(property_name: &str) {
        let TestData {
            mut raw_document,
            document_validator,
            data_contract,
        } = get_test_data();

        let mut too_long_id = Vec::new();
        too_long_id.resize(33, 0u8);
        raw_document
            .insert(
                String::from(property_name),
                Value::Bytes(too_long_id.to_vec()),
            )
            .unwrap();

        let result = document_validator
            .validate_extended(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        let max_items: u32 = schema_error
            .params()
            .get_integer("maxItems")
            .expect("should get limit");

        assert_eq!(max_items, 32);

        assert_eq!(
            format!("/{}", property_name),
            schema_error.instance_path().to_string()
        );
        assert_eq!(
            format!("/properties/{}/maxItems", property_name),
            schema_error.schema_path().to_string()
        );
    }

    #[test]
    fn protocol_version_invalid_type() {
        let TestData {
            mut raw_document,
            document_validator,
            data_contract,
        } = get_test_data();

        raw_document
            .insert(
                String::from("$protocolVersion"),
                Value::Text("1".to_string()),
            )
            .unwrap();

        let result = document_validator
            .validate_extended(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        let param_type = schema_error
            .params()
            .get_str("type")
            .expect("should get type");

        assert_eq!(param_type, "integer");

        assert_eq!(
            "/$protocolVersion",
            schema_error.instance_path().to_string()
        );
    }

    #[test]
    fn type_should_exist() {
        let TestData {
            mut raw_document,
            document_validator,
            data_contract,
        } = get_test_data();

        raw_document
            .remove("$type")
            .expect("the '$type' should exist and be removed");

        let result = document_validator
            .validate_extended(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let validation_error = result.errors.get(0).expect("should return an error");
        assert!(
            matches!(validation_error,  ConsensusError::BasicError(basic_error) if basic_error.to_string() == "$type is not present")
        );
    }

    #[test]
    fn type_should_be_defined_in_data_contact() {
        let TestData {
            mut raw_document,
            document_validator,
            data_contract,
        } = get_test_data();

        raw_document
            .insert(
                "$type".to_string(),
                Value::Text("undefinedDocument".to_string()),
            )
            .unwrap();

        let result = document_validator
            .validate_extended(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let validation_error = result.errors.get(0).expect("the error should exist");
        assert_eq!(1024, validation_error.code());
    }

    #[test]
    fn revision_should_be_number() {
        let TestData {
            mut raw_document,
            document_validator,
            data_contract,
        } = get_test_data();

        raw_document
            .insert(String::from("$revision"), Value::Text("string".to_string()))
            .unwrap();

        let result = document_validator
            .validate_extended(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        let param_type = schema_error
            .params()
            .get_str("type")
            .expect("should get type");

        assert_eq!(param_type, "integer");

        assert_eq!("/$revision", schema_error.instance_path().to_string());
    }

    #[test]
    fn revision_should_be_integer() {
        let TestData {
            mut raw_document,
            document_validator,
            data_contract,
        } = get_test_data();

        raw_document
            .insert(String::from("$revision"), Value::Float(1.1))
            .unwrap();

        let result = document_validator
            .validate_extended(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        let param_type = schema_error
            .params()
            .get_str("type")
            .expect("should get type");

        assert_eq!(param_type, "integer");

        assert_eq!("/$revision", schema_error.instance_path().to_string());
        assert_eq!(
            "/properties/$revision/type",
            schema_error.schema_path().to_string()
        );
    }

    #[test]
    fn should_return_error_if_document_is_not_valid_against_data_contract() {
        let TestData {
            mut raw_document,
            document_validator,
            data_contract,
        } = get_test_data();

        raw_document
            .insert(String::from("name"), Value::U64(1))
            .unwrap();
        let result = document_validator
            .validate_extended(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        let param_type = schema_error
            .params()
            .get_str("type")
            .expect("should get type");

        assert_eq!(param_type, "string");

        assert_eq!(
            "/properties/name/type",
            schema_error.schema_path().to_string()
        );
    }

    #[test]
    fn should_return_error_if_document_contains_undefined_properties() {
        let TestData {
            mut raw_document,
            document_validator,
            data_contract,
        } = get_test_data();

        raw_document
            .insert(String::from("undefined"), Value::U64(1))
            .unwrap();

        let result = document_validator
            .validate_extended(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        let additional_properties = schema_error
            .params()
            .get_array("additionalProperties")
            .expect("should get additionalProperties");

        let additional_property = additional_properties
            .get(0)
            .expect("should have 0 item")
            .as_str()
            .expect("should be a string");

        assert_eq!(additional_property, "undefined");

        assert_eq!(
            "/additionalProperties",
            schema_error.schema_path().to_string()
        );
    }

    #[test]
    fn should_return_invalid_result_if_byte_array_exceeds_max_items() {
        let TestData {
            document_validator,
            data_contract,
            ..
        } = get_test_data();

        let documents = get_extended_documents_fixture(data_contract.clone()).unwrap();
        let document = documents.get(8).unwrap();

        let data = [0u8; 32];
        let mut raw_document = document.to_value().unwrap();
        raw_document
            .set_value("byteArrayField", Value::Bytes32(data))
            .unwrap();

        let result = document_validator
            .validate_extended(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        let max_items: u32 = schema_error
            .params()
            .get_integer("maxItems")
            .expect("should get limit");

        assert_eq!(max_items, 16);

        assert_eq!("/byteArrayField", schema_error.instance_path().to_string());
        assert_eq!(
            format!("/properties/byteArrayField/maxItems"),
            schema_error.schema_path().to_string()
        );
    }

    #[test]
    fn should_return_valid_result_if_document_valid() {
        let TestData {
            raw_document,
            document_validator,
            data_contract,
        } = get_test_data();

        let result = document_validator
            .validate_extended(&raw_document, &data_contract)
            .expect("the validator should return the validation result");

        assert!(result.is_valid())
    }

    fn get_first_schema_error(result: &SimpleConsensusValidationResult) -> &JsonSchemaError {
        json_schema_error(
            result
                .errors
                .get(0)
                .expect("the error should be returned in validation result"),
        )
    }
}
