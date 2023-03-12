use platform_value::Value;
use platform_value::Value::Null;
use std::collections::HashMap;

use crate::consensus::basic::identity::MissingMasterPublicKeyError;
use crate::identity::validation::TPublicKeysValidator;
use crate::identity::{IdentityPublicKey, Purpose, SecurityLevel};
use crate::validation::ValidationResult;
use crate::{DashPlatformProtocolInitError, NonConsensusError, ProtocolError};

#[derive(Eq, Hash, PartialEq)]
struct PurposeKey {
    purpose: Purpose,
    security_level: SecurityLevel,
}

#[derive(Default, Clone, Debug)]
pub struct RequiredPurposeAndSecurityLevelValidator {}

impl TPublicKeysValidator for RequiredPurposeAndSecurityLevelValidator {
    fn validate_keys(
        &self,
        raw_public_keys: &[Value],
    ) -> Result<ValidationResult<()>, NonConsensusError> {
        let mut result = ValidationResult::default();

        let mut key_purposes_and_levels_count: HashMap<PurposeKey, usize> = HashMap::new();

        for raw_public_key in raw_public_keys.iter().filter(|pk| {
            if let Some(disabled_at) = pk
                .get_optional_bool("disabledAt")
                .map_err(ProtocolError::ValueError)?
            {
                disabled_at == false
            } else {
                true
            }
        }) {
            let public_key: IdentityPublicKey = platform_value::from_value(raw_public_key.clone())?;
            let combo = PurposeKey {
                purpose: public_key.purpose,
                security_level: public_key.security_level,
            };
            let count = key_purposes_and_levels_count
                .get(&combo)
                .unwrap_or(&0_usize);
            let count = count + 1;
            key_purposes_and_levels_count.insert(combo, count);
        }

        let master_key = PurposeKey {
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
        };
        if key_purposes_and_levels_count.get(&master_key).is_none() {
            result.add_error(MissingMasterPublicKeyError {});
        }

        Ok(result)
    }
}

impl RequiredPurposeAndSecurityLevelValidator {
    pub fn new() -> Result<Self, DashPlatformProtocolInitError> {
        Ok(Self::default())
    }
}
