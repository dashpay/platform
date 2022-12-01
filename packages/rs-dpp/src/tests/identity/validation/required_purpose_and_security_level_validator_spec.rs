use crate::{
    identity::{
        validation::{RequiredPurposeAndSecurityLevelValidator, TPublicKeysValidator},
        KeyType, Purpose, SecurityLevel,
    },
    util::string_encoding::{decode, Encoding},
};
use serde_json::json;

#[test]
fn should_return_invalid_result_if_state_transition_does_not_contain_master_key() {
    let validator = RequiredPurposeAndSecurityLevelValidator {};
    let raw_public_keys = vec![
        json!({
                    "id": 0,
                    "type" : KeyType::ECDSA_SECP256K1,
                    "purpose" : Purpose::AUTHENTICATION,
                    "securityLevel"  : SecurityLevel::CRITICAL,
                    "data": decode("AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di", Encoding::Base64).unwrap(),
                    "readOnly" : false,
        }),
        // this key must be filtered out
        json!({
                    "id": 0,
                    "type" : KeyType::ECDSA_SECP256K1,
                    "purpose": Purpose::AUTHENTICATION,
                    "securityLevel" : SecurityLevel::CRITICAL,
                    "disabledAt" : 42,
                    "data": decode("AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di", Encoding::Base64).unwrap(),
                    "readOnly" : false,
        }),
    ];

    let result = validator
        .validate_keys(&raw_public_keys)
        .expect("validation result should be returned");

    assert!(matches!(
        result.errors()[0],
        crate::consensus::ConsensusError::MissingMasterPublicKeyError(..)
    ));
    assert_eq!(1046, result.errors()[0].code())
}

#[test]
fn should_return_valid_result() {
    let validator = RequiredPurposeAndSecurityLevelValidator {};
    let raw_public_keys = vec![
        json!({
                    "id": 0,
                    "type" : KeyType::ECDSA_SECP256K1,
                    "purpose" : Purpose::AUTHENTICATION,
                    "securityLevel"  : SecurityLevel::MASTER,
                    "data": decode("AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di", Encoding::Base64).unwrap(),
                    "readOnly" : false,
        }),
        // this key must be filtered out
        json!({
                    "id": 0,
                    "type" : KeyType::ECDSA_SECP256K1,
                    "purpose": Purpose::AUTHENTICATION,
                    "securityLevel" : SecurityLevel::CRITICAL,
                    "disabledAt" : 42,
                    "data": decode("AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di", Encoding::Base64).unwrap(),
                    "readOnly" : false,
        }),
    ];

    let result = validator
        .validate_keys(&raw_public_keys)
        .expect("validation result should be returned");

    assert!(result.is_valid());
}
