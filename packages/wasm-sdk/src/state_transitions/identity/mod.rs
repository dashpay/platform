use crate::error::WasmSdkError;
use crate::queries::utils::identifier_from_js;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::dashcore::PrivateKey;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dash_sdk::dpp::platform_value::{string_encoding::Encoding, BinaryData, Identifier};
use dash_sdk::dpp::prelude::AssetLockProof;
use dash_sdk::dpp::prelude::UserFeeIncrease;
use dash_sdk::dpp::state_transition::identity_credit_transfer_transition::methods::IdentityCreditTransferTransitionMethodsV0;
use dash_sdk::dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::top_up_identity::TopUpIdentity;
use dash_sdk::platform::Fetch;
use js_sys;
use simple_signer::{signer::SimpleSigner, SingleKeySigner};
use tracing::{debug, error};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

/// Parse a KeyType from its string representation.
fn parse_key_type(s: Option<&str>) -> Result<KeyType, WasmSdkError> {
    match s.ok_or_else(|| WasmSdkError::invalid_argument("keyType is required"))? {
        "ECDSA_SECP256K1" => Ok(KeyType::ECDSA_SECP256K1),
        "BLS12_381" => Ok(KeyType::BLS12_381),
        "ECDSA_HASH160" => Ok(KeyType::ECDSA_HASH160),
        "BIP13_SCRIPT_HASH" => Ok(KeyType::BIP13_SCRIPT_HASH),
        "EDDSA_25519_HASH160" => Ok(KeyType::EDDSA_25519_HASH160),
        s => Err(WasmSdkError::invalid_argument(format!(
            "Unknown key type: {}",
            s
        ))),
    }
}

/// Parse a Purpose from its string representation.
fn parse_purpose(s: Option<&str>) -> Result<Purpose, WasmSdkError> {
    match s.ok_or_else(|| WasmSdkError::invalid_argument("purpose is required"))? {
        "AUTHENTICATION" => Ok(Purpose::AUTHENTICATION),
        "ENCRYPTION" => Ok(Purpose::ENCRYPTION),
        "DECRYPTION" => Ok(Purpose::DECRYPTION),
        "TRANSFER" => Ok(Purpose::TRANSFER),
        "SYSTEM" => Ok(Purpose::SYSTEM),
        "VOTING" => Ok(Purpose::VOTING),
        s => Err(WasmSdkError::invalid_argument(format!(
            "Unknown purpose: {}",
            s
        ))),
    }
}

/// Parse a SecurityLevel from its string representation.
/// Defaults to HIGH for None or unknown values.
fn parse_security_level(s: Option<&str>) -> SecurityLevel {
    match s {
        Some("MASTER") => SecurityLevel::MASTER,
        Some("CRITICAL") => SecurityLevel::CRITICAL,
        Some("HIGH") => SecurityLevel::HIGH,
        Some("MEDIUM") => SecurityLevel::MEDIUM,
        _ => SecurityLevel::HIGH,
    }
}

/// Check if an ECDSA-derived public key matches an identity's public key.
/// Supports ECDSA_SECP256K1 (33-byte comparison) and ECDSA_HASH160 (20-byte comparison).
/// Returns false for non-ECDSA key types (BLS, EdDSA, etc.) since they require different derivation.
fn ecdsa_public_key_matches_identity_key(
    public_key_bytes: &[u8],   // 33-byte compressed secp256k1 public key
    public_key_hash160: &[u8], // 20-byte hash160 of the public key
    key: &IdentityPublicKey,
) -> bool {
    match key.key_type() {
        KeyType::ECDSA_SECP256K1 => {
            // Compare full 33-byte compressed public key
            key.data().as_slice() == public_key_bytes
        }
        KeyType::ECDSA_HASH160 => {
            // Compare 20-byte hash160
            key.data().as_slice() == public_key_hash160
        }
        // BLS12_381 keys require separate BLS key derivation - not supported via ECDSA private key
        // BIP13_SCRIPT_HASH and EDDSA_25519_HASH160 are also not derivable from ECDSA private key
        _ => false,
    }
}

/// Parse public keys from JSON for identity update operations.
///
/// This function handles the key parsing logic for `identity_update`, supporting:
/// - ECDSA_SECP256K1: Requires `privateKeyHex` or `privateKeyWif`, adds to signer
/// - BLS12_381: Requires `privateKeyHex` (WIF not supported), adds to signer
/// - ECDSA_HASH160: Accepts `privateKeyHex` or `data` field, does NOT add to signer
/// - BIP13_SCRIPT_HASH, EDDSA_25519_HASH160: Requires `data` field, does NOT add to signer
///
/// Signing key types (where `is_unique_key_type() == true`) are added to the signer
/// so they can sign for themselves during the identity update transition.
fn parse_keys_for_identity_update(
    keys_json: &str,
    starting_key_id: u32,
    signer: &mut SimpleSigner,
    network: dash_sdk::dpp::dashcore::Network,
) -> Result<Vec<IdentityPublicKey>, WasmSdkError> {
    let keys_data: serde_json::Value = serde_json::from_str(keys_json).map_err(|e| {
        WasmSdkError::invalid_argument(format!("Invalid JSON for add_public_keys: {}", e))
    })?;

    let keys_array = keys_data
        .as_array()
        .ok_or_else(|| WasmSdkError::invalid_argument("add_public_keys must be a JSON array"))?;

    let mut next_key_id = starting_key_id;
    let mut keys_result = Vec::new();

    for key_data in keys_array {
        let key_type = parse_key_type(key_data["keyType"].as_str())?;
        let purpose = parse_purpose(key_data["purpose"].as_str())?;
        let security_level = parse_security_level(key_data["securityLevel"].as_str());

        // Handle key data based on key type
        // Signing key types (is_unique_key_type == true) require private key
        // Non-signing key types use the data field
        let (public_key_data, private_key_bytes_opt): (Vec<u8>, Option<[u8; 32]>) = match key_type {
            KeyType::ECDSA_SECP256K1 => {
                // For ECDSA signing keys, require private key (hex or WIF)
                let private_key_bytes = if let Some(pk_hex) = key_data["privateKeyHex"].as_str() {
                    let bytes = hex::decode(pk_hex).map_err(|e| {
                        WasmSdkError::invalid_argument(format!("Invalid private key hex: {}", e))
                    })?;
                    bytes.as_slice().try_into().map_err(|_| {
                        WasmSdkError::invalid_argument(format!(
                            "Private key must be 32 bytes, got {}",
                            bytes.len()
                        ))
                    })?
                } else if let Some(pk_wif) = key_data["privateKeyWif"].as_str() {
                    let pk = PrivateKey::from_wif(pk_wif).map_err(|e| {
                        WasmSdkError::invalid_argument(format!("Invalid WIF private key: {}", e))
                    })?;
                    pk.inner.secret_bytes()
                } else {
                    return Err(WasmSdkError::invalid_argument(
                        "ECDSA_SECP256K1 keys require either privateKeyHex or privateKeyWif",
                    ));
                };

                // Derive public key from private key
                let public_key_data = key_type
                    .public_key_data_from_private_key_data(&private_key_bytes, network)
                    .map_err(|e| {
                        WasmSdkError::generic(format!(
                            "Failed to derive ECDSA_SECP256K1 public key: {}",
                            e
                        ))
                    })?;

                (public_key_data, Some(private_key_bytes))
            }
            KeyType::BLS12_381 => {
                // BLS keys only support hex format (WIF is not valid for BLS)
                if key_data["privateKeyWif"].is_string() {
                    return Err(WasmSdkError::invalid_argument(
                        "BLS12_381 keys do not support WIF format, use privateKeyHex only",
                    ));
                }

                let private_key_bytes = if let Some(pk_hex) = key_data["privateKeyHex"].as_str() {
                    let bytes = hex::decode(pk_hex).map_err(|e| {
                        WasmSdkError::invalid_argument(format!("Invalid private key hex: {}", e))
                    })?;
                    bytes.as_slice().try_into().map_err(|_| {
                        WasmSdkError::invalid_argument(format!(
                            "Private key must be 32 bytes, got {}",
                            bytes.len()
                        ))
                    })?
                } else {
                    return Err(WasmSdkError::invalid_argument(
                        "BLS12_381 keys require privateKeyHex",
                    ));
                };

                // Derive public key from private key
                let public_key_data = key_type
                    .public_key_data_from_private_key_data(&private_key_bytes, network)
                    .map_err(|e| {
                        WasmSdkError::generic(format!(
                            "Failed to derive BLS12_381 public key: {}",
                            e
                        ))
                    })?;

                (public_key_data, Some(private_key_bytes))
            }
            KeyType::ECDSA_HASH160 => {
                // ECDSA_HASH160: Accept privateKeyHex (to derive hash) or data field
                if let Some(pk_hex) = key_data["privateKeyHex"].as_str() {
                    let bytes = hex::decode(pk_hex).map_err(|e| {
                        WasmSdkError::invalid_argument(format!("Invalid private key hex: {}", e))
                    })?;
                    let private_key_bytes: [u8; 32] =
                        bytes.as_slice().try_into().map_err(|_| {
                            WasmSdkError::invalid_argument(format!(
                                "Private key must be 32 bytes, got {}",
                                bytes.len()
                            ))
                        })?;

                    let derived_data = key_type
                        .public_key_data_from_private_key_data(&private_key_bytes, network)
                        .map_err(|e| {
                            WasmSdkError::generic(format!(
                                "Failed to derive ECDSA_HASH160 public key: {}",
                                e
                            ))
                        })?;

                    (derived_data, None) // No signing needed for HASH160
                } else if let Some(data_str) = key_data["data"].as_str() {
                    let key_data_bytes = dash_sdk::dpp::dashcore::base64::decode(data_str)
                        .map_err(|e| {
                            WasmSdkError::invalid_argument(format!(
                                "Invalid base64 key data: {}",
                                e
                            ))
                        })?;
                    if key_data_bytes.len() != 20 {
                        return Err(WasmSdkError::invalid_argument(format!(
                            "ECDSA_HASH160 key data must be 20 bytes, got {}",
                            key_data_bytes.len()
                        )));
                    }
                    (key_data_bytes, None)
                } else {
                    return Err(WasmSdkError::invalid_argument(
                        "ECDSA_HASH160 requires either 'privateKeyHex' or 'data' field",
                    ));
                }
            }
            KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => {
                // These only accept data field (backward compatible)
                let data_str = key_data["data"].as_str().ok_or_else(|| {
                    WasmSdkError::invalid_argument(format!(
                        "{:?} keys require 'data' field",
                        key_type
                    ))
                })?;
                let key_data_bytes =
                    dash_sdk::dpp::dashcore::base64::decode(data_str).map_err(|e| {
                        WasmSdkError::invalid_argument(format!("Invalid base64 key data: {}", e))
                    })?;
                // Validate expected length (20 bytes for hash-based key types)
                if key_data_bytes.len() != 20 {
                    return Err(WasmSdkError::invalid_argument(format!(
                        "{:?} key data must be 20 bytes, got {}",
                        key_type,
                        key_data_bytes.len()
                    )));
                }
                (key_data_bytes, None)
            }
        };

        // Create the identity public key
        use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: next_key_id,
            key_type,
            purpose,
            security_level,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(public_key_data),
            disabled_at: None,
        });

        // Add to signer if this is a signing key type (is_unique_key_type == true)
        if let Some(pk_bytes) = private_key_bytes_opt {
            signer.add_identity_public_key(public_key.clone(), pk_bytes);
        }

        keys_result.push(public_key);
        next_key_id += 1;
    }

    Ok(keys_result)
}

#[wasm_bindgen]
impl WasmSdk {
    /// Create a new identity on Dash Platform.
    ///
    /// # Arguments
    ///
    /// * `asset_lock_proof` - The asset lock proof (transaction hex)
    /// * `asset_lock_proof_private_key` - The private key that controls the asset lock
    /// * `public_keys` - JSON array of public keys to add to the identity. Each key object requirements:
    ///   - ECDSA_SECP256K1: Requires `privateKeyHex` or `privateKeyWif` for signing
    ///   - BLS12_381: Requires `privateKeyHex` for signing (WIF format not supported)
    ///   - ECDSA_HASH160: Accepts either `privateKeyHex` (to derive hash) or `data` field (base64-encoded 20-byte hash)
    ///
    /// # Implementation Notes
    ///
    /// This function uses SimpleSigner to provide individual signatures for each public key as required.
    /// Each ECDSA_SECP256K1 key will be signed with its corresponding private key (from privateKeyHex or privateKeyWif),
    /// and each BLS12_381 key will be signed with its corresponding private key (from privateKeyHex only),
    /// ensuring unique signatures per key as required by DPP validation.
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the new identity
    #[wasm_bindgen(js_name = identityCreate)]
    pub async fn identity_create(
        &self,
        #[wasm_bindgen(js_name = "assetLockProof")] asset_lock_proof: String,
        #[wasm_bindgen(js_name = "assetLockProofPrivateKey")] asset_lock_proof_private_key: String,
        #[wasm_bindgen(js_name = "publicKeys")] public_keys: String,
    ) -> Result<JsValue, WasmSdkError> {
        let sdk = self.inner_clone();

        // Debug log parameters (truncated/sanitized)
        debug!(
            target : "wasm_sdk", len = asset_lock_proof.len(), preview = % if
            asset_lock_proof.len() > 100 { format!("{}...", & asset_lock_proof[..100]) }
            else { asset_lock_proof.clone() }, "identityCreate called"
        );
        debug!(
            target : "wasm_sdk", pk_len = asset_lock_proof_private_key.len(),
            "identityCreate private key length"
        );
        debug!(
            target : "wasm_sdk", public_keys = % public_keys,
            "identityCreate public keys JSON"
        );

        // Parse asset lock proof - try hex first, then JSON
        let asset_lock_proof: AssetLockProof = if asset_lock_proof
            .chars()
            .all(|c| c.is_ascii_hexdigit())
        {
            // It's hex encoded - decode and parse as JSON from the decoded bytes
            let asset_lock_proof_bytes = hex::decode(&asset_lock_proof).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid asset lock proof hex: {}", e))
            })?;

            // Convert bytes to string and parse as JSON
            let json_str = String::from_utf8(asset_lock_proof_bytes).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid UTF-8 in asset lock proof: {}", e))
            })?;

            serde_json::from_str(&json_str).map_err(|e| {
                WasmSdkError::invalid_argument(format!(
                    "Failed to parse asset lock proof JSON: {}",
                    e
                ))
            })?
        } else {
            // Try JSON directly
            serde_json::from_str(&asset_lock_proof).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid asset lock proof JSON: {}", e))
            })?
        };

        // Parse private key - WIF format
        debug!(
            target : "wasm_sdk", pk_len = asset_lock_proof_private_key.len(),
            "Private key format validation"
        );
        let private_key = PrivateKey::from_wif(&asset_lock_proof_private_key)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid private key: {}", e)))?;

        // Parse public keys from JSON
        let keys_data: serde_json::Value = serde_json::from_str(&public_keys).map_err(|e| {
            WasmSdkError::invalid_argument(format!("Invalid JSON for public_keys: {}", e))
        })?;

        let keys_array = keys_data
            .as_array()
            .ok_or_else(|| WasmSdkError::invalid_argument("public_keys must be a JSON array"))?;

        // Create identity public keys and collect private keys for signing
        let mut identity_public_keys = std::collections::BTreeMap::new();
        let mut signer = SimpleSigner::default();

        for (key_id, key_data) in keys_array
            .iter()
            .enumerate()
            .map(|(key, value)| (key as u32, value))
        {
            let key_type = parse_key_type(key_data["keyType"].as_str())?;
            let purpose = parse_purpose(key_data["purpose"].as_str())?;
            let security_level = parse_security_level(key_data["securityLevel"].as_str());

            // Handle key data based on key type
            let (public_key_data, private_key_bytes) = match key_type {
                KeyType::ECDSA_HASH160 => {
                    // Derive HASH160 data from the private key if provided
                    if let Some(private_key_hex) = key_data["privateKeyHex"].as_str() {
                        // Decode private key from hex
                        let bytes = hex::decode(private_key_hex).map_err(|e| {
                            WasmSdkError::invalid_argument(format!(
                                "Invalid private key hex: {}",
                                e
                            ))
                        })?;

                        if bytes.len() != 32 {
                            return Err(WasmSdkError::invalid_argument(format!(
                                "Private key must be 32 bytes, got {}",
                                bytes.len()
                            )));
                        }

                        let mut private_key_array = [0u8; 32];
                        private_key_array.copy_from_slice(&bytes);

                        // Derive HASH160 public key data from private key using network
                        let derived_data = key_type
                            .public_key_data_from_private_key_data(
                                &private_key_array,
                                self.network(),
                            )
                            .map_err(|e| {
                                WasmSdkError::generic(format!(
                                    "Failed to derive ECDSA_HASH160 public key data: {}",
                                    e
                                ))
                            })?;

                        // HASH160 keys are not used for signing during identity creation
                        (derived_data, [0u8; 32])
                    } else if let Some(data_str) = key_data["data"].as_str() {
                        let key_data_bytes = dash_sdk::dpp::dashcore::base64::decode(data_str)
                            .map_err(|e| {
                                WasmSdkError::invalid_argument(format!(
                                    "Invalid base64 key data: {}",
                                    e
                                ))
                            })?;

                        // Enforce correct HASH160 size (20 bytes).
                        if key_data_bytes.len() != 20 {
                            return Err(WasmSdkError::invalid_argument(format!(
                                "ECDSA_HASH160 key data must be 20 bytes, got {}",
                                key_data_bytes.len()
                            )));
                        }

                        (key_data_bytes, [0u8; 32])
                    } else {
                        return Err(
                            WasmSdkError::invalid_argument(
                                "ECDSA_HASH160 requires either 'privateKeyHex' to derive from or 'data' (base64-encoded 20-byte hash)",
                            ),
                        );
                    }
                }
                KeyType::ECDSA_SECP256K1 => {
                    // For ECDSA signing keys, support both hex and WIF formats
                    let private_key_bytes = if let Some(private_key_hex) =
                        key_data["privateKeyHex"].as_str()
                    {
                        // Decode private key from hex
                        let bytes = hex::decode(private_key_hex).map_err(|e| {
                            WasmSdkError::invalid_argument(format!(
                                "Invalid private key hex: {}",
                                e
                            ))
                        })?;

                        if bytes.len() != 32 {
                            return Err(WasmSdkError::invalid_argument(format!(
                                "Private key must be 32 bytes, got {}",
                                bytes.len()
                            )));
                        }

                        let mut private_key_array = [0u8; 32];
                        private_key_array.copy_from_slice(&bytes);
                        private_key_array
                    } else if let Some(private_key_wif) = key_data["privateKeyWif"].as_str() {
                        // Parse WIF format private key
                        let private_key = PrivateKey::from_wif(private_key_wif).map_err(|e| {
                            WasmSdkError::invalid_argument(format!(
                                "Invalid WIF private key: {}",
                                e
                            ))
                        })?;
                        private_key.inner.secret_bytes()
                    } else {
                        return Err(WasmSdkError::invalid_argument(
                            "ECDSA_SECP256K1 keys require either privateKeyHex or privateKeyWif",
                        ));
                    };

                    // Derive public key data from private key
                    let public_key_data = key_type
                        .public_key_data_from_private_key_data(&private_key_bytes, self.network())
                        .map_err(|e| {
                            WasmSdkError::generic(format!(
                                "Failed to derive ECDSA_SECP256K1 public key data: {}",
                                e
                            ))
                        })?;

                    (public_key_data, private_key_bytes)
                }
                KeyType::BLS12_381 => {
                    // BLS12_381 keys only support hex format (WIF is not valid for BLS keys)
                    if key_data["privateKeyWif"].is_string() {
                        return Err(WasmSdkError::invalid_argument(
                            "BLS12_381 keys do not support WIF format, use privateKeyHex only",
                        ));
                    }

                    let private_key_bytes =
                        if let Some(private_key_hex) = key_data["privateKeyHex"].as_str() {
                            // Decode private key from hex
                            let bytes = hex::decode(private_key_hex).map_err(|e| {
                                WasmSdkError::invalid_argument(format!(
                                    "Invalid private key hex: {}",
                                    e
                                ))
                            })?;

                            if bytes.len() != 32 {
                                return Err(WasmSdkError::invalid_argument(format!(
                                    "Private key must be 32 bytes, got {}",
                                    bytes.len()
                                )));
                            }

                            let mut private_key_array = [0u8; 32];
                            private_key_array.copy_from_slice(&bytes);
                            private_key_array
                        } else {
                            return Err(WasmSdkError::invalid_argument(
                                "BLS12_381 keys require privateKeyHex",
                            ));
                        };

                    // Derive public key data from private key
                    let public_key_data = key_type
                        .public_key_data_from_private_key_data(&private_key_bytes, self.network())
                        .map_err(|e| {
                            WasmSdkError::generic(format!(
                                "Failed to derive BLS12_381 public key data: {}",
                                e
                            ))
                        })?;

                    (public_key_data, private_key_bytes)
                }
                _ => {
                    return Err(WasmSdkError::invalid_argument(format!(
                        "Unsupported key type for identity creation: {:?}",
                        key_type
                    )));
                }
            };

            // Create the identity public key
            use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
            let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: key_id,
                key_type,
                purpose,
                security_level,
                contract_bounds: None,
                read_only: false,
                data: BinaryData::new(public_key_data),
                disabled_at: None,
            });

            // Add the public key and its private key to the signer (only for signing key types)
            if key_type != KeyType::ECDSA_HASH160 {
                signer.add_identity_public_key(public_key.clone(), private_key_bytes);
            }

            identity_public_keys.insert(key_id, public_key);
        }

        // Create identity
        use dash_sdk::dpp::identity::v0::IdentityV0;
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::random(),
            public_keys: identity_public_keys,
            balance: 0,
            revision: 0,
        });

        // Use the SimpleSigner we built with all the identity keys
        // The signer now contains all private keys for signing each public key individually

        // Put identity to platform and wait
        let created_identity = match identity
            .put_to_platform_and_wait_for_response(
                &sdk,
                asset_lock_proof,
                &private_key,
                &signer,
                None,
            )
            .await
        {
            Ok(identity) => identity,
            Err(e) => {
                // Extract more detailed error information
                let error_msg = format!("Failed to create identity: {}", e);
                error!(
                    target : "wasm_sdk", msg = % error_msg, "Identity creation failed"
                );
                return Err(WasmSdkError::generic(error_msg));
            }
        };

        // Create JavaScript result object
        let result_obj = js_sys::Object::new();

        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("status"),
            &JsValue::from_str("success"),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set status: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("identityId"),
            &JsValue::from_str(&created_identity.id().to_string(Encoding::Base58)),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set identityId: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("balance"),
            &JsValue::from_f64(created_identity.balance() as f64),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set balance: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("revision"),
            &JsValue::from_f64(created_identity.revision() as f64),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set revision: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("publicKeys"),
            &JsValue::from_f64(created_identity.public_keys().len() as f64),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set publicKeys: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("message"),
            &JsValue::from_str("Identity created successfully"),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set message: {:?}", e)))?;

        Ok(result_obj.into())
    }

    /// Top up an existing identity with additional credits.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity ID to top up
    /// * `asset_lock_proof` - The asset lock proof (transaction hex)
    /// * `asset_lock_proof_private_key` - The private key that controls the asset lock
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the new balance
    #[wasm_bindgen(js_name = identityTopUp)]
    pub async fn identity_top_up(
        &self,
        #[wasm_bindgen(js_name = "identityId")]
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        identity_id: JsValue,
        #[wasm_bindgen(js_name = "assetLockProof")] asset_lock_proof: String,
        #[wasm_bindgen(js_name = "assetLockProofPrivateKey")] asset_lock_proof_private_key: String,
    ) -> Result<JsValue, WasmSdkError> {
        let sdk = self.inner_clone();

        // Parse identity identifier
        let identifier = identifier_from_js(&identity_id, "identity ID")?;
        let identity_base58 = identifier.to_string(Encoding::Base58);

        // Parse asset lock proof - try hex first, then JSON
        let asset_lock_proof: AssetLockProof = if asset_lock_proof
            .chars()
            .all(|c| c.is_ascii_hexdigit())
        {
            // It's hex encoded - decode and parse as JSON from the decoded bytes
            let asset_lock_proof_bytes = hex::decode(&asset_lock_proof).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid asset lock proof hex: {}", e))
            })?;

            // Convert bytes to string and parse as JSON
            let json_str = String::from_utf8(asset_lock_proof_bytes).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid UTF-8 in asset lock proof: {}", e))
            })?;

            serde_json::from_str(&json_str).map_err(|e| {
                WasmSdkError::invalid_argument(format!(
                    "Failed to parse asset lock proof JSON: {}",
                    e
                ))
            })?
        } else {
            // Try JSON directly
            serde_json::from_str(&asset_lock_proof).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid asset lock proof JSON: {}", e))
            })?
        };
        debug!(
            target : "wasm_sdk", pk_len = asset_lock_proof_private_key.len(),
            "Private key format validation"
        );
        let private_key = PrivateKey::from_wif(&asset_lock_proof_private_key)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid private key: {}", e)))?;

        // Fetch the identity
        let identity = match dash_sdk::platform::Identity::fetch(&sdk, identifier).await {
            Ok(Some(identity)) => identity,
            Ok(None) => {
                let error_msg = format!("Identity not found: {}", identifier);
                error!(target : "wasm_sdk", % error_msg);
                return Err(WasmSdkError::not_found(error_msg));
            }
            Err(e) => {
                let error_msg = format!("Failed to fetch identity: {}", e);
                error!(target : "wasm_sdk", % error_msg);
                return Err(WasmSdkError::from(e));
            }
        };

        // Get the initial balance
        let initial_balance = identity.balance();

        // Top up the identity
        let new_balance = match identity
            .top_up_identity(&sdk, asset_lock_proof, &private_key, None, None)
            .await
        {
            Ok(balance) => balance,
            Err(e) => {
                let error_msg = format!("Failed to top up identity: {}", e);
                error!(target : "wasm_sdk", % error_msg);
                return Err(WasmSdkError::from(e));
            }
        };

        let topped_up_amount = new_balance.saturating_sub(initial_balance);

        // Create JavaScript result object
        let result_obj = js_sys::Object::new();

        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("status"),
            &JsValue::from_str("success"),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set status: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("identityId"),
            &JsValue::from_str(&identity_base58),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set identityId: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("newBalance"),
            &JsValue::from_f64(new_balance as f64),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set newBalance: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("toppedUpAmount"),
            &JsValue::from_f64(topped_up_amount as f64),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set toppedUpAmount: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("message"),
            &JsValue::from_str("Identity topped up successfully"),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set message: {:?}", e)))?;

        Ok(result_obj.into())
    }

    /// Transfer credits from one identity to another.
    ///
    /// # Arguments
    ///
    /// * `sender_id` - The identity ID of the sender
    /// * `recipient_id` - The identity ID of the recipient
    /// * `amount` - The amount of credits to transfer
    /// * `private_key_wif` - The private key in WIF format for signing
    /// * `key_id` - Optional key ID to use for signing (if None, will auto-select)
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the transfer result
    #[wasm_bindgen(js_name = identityCreditTransfer)]
    pub async fn identity_credit_transfer(
        &self,
        #[wasm_bindgen(js_name = "senderId")]
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        sender_id: JsValue,
        #[wasm_bindgen(js_name = "recipientId")]
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        recipient_id: JsValue,
        amount: u64,
        #[wasm_bindgen(js_name = "privateKeyWif")] private_key_wif: String,
        #[wasm_bindgen(js_name = "keyId")] key_id: Option<u32>,
    ) -> Result<JsValue, WasmSdkError> {
        let sdk = self.inner_clone();

        // Parse identifiers
        let sender_identifier = identifier_from_js(&sender_id, "sender ID")?;

        let recipient_identifier = identifier_from_js(&recipient_id, "recipient ID")?;

        let sender_base58 = sender_identifier.to_string(Encoding::Base58);
        let recipient_base58 = recipient_identifier.to_string(Encoding::Base58);

        // Validate not sending to self
        if sender_identifier == recipient_identifier {
            return Err(WasmSdkError::invalid_argument(
                "Cannot transfer credits to yourself",
            ));
        }

        // Validate amount
        if amount == 0 {
            return Err(WasmSdkError::invalid_argument(
                "Transfer amount must be greater than 0",
            ));
        }

        // Fetch sender identity
        let sender_identity = dash_sdk::platform::Identity::fetch(&sdk, sender_identifier)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Sender identity not found"))?;

        // Parse private key and find matching public key
        let private_key_bytes = dash_sdk::dpp::dashcore::PrivateKey::from_wif(&private_key_wif)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid private key: {}", e)))?
            .inner
            .secret_bytes();

        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(
            &private_key_bytes,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid secret key: {}", e)))?;
        let public_key =
            dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize();

        // Create public key hash using hash160
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{hash160, Hash};
            hash160::Hash::hash(&public_key_bytes[..])
                .to_byte_array()
                .to_vec()
        };

        // Find matching key - prioritize key_id if provided, otherwise find any matching key
        // Supports ECDSA_SECP256K1 and ECDSA_HASH160 key types
        let matching_key = if let Some(requested_key_id) = key_id {
            // Find specific key by ID
            sender_identity
                .public_keys()
                .get(&requested_key_id)
                .filter(|key| {
                    key.purpose() == Purpose::TRANSFER
                        && ecdsa_public_key_matches_identity_key(
                            &public_key_bytes,
                            &public_key_hash160,
                            key,
                        )
                })
                .ok_or_else(|| {
                    WasmSdkError::not_found(format!(
                        "Key with ID {} not found or doesn't match private key",
                        requested_key_id
                    ))
                })?
        } else {
            // Find any matching transfer key
            sender_identity
                .public_keys()
                .iter()
                .find(|(_, key)| {
                    key.purpose() == Purpose::TRANSFER
                        && ecdsa_public_key_matches_identity_key(
                            &public_key_bytes,
                            &public_key_hash160,
                            key,
                        )
                })
                .map(|(_, key)| key)
                .ok_or_else(|| {
                    WasmSdkError::not_found(
                        "No matching transfer key found for the provided private key",
                    )
                })?
        };

        // Get identity nonce
        let identity_nonce = sdk
            .get_identity_nonce(sender_identifier, true, None)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to get identity nonce: {}", e)))?;

        // Create signer
        let signer = SingleKeySigner::from_string(&private_key_wif, self.network())
            .map_err(WasmSdkError::invalid_argument)?;

        // Create the credit transfer transition
        let state_transition = IdentityCreditTransferTransition::try_from_identity(
            &sender_identity,
            recipient_identifier,
            amount,
            UserFeeIncrease::default(),
            signer,
            Some(matching_key),
            identity_nonce,
            sdk.version(),
            None,
        )
        .map_err(|e| {
            WasmSdkError::generic(format!("Failed to create transfer transition: {}", e))
        })?;

        // Broadcast the transition
        use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
        let _result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast transfer: {}", e)))?;

        // Create JavaScript result object
        let result_obj = js_sys::Object::new();

        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("status"),
            &JsValue::from_str("success"),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set status: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("senderId"),
            &JsValue::from_str(&sender_base58),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set senderId: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("recipientId"),
            &JsValue::from_str(&recipient_base58),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set recipientId: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("amount"),
            &JsValue::from_f64(amount as f64),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set amount: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("message"),
            &JsValue::from_str("Credits transferred successfully"),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set message: {:?}", e)))?;

        Ok(result_obj.into())
    }

    /// Withdraw credits from an identity to a Dash address.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity ID to withdraw from
    /// * `to_address` - The Dash address to send the withdrawn credits to
    /// * `amount` - The amount of credits to withdraw
    /// * `core_fee_per_byte` - Optional core fee per byte (defaults to 1)
    /// * `private_key_wif` - The private key in WIF format for signing
    /// * `key_id` - Optional key ID to use for signing (if None, will auto-select)
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the withdrawal result
    #[wasm_bindgen(js_name = identityCreditWithdrawal)]
    pub async fn identity_credit_withdrawal(
        &self,
        #[wasm_bindgen(js_name = "identityId")]
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        identity_id: JsValue,
        #[wasm_bindgen(js_name = "toAddress")] to_address: String,
        amount: u64,
        #[wasm_bindgen(js_name = "coreFeePerByte")] core_fee_per_byte: Option<u32>,
        #[wasm_bindgen(js_name = "privateKeyWif")] private_key_wif: String,
        #[wasm_bindgen(js_name = "keyId")] key_id: Option<u32>,
    ) -> Result<JsValue, WasmSdkError> {
        let sdk = self.inner_clone();

        // Parse identity identifier
        let identifier = identifier_from_js(&identity_id, "identity ID")?;
        let identity_base58 = identifier.to_string(Encoding::Base58);

        // Parse the Dash address
        use dash_sdk::dpp::dashcore::Address;
        use std::str::FromStr;
        let address = Address::from_str(&to_address)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid Dash address: {}", e)))?
            .assume_checked();

        // Validate amount
        if amount == 0 {
            return Err(WasmSdkError::invalid_argument(
                "Withdrawal amount must be greater than 0",
            ));
        }

        // Fetch the identity
        let identity = dash_sdk::platform::Identity::fetch(&sdk, identifier)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Identity not found"))?;

        // Parse private key and find matching public key
        let private_key_bytes = dash_sdk::dpp::dashcore::PrivateKey::from_wif(&private_key_wif)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid private key: {}", e)))?
            .inner
            .secret_bytes();

        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(
            &private_key_bytes,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid secret key: {}", e)))?;
        let public_key =
            dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize();

        // Create public key hash using hash160
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{hash160, Hash};
            hash160::Hash::hash(&public_key_bytes[..])
                .to_byte_array()
                .to_vec()
        };

        // Find matching key - prioritize key_id if provided, otherwise find any matching key
        // For withdrawals, we can use either TRANSFER or OWNER keys
        // Supports ECDSA_SECP256K1 and ECDSA_HASH160 key types
        let matching_key = if let Some(requested_key_id) = key_id {
            // Find specific key by ID
            identity
                .public_keys()
                .get(&requested_key_id)
                .filter(|key| {
                    (key.purpose() == Purpose::TRANSFER || key.purpose() == Purpose::OWNER)
                        && ecdsa_public_key_matches_identity_key(
                            &public_key_bytes,
                            &public_key_hash160,
                            key,
                        )
                })
                .ok_or_else(|| {
                    WasmSdkError::not_found(format!(
                        "Key with ID {} not found or doesn't match private key",
                        requested_key_id
                    ))
                })?
        } else {
            // Find any matching withdrawal-capable key (prefer TRANSFER keys)
            identity
                .public_keys()
                .iter()
                .find(|(_, key)| {
                    key.purpose() == Purpose::TRANSFER
                        && ecdsa_public_key_matches_identity_key(
                            &public_key_bytes,
                            &public_key_hash160,
                            key,
                        )
                })
                .or_else(|| {
                    identity.public_keys().iter().find(|(_, key)| {
                        key.purpose() == Purpose::OWNER
                            && ecdsa_public_key_matches_identity_key(
                                &public_key_bytes,
                                &public_key_hash160,
                                key,
                            )
                    })
                })
                .map(|(_, key)| key)
                .ok_or_else(|| {
                    WasmSdkError::not_found(
                        "No matching withdrawal key found for the provided private key",
                    )
                })?
        };

        // Create signer
        let signer = SingleKeySigner::from_string(&private_key_wif, self.network())
            .map_err(WasmSdkError::invalid_argument)?;

        // Import the withdraw trait
        use dash_sdk::platform::transition::withdraw_from_identity::WithdrawFromIdentity;

        // Perform the withdrawal
        let remaining_balance = identity
            .withdraw(
                &sdk,
                Some(address),
                amount,
                core_fee_per_byte,
                Some(matching_key),
                signer,
                None,
            )
            .await
            .map_err(|e| WasmSdkError::generic(format!("Withdrawal failed: {}", e)))?;

        // Create JavaScript result object
        let result_obj = js_sys::Object::new();

        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("status"),
            &JsValue::from_str("success"),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set status: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("identityId"),
            &JsValue::from_str(&identity_base58),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set identityId: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("toAddress"),
            &JsValue::from_str(&to_address),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set toAddress: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("amount"),
            &JsValue::from_f64(amount as f64),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set amount: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("remainingBalance"),
            &JsValue::from_f64(remaining_balance as f64),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set remainingBalance: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("message"),
            &JsValue::from_str("Credits withdrawn successfully"),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set message: {:?}", e)))?;

        Ok(result_obj.into())
    }

    /// Update an identity by adding or disabling public keys.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity ID to update
    /// * `add_public_keys` - JSON array of public keys to add
    /// * `disable_public_keys` - Array of key IDs to disable
    /// * `private_key_wif` - The private key in WIF format for signing (must be a master key)
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the update result
    #[wasm_bindgen(js_name = identityUpdate)]
    pub async fn identity_update(
        &self,
        #[wasm_bindgen(js_name = "identityId")]
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        identity_id: JsValue,
        #[wasm_bindgen(js_name = "addPublicKeys")] add_public_keys: Option<String>,
        #[wasm_bindgen(js_name = "disablePublicKeys")] disable_public_keys: Option<Vec<u32>>,
        #[wasm_bindgen(js_name = "privateKeyWif")] private_key_wif: String,
    ) -> Result<JsValue, WasmSdkError> {
        let sdk = self.inner_clone();

        // Parse identity identifier
        let identifier = identifier_from_js(&identity_id, "identity ID")?;
        let identity_base58 = identifier.to_string(Encoding::Base58);

        // Fetch the identity
        let identity = dash_sdk::platform::Identity::fetch(&sdk, identifier)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Identity not found"))?;

        // Get current revision
        let current_revision = identity.revision();

        // Parse private key and verify it's a master key
        let private_key = PrivateKey::from_wif(&private_key_wif)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid private key: {}", e)))?;

        // Create public key hash to find matching master key
        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(
            &private_key.inner.secret_bytes(),
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid secret key: {}", e)))?;
        let public_key =
            dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize();

        // Create public key hash using hash160
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{hash160, Hash};
            hash160::Hash::hash(&public_key_bytes[..])
                .to_byte_array()
                .to_vec()
        };

        // Find matching master key (supports ECDSA_SECP256K1 and ECDSA_HASH160)
        let master_key = identity
            .public_keys()
            .iter()
            .find(|(_, key)| {
                key.purpose() == Purpose::AUTHENTICATION
                    && key.security_level() == SecurityLevel::MASTER
                    && ecdsa_public_key_matches_identity_key(
                        &public_key_bytes,
                        &public_key_hash160,
                        key,
                    )
            })
            .map(|(id, _)| *id)
            .ok_or_else(|| {
                WasmSdkError::invalid_argument("Provided private key does not match any master key")
            })?;

        // Create signer with SimpleSigner to support multiple keys (including new signing keys)
        let mut signer = SimpleSigner::default();

        // Get the master public key object and add it to the signer
        let master_public_key = identity
            .public_keys()
            .get(&master_key)
            .ok_or_else(|| WasmSdkError::invalid_argument("Master key not found"))?
            .clone();

        signer.add_identity_public_key(master_public_key, private_key.inner.secret_bytes());

        // Parse and prepare keys to add
        let keys_to_add: Vec<IdentityPublicKey> = if let Some(keys_json) = add_public_keys {
            let next_key_id = identity.public_keys().keys().max().copied().unwrap_or(0) + 1;
            parse_keys_for_identity_update(&keys_json, next_key_id, &mut signer, self.network())?
        } else {
            Vec::new()
        };

        // Get keys to disable
        let keys_to_disable = disable_public_keys.unwrap_or_default();

        // Save counts before moving
        let added_keys_count = keys_to_add.len();
        let disabled_keys_count = keys_to_disable.len();

        // Validate keys to disable (cannot disable master, critical auth, or transfer keys)
        for key_id in &keys_to_disable {
            if let Some(key) = identity.public_keys().get(key_id) {
                if key.security_level() == SecurityLevel::MASTER {
                    return Err(WasmSdkError::invalid_argument(format!(
                        "Cannot disable master key {}",
                        key_id
                    )));
                }
                if key.purpose() == Purpose::AUTHENTICATION
                    && key.security_level() == SecurityLevel::CRITICAL
                    && key.key_type() == KeyType::ECDSA_SECP256K1
                {
                    return Err(WasmSdkError::invalid_argument(format!(
                        "Cannot disable critical authentication key {}",
                        key_id
                    )));
                }
                if key.purpose() == Purpose::TRANSFER {
                    return Err(WasmSdkError::invalid_argument(format!(
                        "Cannot disable transfer key {}",
                        key_id
                    )));
                }
            } else {
                return Err(WasmSdkError::not_found(format!("Key {} not found", key_id)));
            }
        }

        // Get identity nonce
        let identity_nonce = sdk
            .get_identity_nonce(identifier, true, None)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to get identity nonce: {}", e)))?;

        // Create the identity update transition
        use dash_sdk::dpp::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
        use dash_sdk::dpp::state_transition::identity_update_transition::IdentityUpdateTransition;

        let state_transition = IdentityUpdateTransition::try_from_identity_with_signer(
            &identity,
            &master_key,
            keys_to_add,
            keys_to_disable,
            identity_nonce,
            UserFeeIncrease::default(),
            &signer,
            sdk.version(),
            None,
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to create update transition: {}", e)))?;

        // Broadcast the transition
        use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
        let result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast update: {}", e)))?;

        // Extract updated identity from result
        let updated_revision = match result {
            StateTransitionProofResult::VerifiedIdentity(updated_identity) => {
                updated_identity.revision()
            }
            StateTransitionProofResult::VerifiedPartialIdentity(partial_identity) => {
                partial_identity.revision.unwrap_or(current_revision + 1)
            }
            _ => current_revision + 1,
        };

        // Create JavaScript result object
        let result_obj = js_sys::Object::new();

        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("status"),
            &JsValue::from_str("success"),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set status: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("identityId"),
            &JsValue::from_str(&identity_base58),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set identityId: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("revision"),
            &JsValue::from_f64(updated_revision as f64),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set revision: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("addedKeys"),
            &JsValue::from_f64(added_keys_count as f64),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set addedKeys: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("disabledKeys"),
            &JsValue::from_f64(disabled_keys_count as f64),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set disabledKeys: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("message"),
            &JsValue::from_str("Identity updated successfully"),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set message: {:?}", e)))?;

        Ok(result_obj.into())
    }

    /// Submit a masternode vote for a contested resource.
    ///
    /// # Arguments
    ///
    /// * `pro_tx_hash` - The ProTxHash of the masternode
    /// * `contract_id` - The data contract ID containing the contested resource
    /// * `document_type_name` - The document type name (e.g., "domain")
    /// * `index_name` - The index name (e.g., "parentNameAndLabel")
    /// * `index_values` - JSON array of index values (e.g., ["dash", "username"])
    /// * `vote_choice` - The vote choice: "towardsIdentity:<identity_id>", "abstain", or "lock"
    /// * `private_key_wif` - The masternode voting key in WIF format
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the vote result
    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(js_name = masternodeVote)]
    pub async fn masternode_vote(
        &self,
        #[wasm_bindgen(js_name = "masternodeProTxHash")]
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        masternode_pro_tx_hash: JsValue,
        #[wasm_bindgen(js_name = "contractId")]
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        contract_id: JsValue,
        #[wasm_bindgen(js_name = "documentTypeName")] document_type_name: String,
        #[wasm_bindgen(js_name = "indexName")] index_name: String,
        #[wasm_bindgen(js_name = "indexValues")] index_values: String,
        #[wasm_bindgen(js_name = "voteChoice")] vote_choice: String,
        #[wasm_bindgen(js_name = "votingKeyWif")] voting_key_wif: String,
    ) -> Result<JsValue, WasmSdkError> {
        let sdk = self.inner_clone();

        // Parse ProTxHash
        let pro_tx_hash = identifier_from_js(&masternode_pro_tx_hash, "ProTxHash")?;
        let pro_tx_hash_base58 = pro_tx_hash.to_string(Encoding::Base58);

        // Parse contract ID
        let data_contract_id = identifier_from_js(&contract_id, "contract ID")?;
        let contract_id_base58 = data_contract_id.to_string(Encoding::Base58);

        // Parse index values from JSON
        let index_values_json: serde_json::Value =
            serde_json::from_str(&index_values).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid index values JSON: {}", e))
            })?;

        let index_values_array = index_values_json
            .as_array()
            .ok_or_else(|| WasmSdkError::invalid_argument("index_values must be a JSON array"))?;

        let index_values_vec: Vec<dash_sdk::dpp::platform_value::Value> = index_values_array
            .iter()
            .map(|v| match v {
                serde_json::Value::String(s) => {
                    Ok(dash_sdk::dpp::platform_value::Value::Text(s.clone()))
                }
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Ok(dash_sdk::dpp::platform_value::Value::I64(i))
                    } else if let Some(u) = n.as_u64() {
                        Ok(dash_sdk::dpp::platform_value::Value::U64(u))
                    } else {
                        Ok(dash_sdk::dpp::platform_value::Value::Float(
                            n.as_f64().unwrap(),
                        ))
                    }
                }
                serde_json::Value::Bool(b) => Ok(dash_sdk::dpp::platform_value::Value::Bool(*b)),
                _ => Err(WasmSdkError::invalid_argument(
                    "Unsupported index value type",
                )),
            })
            .collect::<Result<Vec<_>, WasmSdkError>>()?;

        // Parse vote choice
        use dash_sdk::dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
        let resource_vote_choice = if vote_choice == "abstain" {
            ResourceVoteChoice::Abstain
        } else if vote_choice == "lock" {
            ResourceVoteChoice::Lock
        } else if vote_choice.starts_with("towardsIdentity:") {
            let identity_id_str = vote_choice
                .strip_prefix("towardsIdentity:")
                .ok_or_else(|| WasmSdkError::invalid_argument("Invalid vote choice format"))?;
            let identity_id =
                Identifier::from_string(identity_id_str, Encoding::Base58).map_err(|e| {
                    WasmSdkError::invalid_argument(format!(
                        "Invalid identity ID in vote choice: {}",
                        e
                    ))
                })?;
            ResourceVoteChoice::TowardsIdentity(identity_id)
        } else {
            return Err(
                WasmSdkError::invalid_argument(
                    "Invalid vote choice. Must be 'abstain', 'lock', or 'towardsIdentity:<identity_id>'",
                ),
            );
        };

        // Parse private key (try WIF first, then hex)
        let private_key = if voting_key_wif.len() == 64
            && voting_key_wif.chars().all(|c| c.is_ascii_hexdigit())
        {
            // Looks like hex
            let key_bytes = hex::decode(&voting_key_wif).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid hex private key: {}", e))
            })?;
            if key_bytes.len() != 32 {
                return Err(WasmSdkError::invalid_argument(
                    "Private key must be 32 bytes",
                ));
            }
            let key_array: [u8; 32] = key_bytes
                .as_slice()
                .try_into()
                .map_err(|_| WasmSdkError::invalid_argument("Private key must be 32 bytes"))?;
            PrivateKey::from_byte_array(&key_array, self.network()).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid private key bytes: {}", e))
            })?
        } else {
            // Try WIF
            PrivateKey::from_wif(&voting_key_wif).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid WIF private key: {}", e))
            })?
        };

        // Create the voting public key from the private key
        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(
            &private_key.inner.secret_bytes(),
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid secret key: {}", e)))?;
        let public_key =
            dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize();

        // Create voting public key hash using hash160
        let voting_key_hash = {
            use dash_sdk::dpp::dashcore::hashes::{hash160, Hash};
            hash160::Hash::hash(&public_key_bytes[..])
                .to_byte_array()
                .to_vec()
        };

        // Create the voting identity public key
        use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        let voting_public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::VOTING,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(voting_key_hash),
            disabled_at: None,
        });

        // Create the contested document resource vote poll
        use dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
        let vote_poll =
            dash_sdk::dpp::voting::vote_polls::VotePoll::ContestedDocumentResourceVotePoll(
                ContestedDocumentResourceVotePoll {
                    contract_id: data_contract_id,
                    document_type_name: document_type_name.clone(),
                    index_name: index_name.clone(),
                    index_values: index_values_vec,
                },
            );

        // Create the resource vote
        use dash_sdk::dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
        use dash_sdk::dpp::voting::votes::resource_vote::ResourceVote;
        let resource_vote = ResourceVote::V0(ResourceVoteV0 {
            vote_poll,
            resource_vote_choice,
        });

        // Create the vote
        use dash_sdk::dpp::voting::votes::Vote;
        let vote = Vote::ResourceVote(resource_vote);

        // Create signer
        let signer = SingleKeySigner::from_string(&voting_key_wif, self.network())
            .map_err(WasmSdkError::invalid_argument)?;

        // Submit the vote using PutVote trait
        use dash_sdk::platform::transition::vote::PutVote;

        vote.put_to_platform(pro_tx_hash, &voting_public_key, &sdk, &signer, None)
            .await?;

        // Create JavaScript result object
        let result_obj = js_sys::Object::new();

        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("status"),
            &JsValue::from_str("success"),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set status: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("proTxHash"),
            &JsValue::from_str(&pro_tx_hash_base58),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set proTxHash: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("contractId"),
            &JsValue::from_str(&contract_id_base58),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set contractId: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("documentType"),
            &JsValue::from_str(&document_type_name),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set documentType: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("voteChoice"),
            &JsValue::from_str(&vote_choice),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set voteChoice: {:?}", e)))?;
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("message"),
            &JsValue::from_str("Vote submitted successfully"),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set message: {:?}", e)))?;

        Ok(result_obj.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;

    /// Creates a test public key with the given key type and data
    fn create_test_key(key_type: KeyType, data: Vec<u8>) -> IdentityPublicKey {
        IdentityPublicKeyV0 {
            id: 0u32.into(),
            key_type,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            read_only: false,
            disabled_at: None,
            data: data.into(),
            contract_bounds: None,
        }
        .into()
    }

    #[test]
    fn test_private_key_matches_ecdsa_secp256k1() {
        // 33-byte compressed public key (typical ECDSA_SECP256K1 key)
        let public_key_bytes: [u8; 33] = [
            0x02, 0x10, 0x85, 0x8c, 0xce, 0x1f, 0x12, 0xbe, 0x35, 0xd1, 0x3c, 0x65, 0xba, 0xec,
            0x1d, 0xcd, 0x75, 0x64, 0xd7, 0x53, 0xf0, 0x31, 0x10, 0x8e, 0xb2, 0x7b, 0x53, 0x8e,
            0xe2, 0xd0, 0x74, 0xdb, 0x6b,
        ];

        // Hash160 of the public key (20 bytes)
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{hash160, Hash};
            hash160::Hash::hash(&public_key_bytes[..])
                .to_byte_array()
                .to_vec()
        };

        // Create ECDSA_SECP256K1 key with 33-byte data
        let secp_key = create_test_key(KeyType::ECDSA_SECP256K1, public_key_bytes.to_vec());

        // Should match when comparing full public key bytes
        assert!(
            ecdsa_public_key_matches_identity_key(
                &public_key_bytes,
                &public_key_hash160,
                &secp_key
            ),
            "ECDSA_SECP256K1 key should match with correct public key bytes"
        );

        // Should not match with wrong public key
        let wrong_bytes = [0u8; 33];
        assert!(
            !ecdsa_public_key_matches_identity_key(&wrong_bytes, &public_key_hash160, &secp_key),
            "ECDSA_SECP256K1 key should not match with wrong public key bytes"
        );
    }

    #[test]
    fn test_private_key_matches_ecdsa_hash160() {
        // 33-byte compressed public key
        let public_key_bytes: [u8; 33] = [
            0x02, 0x10, 0x85, 0x8c, 0xce, 0x1f, 0x12, 0xbe, 0x35, 0xd1, 0x3c, 0x65, 0xba, 0xec,
            0x1d, 0xcd, 0x75, 0x64, 0xd7, 0x53, 0xf0, 0x31, 0x10, 0x8e, 0xb2, 0x7b, 0x53, 0x8e,
            0xe2, 0xd0, 0x74, 0xdb, 0x6b,
        ];

        // Hash160 of the public key (20 bytes)
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{hash160, Hash};
            hash160::Hash::hash(&public_key_bytes[..])
                .to_byte_array()
                .to_vec()
        };

        // Create ECDSA_HASH160 key with 20-byte hash data
        let hash160_key = create_test_key(KeyType::ECDSA_HASH160, public_key_hash160.clone());

        // Should match when comparing hash160
        assert!(
            ecdsa_public_key_matches_identity_key(
                &public_key_bytes,
                &public_key_hash160,
                &hash160_key
            ),
            "ECDSA_HASH160 key should match with correct hash160"
        );

        // Should not match with wrong hash
        let wrong_hash = [0u8; 20].to_vec();
        assert!(
            !ecdsa_public_key_matches_identity_key(&public_key_bytes, &wrong_hash, &hash160_key),
            "ECDSA_HASH160 key should not match with wrong hash160"
        );
    }

    #[test]
    fn test_private_key_does_not_match_bls_keys() {
        // 33-byte compressed public key
        let public_key_bytes: [u8; 33] = [
            0x02, 0x10, 0x85, 0x8c, 0xce, 0x1f, 0x12, 0xbe, 0x35, 0xd1, 0x3c, 0x65, 0xba, 0xec,
            0x1d, 0xcd, 0x75, 0x64, 0xd7, 0x53, 0xf0, 0x31, 0x10, 0x8e, 0xb2, 0x7b, 0x53, 0x8e,
            0xe2, 0xd0, 0x74, 0xdb, 0x6b,
        ];

        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{hash160, Hash};
            hash160::Hash::hash(&public_key_bytes[..])
                .to_byte_array()
                .to_vec()
        };

        // BLS keys require separate BLS key derivation, so ECDSA private keys should never match
        let bls_key = create_test_key(KeyType::BLS12_381, vec![0u8; 48]);

        assert!(
            !ecdsa_public_key_matches_identity_key(
                &public_key_bytes,
                &public_key_hash160,
                &bls_key
            ),
            "BLS12_381 key should not match with ECDSA private key"
        );
    }

    #[test]
    fn test_private_key_does_not_match_other_key_types() {
        let public_key_bytes: [u8; 33] = [0x02; 33];
        let public_key_hash160 = vec![0u8; 20];

        // BIP13_SCRIPT_HASH should not match
        let script_hash_key = create_test_key(KeyType::BIP13_SCRIPT_HASH, vec![0u8; 20]);
        assert!(
            !ecdsa_public_key_matches_identity_key(
                &public_key_bytes,
                &public_key_hash160,
                &script_hash_key
            ),
            "BIP13_SCRIPT_HASH key should not match with ECDSA private key"
        );

        // EDDSA_25519_HASH160 should not match
        let eddsa_key = create_test_key(KeyType::EDDSA_25519_HASH160, vec![0u8; 20]);
        assert!(
            !ecdsa_public_key_matches_identity_key(
                &public_key_bytes,
                &public_key_hash160,
                &eddsa_key
            ),
            "EDDSA_25519_HASH160 key should not match with ECDSA private key"
        );
    }

    #[test]
    fn test_regression_secp256k1_key_not_mistaken_for_hash160() {
        // This is the regression test for the original bug:
        // identity_update was only checking for ECDSA_HASH160 keys,
        // so ECDSA_SECP256K1 keys would never match even with the correct private key.

        // Real public key bytes (33-byte compressed secp256k1)
        let public_key_bytes: [u8; 33] = [
            0x02, 0x10, 0x85, 0x8c, 0xce, 0x1f, 0x12, 0xbe, 0x35, 0xd1, 0x3c, 0x65, 0xba, 0xec,
            0x1d, 0xcd, 0x75, 0x64, 0xd7, 0x53, 0xf0, 0x31, 0x10, 0x8e, 0xb2, 0x7b, 0x53, 0x8e,
            0xe2, 0xd0, 0x74, 0xdb, 0x6b,
        ];

        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{hash160, Hash};
            hash160::Hash::hash(&public_key_bytes[..])
                .to_byte_array()
                .to_vec()
        };

        // The bug: Code was only checking ECDSA_HASH160 keys
        // But most identities (DashPay) use ECDSA_SECP256K1 with the full 33-byte key
        let master_key = create_test_key(KeyType::ECDSA_SECP256K1, public_key_bytes.to_vec());

        // Before fix: This would return false because the code only checked ECDSA_HASH160
        // After fix: This correctly returns true
        assert!(
            ecdsa_public_key_matches_identity_key(
                &public_key_bytes,
                &public_key_hash160,
                &master_key
            ),
            "REGRESSION: ECDSA_SECP256K1 master key must be matchable with the correct private key"
        );
    }

    // Tests for parse_keys_for_identity_update helper function
    // These test the fix for: identity_update cannot add ECDSA_SECP256K1 keys

    #[test]
    fn test_parse_keys_ecdsa_secp256k1_with_private_key_hex() {
        use dash_sdk::dpp::dashcore::Network;

        let mut signer = SimpleSigner::default();
        let keys_json = r#"[{
            "keyType": "ECDSA_SECP256K1",
            "purpose": "AUTHENTICATION",
            "securityLevel": "HIGH",
            "privateKeyHex": "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"
        }]"#;

        let keys = parse_keys_for_identity_update(keys_json, 1, &mut signer, Network::Testnet)
            .expect("Should parse ECDSA_SECP256K1 key with privateKeyHex");

        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].key_type(), KeyType::ECDSA_SECP256K1);
        assert_eq!(keys[0].data().len(), 33); // Compressed public key
        assert_eq!(keys[0].id(), 1);
        assert_eq!(keys[0].purpose(), Purpose::AUTHENTICATION);
        assert_eq!(keys[0].security_level(), SecurityLevel::HIGH);

        // Key should be added to signer (is_unique_key_type == true)
        assert_eq!(signer.private_keys.len(), 1);
        assert!(signer.private_keys.contains_key(&keys[0]));
    }

    #[test]
    fn test_parse_keys_ecdsa_secp256k1_with_private_key_wif() {
        use dash_sdk::dpp::dashcore::{Network, PrivateKey};

        // Generate valid WIF from known private key bytes
        let private_key_bytes: [u8; 32] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
            0x1d, 0x1e, 0x1f, 0x20,
        ];
        let private_key = PrivateKey::from_byte_array(&private_key_bytes, Network::Testnet)
            .expect("Valid private key bytes");
        let wif = private_key.to_wif();

        let mut signer = SimpleSigner::default();
        let keys_json = format!(
            r#"[{{
            "keyType": "ECDSA_SECP256K1",
            "purpose": "AUTHENTICATION",
            "securityLevel": "HIGH",
            "privateKeyWif": "{}"
        }}]"#,
            wif
        );

        let keys = parse_keys_for_identity_update(&keys_json, 5, &mut signer, Network::Testnet)
            .expect("Should parse ECDSA_SECP256K1 key with privateKeyWif");

        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].key_type(), KeyType::ECDSA_SECP256K1);
        assert_eq!(keys[0].data().len(), 33);
        assert_eq!(keys[0].id(), 5);

        // Key should be added to signer
        assert_eq!(signer.private_keys.len(), 1);
        assert!(signer.private_keys.contains_key(&keys[0]));
    }

    #[test]
    fn test_parse_keys_ecdsa_secp256k1_missing_private_key_fails() {
        use dash_sdk::dpp::dashcore::Network;

        let mut signer = SimpleSigner::default();
        let keys_json = r#"[{
            "keyType": "ECDSA_SECP256K1",
            "purpose": "AUTHENTICATION",
            "securityLevel": "HIGH"
        }]"#;

        let result = parse_keys_for_identity_update(keys_json, 1, &mut signer, Network::Testnet);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("privateKeyHex or privateKeyWif"),
            "Error should mention required fields"
        );
    }

    #[test]
    fn test_parse_keys_bls12_381_with_private_key_hex() {
        use dash_sdk::dpp::dashcore::Network;

        let mut signer = SimpleSigner::default();
        let keys_json = r#"[{
            "keyType": "BLS12_381",
            "purpose": "VOTING",
            "securityLevel": "HIGH",
            "privateKeyHex": "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"
        }]"#;

        let keys = parse_keys_for_identity_update(keys_json, 1, &mut signer, Network::Testnet)
            .expect("Should parse BLS12_381 key with privateKeyHex");

        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].key_type(), KeyType::BLS12_381);
        assert_eq!(keys[0].data().len(), 48); // BLS public key is 48 bytes

        // Key should be added to signer (is_unique_key_type == true)
        assert_eq!(signer.private_keys.len(), 1);
        assert!(signer.private_keys.contains_key(&keys[0]));
    }

    #[test]
    fn test_parse_keys_bls12_381_rejects_wif() {
        use dash_sdk::dpp::dashcore::Network;

        let mut signer = SimpleSigner::default();
        let keys_json = r#"[{
            "keyType": "BLS12_381",
            "purpose": "VOTING",
            "securityLevel": "HIGH",
            "privateKeyWif": "cNurmMT7bXe3S92JijPi2V5wSHvWzrk4vKJ7fgLPNo5Ajv2YAP3z"
        }]"#;

        let result = parse_keys_for_identity_update(keys_json, 1, &mut signer, Network::Testnet);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("do not support WIF format"),
            "Error should indicate WIF is not supported for BLS"
        );
    }

    #[test]
    fn test_parse_keys_ecdsa_hash160_not_added_to_signer() {
        use dash_sdk::dpp::dashcore::Network;

        let mut signer = SimpleSigner::default();
        let keys_json = r#"[{
            "keyType": "ECDSA_HASH160",
            "purpose": "TRANSFER",
            "securityLevel": "HIGH",
            "privateKeyHex": "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"
        }]"#;

        let keys = parse_keys_for_identity_update(keys_json, 1, &mut signer, Network::Testnet)
            .expect("Should parse ECDSA_HASH160 key");

        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].key_type(), KeyType::ECDSA_HASH160);
        assert_eq!(keys[0].data().len(), 20); // Hash160 is 20 bytes

        // Key should NOT be added to signer (is_unique_key_type == false)
        assert_eq!(signer.private_keys.len(), 0);
    }

    #[test]
    fn test_parse_keys_ecdsa_hash160_with_data_field() {
        use dash_sdk::dpp::dashcore::Network;

        let mut signer = SimpleSigner::default();
        // Base64-encoded 20-byte hash (20 bytes = 27 base64 chars + 1 padding)
        let keys_json = r#"[{
            "keyType": "ECDSA_HASH160",
            "purpose": "TRANSFER",
            "securityLevel": "HIGH",
            "data": "AAAAAAAAAAAAAAAAAAAAAAAAAAA="
        }]"#;

        let keys = parse_keys_for_identity_update(keys_json, 1, &mut signer, Network::Testnet)
            .expect("Should parse ECDSA_HASH160 with data field");

        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].key_type(), KeyType::ECDSA_HASH160);
        assert_eq!(keys[0].data().len(), 20);

        // Key should NOT be added to signer
        assert_eq!(signer.private_keys.len(), 0);
    }

    #[test]
    fn test_parse_keys_multiple_keys_mixed_types() {
        use dash_sdk::dpp::dashcore::Network;

        let mut signer = SimpleSigner::default();
        let keys_json = r#"[
            {
                "keyType": "ECDSA_SECP256K1",
                "purpose": "AUTHENTICATION",
                "securityLevel": "HIGH",
                "privateKeyHex": "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"
            },
            {
                "keyType": "ECDSA_HASH160",
                "purpose": "TRANSFER",
                "securityLevel": "HIGH",
                "privateKeyHex": "2102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f21"
            }
        ]"#;

        let keys = parse_keys_for_identity_update(keys_json, 10, &mut signer, Network::Testnet)
            .expect("Should parse multiple keys");

        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].id(), 10);
        assert_eq!(keys[1].id(), 11);
        assert_eq!(keys[0].key_type(), KeyType::ECDSA_SECP256K1);
        assert_eq!(keys[1].key_type(), KeyType::ECDSA_HASH160);

        // Only ECDSA_SECP256K1 should be in signer
        assert_eq!(signer.private_keys.len(), 1);
        assert!(signer.private_keys.contains_key(&keys[0]));
        assert!(!signer.private_keys.contains_key(&keys[1]));
    }

    #[test]
    fn test_parse_keys_empty_array() {
        use dash_sdk::dpp::dashcore::Network;

        let mut signer = SimpleSigner::default();
        let keys_json = "[]";

        let keys = parse_keys_for_identity_update(keys_json, 1, &mut signer, Network::Testnet)
            .expect("Should handle empty array");

        assert_eq!(keys.len(), 0);
        assert_eq!(signer.private_keys.len(), 0);
    }

    #[test]
    fn test_parse_keys_invalid_json_fails() {
        use dash_sdk::dpp::dashcore::Network;

        let mut signer = SimpleSigner::default();
        let keys_json = "not valid json";

        let result = parse_keys_for_identity_update(keys_json, 1, &mut signer, Network::Testnet);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_keys_invalid_private_key_hex_fails() {
        use dash_sdk::dpp::dashcore::Network;

        let mut signer = SimpleSigner::default();
        let keys_json = r#"[{
            "keyType": "ECDSA_SECP256K1",
            "purpose": "AUTHENTICATION",
            "securityLevel": "HIGH",
            "privateKeyHex": "not_valid_hex"
        }]"#;

        let result = parse_keys_for_identity_update(keys_json, 1, &mut signer, Network::Testnet);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_keys_wrong_length_private_key_fails() {
        use dash_sdk::dpp::dashcore::Network;

        let mut signer = SimpleSigner::default();
        // Only 16 bytes instead of 32
        let keys_json = r#"[{
            "keyType": "ECDSA_SECP256K1",
            "purpose": "AUTHENTICATION",
            "securityLevel": "HIGH",
            "privateKeyHex": "0102030405060708090a0b0c0d0e0f10"
        }]"#;

        let result = parse_keys_for_identity_update(keys_json, 1, &mut signer, Network::Testnet);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("32 bytes"),
            "Error should mention expected length"
        );
    }
}
