use crate::identity::validation::PublicKeysValidator;
use crate::identity::{KeyType, Purpose, SecurityLevel};
use crate::tests::utils::{assert_json_schema_error, assert_validation_error, decode_hex, serde_remove, serde_remove_ref, serde_set_ref};
use jsonschema::error::ValidationErrorKind;
use serde_json::{json, Value};
use crate::consensus::basic::identity::InvalidIdentityPublicKeyDataError;
use crate::consensus::ConsensusError;

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

#[test]
pub fn id_should_be_present() {
    let (mut raw_public_keys, validator) = setup_test();
    serde_remove_ref(raw_public_keys.get_mut(1).unwrap(), "id");

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_json_schema_error(&result, 1);
    let error = errors.get(0).unwrap();

    assert_eq!(error.instance_path(), "");
    assert_eq!(error.keyword(), "required");

    match error.kind() {
        ValidationErrorKind::Required { property } => {
            assert_eq!(property.to_string(), "\"id\"");
        }
        _ => panic!("Expected to be missing property"),
    }
}

#[test]
pub fn id_should_be_a_number() {
    let (mut raw_public_keys, validator) = setup_test();
    serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "id", "string");

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_json_schema_error(&result, 1);
    let error = errors.get(0).unwrap();

    assert_eq!(error.get_instance_path(), "/id");
    assert_eq!(error.keyword(), "type");
}

#[test]
pub fn id_should_be_an_integer() {
    let (mut raw_public_keys, validator) = setup_test();
    serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "id", 1.1);

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_json_schema_error(&result, 1);
    let error = errors.get(0).unwrap();

    assert_eq!(error.get_instance_path(), "/id");
    assert_eq!(error.keyword(), "type");
}

#[test]
pub fn id_should_be_greater_or_equal_to_zero() {
    let (mut raw_public_keys, validator) = setup_test();
    serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "id", -1);

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_json_schema_error(&result, 1);
    let error = errors.get(0).unwrap();

    assert_eq!(error.get_instance_path(), "/id");
    assert_eq!(error.keyword(), "minimum");
}

#[test]
pub fn type_should_be_present() {
    let (mut raw_public_keys, validator) = setup_test();
    serde_remove_ref(raw_public_keys.get_mut(1).unwrap(), "type");

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_json_schema_error(&result, 1);
    let error = errors.get(0).unwrap();

    assert_eq!(error.get_instance_path(), "/data");
    assert_eq!(error.keyword(), "minItems");
}

#[test]
pub fn type_should_be_a_number() {
    let (mut raw_public_keys, validator) = setup_test();
    serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "type", "string");

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_json_schema_error(&result, 1);
    let error = errors.get(0).unwrap();

    assert_eq!(error.get_instance_path(), "/type");
    assert_eq!(error.keyword(), "type");
}

// describe("data() {
#[test]
pub fn data_should_be_present() {
    let (mut raw_public_keys, validator) = setup_test();
    serde_remove_ref(raw_public_keys.get_mut(1).unwrap(), "data");

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_json_schema_error(&result, 1);
    let error = errors.get(0).unwrap();

    assert_eq!(error.get_instance_path(), "");
    assert_eq!(error.keyword(), "required");

    match error.kind() {
        ValidationErrorKind::Required { property } => {
            assert_eq!(property.to_string(), "\"data\"");
        }
        _ => panic!("Expected to be missing property"),
    }
}

#[test]
pub fn data_should_be_a_byte_array() {
    assert!(false);
    // rawPublicKeys[1].data = new Array(33).fill("string");
    //
    // let result = validator.validate(rawPublicKeys);
    //
    // assert_eq!JsonSchemaError(result, 2);
    //
    // let [error, byteArrayError] = result.errors();
    //
    // assert_eq!(error.get_instance_path(),"/data/0");
    // assert_eq!(error.keyword(),"type");
    //
    // assert_eq!(byteArrayError.keyword(), "byteArray");
}

// describe("ECDSA_SECP256K1() {
#[test]
pub fn ecdsa_secp256k1_should_be_no_less_than_33_bytes() {
    let (mut raw_public_keys, validator) = setup_test();
    serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 32]);

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_json_schema_error(&result, 1);
    let error = errors.get(0).unwrap();

    assert_eq!(error.get_instance_path(), "/data");
    assert_eq!(error.keyword(), "minItems");
}

#[test]
pub fn ecdsa_secp256k1_should_be_no_longer_than_33_bytes() {
    let (mut raw_public_keys, validator) = setup_test();
    serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 34]);

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_json_schema_error(&result, 1);
    let error = errors.get(0).unwrap();

    assert_eq!(error.get_instance_path(), "/data");
    assert_eq!(error.keyword(), "maxItems");
}

// describe("BLS12_381() {
#[test]
pub fn bls12_381_should_be_no_less_than_48_bytes() {
    let (mut raw_public_keys, validator) = setup_test();
    serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 47]);
    serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "type", 1);

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_json_schema_error(&result, 1);
    let error = errors.get(0).unwrap();

    assert_eq!(error.get_instance_path(), "/data");
    assert_eq!(error.keyword(), "minItems");
}

#[test]
pub fn bls12_381_should_be_no_longer_than_48_bytes() {
    let (mut raw_public_keys, validator) = setup_test();
    serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 49]);
    serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "type", 1);

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_json_schema_error(&result, 1);
    let error = errors.get(0).unwrap();

    assert_eq!(error.get_instance_path(), "/data");
    assert_eq!(error.keyword(), "maxItems");
}

#[test]
pub fn should_return_invalid_result_if_there_are_duplicate_key_ids() {
    let (mut raw_public_keys, validator) = setup_test();
    let mut key1 = raw_public_keys.get_mut(1).unwrap();
    let key0 = raw_public_keys.get(0).unwrap().clone();
    serde_set_ref(
        &mut key1,
        "id",
        key0.as_object().unwrap().get("id").unwrap().clone(),
    );

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_json_schema_error(&result, 1);
    let error = errors.get(0).unwrap();

    assert_eq!(error.code(), 1030);
    assert_eq!(error.duplicated_ids(), vec![raw_public_keys[1].id]);
}

#[test]
pub fn should_return_invalid_result_if_there_are_duplicate_keys() {
    let (mut raw_public_keys, validator) = setup_test();
    let mut key1 = raw_public_keys.get_mut(1).unwrap();
    let key0 = raw_public_keys.get(0).unwrap().clone();
    serde_set_ref(
        &mut key1,
        "data",
        key0.as_object().unwrap().get("data").unwrap().clone(),
    );

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_json_schema_error(&result, 1);
    let error = errors.get(0).unwrap();

    assert_eq!(error.code(), 1029);
    assert_eq!(
        error.duplicated_public_keys_ids(),
        vec![raw_public_keys[1].id]
    );
}

#[test]
pub fn should_return_invalid_result_if_key_data_is_not_a_valid_der() {
    let (mut raw_public_keys, validator) = setup_test();
    serde_set_ref(raw_public_keys.get_mut(1).unwrap(), "data", vec![0; 33]);

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    assert_eq!(result.errors().len(), 1);

    let consensus_error = result.errors().get(0).unwrap();

    match consensus_error {
        ConsensusError::InvalidIdentityPublicKeyDataError(error) => {
            assert_eq!(consensus_error.code(), 1040);
            assert_eq!(error.public_key_id(), raw_public_keys[1].get("id").unwrap());
            assert_eq!(
                error.validation_error().message,
                "Invalid DER format public key"
            );
        }
        _ => panic!("Expected error to be an InvalidIdentityPublicKeyDataError variant")
    }
}

#[test]
pub fn should_return_invalid_result_if_key_has_an_invalid_combination_of_purpose_and_security_level(
) {
    let (mut raw_public_keys, validator) = setup_test();
    serde_set_ref(
        raw_public_keys.get_mut(1).unwrap(),
        "purpose",
        Purpose::ENCRYPTION,
    );
    serde_set_ref(
        raw_public_keys.get_mut(1).unwrap(),
        "securityLevel",
        SecurityLevel::MASTER,
    );

    let result = validator.validate_keys(&raw_public_keys).unwrap();
    let errors = assert_json_schema_error(&result, 1);
    let error = errors.get(0).unwrap();

    assert_eq!(error.code(), 1047);
    assert_eq!(error.public_key_id(), raw_public_keys[1].get("id").unwrap());
    assert_eq!(
        error.getPublicKeySecurityLevel(),
        raw_public_keys[1].get("securityLevel").unwrap()
    );
    assert_eq!(
        error.getPublicKeyPurpose(),
        raw_public_keys[1].get("purpose").unwrap()
    );
}

#[test]
pub fn should_pass_valid_public_keys() {
    let (mut raw_public_keys, validator) = setup_test();
    let result = validator.validate(raw_public_keys);

    assert!(result.is_valid());
}

#[test]
pub fn should_pass_valid_bls12_381_public_key() {
    let (mut raw_public_keys, validator) = setup_test();
    let raw_public_keys = json!([{
        "id": 0,
        "type": KeyType::BLS12_381,
        "purpose": 0,
        "securityLevel": 0,
        "readOnly": true,
        "data": decode_hex("01fac99ca2c8f39c286717c213e190aba4b7af76db320ec43f479b7d9a2012313a0ae59ca576edf801444bc694686694"),
    }]);

    let result = validator.validate(raw_public_keys);

    assert!(result.is_valid());
}

#[test]
pub fn should_pass_valid_ecdsa_hash160_public_key() {
    let (mut raw_public_keys, validator) = setup_test();
    let raw_public_keys = json!([{
        "id": 0,
        "type": KeyType::ECDSA_HASH160,
        "purpose": 0,
        "securityLevel": 0,
        "readOnly": true,
        "data": decode_hex("6086389d3fa4773aa950b8de18c5bd6d8f2b73bc"),
    }]);

    let result = validator.validate(raw_public_keys);

    assert!(result.is_valid());
}

#[test]
pub fn should_return_invalid_result_if_bls12_381_public_key_is_invalid() {
    let (mut raw_public_keys, validator) = setup_test();
    let raw_public_keys = json!([{
        "id": 0,
        "type": KeyType::BLS12_381,
        "purpose": 0,
        "securityLevel": 0,
        "readOnly": true,
        "data": decode_hex("11fac99ca2c8f39c286717c213e190aba4b7af76db320ec43f479b7d9a2012313a0ae59ca576edf801444bc694686694"),
    }]);

    let result = validator.validate(&raw_public_keys).unwrap();

    //assert_eq!(result, InvalidIdentityPublicKeyDataError);

    let errors = result.errors();
    assert_eq!(errors.len(), 1);
    let error = errors.get(0).unwrap();

    assert_eq!(error.code(), 1040);
    assert_eq!(error.public_key_id(), raw_public_keys[0].id);
    //assert_eq!(error.validation_error(), TypeError);
    assert_eq!(error.validation_error().message, "Invalid public key");
}
