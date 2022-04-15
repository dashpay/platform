use crate::errors::consensus::ConsensusError;
use crate::version::ProtocolVersionValidator;
use std::collections::HashMap;

pub fn setup_test() -> (u64, u64, HashMap<u64, u64>) {
    let current_protocol_version = 1;
    let latest_protocol_version = 1;
    let mut version_compatibility_map = HashMap::new();
    version_compatibility_map.insert(1, 1);
    (
        current_protocol_version,
        latest_protocol_version,
        version_compatibility_map,
    )
}

#[test]
pub fn should_throw_unsupported_protocol_version_error_if_protocol_version_is_higher_than_latest_version(
) {
    let (current_protocol_version, latest_protocol_version, version_compatibility_map) =
        setup_test();
    let validator = ProtocolVersionValidator::new(
        current_protocol_version,
        latest_protocol_version,
        version_compatibility_map,
    );

    let protocol_version = current_protocol_version + 1;

    let result = validator.validate(protocol_version).unwrap();
    let consensus_error = result.first_error().unwrap();

    let error = match consensus_error {
        ConsensusError::UnsupportedProtocolVersionError(err) => err,
        _ => {
            panic!("Expected UnsupportedProtocolVersionError")
        }
    };

    assert_eq!(error.parsed_protocol_version(), protocol_version);
    assert_eq!(error.latest_version(), current_protocol_version);
    assert_eq!(consensus_error.code(), 1002);
}

#[test]
pub fn should_throw_compatible_protocol_version_is_not_defined_error_if_compatible_version_is_not_defined_for_the_current_protocol_version(
) {
    let current_protocol_version = 2;
    let latest_protocol_version = 2;
    let mut version_compatibility_map = HashMap::new();
    version_compatibility_map.insert(1, 1);

    let validator = ProtocolVersionValidator::new(
        current_protocol_version,
        latest_protocol_version,
        version_compatibility_map,
    );
    let protocol_version = current_protocol_version;

    let err = validator
        .validate(protocol_version)
        .err()
        .expect("should return CompatibleProtocolVersionIsNotDefinedError");
    assert_eq!(err.current_protocol_version(), current_protocol_version)
}

#[test]
pub fn should_throw_incompatible_protocol_version_error_if_parsed_version_is_lower_than_compatible_one(
) {
    let minimal_protocol_version = 1;
    let protocol_version = 0;
    let current_protocol_version = 5;
    let mut version_compatibility_map = HashMap::new();
    version_compatibility_map.insert(1, 1);
    version_compatibility_map.insert(current_protocol_version, minimal_protocol_version);

    let validator = ProtocolVersionValidator::new(
        current_protocol_version,
        current_protocol_version,
        version_compatibility_map,
    );

    let result = validator.validate(protocol_version).unwrap();
    let consensus_error = result.first_error().unwrap();
    let error = match consensus_error {
        ConsensusError::IncompatibleProtocolVersionError(err) => err,
        _ => {
            panic!("Expected IncompatibleProtocolVersionError")
        }
    };

    assert_eq!(error.parsed_protocol_version(), protocol_version);
    assert_eq!(error.minimal_protocol_version(), minimal_protocol_version);
    assert_eq!(consensus_error.code(), 1003);
}
