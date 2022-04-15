use crate::assert_consensus_errors;
use crate::consensus::basic::identity::InvalidIdentityPublicKeyDataError;
use crate::consensus::ConsensusError;
use crate::identity::validation::PublicKeysValidator;
use crate::identity::{KeyType, Purpose, SecurityLevel};
use crate::tests::utils::{
    assert_json_schema_error, decode_hex, serde_remove, serde_remove_ref, serde_set_ref,
};
use jsonschema::error::ValidationErrorKind;
use serde_json::{json, Value};

fn setup_test() -> (Vec<Value>, PublicKeysValidator) {
    (
        crate::tests::fixtures::identity_fixture_json()
            .as_object()
            .unwrap()
            .get("publicKeys")
            .unwrap()
            .clone()
            .as_array_mut()
            .unwrap()
            .clone(),
        PublicKeysValidator::new().unwrap(),
    )
}

pub mod id {
    use crate::assert_consensus_errors;
    use crate::errors::consensus::ConsensusError;
    use crate::tests::identity::validation::public_keys_validator_spec::setup_test;
    use crate::tests::utils::{assert_json_schema_error, serde_remove_ref, serde_set_ref};
    use jsonschema::error::ValidationErrorKind;

    #[test]
    pub fn should_be_present() {
        let (mut raw_public_keys, validator) = setup_test();
        serde_remove_ref(raw_public_keys.get_mut(1).unwrap(), "id");

        let result = validator.validate_keys(&raw_public_keys).unwrap();
        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
        let error = errors.get(0).unwrap();

        assert_eq!(error.instance_path().to_string(), "");
        assert_eq!(error.keyword().unwrap(), "required");

        match error.kind() {
            ValidationErrorKind::Required { property } => {
                assert_eq!(property.to_string(), "\"id\"");
            }
            _ => panic!("Expected to be missing property"),
        }
    }

    #[test]
    pub fn should_be_a_number() {
        let (mut raw_public_keys, validator) = setup_test();
        serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "id", "string");

        let result = validator.validate_keys(&raw_public_keys).unwrap();
        let errors = assert_json_schema_error(&result, 1);
        let error = errors.get(0).unwrap();

        assert_eq!(error.instance_path().to_string(), "/id");
        assert_eq!(error.keyword().unwrap(), "type");
    }

    #[test]
    pub fn should_be_an_integer() {
        let (mut raw_public_keys, validator) = setup_test();
        serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "id", 1.1);

        let result = validator.validate_keys(&raw_public_keys).unwrap();
        let errors = assert_json_schema_error(&result, 1);
        let error = errors.get(0).unwrap();

        assert_eq!(error.instance_path().to_string(), "/id");
        assert_eq!(error.keyword().unwrap(), "type");
    }

    #[test]
    pub fn should_be_greater_or_equal_to_zero() {
        let (mut raw_public_keys, validator) = setup_test();
        serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "id", -1);

        let result = validator.validate_keys(&raw_public_keys).unwrap();
        let errors = assert_json_schema_error(&result, 1);
        let error = errors.get(0).unwrap();

        assert_eq!(error.instance_path().to_string(), "/id");
        assert_eq!(error.keyword().unwrap(), "minimum");
    }
}

pub mod key_type {
    use crate::assert_consensus_errors;
    use crate::errors::consensus::ConsensusError;
    use crate::tests::identity::validation::public_keys_validator_spec::setup_test;
    use crate::tests::utils::{assert_json_schema_error, serde_remove_ref, serde_set_ref};

    #[test]
    pub fn should_be_present() {
        let (mut raw_public_keys, validator) = setup_test();
        serde_remove_ref(raw_public_keys.get_mut(1).unwrap(), "type");

        let result = validator.validate_keys(&raw_public_keys).unwrap();
        // TODO: in the original code, there was only one error
        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 3);
        let error = errors.get(0).unwrap();

        assert_eq!(error.instance_path().to_string(), "/data");
        assert_eq!(error.keyword().unwrap(), "minItems");
    }

    #[test]
    pub fn should_be_a_number() {
        let (mut raw_public_keys, validator) = setup_test();
        serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "type", "string");

        let result = validator.validate_keys(&raw_public_keys).unwrap();
        // TODO: in the original code, there was only one error
        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 2);
        let error = errors.first().unwrap();

        assert_eq!(error.instance_path().to_string(), "/type");
        assert_eq!(error.keyword().unwrap(), "type");
    }
}

pub mod data {
    use crate::assert_consensus_errors;
    use crate::errors::consensus::ConsensusError;
    use crate::tests::identity::validation::public_keys_validator_spec::setup_test;
    use crate::tests::utils::{assert_json_schema_error, serde_remove_ref, serde_set_ref};
    use jsonschema::error::ValidationErrorKind;

    #[test]
    pub fn should_be_present() {
        let (mut raw_public_keys, validator) = setup_test();
        serde_remove_ref(raw_public_keys.get_mut(1).unwrap(), "data");

        let result = validator.validate_keys(&raw_public_keys).unwrap();
        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 1);
        let error = errors.get(0).unwrap();

        assert_eq!(error.instance_path().to_string(), "");
        assert_eq!(error.keyword().unwrap(), "required");

        match error.kind() {
            ValidationErrorKind::Required { property } => {
                assert_eq!(property.to_string(), "\"data\"");
            }
            _ => panic!("Expected to be missing property"),
        }
    }

    #[test]
    pub fn should_be_a_byte_array() {
        let (mut raw_public_keys, validator) = setup_test();
        serde_set_ref(
            raw_public_keys.get_mut(1).unwrap(),
            "data",
            vec!["string"; 33],
        );

        let result = validator.validate_keys(&raw_public_keys).unwrap();

        let errors = assert_consensus_errors!(&result, ConsensusError::JsonSchemaError, 33);

        let error = errors.first().unwrap();
        let byte_array_error = errors.last().unwrap();

        for (i, err) in errors.iter().enumerate() {
            assert_eq!(err.instance_path().to_string(), format!("/data/{}", i));
            assert_eq!(err.keyword().unwrap(), "type");
        }

        // TODO: do we need to bring that back?
        //assert_eq!(byte_array_error.keyword().unwrap(), "byteArray");
    }

    pub mod ecdsa_secp256k1 {
        use crate::tests::identity::validation::public_keys_validator_spec::setup_test;
        use crate::tests::utils::{assert_json_schema_error, serde_set_ref};

        #[test]
        pub fn should_be_no_less_than_33_bytes() {
            let (mut raw_public_keys, validator) = setup_test();
            serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 32]);

            let result = validator.validate_keys(&raw_public_keys).unwrap();
            let errors = assert_json_schema_error(&result, 1);
            let error = errors.get(0).unwrap();

            assert_eq!(error.instance_path().to_string(), "/data");
            assert_eq!(error.keyword().unwrap(), "minItems");
        }

        #[test]
        pub fn should_be_no_longer_than_33_bytes() {
            let (mut raw_public_keys, validator) = setup_test();
            serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 34]);

            let result = validator.validate_keys(&raw_public_keys).unwrap();
            let errors = assert_json_schema_error(&result, 1);
            let error = errors.get(0).unwrap();

            assert_eq!(error.instance_path().to_string(), "/data");
            assert_eq!(error.keyword().unwrap(), "maxItems");
        }
    }

    pub mod bls12_381 {
        use crate::tests::identity::validation::public_keys_validator_spec::setup_test;
        use crate::tests::utils::{assert_json_schema_error, serde_set_ref};

        #[test]
        pub fn should_be_no_less_than_48_bytes() {
            let (mut raw_public_keys, validator) = setup_test();
            serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 47]);
            serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "type", 1);

            let result = validator.validate_keys(&raw_public_keys).unwrap();
            let errors = assert_json_schema_error(&result, 1);
            let error = errors.get(0).unwrap();

            assert_eq!(error.instance_path().to_string(), "/data");
            assert_eq!(error.keyword().unwrap(), "minItems");
        }

        #[test]
        pub fn should_be_no_longer_than_48_bytes() {
            let (mut raw_public_keys, validator) = setup_test();
            serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 49]);
            serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "type", 1);

            let result = validator.validate_keys(&raw_public_keys).unwrap();
            let errors = assert_json_schema_error(&result, 1);
            let error = errors.get(0).unwrap();

            assert_eq!(error.instance_path().to_string(), "/data");
            assert_eq!(error.keyword().unwrap(), "maxItems");
        }
    }
}

#[test]
pub fn should_return_invalid_result_if_there_are_duplicate_key_ids() {
    let (mut raw_public_keys, validator) = setup_test();
    let key0 = raw_public_keys.get(0).unwrap().clone();
    let mut key1 = raw_public_keys.get_mut(1).unwrap();
    serde_set_ref(
        &mut key1,
        "id",
        key0.as_object().unwrap().get("id").unwrap().clone(),
    );

    let result = validator.validate_keys(&raw_public_keys).unwrap();

    let errors = assert_consensus_errors!(
        result,
        ConsensusError::DuplicatedIdentityPublicKeyIdError,
        1
    );
    let consensus_error = result.errors().first().unwrap();
    let error = errors.first().unwrap();

    let expected_ids = vec![raw_public_keys
        .get(1)
        .unwrap()
        .as_object()
        .unwrap()
        .get("id")
        .unwrap()
        .as_u64()
        .unwrap()];

    assert_eq!(consensus_error.code(), 1030);
    assert_eq!(error.duplicated_ids(), &expected_ids);
}

#[test]
pub fn should_return_invalid_result_if_there_are_duplicate_keys() {
    let (mut raw_public_keys, validator) = setup_test();
    let key0 = raw_public_keys.get(0).unwrap().clone();
    let mut key1 = raw_public_keys.get_mut(1).unwrap();
    serde_set_ref(
        &mut key1,
        "data",
        key0.as_object().unwrap().get("data").unwrap().clone(),
    );

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors =
        assert_consensus_errors!(&result, ConsensusError::DuplicatedIdentityPublicKeyError, 1);

    let consensus_error = result.errors().first().unwrap();
    let error = errors.get(0).unwrap();

    let expected_ids = vec![raw_public_keys
        .get(1)
        .unwrap()
        .as_object()
        .unwrap()
        .get("id")
        .unwrap()
        .as_u64()
        .unwrap()];

    assert_eq!(consensus_error.code(), 1029);
    assert_eq!(error.duplicated_public_keys_ids(), &expected_ids);
}

#[test]
pub fn should_return_invalid_result_if_key_data_is_not_a_valid_der() {
    let (mut raw_public_keys, validator) = setup_test();
    serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 33]);

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_consensus_errors!(
        &result,
        ConsensusError::InvalidIdentityPublicKeyDataError,
        1
    );

    let consensus_error = result.errors().first().unwrap();
    let error = errors.first().unwrap();

    assert_eq!(consensus_error.code(), 1040);
    assert_eq!(
        error.public_key_id(),
        raw_public_keys[1].get("id").unwrap().as_u64().unwrap()
    );
    assert_eq!(
        error.validation_error().as_ref().unwrap().message(),
        "Invalid public key"
    );
}

#[test]
pub fn should_return_invalid_result_if_key_has_an_invalid_combination_of_purpose_and_security_level(
) {
    let (mut raw_public_keys, validator) = setup_test();
    serde_set_ref(
        raw_public_keys.get_mut(1).unwrap(),
        "purpose",
        Purpose::ENCRYPTION as u64,
    );
    serde_set_ref(
        raw_public_keys.get_mut(1).unwrap(),
        "securityLevel",
        SecurityLevel::MASTER as u64,
    );

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_consensus_errors!(
        &result,
        ConsensusError::InvalidIdentityPublicKeySecurityLevelError,
        1
    );
    let consensus_error = result.errors().first().unwrap();
    let error = errors.first().unwrap();

    assert_eq!(consensus_error.code(), 1047);
    assert_eq!(
        error.public_key_id(),
        raw_public_keys[1].get("id").unwrap().as_u64().unwrap()
    );
    assert_eq!(
        error.security_level() as u64,
        raw_public_keys[1]
            .get("securityLevel")
            .unwrap()
            .as_u64()
            .unwrap()
    );
    assert_eq!(
        error.purpose() as u64,
        raw_public_keys[1].get("purpose").unwrap().as_u64().unwrap()
    );
}

#[test]
pub fn should_pass_valid_public_keys() {
    let (raw_public_keys, validator) = setup_test();
    let result = validator.validate_keys(&raw_public_keys).unwrap();

    assert!(result.is_valid());
}

#[test]
pub fn should_pass_valid_bls12_381_public_key() {
    let (_, validator) = setup_test();
    let raw_public_keys_json = json!([{
        "id": 0,
        "type": KeyType::BLS12_381 as u64,
        "purpose": 0,
        "securityLevel": 0,
        "readOnly": true,
        "data": decode_hex("01fac99ca2c8f39c286717c213e190aba4b7af76db320ec43f479b7d9a2012313a0ae59ca576edf801444bc694686694").unwrap(),
    }]);
    let raw_public_keys = raw_public_keys_json.as_array().unwrap();
    let result = validator.validate_keys(raw_public_keys).unwrap();

    for err in result.errors() {
        println!("{:?}", err);
    }

    assert!(result.is_valid());
}

#[test]
pub fn should_pass_valid_ecdsa_hash160_public_key() {
    let (raw_public_keys, validator) = setup_test();
    let raw_public_keys_json = json!([{
        "id": 0,
        "type": KeyType::ECDSA_HASH160 as u64,
        "purpose": 0,
        "securityLevel": 0,
        "readOnly": true,
        "data": decode_hex("6086389d3fa4773aa950b8de18c5bd6d8f2b73bc").unwrap(),
    }]);
    let raw_public_keys = raw_public_keys_json.as_array().unwrap();

    let result = validator.validate_keys(raw_public_keys).unwrap();

    assert!(result.is_valid());
}

#[test]
pub fn should_return_invalid_result_if_bls12_381_public_key_is_invalid() {
    let (_, validator) = setup_test();
    let raw_public_keys_json = json!([{
        "id": 0,
        "type": KeyType::BLS12_381,
        "purpose": 0,
        "securityLevel": 0,
        "readOnly": true,
        "data": decode_hex("11fac99ca2c8f39c286717c213e190aba4b7af76db320ec43f479b7d9a2012313a0ae59ca576edf801444bc694686694").unwrap(),
    }]);
    let raw_public_keys = raw_public_keys_json.as_array().unwrap();

    let result = validator.validate_keys(&raw_public_keys).unwrap();

    let errors = assert_consensus_errors!(
        &result,
        ConsensusError::InvalidIdentityPublicKeyDataError,
        1
    );
    let consensus_error = result.errors().first().unwrap();
    let error = errors.first().unwrap();

    assert_eq!(consensus_error.code(), 1040);
    assert_eq!(
        error.public_key_id(),
        raw_public_keys
            .get(0)
            .unwrap()
            .as_object()
            .unwrap()
            .get("id")
            .unwrap()
            .as_u64()
            .unwrap()
    );
    // TODO
    //assert_eq!(error.validation_error(), TypeError);
    assert_eq!(
        error.validation_error().as_ref().unwrap().message(),
        "Group decode error"
    );
}
