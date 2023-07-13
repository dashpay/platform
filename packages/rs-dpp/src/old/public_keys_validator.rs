use std::collections::HashMap;

use dashcore::PublicKey;
use lazy_static::lazy_static;

use crate::errors::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyBasicError, DuplicatedIdentityPublicKeyIdBasicError,
    InvalidIdentityPublicKeyDataError, InvalidIdentityPublicKeySecurityLevelError,
};
use crate::identity::{IdentityPublicKey, KeyID, KeyType};
use crate::validation::{JsonSchemaValidator, SimpleConsensusValidationResult};
use crate::{
    BlsModule, DashPlatformProtocolInitError, NonConsensusError, PublicKeyValidationError,
};

use crate::identity::security_level::ALLOWED_SECURITY_LEVELS;
use crate::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyInCreation;
#[cfg(test)]
use mockall::{automock, predicate::*};
use platform_value::Value;
use serde_json::Value as JsonValue;

lazy_static! {
    pub static ref PUBLIC_KEY_SCHEMA: JsonValue =
        serde_json::from_str(include_str!("./../../schema/identity/publicKey.json")).unwrap();
    pub static ref PUBLIC_KEY_SCHEMA_FOR_TRANSITION: JsonValue = serde_json::from_str(
        include_str!("./../../schema/identity/stateTransition/publicKey.json")
    )
    .unwrap();
}

#[cfg_attr(test, automock)]
pub trait TPublicKeysValidator {
    fn validate_keys(
        &self,
        raw_public_keys: &[Value],
    ) -> Result<SimpleConsensusValidationResult, NonConsensusError>;
}

pub struct PublicKeysValidator<T: BlsModule> {
    public_key_schema_validator: JsonSchemaValidator,
    bls_validator: T,
}

impl<T: BlsModule> TPublicKeysValidator for PublicKeysValidator<T> {
    fn validate_keys(
        &self,
        raw_public_keys: &[Value],
    ) -> Result<SimpleConsensusValidationResult, NonConsensusError> {
        let mut result = SimpleConsensusValidationResult::default();

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
            let pk: IdentityPublicKey = platform_value::from_value(raw_public_key.clone())?;
            public_keys.push(pk);
        }

        // Check that there's no duplicated key ids in the state transition
        let duplicated_ids = duplicated_key_ids(&public_keys);

        if !duplicated_ids.is_empty() {
            result.add_error(DuplicatedIdentityPublicKeyIdBasicError::new(duplicated_ids));
        }

        // Check that there's no duplicated keys
        let duplicated_key_ids = duplicated_keys(&public_keys);

        if !duplicated_key_ids.is_empty() {
            result.add_error(DuplicatedIdentityPublicKeyBasicError::new(
                duplicated_key_ids,
            ));
        }

        let mut validation_error: Option<PublicKeyValidationError>;

        for public_key in public_keys.iter() {
            validation_error = match public_key.key_type {
                KeyType::ECDSA_SECP256K1 => {
                    match PublicKey::from_slice(public_key.data.as_slice()) {
                        Ok(_) => None,
                        Err(e) => Some(PublicKeyValidationError::new(e.to_string())),
                    }
                }
                KeyType::BLS12_381 => {
                    match self
                        .bls_validator
                        .validate_public_key(public_key.data.as_slice())
                    {
                        Ok(_) => None,
                        Err(e) => Some(e),
                    }
                }
                // Do nothing
                KeyType::ECDSA_HASH160 => None,
                // Do nothing
                KeyType::BIP13_SCRIPT_HASH => None,
                // Do nothing
                KeyType::EDDSA_25519_HASH160 => None,
            };

            if let Some(error) = validation_error {
                result.add_error(InvalidIdentityPublicKeyDataError::new(public_key.id, error));
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

impl<T: BlsModule> PublicKeysValidator<T> {
    pub fn new(bls_validator: T) -> Result<Self, DashPlatformProtocolInitError> {
        let public_key_schema_validator = JsonSchemaValidator::new(PUBLIC_KEY_SCHEMA.clone())?;

        let public_keys_validator = Self {
            public_key_schema_validator,
            bls_validator,
        };

        Ok(public_keys_validator)
    }

    pub fn new_with_schema(
        schema: JsonValue,
        bls_validator: T,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let public_key_schema_validator = JsonSchemaValidator::new(schema)?;

        let public_keys_validator = Self {
            public_key_schema_validator,
            bls_validator,
        };

        Ok(public_keys_validator)
    }

    pub fn validate_public_key_structure(
        &self,
        public_key: &Value,
    ) -> Result<SimpleConsensusValidationResult, NonConsensusError> {
        self.public_key_schema_validator.validate(
            &public_key
                .try_to_validating_json()
                .map_err(NonConsensusError::ValueError)?,
        )
    }
}

pub(crate) fn duplicated_keys(public_keys: &[IdentityPublicKey]) -> Vec<KeyID> {
    let mut keys_count = HashMap::<Vec<u8>, usize>::new();
    let mut duplicated_key_ids = vec![];

    for public_key in public_keys.iter() {
        let data = public_key.data.as_slice();
        let count = *keys_count.get(data).unwrap_or(&0_usize);
        let count = count + 1;
        keys_count.insert(data.to_vec(), count);

        if count > 1 {
            duplicated_key_ids.push(public_key.id);
        }
    }

    duplicated_key_ids
}

pub fn duplicated_keys_witness(public_keys: &[IdentityPublicKeyInCreation]) -> Vec<KeyID> {
    let mut keys_count = HashMap::<Vec<u8>, usize>::new();
    let mut duplicated_key_ids = vec![];

    for public_key in public_keys.iter() {
        let data = public_key.data.as_slice();
        let count = *keys_count.get(data).unwrap_or(&0_usize);
        let count = count + 1;
        keys_count.insert(data.to_vec(), count);

        if count > 1 {
            duplicated_key_ids.push(public_key.id);
        }
    }

    duplicated_key_ids
}

pub(crate) fn duplicated_key_ids(public_keys: &[IdentityPublicKey]) -> Vec<KeyID> {
    let mut duplicated_ids = Vec::<KeyID>::new();
    let mut ids_count = HashMap::<KeyID, usize>::new();

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

pub fn duplicated_key_ids_witness(public_keys: &[IdentityPublicKeyInCreation]) -> Vec<KeyID> {
    let mut duplicated_ids = Vec::<KeyID>::new();
    let mut ids_count = HashMap::<KeyID, usize>::new();

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
