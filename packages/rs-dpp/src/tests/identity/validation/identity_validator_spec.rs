use std::sync::Arc;

use serde_json::Value;

use crate::assert_consensus_errors;
use crate::errors::consensus::ConsensusError;
use crate::identity::validation::{IdentityValidator, PublicKeysValidator};
use crate::version::ProtocolVersionValidator;

fn setup_test() -> (Value, IdentityValidator<PublicKeysValidator>) {
    let protocol_version_validator = ProtocolVersionValidator::default();
    let public_keys_validator = PublicKeysValidator::new().unwrap();
    (
        crate::tests::fixtures::identity_fixture_json(),
        IdentityValidator::new(
            Arc::new(protocol_version_validator),
            Arc::new(public_keys_validator),
        )
        .unwrap(),
    )
}

pub mod protocol_version {
    use jsonschema::error::ValidationErrorKind;

    use crate::assert_consensus_errors;
    use crate::consensus::ConsensusError;
    use crate::tests::identity::validation::identity_validator_spec::setup_test;
    use crate::tests::utils::{serde_remove, serde_set};

    #[test]
    pub fn should_be_present() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_remove(identity, "protocolVersion");

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
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

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "type");
        assert_eq!(error.instance_path().to_string(), "/protocolVersion");
    }

    #[test]
    pub fn should_be_valid() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_set(identity, "protocolVersion", -1);

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "minimum");
        assert_eq!(error.instance_path().to_string(), "/protocolVersion");
    }
}

pub mod id {
    use jsonschema::error::ValidationErrorKind;
    use serde_json::Value;

    use crate::assert_consensus_errors;
    use crate::consensus::ConsensusError;
    use crate::tests::identity::validation::identity_validator_spec::setup_test;
    use crate::tests::utils::{serde_remove, serde_set};

    #[test]
    pub fn should_be_present() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_remove(identity, "id");

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
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
        let (mut identity, identity_validator) = setup_test();
        identity = serde_set(identity, "id", vec![Value::from("string"); 32]);

        let result = identity_validator.validate_identity(&identity).unwrap();
        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 32);

        for (i, err) in errors.iter().enumerate() {
            assert_eq!(err.instance_path().to_string(), format!("/id/{}", i));
            assert_eq!(err.keyword().unwrap(), "type");
        }
    }

    #[test]
    pub fn should_not_be_less_than_32_bytes() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_set(identity, "id", vec![Value::from(15); 31]);

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "minItems");
        assert_eq!(error.instance_path().to_string(), "/id");
    }

    #[test]
    pub fn should_not_be_more_than_32_bytes() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_set(identity, "id", vec![Value::from(15); 33]);

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "maxItems");
        assert_eq!(error.instance_path().to_string(), "/id");
    }
}

pub mod balance {
    use jsonschema::error::ValidationErrorKind;

    use crate::assert_consensus_errors;
    use crate::errors::consensus::ConsensusError;
    use crate::tests::identity::validation::identity_validator_spec::setup_test;
    use crate::tests::utils::{serde_remove, serde_set};

    #[test]
    pub fn should_be_present() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_remove(identity, "balance");

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
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

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "type");
        assert_eq!(error.instance_path().to_string(), "/balance");
    }

    #[test]
    pub fn should_be_greater_or_equal_0() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_set(identity, "balance", -1);

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "minimum");
        assert_eq!(error.instance_path().to_string(), "/balance");

        identity = serde_set(identity, "balance", 0);
        let result = identity_validator.validate_identity(&identity).unwrap();

        assert!(result.is_valid());
    }
}

pub mod public_keys {
    use jsonschema::error::ValidationErrorKind;
    use serde_json::Value;

    use crate::assert_consensus_errors;
    use crate::errors::consensus::ConsensusError;
    use crate::tests::identity::validation::identity_validator_spec::setup_test;
    use crate::tests::utils::{serde_remove, serde_set};

    #[test]
    pub fn should_be_present() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_remove(identity, "publicKeys");

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
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

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "type");
        assert_eq!(error.instance_path().to_string(), "/publicKeys");
    }

    #[test]
    pub fn should_not_be_empty() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_set(identity, "publicKeys", Value::Array(vec![]));

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
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

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
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

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 2);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword().unwrap(), "maxItems");
        assert_eq!(error.instance_path().to_string(), "/publicKeys");
    }
}

pub mod revision {
    use jsonschema::error::ValidationErrorKind;

    use crate::assert_consensus_errors;
    use crate::errors::consensus::ConsensusError;
    use crate::tests::identity::validation::identity_validator_spec::setup_test;
    use crate::tests::utils::{serde_remove, serde_set};

    // revision tests
    #[test]
    pub fn should_be_present() {
        let (mut identity, identity_validator) = setup_test();
        identity = serde_remove(identity, "revision");

        let result = identity_validator.validate_identity(&identity).unwrap();

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
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
        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);

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
        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);

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
    assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 0);

    assert!(result.is_valid());
}
