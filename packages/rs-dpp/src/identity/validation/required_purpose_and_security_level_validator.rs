use platform_value::Value;
use std::collections::HashMap;

use crate::consensus::basic::identity::MissingMasterPublicKeyError;
use crate::identity::validation::TPublicKeysValidator;
use crate::identity::{IdentityPublicKey, Purpose, SecurityLevel};
use crate::validation::SimpleConsensusValidationResult;
use crate::{DashPlatformProtocolInitError, NonConsensusError};

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
    ) -> Result<SimpleConsensusValidationResult, NonConsensusError> {
        let mut result = SimpleConsensusValidationResult::default();

        let mut key_purposes_and_levels_count: HashMap<PurposeKey, usize> = HashMap::new();

        for raw_public_key in raw_public_keys
            .iter()
            .filter_map(|pk| {
                match pk
                    .get_optional_integer::<u64>("disabledAt")
                    .map_err(NonConsensusError::ValueError)
                {
                    Ok(Some(_)) => None,
                    Ok(None) => Some(Ok(pk)),
                    Err(e) => Some(Err(e)),
                }
            })
            .collect::<Result<Vec<_>, NonConsensusError>>()?
        {
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
