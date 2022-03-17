use std::process::id;
use jsonschema::error::{TypeKind, ValidationErrorKind};
use jsonschema::paths::PathChunk;
use jsonschema::primitive_type::PrimitiveType::Integer;
use log::error;
use serde::de::Unexpected::Option;
use serde_json::Value;
use crate::errors::consensus::ConsensusError;
use crate::identity::validation::IdentityValidator;
use crate::tests::utils::assert_json_schema_error;

// Id tests
#[test]
pub fn id_should_be_present() {
    let mut identity = crate::tests::fixtures::identity_fixture_json();
    let map = identity.as_object_mut().expect("Expected value to be an JSON object");
    map.remove("id");

    let identity_validator = IdentityValidator::new().unwrap();
    let result = identity_validator.validate_identity(&identity);

    assert_json_schema_error(&result, 1);

    let error = result.errors().first().expect("Expected to be at least one validation error");

    match error {
        ConsensusError::JsonSchemaError(error) => {
            let keyword = error.keyword().expect("Expected to have a keyword");

            assert_eq!(keyword, "required");
            assert_eq!(error.instance_path().to_string(), "");

            // TODO: note that in the original code that was under "getParams().missingProperty"
            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"id\"");
                }
                _ => panic!("Expected to be missing property")
            }
        }
        _ => {}
    }
}

#[test]
pub fn id_should_be_a_byte_array() {
    let mut identity = crate::tests::fixtures::identity_fixture_json();
    let map = identity.as_object_mut().expect("Expected value to be an JSON object");
    map.insert("id".parse().unwrap(), Value::from(vec!["string"; 32]));

    let identity_validator = IdentityValidator::new().unwrap();
    let result = identity_validator.validate_identity(&identity);

    assert_json_schema_error(&result, 2);

    let error = result.errors().first().expect("Expected to be at least one validation error");
    let byte_array_error = result.errors().get(2).unwrap();

    match error {
        ConsensusError::JsonSchemaError(error) => {
            let keyword = error.keyword().expect("Expected to have a keyword");

            assert_eq!(keyword, "type");
            assert_eq!(error.instance_path().to_string(), "/id/0");
        }
        _ => {}
    }

    let err = byte_array_error.json_schema_error().expect("Expected to be a JsonSchemaError");
    assert_eq!(err.keyword(), Some("byteArray"))
}

// Balance tests
#[test]
pub fn balance_should_be_present() {
    let mut identity = crate::tests::fixtures::identity_fixture_json();
    let map = identity.as_object_mut().expect("Expected value to be an JSON object");
    map.remove("balance");

    let identity_validator = IdentityValidator::new().unwrap();
    let result = identity_validator.validate_identity(&identity);

    assert_json_schema_error(&result, 1);

    let error = result.errors().first().expect("Expected to be at least one validation error");

    match error {
        ConsensusError::JsonSchemaError(error) => {
            let keyword = error.keyword().expect("Expected to have a keyword");

            assert_eq!(keyword, "required");
            assert_eq!(error.instance_path().to_string(), "");

            // TODO: note that in the original code that was under "getParams().missingProperty"
            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"balance\"");
                }
                _ => panic!("Expected to be missing property")
            }
        }
        _ => {}
    }
}

#[test]
pub fn balance_should_be_an_integer() {
    let mut identity = crate::tests::fixtures::identity_fixture_json();
    let map = identity.as_object_mut().expect("Expected value to be an JSON object");
    map.insert("balance".parse().unwrap(), Value::from(1.2));

    let identity_validator = IdentityValidator::new().unwrap();
    let result = identity_validator.validate_identity(&identity);

    assert_json_schema_error(&result, 1);

    let error = result.errors().first().expect("Expected to be at least one validation error");

    match error {
        ConsensusError::JsonSchemaError(error) => {
            let keyword = error.keyword().expect("Expected to have a keyword");

            assert_eq!(keyword, "type");
            assert_eq!(error.instance_path().to_string(), "/balance");
        }
        _ => {}
    }
}

#[test]
pub fn balance_should_be_greater_or_equal_0() {
    let mut identity = crate::tests::fixtures::identity_fixture_json();
    let map = identity.as_object_mut().expect("Expected value to be an JSON object");
    map.insert("balance".parse().unwrap(), Value::from(-1));

    let identity_validator = IdentityValidator::new().unwrap();
    let result = identity_validator.validate_identity(&identity);

    assert_json_schema_error(&result, 1);

    let error = result.errors().first().expect("Expected to be at least one validation error");

    match error {
        ConsensusError::JsonSchemaError(error) => {
            let keyword = error.keyword().expect("Expected to have a keyword");

            assert_eq!(keyword, "minimum");
            assert_eq!(error.instance_path().to_string(), "/balance");
        }
        _ => {}
    }

    let mut identity = crate::tests::fixtures::identity_fixture_json();
    let map = identity.as_object_mut().expect("Expected value to be an JSON object");
    map.insert("balance".parse().unwrap(), Value::from(0));

    let result = identity_validator.validate_identity(&identity);

    assert!(result.is_valid());
}