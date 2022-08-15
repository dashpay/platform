use std::collections::HashMap;

use bls_signatures::{PublicKey as BlsPublicKey, Serialize};
use dashcore::PublicKey;
use serde_json::Value;

use crate::errors::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyError, DuplicatedIdentityPublicKeyIdError,
    InvalidIdentityPublicKeyDataError, InvalidIdentityPublicKeySecurityLevelError,
};
use crate::identity::{IdentityPublicKey, KeyType, ALLOWED_SECURITY_LEVELS};
use crate::validation::{JsonSchemaValidator, ValidationResult};
use crate::{DashPlatformProtocolInitError, NonConsensusError, PublicKeyValidationError};

pub trait TPublicKeysValidator {
    fn validate_keys(
        &self,
        raw_public_keys: &[Value],
    ) -> Result<ValidationResult<()>, NonConsensusError>;
}

pub struct PublicKeysValidator {
    public_key_schema_validator: JsonSchemaValidator,
}

impl TPublicKeysValidator for PublicKeysValidator {
    fn validate_keys(
        &self,
        raw_public_keys: &[Value],
    ) -> Result<ValidationResult<()>, NonConsensusError> {
        let mut result = ValidationResult::new(None);

        // TODO: convert buffers to arrays?
        // Validate public key structure
        for raw_public_key in raw_public_keys.iter() {
            result.merge(self.validate_public_key_structure(raw_public_key)?)
        }

        if !result.is_valid() {
            return Ok(result);
        }

        // Public keys already passed json schema validation at this point
        let mut public_keys = Vec::<IdentityPublicKey>::with_capacity(raw_public_keys.len());
        for raw_public_key in raw_public_keys {
            let pk: IdentityPublicKey = serde_json::from_value(raw_public_key.clone())?;
            public_keys.push(pk);
        }

        // Check that there's no duplicated key ids in the state transition
        let duplicated_ids = duplicated_key_ids(&public_keys);

        if !duplicated_ids.is_empty() {
            result.add_error(DuplicatedIdentityPublicKeyIdError::new(duplicated_ids));
        }

        // Check that there's no duplicated keys
        let duplicated_key_ids = duplicated_keys(&public_keys);

        if !duplicated_key_ids.is_empty() {
            result.add_error(DuplicatedIdentityPublicKeyError::new(duplicated_key_ids));
        }

        let mut validation_error: Option<PublicKeyValidationError>;

        for public_key in public_keys.iter() {
            validation_error = match public_key.key_type {
                KeyType::ECDSA_SECP256K1 => {
                    let key_bytes = &public_key.as_ecdsa_array()?;
                    match PublicKey::from_slice(key_bytes) {
                        Ok(_) => None,
                        Err(e) => Some(PublicKeyValidationError::new(e.to_string())),
                    }
                }
                KeyType::BLS12_381 => match BlsPublicKey::from_bytes(&public_key.data) {
                    Ok(_) => None,
                    Err(e) => Some(PublicKeyValidationError::new(e.to_string())),
                },
                // Do nothing
                KeyType::ECDSA_HASH160 => None,
            };

            if let Some(error) = validation_error {
                result.add_error(InvalidIdentityPublicKeyDataError::new(
                    public_key.id,
                    error.to_string(),
                    Some(error),
                ));
            }
        }

        // Validate that public keys have correct purpose and security level
        for raw_public_key in public_keys.iter() {
            let key_purpose = raw_public_key.purpose;
            let allowed_security_levels = ALLOWED_SECURITY_LEVELS.get(&key_purpose);

            if let Some(levels) = allowed_security_levels {
                if !levels.contains(&raw_public_key.security_level) {
                    let error = InvalidIdentityPublicKeySecurityLevelError::new(
                        raw_public_key.id,
                        raw_public_key.purpose,
                        raw_public_key.security_level,
                        Some(levels.clone()),
                    );

                    result.add_error(error);
                }
            } else {
                let error = InvalidIdentityPublicKeySecurityLevelError::new(
                    raw_public_key.id,
                    raw_public_key.purpose,
                    raw_public_key.security_level,
                    None,
                );

                result.add_error(error);
            }
        }

        Ok(result)
    }
}

impl PublicKeysValidator {
    pub fn new() -> Result<Self, DashPlatformProtocolInitError> {
        let public_key_schema_validator =
            JsonSchemaValidator::new(crate::schema::identity::public_key_json()?)?;

        let public_keys_validator = Self {
            public_key_schema_validator,
        };

        Ok(public_keys_validator)
    }

    pub fn validate_public_key_structure(
        &self,
        public_key: &Value,
    ) -> Result<ValidationResult<()>, NonConsensusError> {
        self.public_key_schema_validator.validate(public_key)
    }
}

fn duplicated_keys(public_keys: &[IdentityPublicKey]) -> Vec<u64> {
    let mut keys_count = HashMap::<Vec<u8>, usize>::new();
    let mut duplicated_key_ids = vec![];

    for public_key in public_keys.iter() {
        let data = &public_key.data;
        let count = *keys_count.get(&data.clone()).unwrap_or(&0_usize);
        let count = count + 1;
        keys_count.insert(data.clone(), count);

        if count > 1 {
            duplicated_key_ids.push(public_key.id);
        }
    }

    duplicated_key_ids
}

fn duplicated_key_ids(public_keys: &[IdentityPublicKey]) -> Vec<u64> {
    let mut duplicated_ids = Vec::<u64>::new();
    let mut ids_count = HashMap::<u64, usize>::new();

    for public_key in public_keys.iter() {
        let id = public_key.id;
        let count = *ids_count.get(&id).unwrap_or(&0_usize);
        let count = count + 1;
        ids_count.insert(id, count);

        if count > 1 {
            duplicated_ids.push(id);
        }
    }

    duplicated_ids
}
