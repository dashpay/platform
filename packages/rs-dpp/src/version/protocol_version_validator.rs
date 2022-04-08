use crate::errors::consensus::basic::{IncompatibleProtocolVersionError, UnsupportedProtocolVersionError};
use crate::validation::ValidationResult;
use super::{LATEST_VERSION, COMPATIBILITY_MAP};
use std::cmp;

#[derive(Clone)]
pub struct ProtocolVersionValidator {
    current_protocol_version: u32
}

impl ProtocolVersionValidator {
    pub fn validate(&self, protocol_version: u32) -> ValidationResult {
        let mut result = ValidationResult::new(None);

        // Parsed protocol version must be equal or lower than latest protocol version
        if protocol_version > LATEST_VERSION {
            result.add_error(
                UnsupportedProtocolVersionError::new(
                    protocol_version,
                    LATEST_VERSION,
                ),
            );

            return result;
        }

        // The highest version should be used for the compatibility map
        // to get minimal compatible version
        let max_protocol_version = cmp::max(protocol_version, self.protocol_version());

        // The lowest version should be used to compare with the minimal compatible version
        let min_protocol_version = cmp::min(protocol_version, self.protocol_version());

        if let Some(minimal_compatible_protocol_version) = COMPATIBILITY_MAP.get(&max_protocol_version) {
            let minimal_compatible_protocol_version = *minimal_compatible_protocol_version;
            // Parsed protocol version (or current network protocol version) must higher
            // or equal to the minimum compatible version
            if min_protocol_version < minimal_compatible_protocol_version {
                result.add_error(
                    IncompatibleProtocolVersionError::new(
                        protocol_version,
                        minimal_compatible_protocol_version,
                    ),
                );

                return result;
            }
        } else {
            // TODO: In the original js code it was a throw. Figure out what to do with that
            //  result.add_error(CompatibleProtocolVersionIsNotDefinedError::new(max_protocol_version));
            //  return result;
        }

        return result;
    }

    pub fn protocol_version(&self) -> u32 {
        self.current_protocol_version
    }
}