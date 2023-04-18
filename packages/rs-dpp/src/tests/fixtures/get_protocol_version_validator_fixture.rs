use crate::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};

pub fn get_protocol_version_validator_fixture() -> ProtocolVersionValidator {
    ProtocolVersionValidator::new(LATEST_VERSION, LATEST_VERSION, COMPATIBILITY_MAP.clone())
}
