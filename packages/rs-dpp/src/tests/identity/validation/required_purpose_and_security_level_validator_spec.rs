use crate::consensus::basic::BasicError;
use crate::consensus::codes::ErrorWithCode;
use crate::consensus::ConsensusError;
use crate::identity::{
    validation::{RequiredPurposeAndSecurityLevelValidator, TPublicKeysValidator},
    KeyType, Purpose, SecurityLevel,
};
use platform_value::platform_value;
use platform_value::string_encoding::{decode, Encoding};
use platform_value::BinaryData;

#[test]
fn should_return_invalid_result_if_state_transition_does_not_contain_master_key() {
    let validator = RequiredPurposeAndSecurityLevelValidator {};
    let raw_public_keys = vec![
        platform_value!({
                    "id": 0u32,
                    "type" : KeyType::ECDSA_SECP256K1 as u8,
                    "purpose" : Purpose::AUTHENTICATION as u8,
                    "securityLevel"  : SecurityLevel::CRITICAL as u8,
                    "data": BinaryData::new(decode("AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di", Encoding::Base64).unwrap()),
                    "readOnly" : false,
        }),
        // this key must be filtered out
        platform_value!({
                    "id": 0u32,
                    "type" : KeyType::ECDSA_SECP256K1 as u8,
                    "purpose": Purpose::AUTHENTICATION as u8,
                    "securityLevel" : SecurityLevel::CRITICAL as u8,
                    "disabledAt" : 42,
                    "data": BinaryData::new(decode("AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di", Encoding::Base64).unwrap()),
                    "readOnly" : false,
        }),
    ];

    let result = validator
        .validate_keys(raw_public_keys.as_slice())
        .expect("validation result should be returned");

    assert!(matches!(
        result.errors[0],
        ConsensusError::BasicError(BasicError::MissingMasterPublicKeyError(..))
    ));
    assert_eq!(1046, result.errors[0].code())
}

#[test]
fn should_return_valid_result() {
    let validator = RequiredPurposeAndSecurityLevelValidator {};
    let raw_public_keys = vec![
        platform_value!({
                    "id": 0u32,
                    "type" : KeyType::ECDSA_SECP256K1 as u8,
                    "purpose" : Purpose::AUTHENTICATION as u8,
                    "securityLevel"  : SecurityLevel::MASTER as u8,
                    "data": BinaryData::new(decode("AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di", Encoding::Base64).unwrap()),
                    "readOnly" : false,
        }),
        // this key must be filtered out
        platform_value!({
                    "id": 0u32,
                    "type" : KeyType::ECDSA_SECP256K1 as u8,
                    "purpose": Purpose::AUTHENTICATION as u8,
                    "securityLevel" : SecurityLevel::CRITICAL as u8,
                    "disabledAt" : 42u64,
                    "data": BinaryData::new(decode("AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di", Encoding::Base64).unwrap()),
                    "readOnly" : false,
        }),
    ];

    let result = validator
        .validate_keys(&raw_public_keys)
        .expect("validation result should be returned");

    assert!(result.is_valid());
}
