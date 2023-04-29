use platform_value::Value;
use std::sync::Arc;

use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::identity::validation::{IdentityValidator, PublicKeysValidator, PUBLIC_KEY_SCHEMA};
use crate::version::ProtocolVersionValidator;
use crate::{assert_basic_consensus_errors, NativeBlsModule};

fn setup_test() -> (
    Value,
    IdentityValidator<PublicKeysValidator<NativeBlsModule>>,
) {
    let protocol_version_validator = ProtocolVersionValidator::default();
    let public_keys_validator =
        PublicKeysValidator::new_with_schema(PUBLIC_KEY_SCHEMA.clone(), NativeBlsModule::default())
            .unwrap();
    (
        crate::tests::fixtures::identity_fixture_raw_object(),
        IdentityValidator::new(
            Arc::new(protocol_version_validator),
            Arc::new(public_keys_validator),
        )
        .unwrap(),
    )
}

pub mod protocol_version {

    use crate::assert_basic_consensus_errors;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::tests::identity::validation::identity_validator_spec::setup_test;

    #[test]
    pub fn should_be_present() {
        let (mut identity, identity_validator) = setup_test();
        identity
            .remove("protocolVersion")
            .expect("expected to remove protocol version");

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword(), "required");
        assert_eq!(error.instance_path(), "");
        assert_eq!(error.property_name(), "protocolVersion");
    }

    #[test]
    pub fn should_be_an_integer() {
        let (mut identity, identity_validator) = setup_test();
        identity.set_into_value("protocolVersion", "1").unwrap();

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword(), "type");
        assert_eq!(error.instance_path(), "/protocolVersion");
    }

    #[test]
    pub fn should_be_valid() {
        let (mut identity, identity_validator) = setup_test();
        identity.set_into_value("protocolVersion", -1i32).unwrap();

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword(), "minimum");
        assert_eq!(error.instance_path(), "/protocolVersion");
    }
}

pub mod id {
    use super::*;

    use platform_value::Value;

    use crate::consensus::ConsensusError;
    use crate::tests::identity::validation::identity_validator_spec::setup_test;

    #[test]
    pub fn should_be_present() {
        let (mut identity, identity_validator) = setup_test();
        identity.remove("id").expect("expected to remove id");

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword(), "required");
        assert_eq!(error.instance_path(), "");
        assert_eq!(error.property_name(), "id");
    }

    #[test]
    pub fn should_be_a_byte_array() {
        let (mut identity, identity_validator) = setup_test();
        identity
            .set_into_value("id", vec![Value::from("string"); 32])
            .unwrap();

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();
        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 32);

        for (i, err) in errors.iter().enumerate() {
            assert_eq!(err.instance_path(), format!("/id/{}", i));
            assert_eq!(err.keyword(), "type");
        }
    }

    #[test]
    pub fn should_not_be_less_than_32_bytes() {
        let (mut identity, identity_validator) = setup_test();
        identity
            .set_into_value("id", vec![Value::from(15); 31])
            .unwrap();

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword(), "minItems");
        assert_eq!(error.instance_path(), "/id");
    }

    #[test]
    pub fn should_not_be_more_than_32_bytes() {
        let (mut identity, identity_validator) = setup_test();
        identity
            .set_into_value("id", vec![Value::from(15); 33])
            .unwrap();

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword(), "maxItems");
        assert_eq!(error.instance_path(), "/id");
    }
}

pub mod balance {
    use super::*;

    use crate::errors::consensus::ConsensusError;
    use crate::tests::identity::validation::identity_validator_spec::setup_test;

    #[test]
    pub fn should_be_present() {
        let (mut identity, identity_validator) = setup_test();
        identity
            .remove("balance")
            .expect("expected to remove balance");

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword(), "required");
        assert_eq!(error.instance_path(), "");
        assert_eq!(error.property_name(), "balance");
    }

    #[test]
    pub fn should_be_an_integer() {
        let (mut identity, identity_validator) = setup_test();
        identity.set_into_value("balance", 1.2).unwrap();

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword(), "type");
        assert_eq!(error.instance_path(), "/balance");
    }

    #[test]
    pub fn should_be_greater_or_equal_0() {
        let (mut identity, identity_validator) = setup_test();
        identity.set_into_value("balance", -1i64).unwrap();

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword(), "minimum");
        assert_eq!(error.instance_path(), "/balance");

        identity.set_into_value("balance", 0u64).unwrap();
        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        assert!(result.is_valid());
    }
}

pub mod public_keys {
    use super::*;

    use crate::errors::consensus::ConsensusError;
    use crate::tests::identity::validation::identity_validator_spec::setup_test;
    use platform_value::Value;

    #[test]
    pub fn should_be_present() {
        let (mut identity, identity_validator) = setup_test();
        identity
            .remove("publicKeys")
            .expect("expected to remove public keys");

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword(), "required");
        assert_eq!(error.instance_path(), "");
        assert_eq!(error.property_name(), "publicKeys");
    }

    #[test]
    pub fn should_be_an_array() {
        let (mut identity, identity_validator) = setup_test();
        identity.set_into_value("publicKeys", 1u64).unwrap();

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword(), "type");
        assert_eq!(error.instance_path(), "/publicKeys");
    }

    #[test]
    pub fn should_not_be_empty() {
        let (mut identity, identity_validator) = setup_test();
        identity
            .set_into_value("publicKeys", Value::Array(vec![]))
            .unwrap();

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword(), "minItems");
        assert_eq!(error.instance_path().to_string(), "/publicKeys");
    }

    #[test]
    pub fn should_be_unique() {
        let (mut identity, identity_validator) = setup_test();

        let public_key = identity
            .get_array_slice("publicKeys")
            .unwrap()
            .get(0)
            .unwrap()
            .clone();

        identity
            .set_into_value(
                "publicKeys",
                Value::Array(vec![public_key.clone(), public_key]),
            )
            .unwrap();

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword(), "uniqueItems");
        assert_eq!(error.instance_path().to_string(), "/publicKeys");
    }

    #[test]
    pub fn should_throw_an_error_if_public_keys_have_more_than_100_keys() {
        let (mut identity, identity_validator) = setup_test();

        let public_key = identity
            .get_array_slice("publicKeys")
            .unwrap()
            .get(0)
            .unwrap()
            .clone();

        identity
            .set_into_value("publicKeys", Value::Array(vec![public_key; 101]))
            .unwrap();

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 2);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword(), "maxItems");
        assert_eq!(error.instance_path().to_string(), "/publicKeys");
    }
}

pub mod revision {
    use super::*;

    use crate::assert_basic_consensus_errors;
    use crate::errors::consensus::ConsensusError;
    use crate::tests::identity::validation::identity_validator_spec::setup_test;

    // revision tests
    #[test]
    pub fn should_be_present() {
        let (mut identity, identity_validator) = setup_test();
        identity
            .remove("protocolVersion")
            .expect("expected to remove revision");

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
        let error = errors.first().unwrap();

        assert_eq!(error.keyword(), "required");
        assert_eq!(error.instance_path().to_string(), "");
        assert_eq!(error.property_name(), "protocolVersion");
    }

    #[test]
    pub fn should_be_an_integer() {
        let (mut identity, identity_validator) = setup_test();

        identity.set_into_value("revision", 1.2).unwrap();

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();
        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

        let error = errors
            .first()
            .expect("Expected to be at least one validation error");

        assert_eq!(error.keyword(), "type");
        assert_eq!(error.instance_path().to_string(), "/revision");
    }

    #[test]
    pub fn should_should_be_greater_or_equal_0() {
        let (mut identity, identity_validator) = setup_test();

        identity.set_into_value("revision", -1i32).unwrap();

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();
        let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

        let error = errors
            .first()
            .expect("Expected to be at least one validation error");

        assert_eq!(error.keyword(), "minimum");
        assert_eq!(error.instance_path().to_string(), "/revision");

        identity.set_into_value("revision", 0).unwrap();

        let result = identity_validator
            .validate_identity_object(&identity)
            .unwrap();

        assert!(result.is_valid());
    }
}

#[test]
pub fn should_return_valid_result_if_a_raw_identity_is_valid() {
    let (identity, identity_validator) = setup_test();

    let result = identity_validator
        .validate_identity_object(&identity)
        .unwrap();
    assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 0);

    assert!(result.is_valid());
}
