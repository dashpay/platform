use std::sync::Arc;

use crate::{
    consensus::basic::BasicError,
    data_contract::{
        enrich_data_contract_with_base_schema::enrich_data_contract_with_base_schema,
        enrich_data_contract_with_base_schema::PREFIX_BYTE_0, DataContract,
    },
    util::json_value::JsonValueExt,
    validation::{JsonSchemaValidator, ValidationResult},
    version::ProtocolVersionValidator,
    ProtocolError,
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
    ) -> Result<ValidationResult, ProtocolError> {
        let mut result = ValidationResult::default();

        let maybe_document_type = raw_document.get(PROPERTY_DOCUMENT_TYPE);
        if maybe_document_type.is_none() {
            result.add_error(BasicError::MissingDocumentTypeError);
            return Ok(result);
        }

        let document_type = maybe_document_type.unwrap().as_str().ok_or_else(|| {
            anyhow!(
                "the document type '{:?}' cannot be converted into the string",
                maybe_document_type
            )
        })?;

        if !data_contract.is_document_defined(document_type) {
            result.add_error(BasicError::InvalidDocumentTypeError {
                document_type: document_type.to_owned(),
                data_contract_id: data_contract.id.to_owned(),
            });
            return Ok(result);
        }

        let enriched_data_contract = enrich_data_contract_with_base_schema(
            data_contract,
            &BASE_DOCUMENT_SCHEMA,
            PREFIX_BYTE_0,
            &[],
        )?;
        let document_schema = enriched_data_contract
            .get_document_schema(document_type)?
            .to_owned();

        let json_schema_validator =
            JsonSchemaValidator::new_with_definitions(document_schema, &data_contract.defs)
                .map_err(|e| anyhow!("unable to process the contract: {}", e))?;

        let json_schema_validation_result = json_schema_validator.validate(raw_document)?;
        result.merge(json_schema_validation_result);
        if !result.is_valid() {
            return Ok(result);
        }

        let protocol_version = raw_document.get_u64(PROPERTY_PROTOCOL_VERSION)? as u32;
        result.merge(self.protocol_version_validator.validate(protocol_version)?);

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::{
        codes::ErrorWithCode,
        consensus::{basic::JsonSchemaError, ConsensusError},
        data_contract::DataContract,
        tests::fixtures::{get_data_contract_fixture, get_documents_fixture},
        util::json_value::JsonValueExt,
        validation::ValidationResult,
        version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
    };

    use super::DocumentValidator;
    use jsonschema::{
        error::{TypeKind, ValidationErrorKind},
        primitive_type::PrimitiveType,
    };
    use serde_json::Value as JsonValue;
    use std::sync::Arc;
    use test_case::test_case;

    struct TestData {
        data_contract: DataContract,
        raw_document: JsonValue,
        document_validator: DocumentValidator,
    }

    fn get_test_data() -> TestData {
        let data_contract = get_data_contract_fixture(None);
        let documents = get_documents_fixture(data_contract.clone()).unwrap();
        let raw_document = documents
            .iter()
            .map(|d| d.to_object(false))
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
            .validate(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Required {
                property: JsonValue::String(protocol_version)
            } if protocol_version == property_name
        ));
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
            .insert(String::from(property_name), json!("string"))
            .unwrap();

        let result = document_validator
            .validate(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Type {
                kind: TypeKind::Single(primitive_type)
            } if matches!(primitive_type, PrimitiveType::Array)
        ));

        assert_eq!(
            format!("/{}", property_name),
            schema_error.instance_path().to_string()
        );
        assert_eq!(
            format!("/properties/{}/type", property_name),
            schema_error.schema_path().to_string()
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
            .insert(String::from(property_name), json!(too_short_id))
            .unwrap();

        let result = document_validator
            .validate(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::MinItems {limit} if limit == &32));

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
                serde_json::to_value(too_long_id).unwrap(),
            )
            .unwrap();

        let result = document_validator
            .validate(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::MaxItems {limit} if limit == &32));

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
            .insert(String::from("$protocolVersion"), json!("1"))
            .unwrap();

        let result = document_validator
            .validate(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Type {
                kind: TypeKind::Single(primitive_type)
            } if matches!(primitive_type, PrimitiveType::Integer)
        ));

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
            .validate(&raw_document, &data_contract)
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
            .insert("$type".to_string(), json!("undefinedDocument"))
            .unwrap();

        let result = document_validator
            .validate(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let validation_error = result.errors.get(0).expect("the error should exist");
        assert_eq!(1024, validation_error.get_code());
    }

    #[test]
    fn revision_should_be_number() {
        let TestData {
            mut raw_document,
            document_validator,
            data_contract,
        } = get_test_data();

        raw_document
            .insert(String::from("$revision"), json!("string"))
            .unwrap();

        let result = document_validator
            .validate(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Type {
                kind: TypeKind::Single(primitive_type)
            } if matches!(primitive_type, PrimitiveType::Integer)
        ));
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
            .insert(String::from("$revision"), json!(1.1))
            .unwrap();

        let result = document_validator
            .validate(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Type {
                kind: TypeKind::Single(primitive_type)
            } if matches!(primitive_type, PrimitiveType::Integer)
        ));
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

        raw_document.insert(String::from("name"), json!(1)).unwrap();
        let result = document_validator
            .validate(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Type {
                kind: TypeKind::Single(primitive_type)
            } if matches!(primitive_type, PrimitiveType::String)
        ));
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
            .insert(String::from("undefined"), json!(1))
            .unwrap();

        let result = document_validator
            .validate(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::AdditionalProperties {
                unexpected,
            } if unexpected == &["undefined"]));
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

        let documents = get_documents_fixture(data_contract.clone()).unwrap();
        let document = documents.get(8).unwrap();

        let data = [0u8; 32];
        let mut raw_document = document.to_object(false).unwrap();
        raw_document
            .insert("byteArrayField".to_string(), json!(data))
            .unwrap();

        let result = document_validator
            .validate(&raw_document, &data_contract)
            .expect("the validator should return the validation result");
        let schema_error = get_first_schema_error(&result);

        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::MaxItems {limit} if limit == &16));

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
            .validate(&raw_document, &data_contract)
            .expect("the validator should return the validation result");

        assert!(result.is_valid())
    }

    fn get_first_schema_error(result: &ValidationResult) -> &JsonSchemaError {
        result
            .errors
            .get(0)
            .expect("the error should be returned in validation result")
            .json_schema_error()
            .expect("the error should be json schema error")
    }
}
