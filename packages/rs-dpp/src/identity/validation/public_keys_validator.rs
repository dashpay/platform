use crate::consensus::basic::identity::{DuplicatedIdentityPublicKeyIdError, InvalidIdentityPublicKeyDataError, InvalidIdentityPublicKeySecurityLevelError};
use crate::identity::{ALLOWED_SECURITY_LEVELS, IdentityPublicKey, KeyType};
use crate::validation::{byte_array_meta, JsonSchemaValidator, ValidationResult};
use crate::{DashPlatformProtocolInitError, NonConsensusError, PublicKeyValidationError};
use jsonschema::{JSONSchema, KeywordDefinition};
use libsecp256k1::PublicKey;
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct PublicKeysValidator {
    public_key_schema_validator: JsonSchemaValidator,
}

impl PublicKeysValidator {
    pub fn new() -> Result<Self, DashPlatformProtocolInitError> {
        let public_key_schema_validator =
            JsonSchemaValidator::new(crate::schema::identity::public_key_json()?)?;

        let mut public_keys_validator = Self {
            public_key_schema_validator,
        };

        Ok(public_keys_validator)
    }

    pub fn validate_public_key_structure(
        &self,
        public_key: &Value,
    ) -> Result<ValidationResult, NonConsensusError> {
        self.public_key_schema_validator.validate(public_key)
    }

    pub fn validate_keys(
        &self,
        raw_public_keys: &Vec<Value>,
    ) -> Result<ValidationResult, NonConsensusError> {
        let mut result = ValidationResult::new(None);

        // TODO: convert buffers to arrays?
        // Validate public key structure
        for raw_public_key in raw_public_keys.iter() {
            result.merge(self.validate_public_key_structure(raw_public_key)?)
        }

        if !result.is_valid() {
            return Ok(result);
        }

        let mut pks = Vec::<IdentityPublicKey>::with_capacity(raw_public_keys.capacity());
        for rpk in raw_public_keys {
            let pk: IdentityPublicKey = serde_json::from_value(rpk.clone())?;
            pks.push(pk);
        }

        // // Public keys already passed json schema validation at this point
        // let public_keys: Vec<IdentityPublicKey> = raw_public_keys.iter().map(|raw_public_key| {
        //     let pk: IdentityPublicKey = serde_json::from_value(raw_public_key.clone())?;
        //     pk
        // }).collect();

        // Check that there's no duplicated key ids in the state transition
        let mut duplicated_ids = Vec::<u64>::new();
        let mut ids_count = HashMap::<u64, u32>::new();

        // for raw_public_key in raw_public_keys {
        //     let id = raw_public_key.as_object().unwrap().get("id").unwrap();
        //     let count = ids_count.get(id.into()).unwrap_or(0);
        //     let count = count + 1;
        //     ids_count.insert(id.into(), count);
        //
        //     if count > 1 {
        //         duplicated_ids.push(id);
        //     }
        // }
        //
        // // raw_public_keys.forEach((rawPublicKey) => {
        // //     ids_count[rawPublicKey.id] = !ids_count[rawPublicKey.id] ? 1 : idsCount[rawPublicKey.id] + 1;
        // //     if (ids_count[rawPublicKey.id] > 1) {
        // //         duplicated_ids.push(rawPublicKey.id);
        // //     }
        // // });
        //
        // if duplicated_ids.len() > 0 {
        //     result.add_error(
        //         DuplicatedIdentityPublicKeyIdError::new(duplicated_ids),
        //     );
        // }
        //
        // // Check that there's no duplicated keys
        // let keys_count = {};
        // let duplicated_key_ids = [];
        //
        // for raw_public_key in raw_public_keys {
        //     let data_hex = raw_public_key.data.toString("hex");
        //
        //     keys_count[data_hex] = if !keys_count[data_hex] { 1 } else { keysCount[data_hex] + 1 };
        //
        //     if keys_count[data_hex] > 1 {
        //         duplicated_key_ids.push(rawPublicKey.id);
        //     }
        // }
        // // raw_public_keys.forEach((rawPublicKey) => {
        // //     let data_hex = rawPublicKey.data.toString('hex');
        // //
        // //     keys_count[data_hex] = !keys_count[data_hex]
        // //         ? 1 : keysCount[data_hex] + 1;
        // //
        // //     if (keys_count[data_hex] > 1) {
        // //         duplicated_key_ids.push(rawPublicKey.id);
        // //     }
        // // });
        //
        // if duplicated_key_ids.length > 0 {
        //     result.add_error(
        //         DuplicatedIdentityPublicKeyError::new(duplicated_key_ids),
        //     );
        // }

        let mut validation_error: Option<PublicKeyValidationError> = None;

        for public_key in pks.iter() {
            validation_error = match public_key.key_type {
                KeyType::ECDSA_SECP256K1 => {
                    let key_bytes = &public_key.data_as_arr_33()?;
                    match PublicKey::parse_compressed(key_bytes) {
                        Ok(_) => None,
                        Err(e) => Some(PublicKeyValidationError::new(e.to_string())),
                    }
                }
                KeyType::BLS12_381 => Some(PublicKeyValidationError::new("Not implemented")),
                KeyType::ECDSA_HASH160 => Some(PublicKeyValidationError::new("Not implemented")),
            };

            if let Some(error) = validation_error {
                result.add_error(InvalidIdentityPublicKeyDataError::new(
                    public_key.id,
                    error.to_string(),
                    Some(error)
                ));
            }
        }

        // // validate key data
        // // raw_public_keys
        // //     .forEach((rawPublicKey) => {
        // //         let validationError;
        // //
        // //         switch (rawPublicKey.type) {
        // //             case IdentityPublicKey.TYPES.ECDSA_SECP256K1: {
        // //                 const dataHex = rawPublicKey.data.toString('hex');
        // //
        // //                 if (!PublicKey.isValid(dataHex)) {
        // //                     validationError = PublicKey.getValidationError(dataHex);
        // //                 }
        // //                 break;
        // //             }
        // //             case IdentityPublicKey.TYPES.BLS12_381: {
        // //                 try {
        // //                     bls.PublicKey.fromBytes(
        // //                         Uint8Array.from(rawPublicKey.data),
        // //                     );
        // //                 } catch (e) {
        // //                     validationError = new TypeError('Invalid public key');
        // //                 }
        // //                 break;
        // //             }
        // //             case IdentityPublicKey.TYPES.ECDSA_HASH160:
        // //             // Do nothing
        // //             break;
        // //             default:
        // //                 throw new TypeError(`Unknown public key type: ${rawPublicKey.type}`);
        // //         }
        // //
        // //         if (validationError !== undefined) {
        // //             const consensusError = new InvalidIdentityPublicKeyDataError(
        // //                 rawPublicKey.id,
        // //                 validationError.message,
        // //             );
        // //
        // //             consensusError.setValidationError(validationError);
        // //
        // //             result.addError(consensusError);
        // //         }
        // //     });
        //

        // Validate that public keys have correct purpose and security level
        for raw_public_key in pks.iter() {
            let key_purpose = raw_public_key.purpose;
            let allowed_security_levels = ALLOWED_SECURITY_LEVELS.get(&key_purpose);

            if let Some(levels) = allowed_security_levels {
                if !levels.contains(&raw_public_key.security_level) {
                    let error = InvalidIdentityPublicKeySecurityLevelError::new(
                        raw_public_key.id,
                        raw_public_key.purpose,
                        raw_public_key.security_level,
                        allowed_security_levels,
                    );

                    result.add_error(error);
                }
            } else {
                let error = InvalidIdentityPublicKeySecurityLevelError::new(
                    raw_public_key.id,
                    raw_public_key.purpose,
                    raw_public_key.security_level,
                    allowed_security_levels,
                );

                result.add_error(error);
            }
        }

        return Ok(result);
    }
}
