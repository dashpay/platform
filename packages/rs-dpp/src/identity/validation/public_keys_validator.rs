use std::collections::HashMap;
use libsecp256k1::PublicKey;
use serde_json::Value;
use crate::consensus::basic::identity::DuplicatedIdentityPublicKeyIdError;
use crate::identity::{IdentityPublicKey, KeyType};
use crate::NonConsensusError;
use crate::validation::ValidationResult;

pub struct PublicKeysValidator {

}

impl PublicKeysValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_public_key_structure(public_key: &Value) -> ValidationResult {
        ValidationResult::new(None)
    }

    pub fn validate_keys(&self, raw_public_keys: Vec<Value>) -> Result<ValidationResult, NonConsensusError> {
        let mut result = ValidationResult::new(None);

        // TODO: convert buffers to arrays?
        // Validate public key structure
        for raw_public_key in raw_public_keys {
            result.merge(self.validate_against_schema(raw_public_key))
        }

        if !result.is_valid() {
            return Ok(result);
        }

        // Check that there's no duplicated key ids in the state transition
        let mut duplicated_ids = vec![];
        let mut ids_count = HashMap::<u64, u32>::new();

        for raw_public_key in raw_public_keys {
            let id = raw_public_key.as_object().unwrap().get("id").unwrap();
            let count = ids_count.get(id.into()).unwrap_or(0);
            let count = count + 1;
            ids_count.insert(id.into(), count);

            if count > 1 {
                duplicated_ids.push(id);
            }
        }

        // raw_public_keys.forEach((rawPublicKey) => {
        //     ids_count[rawPublicKey.id] = !ids_count[rawPublicKey.id] ? 1 : idsCount[rawPublicKey.id] + 1;
        //     if (ids_count[rawPublicKey.id] > 1) {
        //         duplicated_ids.push(rawPublicKey.id);
        //     }
        // });

        if duplicated_ids.len() > 0 {
            result.add_error(
                DuplicatedIdentityPublicKeyIdError::new(duplicated_ids),
            );
        }

        // Check that there's no duplicated keys
        let keys_count = {};
        let duplicated_key_ids = [];

        for raw_public_key in raw_public_keys {
            let data_hex = raw_public_key.data.toString("hex");

            keys_count[data_hex] = if !keys_count[data_hex] { 1 } else { keysCount[data_hex] + 1 };

            if keys_count[data_hex] > 1 {
                duplicated_key_ids.push(rawPublicKey.id);
            }
        }
        // raw_public_keys.forEach((rawPublicKey) => {
        //     let data_hex = rawPublicKey.data.toString('hex');
        //
        //     keys_count[data_hex] = !keys_count[data_hex]
        //         ? 1 : keysCount[data_hex] + 1;
        //
        //     if (keys_count[data_hex] > 1) {
        //         duplicated_key_ids.push(rawPublicKey.id);
        //     }
        // });

        if duplicated_key_ids.length > 0 {
            result.add_error(
                DuplicatedIdentityPublicKeyError::new(duplicated_key_ids),
            );
        }

        for raw_public_key in raw_public_keys {
            let validation_error;

            match raw_public_key {
                KeyType::ECDSA_SECP256K1 => {
                    let data_hex = rawPublicKey.data.toString('hex');

                    if (!PublicKey.isValid(data_hex)) {
                        validation_error = PublicKey.getValidationError(data_hex);
                    }
                }
                KeyType::BLS12_381 => {
                    try {
                        bls.PublicKey.fromBytes(
                            Uint8Array.from(rawPublicKey.data),
                        );
                    } catch (e) {
                        validation_error = new TypeError('Invalid public key');
                    }
                    break;
                }
               KeyType::ECDSA_HASH160:
                // Do nothing
                break;
                default:
                    throw new TypeError(`Unknown public key type: ${rawPublicKey.type}`);
            }

            if validation_error !== undefined {
                let consensus_error = InvalidIdentityPublicKeyDataError::new(
                    rawPublicKey.id,
                    validation_error.message,
                );

                consensus_error.setValidationError(validation_error);

                result.add_error(consensus_error);
            }
        }

        // validate key data
        // raw_public_keys
        //     .forEach((rawPublicKey) => {
        //         let validationError;
        //
        //         switch (rawPublicKey.type) {
        //             case IdentityPublicKey.TYPES.ECDSA_SECP256K1: {
        //                 const dataHex = rawPublicKey.data.toString('hex');
        //
        //                 if (!PublicKey.isValid(dataHex)) {
        //                     validationError = PublicKey.getValidationError(dataHex);
        //                 }
        //                 break;
        //             }
        //             case IdentityPublicKey.TYPES.BLS12_381: {
        //                 try {
        //                     bls.PublicKey.fromBytes(
        //                         Uint8Array.from(rawPublicKey.data),
        //                     );
        //                 } catch (e) {
        //                     validationError = new TypeError('Invalid public key');
        //                 }
        //                 break;
        //             }
        //             case IdentityPublicKey.TYPES.ECDSA_HASH160:
        //             // Do nothing
        //             break;
        //             default:
        //                 throw new TypeError(`Unknown public key type: ${rawPublicKey.type}`);
        //         }
        //
        //         if (validationError !== undefined) {
        //             const consensusError = new InvalidIdentityPublicKeyDataError(
        //                 rawPublicKey.id,
        //                 validationError.message,
        //             );
        //
        //             consensusError.setValidationError(validationError);
        //
        //             result.addError(consensusError);
        //         }
        //     });

        // Validate that public keys have correct purpose and security level
        for raw_public_key in raw_public_keys {
            let key_purpose = rawPublicKey.purpose;
            let allowed_security_levels = IdentityPublicKey.ALLOWED_SECURITY_LEVELS[key_purpose];

            if !allowed_security_levels || !allowed_security_levels.includes(rawPublicKey.securityLevel) {
                let error = InvalidIdentityPublicKeySecurityLevelError::new(
                    rawPublicKey.id,
                    rawPublicKey.purpose,
                    rawPublicKey.securityLevel,
                    allowed_security_levels,
                );

                result.add_error(error);
            }
        }

        return Ok(result);
    }
}