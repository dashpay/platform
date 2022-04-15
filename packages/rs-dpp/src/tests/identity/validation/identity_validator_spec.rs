use crate::identity::validation::IdentityValidator;
use crate::tests::utils::assert_json_schema_error;
use crate::version::ProtocolVersionValidator;
use serde_json::Value;
use std::sync::Arc;

fn setup_test() -> (Value, IdentityValidator) {
    let protocol_version_validator = ProtocolVersionValidator::default();
    (
        crate::tests::fixtures::identity_fixture_json(),
        IdentityValidator::new(Arc::new(protocol_version_validator)).unwrap(),
    )
}

pub mod protocol_version {
    use crate::tests::identity::validation::identity_validator_spec::setup_test;
    use crate::tests::utils::{assert_json_schema_error, serde_remove, serde_set};
    use jsonschema::error::ValidationErrorKind;

    #[test]
    pub fn should_be_present() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_remove(identity, "protocolVersion");

        let result = identity_validator.validate_identity(&identity).unwrap();

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
    pub fn should_be_an_integer() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_set(identity, "protocolVersion", "1");

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_json_schema_error(&result, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "type");
        assert_eq!(error.instance_path().to_string(), "/protocolVersion");
    }

    #[test]
    pub fn should_be_valid() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_set(identity, "protocolVersion", -1);

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_json_schema_error(&result, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "type");
        assert_eq!(error.instance_path().to_string(), "/protocolVersion");
    }
}

pub mod id {
    use crate::consensus::ConsensusError;
    use crate::tests::identity::validation::identity_validator_spec::setup_test;
    use crate::tests::utils::{assert_json_schema_error, serde_remove, serde_set};
    use jsonschema::error::ValidationErrorKind;
    use serde_json::Value;

    #[test]
    pub fn should_be_present() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_remove(identity, "id");

        let result = identity_validator.validate_identity(&identity).unwrap();

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
    pub fn should_be_a_byte_array() {
        // TODO: fix this test
        let (mut identity, identity_validator) = setup_test();
        identity = serde_set(identity, "id", vec![Value::from("string"); 32]);

        let result = identity_validator.validate_identity(&identity);

        //assert_json_schema_error(&result, 2);

        let validation_result = result.unwrap();

        let error = validation_result
            .errors()
            .first()
            .expect("Expected to be at least one validation error");
        let byte_array_error = validation_result.errors().last().unwrap();

        match error {
            ConsensusError::JsonSchemaError(error) => {
                let keyword = error.keyword().expect("Expected to have a keyword");

                assert_eq!(keyword, "type");
                assert_eq!(error.instance_path().to_string(), "/id/0");
            }
            _ => {
                panic!("Expected JSON schema error")
            }
        }

        let err = byte_array_error
            .json_schema_error()
            .expect("Expected to be a JsonSchemaError");
        assert_eq!(err.keyword(), Some("byteArray"))
    }

    #[test]
    pub fn should_not_be_less_than_32_bytes() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_set(identity, "id", vec![Value::from(15); 31]);

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_json_schema_error(&result, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "minItems");
        assert_eq!(error.instance_path().to_string(), "/id");
    }

    #[test]
    pub fn should_not_be_more_than_32_bytes() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_set(identity, "id", vec![Value::from(15); 33]);

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_json_schema_error(&result, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "maxItems");
        assert_eq!(error.instance_path().to_string(), "/id");
    }
}

pub mod balance {
    use crate::tests::identity::validation::identity_validator_spec::setup_test;
    use crate::tests::utils::{assert_json_schema_error, serde_remove, serde_set};
    use jsonschema::error::ValidationErrorKind;

    #[test]
    pub fn should_be_present() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_remove(identity, "balance");

        let result = identity_validator.validate_identity(&identity).unwrap();

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
    pub fn should_be_an_integer() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_set(identity, "balance", 1.2);

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_json_schema_error(&result, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "type");
        assert_eq!(error.instance_path().to_string(), "/balance");
    }

    #[test]
    pub fn should_be_greater_or_equal_0() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_set(identity, "balance", -1);

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_json_schema_error(&result, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "minimum");
        assert_eq!(error.instance_path().to_string(), "/balance");

        identity = serde_set(identity, "balance", 0);
        let result = identity_validator.validate_identity(&identity).unwrap();

        assert!(result.is_valid());
    }
}

pub mod public_keys {
    use crate::tests::identity::validation::identity_validator_spec::setup_test;
    use crate::tests::utils::{assert_json_schema_error, serde_remove, serde_set};
    use jsonschema::error::ValidationErrorKind;
    use serde_json::Value;

    #[test]
    pub fn should_be_present() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_remove(identity, "publicKeys");

        let result = identity_validator.validate_identity(&identity).unwrap();

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
    pub fn should_be_an_array() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_set(identity, "publicKeys", 1);

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_json_schema_error(&result, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "type");
        assert_eq!(error.instance_path().to_string(), "/publicKeys");
    }

    #[test]
    pub fn should_not_be_empty() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_set(identity, "publicKeys", Value::Array(vec![]));

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_json_schema_error(&result, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "minItems");
        assert_eq!(error.instance_path().to_string(), "/publicKeys");
    }

    #[test]
    pub fn should_be_unique() {
        let (mut identity, identity_validator) = setup_test();

        let public_key = identity
            .get("publicKeys")
            .unwrap()
            .as_array()
            .unwrap()
            .get(0)
            .unwrap()
            .clone();

        identity = serde_set(
            identity,
            "publicKeys",
            Value::Array(vec![public_key.clone(), public_key.clone()]),
        );

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_json_schema_error(&result, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "uniqueItems");
        assert_eq!(error.instance_path().to_string(), "/publicKeys");
    }

    #[test]
    pub fn should_throw_an_error_if_public_keys_have_more_than_100_keys() {
        let (mut identity, identity_validator) = setup_test();

        let public_key = identity
            .get("publicKeys")
            .unwrap()
            .as_array()
            .unwrap()
            .get(0)
            .unwrap()
            .clone();

        identity = serde_set(
            identity,
            "publicKeys",
            Value::Array(vec![public_key.clone(); 101]),
        );

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_json_schema_error(&result, 2);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "maxItems");
        assert_eq!(error.instance_path().to_string(), "/publicKeys");
    }
}

pub mod revision {
    use crate::tests::identity::validation::identity_validator_spec::setup_test;
    use crate::tests::utils::{assert_json_schema_error, serde_remove, serde_set};
    use jsonschema::error::ValidationErrorKind;

    // revision tests
    #[test]
    pub fn should_be_present() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_remove(identity, "revision");

        let result = identity_validator.validate_identity(&identity).unwrap();

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
    pub fn should_be_an_integer() {
        let (mut identity, identity_validator) = setup_test();

        identity = serde_set(identity, "revision", 1.2);

        let result = identity_validator.validate_identity(&identity).unwrap();
        let errors = assert_json_schema_error(&result, 1);

        let error = errors
            .first()
            .expect("Expected to be at least one validation error");

        assert_eq!(error.keyword().unwrap(), "type");
        assert_eq!(error.instance_path().to_string(), "/revision");
    }

    #[test]
    pub fn should_should_be_greater_or_equal_0() {
        let (mut identity, identity_validator) = setup_test();

        identity = serde_set(identity, "revision", -1);

        let result = identity_validator.validate_identity(&identity).unwrap();
        let errors = assert_json_schema_error(&result, 1);

        let error = errors
            .first()
            .expect("Expected to be at least one validation error");

        assert_eq!(error.keyword().unwrap(), "minimum");
        assert_eq!(error.instance_path().to_string(), "/revision");

        identity = serde_set(identity, "revision", 0);

        let result = identity_validator.validate_identity(&identity).unwrap();

        assert!(result.is_valid());
    }
}

#[test]
pub fn should_return_valid_result_if_a_raw_identity_is_valid() {
    let (identity, identity_validator) = setup_test();

    let result = identity_validator.validate_identity(&identity).unwrap();
    assert_json_schema_error(&result, 0);

    assert!(result.is_valid());
}
