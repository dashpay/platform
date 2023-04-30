use crate::consensus::ConsensusError;
use crate::identity::validation::PublicKeysValidator;
use crate::identity::validation::TPublicKeysValidator;
use crate::identity::{KeyID, KeyType, Purpose, SecurityLevel};
use crate::tests::fixtures::get_public_keys_validator;

use crate::consensus::basic::BasicError;
use crate::consensus::codes::ErrorWithCode;
use crate::{assert_basic_consensus_errors, NativeBlsModule};
use platform_value::BinaryData;
use platform_value::{platform_value, Value};

fn setup_test() -> (Vec<Value>, PublicKeysValidator<NativeBlsModule>) {
    (
        crate::tests::fixtures::identity_fixture_raw_object()
            .get_array("publicKeys")
            .unwrap(),
        get_public_keys_validator(),
    )
}

pub mod id {
    use crate::assert_basic_consensus_errors;
    use crate::consensus::basic::BasicError;
    use crate::errors::consensus::ConsensusError;
    use crate::identity::validation::TPublicKeysValidator;
    use crate::identity::KeyID;
    use crate::tests::identity::validation::public_keys_validator_spec::setup_test;
    use crate::tests::utils::platform_value_set_ref;

    #[test]
    pub fn should_be_present() {
        let (mut raw_public_keys, validator) = setup_test();
        raw_public_keys
            .get_mut(1)
            .unwrap()
            .remove_integer::<KeyID>("id")
            .unwrap();

        let result = validator.validate_keys(&raw_public_keys).unwrap();
        let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 1);
        let error = errors.get(0).unwrap();

        assert_eq!(error.instance_path(), "");
        assert_eq!(error.keyword(), "required");
        assert_eq!(error.property_name(), "id");
    }

    #[test]
    pub fn should_be_a_number() {
        let (mut raw_public_keys, validator) = setup_test();
        raw_public_keys
            .get_mut(1)
            .unwrap()
            .set_value("id", "string".into())
            .unwrap();

        let result = validator.validate_keys(&raw_public_keys).unwrap();
        let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 1);
        let error = errors.get(0).unwrap();

        assert_eq!(error.instance_path(), "/id");
        assert_eq!(error.keyword(), "type");
    }

    #[test]
    pub fn should_be_an_integer() {
        let (mut raw_public_keys, validator) = setup_test();
        platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "id", 1.1);

        let result = validator.validate_keys(&raw_public_keys).unwrap();
        let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 1);
        let error = errors.get(0).unwrap();

        assert_eq!(error.instance_path(), "/id");
        assert_eq!(error.keyword(), "type");
    }

    #[test]
    pub fn should_be_greater_or_equal_to_zero() {
        let (mut raw_public_keys, validator) = setup_test();
        platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "id", -1);

        let result = validator.validate_keys(&raw_public_keys).unwrap();
        let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 1);
        let error = errors.get(0).unwrap();

        assert_eq!(error.instance_path(), "/id");
        assert_eq!(error.keyword(), "minimum");
    }
}

pub mod key_type {
    use crate::assert_basic_consensus_errors;
    use crate::consensus::basic::BasicError;
    use crate::errors::consensus::ConsensusError;
    use crate::identity::validation::TPublicKeysValidator;
    use crate::tests::identity::validation::public_keys_validator_spec::setup_test;
    use crate::tests::utils::platform_value_set_ref;

    #[test]
    pub fn should_be_present() {
        let (mut raw_public_keys, validator) = setup_test();
        raw_public_keys.get_mut(1).unwrap().remove("type").unwrap();

        let result = validator.validate_keys(&raw_public_keys).unwrap();
        // TODO: in the original code, there was only one error
        let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 4);
        let error = errors.get(0).unwrap();

        assert_eq!(error.instance_path(), "/data");
        assert_eq!(error.keyword(), "minItems");
    }

    #[test]
    pub fn should_be_a_number() {
        let (mut raw_public_keys, validator) = setup_test();
        platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "type", "string");

        let result = validator.validate_keys(&raw_public_keys).unwrap();
        // TODO: in the original code, there was only one error
        let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 2);
        let error = errors.first().unwrap();

        assert_eq!(error.instance_path(), "/type");
        assert_eq!(error.keyword(), "type");
    }
}

pub mod data {
    use crate::assert_basic_consensus_errors;
    use crate::consensus::basic::BasicError;
    use crate::errors::consensus::ConsensusError;
    use crate::identity::validation::TPublicKeysValidator;
    use crate::tests::identity::validation::public_keys_validator_spec::setup_test;

    #[test]
    pub fn should_be_present() {
        let (mut raw_public_keys, validator) = setup_test();
        raw_public_keys.get_mut(1).unwrap().remove("data").unwrap();

        let result = validator.validate_keys(&raw_public_keys).unwrap();
        let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 1);
        let error = errors.get(0).unwrap();

        assert_eq!(error.instance_path().to_string(), "");
        assert_eq!(error.keyword(), "required");
        assert_eq!(error.property_name(), "data");
    }

    #[test]
    pub fn should_be_a_byte_array() {
        let (mut raw_public_keys, validator) = setup_test();
        raw_public_keys
            .get_mut(1)
            .unwrap()
            .set_into_value("data", vec!["string"; 33])
            .unwrap();

        let result = validator.validate_keys(&raw_public_keys).unwrap();

        let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 33);

        // let byte_array_error = errors.last().unwrap();

        for (i, err) in errors.iter().enumerate() {
            assert_eq!(err.instance_path().to_string(), format!("/data/{}", i));
            assert_eq!(err.keyword(), "type");
        }

        // TODO: do we need to bring that back?
        //assert_eq!(byte_array_error.keyword(), "byteArray");
    }

    pub mod ecdsa_secp256k1 {
        use crate::assert_basic_consensus_errors;
        use crate::consensus::basic::BasicError;
        use crate::errors::consensus::ConsensusError;
        use crate::identity::validation::TPublicKeysValidator;
        use crate::tests::identity::validation::public_keys_validator_spec::setup_test;
        use crate::tests::utils::platform_value_set_ref;

        #[test]
        pub fn should_be_no_less_than_33_bytes() {
            let (mut raw_public_keys, validator) = setup_test();
            platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 32]);

            let result = validator.validate_keys(&raw_public_keys).unwrap();
            let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 1);
            let error = errors.get(0).unwrap();

            assert_eq!(error.instance_path().to_string(), "/data");
            assert_eq!(error.keyword(), "minItems");
        }

        #[test]
        pub fn should_be_no_longer_than_33_bytes() {
            let (mut raw_public_keys, validator) = setup_test();
            platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 34]);

            let result = validator.validate_keys(&raw_public_keys).unwrap();
            let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 1);
            let error = errors.get(0).unwrap();

            assert_eq!(error.instance_path().to_string(), "/data");
            assert_eq!(error.keyword(), "maxItems");
        }
    }

    pub mod bls12_381 {
        use crate::assert_basic_consensus_errors;
        use crate::consensus::basic::BasicError;
        use crate::errors::consensus::ConsensusError;
        use crate::identity::validation::TPublicKeysValidator;
        use crate::tests::identity::validation::public_keys_validator_spec::setup_test;
        use crate::tests::utils::platform_value_set_ref;

        #[test]
        pub fn should_be_no_less_than_48_bytes() {
            let (mut raw_public_keys, validator) = setup_test();
            platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 47]);
            platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "type", 1);

            let result = validator.validate_keys(&raw_public_keys).unwrap();
            let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 1);
            let error = errors.get(0).unwrap();

            assert_eq!(error.instance_path().to_string(), "/data");
            assert_eq!(error.keyword(), "minItems");
        }

        #[test]
        pub fn should_be_no_longer_than_48_bytes() {
            let (mut raw_public_keys, validator) = setup_test();
            platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 49]);
            platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "type", 1);

            let result = validator.validate_keys(&raw_public_keys).unwrap();
            let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 1);
            let error = errors.get(0).unwrap();

            assert_eq!(error.instance_path().to_string(), "/data");
            assert_eq!(error.keyword(), "maxItems");
        }
    }

    pub mod bip13_hash_script {
        use crate::assert_basic_consensus_errors;
        use crate::consensus::basic::BasicError;
        use crate::errors::consensus::ConsensusError;
        use crate::identity::validation::TPublicKeysValidator;
        use crate::tests::identity::validation::public_keys_validator_spec::setup_test;
        use crate::tests::utils::platform_value_set_ref;

        #[test]
        pub fn should_be_no_less_than_20_bytes() {
            let (mut raw_public_keys, validator) = setup_test();
            platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 19]);
            platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "type", 3);

            let result = validator.validate_keys(&raw_public_keys).unwrap();
            let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 1);
            let error = errors.get(0).unwrap();

            assert_eq!(error.instance_path().to_string(), "/data");
            assert_eq!(error.keyword(), "minItems");
        }

        #[test]
        pub fn should_be_no_longer_than_20_bytes() {
            let (mut raw_public_keys, validator) = setup_test();
            platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 21]);
            platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "type", 3);

            let result = validator.validate_keys(&raw_public_keys).unwrap();
            let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 1);
            let error = errors.get(0).unwrap();

            assert_eq!(error.instance_path().to_string(), "/data");
            assert_eq!(error.keyword(), "maxItems");
        }
    }

    pub mod ecdsa_hash_160 {
        use crate::assert_basic_consensus_errors;
        use crate::consensus::basic::BasicError;
        use crate::errors::consensus::ConsensusError;
        use crate::identity::validation::TPublicKeysValidator;
        use crate::tests::identity::validation::public_keys_validator_spec::setup_test;
        use crate::tests::utils::platform_value_set_ref;

        #[test]
        pub fn should_be_no_less_than_20_bytes() {
            let (mut raw_public_keys, validator) = setup_test();
            platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 19]);
            platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "type", 2);

            let result = validator.validate_keys(&raw_public_keys).unwrap();
            let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 1);
            let error = errors.get(0).unwrap();

            assert_eq!(error.instance_path().to_string(), "/data");
            assert_eq!(error.keyword(), "minItems");
        }

        #[test]
        pub fn should_be_no_longer_than_20_bytes() {
            let (mut raw_public_keys, validator) = setup_test();
            platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 21]);
            platform_value_set_ref(raw_public_keys.get_mut(1).unwrap(), "type", 2);

            let result = validator.validate_keys(&raw_public_keys).unwrap();
            let errors = assert_basic_consensus_errors!(&result, BasicError::JsonSchemaError, 1);
            let error = errors.get(0).unwrap();

            assert_eq!(error.instance_path().to_string(), "/data");
            assert_eq!(error.keyword(), "maxItems");
        }
    }
}

#[test]
pub fn should_return_invalid_result_if_there_are_duplicate_key_ids() {
    let (mut raw_public_keys, validator) = setup_test();
    let key0 = raw_public_keys.get(0).unwrap().clone();
    let key1 = raw_public_keys.get_mut(1).unwrap();
    key1.set_value("id", key0.get_value("id").unwrap().clone())
        .unwrap();

    let result = validator.validate_keys(&raw_public_keys).unwrap();

    let errors = assert_basic_consensus_errors!(
        result,
        BasicError::DuplicatedIdentityPublicKeyIdBasicError,
        1
    );
    let consensus_error = result.errors.first().unwrap();
    let error = errors.first().unwrap();

    let expected_ids = vec![raw_public_keys
        .get(1)
        .unwrap()
        .get_integer::<u32>("id")
        .unwrap() as KeyID];

    assert_eq!(consensus_error.code(), 1030);
    assert_eq!(error.duplicated_ids(), &expected_ids);
}

#[test]
pub fn should_return_invalid_result_if_there_are_duplicate_keys() {
    let (mut raw_public_keys, validator) = setup_test();
    let key0 = raw_public_keys.get(0).unwrap().clone();
    let key1 = raw_public_keys.get_mut(1).unwrap();
    key1.set_value("data", key0.get_value("data").unwrap().clone())
        .expect("expected to set data");

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_basic_consensus_errors!(
        &result,
        BasicError::DuplicatedIdentityPublicKeyBasicError,
        1
    );

    let consensus_error = result.errors.first().unwrap();
    let error = errors.get(0).unwrap();

    let expected_ids = vec![raw_public_keys
        .get(1)
        .unwrap()
        .get_integer::<KeyID>("id")
        .unwrap()];

    assert_eq!(consensus_error.code(), 1029);
    assert_eq!(error.duplicated_public_keys_ids(), &expected_ids);
}

#[test]
pub fn should_return_invalid_result_if_key_data_is_not_a_valid_der() {
    let (mut raw_public_keys, validator) = setup_test();
    raw_public_keys
        .get_mut(1)
        .unwrap()
        .set_into_binary_data("data", vec![0; 33])
        .expect("expected to set data");

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors =
        assert_basic_consensus_errors!(&result, BasicError::InvalidIdentityPublicKeyDataError, 1);

    let consensus_error = result.errors.first().unwrap();
    let error = errors.first().unwrap();

    assert_eq!(consensus_error.code(), 1040);
    assert_eq!(
        error.public_key_id(),
        raw_public_keys[1].get_integer::<KeyID>("id").unwrap()
    );
    assert_eq!(
        error.validation_error(),
        "Key secp256k1 error: malformed public key"
    );
}

#[test]
pub fn should_return_invalid_result_if_key_has_an_invalid_combination_of_purpose_and_security_level(
) {
    let (mut raw_public_keys, validator) = setup_test();

    raw_public_keys
        .get_mut(1)
        .unwrap()
        .set_into_value("purpose", Purpose::ENCRYPTION as u8)
        .unwrap();
    raw_public_keys
        .get_mut(1)
        .unwrap()
        .set_into_value("securityLevel", SecurityLevel::MASTER as u8)
        .unwrap();

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_basic_consensus_errors!(
        &result,
        BasicError::InvalidIdentityPublicKeySecurityLevelError,
        1
    );
    let consensus_error = result.errors.first().unwrap();
    let error = errors.first().unwrap();

    assert_eq!(consensus_error.code(), 1047);
    assert_eq!(
        error.public_key_id(),
        raw_public_keys[1].get_integer::<u32>("id").unwrap()
    );
    assert_eq!(
        error.security_level() as u8,
        raw_public_keys[1]
            .get_integer::<u8>("securityLevel")
            .unwrap()
    );
    assert_eq!(
        error.purpose() as u8,
        raw_public_keys[1].get_integer::<u8>("purpose").unwrap()
    );
}

#[test]
pub fn should_pass_valid_public_keys() {
    let (raw_public_keys, validator) = setup_test();
    let result = validator.validate_keys(&raw_public_keys).unwrap();

    assert!(result.is_valid());
}

#[ignore]
#[test]
pub fn should_pass_valid_bls12_381_public_key() {
    //TODO: this test is broken due to the legacy key format that is used in the test.
    // needs reevaluation once v19 is released.

    let (_, validator) = setup_test();
    let raw_public_keys_json = platform_value!([{
        "id": 0u32,
        "type": KeyType::BLS12_381 as u8,
        "purpose": 0u8,
        "securityLevel": 0u8,
        "readOnly": true,
        "data": hex::decode("01fac99ca2c8f39c286717c213e190aba4b7af76db320ec43f479b7d9a2012313a0ae59ca576edf801444bc694686694").unwrap(),
    }]);
    let raw_public_keys = raw_public_keys_json.as_array().unwrap();
    let result = validator.validate_keys(raw_public_keys).unwrap();

    for err in &result.errors {
        println!("{:?}", err);
    }

    assert!(result.is_valid());
}

#[test]
pub fn should_pass_valid_ecdsa_hash160_public_key() {
    let (_, validator) = setup_test();
    let raw_public_keys_json = platform_value!([{
        "id": 0u32,
        "type": KeyType::ECDSA_HASH160 as u8,
        "purpose": 0u8,
        "securityLevel": 0u8,
        "readOnly": true,
        "data": BinaryData::new(hex::decode("6086389d3fa4773aa950b8de18c5bd6d8f2b73bc").unwrap()),
    }]);
    let raw_public_keys = raw_public_keys_json.as_array().unwrap();

    let result = validator.validate_keys(raw_public_keys).unwrap();

    assert!(result.is_valid());
}

#[test]
pub fn should_return_invalid_result_if_bls12_381_public_key_is_invalid() {
    let (_, validator) = setup_test();
    let raw_public_keys_json = platform_value!([{
        "id": 0u32,
        "type": KeyType::BLS12_381 as u8,
        "purpose": 0u8,
        "securityLevel": 0u8,
        "readOnly": true,
        "data": BinaryData::new(hex::decode("11fac99ca2c8f39c286717c213e190aba4b7af76db320ec43f479b7d9a2012313a0ae59ca576edf801444bc694686694").unwrap()),
    }]);
    let raw_public_keys = raw_public_keys_json.as_array().unwrap();

    let result = validator.validate_keys(raw_public_keys).unwrap();

    let errors =
        assert_basic_consensus_errors!(&result, BasicError::InvalidIdentityPublicKeyDataError, 1);
    let consensus_error = result.errors.first().unwrap();
    let error = errors.first().unwrap();

    assert_eq!(consensus_error.code(), 1040);
    assert_eq!(
        error.public_key_id(),
        raw_public_keys
            .get(0)
            .unwrap()
            .get_integer::<KeyID>("id")
            .unwrap()
    );
    // TODO
    //assert_eq!(error.validation_error(), TypeError);
    assert_eq!(
        error.validation_error().to_string(),
        "Given G1 non-infinity element must start with 0b10"
    );
}
