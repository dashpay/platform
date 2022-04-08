use crate::errors::consensus::basic::{IncompatibleProtocolVersionError, UnsupportedProtocolVersionError};
use crate::validation::ValidationResult;
use std::cmp;
use std::collections::HashMap;
use crate::errors::CompatibleProtocolVersionIsNotDefinedError;
use crate::version::{COMPATIBILITY_MAP, LATEST_VERSION};

#[derive(Clone)]
pub struct ProtocolVersionValidator {
    current_protocol_version: u32,
    latest_protocol_version: u32,
    compatibility_map: HashMap<u32, u32>
}

impl Default for ProtocolVersionValidator {
    fn default() -> Self {
        Self {
            current_protocol_version: LATEST_VERSION,
            latest_protocol_version: LATEST_VERSION,
            compatibility_map: COMPATIBILITY_MAP.clone()
        }
    }
}

impl ProtocolVersionValidator {
    pub fn new(current_protocol_version: u32, latest_protocol_version: u32, compatibility_map: HashMap<u32, u32>) -> Self {
        Self { current_protocol_version, latest_protocol_version, compatibility_map }
    }

    pub fn validate(&self, protocol_version: u32) -> Result<ValidationResult, CompatibleProtocolVersionIsNotDefinedError> {
        let mut result = ValidationResult::new(None);

        // Parsed protocol version must be equal or lower than latest protocol version
        if protocol_version > self.latest_protocol_version {
            result.add_error(
                UnsupportedProtocolVersionError::new(
                    protocol_version,
                    self.latest_protocol_version,
                ),
            );

            return Ok(result);
        }

        // The highest version should be used for the compatibility map
        // to get minimal compatible version
        let max_protocol_version = cmp::max(protocol_version, self.protocol_version());

        // The lowest version should be used to compare with the minimal compatible version
        let min_protocol_version = cmp::min(protocol_version, self.protocol_version());

        if let Some(minimal_compatible_protocol_version) = self.compatibility_map.get(&max_protocol_version) {
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

                return Ok(result);
            }
        } else {
            return Err(CompatibleProtocolVersionIsNotDefinedError::new(max_protocol_version));
        }

        return Ok(result);
    }

    pub fn protocol_version(&self) -> u32 {
        self.current_protocol_version
    }
}