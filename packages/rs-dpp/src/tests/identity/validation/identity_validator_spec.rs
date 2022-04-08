use crate::errors::consensus::ConsensusError;
use crate::identity::validation::IdentityValidator;
use crate::tests::utils::{assert_json_schema_error, serde_remove, serde_set};
use jsonschema::error::{TypeKind, ValidationErrorKind};
use jsonschema::paths::PathChunk;
use jsonschema::primitive_type::PrimitiveType::Integer;
use log::error;
use serde::de::Unexpected::Option;
use serde_json::Value;
use crate::version::COMPATIBILITY_MAP;

fn setup_test() -> (Value, IdentityValidator) {
    (
        crate::tests::fixtures::identity_fixture_json(),
        IdentityValidator::new().unwrap(),
    )
}

#[test]
pub fn protocol_version_should_be_present() {
    let (mut identity, identity_validator) = setup_test();
    identity = serde_remove(identity, "protocolVersion");

    let result = identity_validator.validate_identity(&identity);

    let errors = assert_json_schema_error(&result, 1);
    let error = errors.first().unwrap();

    assert_eq!(error.keyword().unwrap(), "required");
    assert_eq!(error.instance_path().to_string(), "");

    match error.kind() {
        ValidationErrorKind::Required { property } => {
            assert_eq!(property.to_string(), "\"protocolVersion\"");
        }
        _ => panic!("Expected to be missing property"),
    }
}

#[test]
pub fn protocol_version_should_be_an_integer() {
    let (mut identity, identity_validator) = setup_test();
    identity = serde_set(identity, "protocolVersion", "1");

    let result = identity_validator.validate_identity(&identity);

    let errors = assert_json_schema_error(&result, 1);
    let error = errors.first().unwrap();

    assert_eq!(error.keyword().unwrap(), "type");
    assert_eq!(error.instance_path().to_string(), "/protocolVersion");
}

#[test]
pub fn protocol_version_should_be_valid() {
    let (mut identity, identity_validator) = setup_test();
    identity = serde_set(identity, "protocolVersion", -1);

    let result = identity_validator.validate_identity(&identity);

    let errors = assert_json_schema_error(&result, 1);
    let error = errors.first().unwrap();

    assert_eq!(error.keyword().unwrap(), "type");
    assert_eq!(error.instance_path().to_string(), "/protocolVersion");
}

// Id tests
#[test]
pub fn id_should_be_present() {
    let (mut identity, identity_validator) = setup_test();
    identity = serde_remove(identity, "id");

    let result = identity_validator.validate_identity(&identity);

    let errors = assert_json_schema_error(&result, 1);
    let error = errors.first().unwrap();

    assert_eq!(error.keyword().unwrap(), "required");
    assert_eq!(error.instance_path().to_string(), "");

    match error.kind() {
        ValidationErrorKind::Required { property } => {
            assert_eq!(property.to_string(), "\"id\"");
        }
        _ => panic!("Expected to be missing property"),
    }
}

#[test]
pub fn id_should_be_a_byte_array() {
    // TODO: fix this test
    let (mut identity, identity_validator) = setup_test();
    identity = serde_set(identity, "id", vec![Value::from("string"); 32]);

    let result = identity_validator.validate_identity(&identity);

    //assert_json_schema_error(&result, 2);

    let error = result
        .errors()
        .first()
        .expect("Expected to be at least one validation error");
    let byte_array_error = result.errors().last().unwrap();

    match error {
        ConsensusError::JsonSchemaError(error) => {
            let keyword = error.keyword().expect("Expected to have a keyword");

            assert_eq!(keyword, "type");
            assert_eq!(error.instance_path().to_string(), "/id/0");
        }
        _ => {}
    }

    let err = byte_array_error
        .json_schema_error()
        .expect("Expected to be a JsonSchemaError");
    assert_eq!(err.keyword(), Some("byteArray"))
}

#[test]
pub fn id_should_not_be_less_than_32_bytes() {
    let (mut identity, identity_validator) = setup_test();
    identity = serde_set(identity, "id", vec![Value::from(15); 31]);

    let result = identity_validator.validate_identity(&identity);

    let errors = assert_json_schema_error(&result, 1);
    let error = errors.first().unwrap();

    assert_eq!(error.keyword().unwrap(), "minItems");
    assert_eq!(error.instance_path().to_string(), "/id");
}

#[test]
pub fn id_should_not_be_more_than_32_bytes() {
    let (mut identity, identity_validator) = setup_test();
    identity = serde_set(identity, "id", vec![Value::from(15); 33]);

    let result = identity_validator.validate_identity(&identity);

    let errors = assert_json_schema_error(&result, 1);
    let error = errors.first().unwrap();

    assert_eq!(error.keyword().unwrap(), "maxItems");
    assert_eq!(error.instance_path().to_string(), "/id");
}

// Balance tests
#[test]
pub fn balance_should_be_present() {
    let (mut identity, identity_validator) = setup_test();
    identity = serde_remove(identity, "balance");

    let result = identity_validator.validate_identity(&identity);

    let errors = assert_json_schema_error(&result, 1);
    let error = errors.first().unwrap();

    assert_eq!(error.keyword().unwrap(), "required");
    assert_eq!(error.instance_path().to_string(), "");

    match error.kind() {
        ValidationErrorKind::Required { property } => {
            assert_eq!(property.to_string(), "\"balance\"");
        }
        _ => panic!("Expected to be missing property"),
    }
}

#[test]
pub fn balance_should_be_an_integer() {
    let (mut identity, identity_validator) = setup_test();
    identity = serde_set(identity, "balance", 1.2);

    let result = identity_validator.validate_identity(&identity);

    let errors = assert_json_schema_error(&result, 1);
    let error = errors.first().unwrap();

    assert_eq!(error.keyword().unwrap(), "type");
    assert_eq!(error.instance_path().to_string(), "/balance");
}

#[test]
pub fn balance_should_be_greater_or_equal_0() {
    let (mut identity, identity_validator) = setup_test();
    identity = serde_set(identity, "balance", -1);

    let result = identity_validator.validate_identity(&identity);

    let errors = assert_json_schema_error(&result, 1);
    let error = errors.first().unwrap();

    assert_eq!(error.keyword().unwrap(), "minimum");
    assert_eq!(error.instance_path().to_string(), "/balance");

    identity = serde_set(identity, "balance", 0);
    let result = identity_validator.validate_identity(&identity);

    assert!(result.is_valid());
}

// publicKeys test
#[test]
pub fn public_keys_should_be_present() {
    let (mut identity, identity_validator) = setup_test();
    identity = serde_remove(identity, "publicKeys");

    let result = identity_validator.validate_identity(&identity);

    let errors = assert_json_schema_error(&result, 1);
    let error = errors.first().unwrap();

    assert_eq!(error.keyword().unwrap(), "required");
    assert_eq!(error.instance_path().to_string(), "");

    match error.kind() {
        ValidationErrorKind::Required { property } => {
            assert_eq!(property.to_string(), "\"publicKeys\"");
        }
        _ => panic!("Expected to be missing property"),
    }
}

#[test]
pub fn public_keys_should_be_an_array() {
    let (mut identity, identity_validator) = setup_test();
    identity = serde_set(identity, "publicKeys", 1);

    let result = identity_validator.validate_identity(&identity);

    let errors = assert_json_schema_error(&result, 1);
    let error = errors.first().unwrap();

    assert_eq!(error.keyword().unwrap(), "type");
    assert_eq!(error.instance_path().to_string(), "/publicKeys");
}

#[test]
pub fn public_keys_should_not_be_empty() {
    let (mut identity, identity_validator) = setup_test();
    identity = serde_set(identity, "publicKeys", Value::Array(vec![]));

    let result = identity_validator.validate_identity(&identity);

    let errors = assert_json_schema_error(&result, 1);
    let error = errors.first().unwrap();

    assert_eq!(error.keyword().unwrap(), "minItems");
    assert_eq!(error.instance_path().to_string(), "/publicKeys");
}

#[test]
pub fn public_keys_should_be_unique() {
    let (mut identity, identity_validator) = setup_test();

    let public_key = identity
        .get("publicKeys")
        .unwrap()
        .as_array()
        .unwrap()
        .get(0)
        .unwrap()
        .clone();

    identity = serde_set(identity, "publicKeys", Value::Array(vec![public_key.clone(), public_key.clone()]));

    let result = identity_validator.validate_identity(&identity);

    let errors = assert_json_schema_error(&result, 1);
    let error = errors.first().unwrap();

    assert_eq!(error.keyword().unwrap(), "uniqueItems");
    assert_eq!(error.instance_path().to_string(), "/publicKeys");
}

#[test]
pub fn public_keys_should_throw_an_error_if_public_keys_have_more_than_100_keys() {
    let (mut identity, identity_validator) = setup_test();

    let public_key = identity
        .get("publicKeys")
        .unwrap()
        .as_array()
        .unwrap()
        .get(0)
        .unwrap()
        .clone();

    identity = serde_set(identity, "publicKeys", Value::Array(vec![public_key.clone(); 101]));

    let result = identity_validator.validate_identity(&identity);

    let errors = assert_json_schema_error(&result, 2);
    let error = errors.first().unwrap();

    assert_eq!(error.keyword().unwrap(), "maxItems");
    assert_eq!(error.instance_path().to_string(), "/publicKeys");
}

// revision tests
#[test]
pub fn revision_should_be_present() {
    let (mut identity, identity_validator) = setup_test();
    identity = serde_remove(identity, "revision");

    let result = identity_validator.validate_identity(&identity);

    let errors = assert_json_schema_error(&result, 1);
    let error = errors.first().unwrap();

    assert_eq!(error.keyword().unwrap(), "required");
    assert_eq!(error.instance_path().to_string(), "");

    match error.kind() {
        ValidationErrorKind::Required { property } => {
            assert_eq!(property.to_string(), "\"revision\"");
        }
        _ => panic!("Expected to be missing property"),
    }
}

#[test]
pub fn revision_should_be_an_integer() {
    let mut identity = crate::tests::fixtures::identity_fixture_json();
    let identity_validator = IdentityValidator::new().unwrap();

    identity = serde_set(identity, "revision", 1.2);

    let result = identity_validator.validate_identity(&identity);
    let errors = assert_json_schema_error(&result, 1);

    let error = errors
        .first()
        .expect("Expected to be at least one validation error");

    assert_eq!(error.keyword().unwrap(), "type");
    assert_eq!(error.instance_path().to_string(), "/revision");
}

#[test]
pub fn revision_should_should_be_greater_or_equal_0() {
    let mut identity = crate::tests::fixtures::identity_fixture_json();
    let identity_validator = IdentityValidator::new().unwrap();

    identity = serde_set(identity, "revision", -1);

    let result = identity_validator.validate_identity(&identity);
    let errors = assert_json_schema_error(&result, 1);

    let error = errors
        .first()
        .expect("Expected to be at least one validation error");

    assert_eq!(error.keyword().unwrap(), "minimum");
    assert_eq!(error.instance_path().to_string(), "/revision");

    identity = serde_set(identity, "revision", 0);

    let result = identity_validator.validate_identity(&identity);

    assert!(result.is_valid());
}

// general tests
#[test]
pub fn should_return_valid_result_if_a_raw_identity_is_valid() {
    let (mut identity, identity_validator) = setup_test();

    let result = identity_validator.validate_identity(&identity);
    assert_json_schema_error(&result, 0);

    assert!(result.is_valid());
}

#[test]
pub fn should_return_valid_result_if_an_identity_model_is_valid() {
    assert!(false);
}
