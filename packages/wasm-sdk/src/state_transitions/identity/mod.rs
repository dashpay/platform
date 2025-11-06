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
        let asset_lock_proof: AssetLockProof = if asset_lock_proof
            .chars()
            .all(|c| c.is_ascii_hexdigit())
        {
            let asset_lock_proof_bytes = hex::decode(&asset_lock_proof).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid asset lock proof hex: {}", e))
            })?;
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
        let keys_data: serde_json::Value = serde_json::from_str(&public_keys).map_err(|e| {
            WasmSdkError::invalid_argument(format!("Invalid JSON for public_keys: {}", e))
        })?;
        let keys_array = keys_data
            .as_array()
            .ok_or_else(|| WasmSdkError::invalid_argument("public_keys must be a JSON array"))?;
        let mut identity_public_keys = std::collections::BTreeMap::new();
        let mut signer = SimpleSigner::default();
        for (key_id, key_data) in keys_array
            .iter()
            .enumerate()
            .map(|(key, value)| (key as u32, value))
        {
            let key_type_str = key_data["keyType"]
                .as_str()
                .ok_or_else(|| WasmSdkError::invalid_argument("keyType is required"))?;
            let purpose_str = key_data["purpose"]
                .as_str()
                .ok_or_else(|| WasmSdkError::invalid_argument("purpose is required"))?;
            let security_level_str = key_data["securityLevel"].as_str().unwrap_or("HIGH");
            let key_type = match key_type_str {
                "ECDSA_SECP256K1" => KeyType::ECDSA_SECP256K1,
                "BLS12_381" => KeyType::BLS12_381,
                "ECDSA_HASH160" => KeyType::ECDSA_HASH160,
                "BIP13_SCRIPT_HASH" => KeyType::BIP13_SCRIPT_HASH,
                "EDDSA_25519_HASH160" => KeyType::EDDSA_25519_HASH160,
                _ => {
                    return Err(WasmSdkError::invalid_argument(format!(
                        "Unknown key type: {}",
                        key_type_str
                    )));
                }
            };
            let purpose = match purpose_str {
                "AUTHENTICATION" => Purpose::AUTHENTICATION,
                "ENCRYPTION" => Purpose::ENCRYPTION,
                "DECRYPTION" => Purpose::DECRYPTION,
                "TRANSFER" => Purpose::TRANSFER,
                "SYSTEM" => Purpose::SYSTEM,
                "VOTING" => Purpose::VOTING,
                _ => {
                    return Err(WasmSdkError::invalid_argument(format!(
                        "Unknown purpose: {}",
                        purpose_str
                    )));
                }
            };
            let security_level = match security_level_str {
                "MASTER" => SecurityLevel::MASTER,
                "CRITICAL" => SecurityLevel::CRITICAL,
                "HIGH" => SecurityLevel::HIGH,
                "MEDIUM" => SecurityLevel::MEDIUM,
                _ => SecurityLevel::HIGH,
            };
            let (public_key_data, private_key_bytes) = match key_type {
                KeyType::ECDSA_HASH160 => {
                    if let Some(private_key_hex) = key_data["privateKeyHex"].as_str() {
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
                        (derived_data, [0u8; 32])
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
                    let private_key_bytes = if let Some(private_key_hex) =
                        key_data["privateKeyHex"].as_str()
                    {
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
                    if key_data["privateKeyWif"].is_string() {
                        return Err(WasmSdkError::invalid_argument(
                            "BLS12_381 keys do not support WIF format, use privateKeyHex only",
                        ));
                    }
                    let private_key_bytes =
                        if let Some(private_key_hex) = key_data["privateKeyHex"].as_str() {
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
                        "Unsupported key type for identity creation: {}",
                        key_type_str
                    )));
                }
            };
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
            if key_type != KeyType::ECDSA_HASH160 {
                signer.add_key(public_key.clone(), private_key_bytes);
            }
            identity_public_keys.insert(key_id, public_key);
        }
        use dash_sdk::dpp::identity::v0::IdentityV0;
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::random(),
            public_keys: identity_public_keys,
            balance: 0,
            revision: 0,
        });
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
                let error_msg = format!("Failed to create identity: {}", e);
                error!(
                    target : "wasm_sdk", msg = % error_msg, "Identity creation failed"
                );
                return Err(WasmSdkError::generic(error_msg));
            }
        };
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
        let identifier = identifier_from_js(&identity_id, "identity ID")?;
        let identity_base58 = identifier.to_string(Encoding::Base58);
        let asset_lock_proof: AssetLockProof = if asset_lock_proof
            .chars()
            .all(|c| c.is_ascii_hexdigit())
        {
            let asset_lock_proof_bytes = hex::decode(&asset_lock_proof).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid asset lock proof hex: {}", e))
            })?;
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
        let initial_balance = identity.balance();
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
        let sender_identifier = identifier_from_js(&sender_id, "sender ID")?;
        let recipient_identifier = identifier_from_js(&recipient_id, "recipient ID")?;
        let sender_base58 = sender_identifier.to_string(Encoding::Base58);
        let recipient_base58 = recipient_identifier.to_string(Encoding::Base58);
        if sender_identifier == recipient_identifier {
            return Err(WasmSdkError::invalid_argument(
                "Cannot transfer credits to yourself",
            ));
        }
        if amount == 0 {
            return Err(WasmSdkError::invalid_argument(
                "Transfer amount must be greater than 0",
            ));
        }
        let sender_identity = dash_sdk::platform::Identity::fetch(&sdk, sender_identifier)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Sender identity not found"))?;
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
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{hash160, Hash};
            hash160::Hash::hash(&public_key_bytes[..])
                .to_byte_array()
                .to_vec()
        };
        let matching_key = if let Some(requested_key_id) = key_id {
            sender_identity
                .public_keys()
                .get(&requested_key_id)
                .filter(|key| {
                    key.purpose() == Purpose::TRANSFER
                        && key.key_type() == KeyType::ECDSA_HASH160
                        && key.data().as_slice() == public_key_hash160.as_slice()
                })
                .ok_or_else(|| {
                    WasmSdkError::not_found(format!(
                        "Key with ID {} not found or doesn't match private key",
                        requested_key_id
                    ))
                })?
        } else {
            sender_identity
                .public_keys()
                .iter()
                .find(|(_, key)| {
                    key.purpose() == Purpose::TRANSFER
                        && key.key_type() == KeyType::ECDSA_HASH160
                        && key.data().as_slice() == public_key_hash160.as_slice()
                })
                .map(|(_, key)| key)
                .ok_or_else(|| {
                    WasmSdkError::not_found(
                        "No matching transfer key found for the provided private key",
                    )
                })?
        };
        let identity_nonce = sdk
            .get_identity_nonce(sender_identifier, true, None)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to get identity nonce: {}", e)))?;
        let signer = SingleKeySigner::from_string(&private_key_wif, self.network())
            .map_err(WasmSdkError::invalid_argument)?;
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
        use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
        let _result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast transfer: {}", e)))?;
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
        let identifier = identifier_from_js(&identity_id, "identity ID")?;
        let identity_base58 = identifier.to_string(Encoding::Base58);
        use dash_sdk::dpp::dashcore::Address;
        use std::str::FromStr;
        let address = Address::from_str(&to_address)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid Dash address: {}", e)))?
            .assume_checked();
        if amount == 0 {
            return Err(WasmSdkError::invalid_argument(
                "Withdrawal amount must be greater than 0",
            ));
        }
        let identity = dash_sdk::platform::Identity::fetch(&sdk, identifier)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Identity not found"))?;
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
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{hash160, Hash};
            hash160::Hash::hash(&public_key_bytes[..])
                .to_byte_array()
                .to_vec()
        };
        let matching_key = if let Some(requested_key_id) = key_id {
            identity
                .public_keys()
                .get(&requested_key_id)
                .filter(|key| {
                    (key.purpose() == Purpose::TRANSFER || key.purpose() == Purpose::OWNER)
                        && key.key_type() == KeyType::ECDSA_HASH160
                        && key.data().as_slice() == public_key_hash160.as_slice()
                })
                .ok_or_else(|| {
                    WasmSdkError::not_found(format!(
                        "Key with ID {} not found or doesn't match private key",
                        requested_key_id
                    ))
                })?
        } else {
            identity
                .public_keys()
                .iter()
                .find(|(_, key)| {
                    key.purpose() == Purpose::TRANSFER
                        && key.key_type() == KeyType::ECDSA_HASH160
                        && key.data().as_slice() == public_key_hash160.as_slice()
                })
                .or_else(|| {
                    identity.public_keys().iter().find(|(_, key)| {
                        key.purpose() == Purpose::OWNER
                            && key.key_type() == KeyType::ECDSA_HASH160
                            && key.data().as_slice() == public_key_hash160.as_slice()
                    })
                })
                .map(|(_, key)| key)
                .ok_or_else(|| {
                    WasmSdkError::not_found(
                        "No matching withdrawal key found for the provided private key",
                    )
                })?
        };
        let signer = SingleKeySigner::from_string(&private_key_wif, self.network())
            .map_err(WasmSdkError::invalid_argument)?;
        use dash_sdk::platform::transition::withdraw_from_identity::WithdrawFromIdentity;
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
        let identifier = identifier_from_js(&identity_id, "identity ID")?;
        let identity_base58 = identifier.to_string(Encoding::Base58);
        let identity = dash_sdk::platform::Identity::fetch(&sdk, identifier)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Identity not found"))?;
        let current_revision = identity.revision();
        let private_key = PrivateKey::from_wif(&private_key_wif)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid private key: {}", e)))?;
        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(
            &private_key.inner.secret_bytes(),
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid secret key: {}", e)))?;
        let public_key =
            dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize();
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{hash160, Hash};
            hash160::Hash::hash(&public_key_bytes[..])
                .to_byte_array()
                .to_vec()
        };
        let master_key = identity
            .public_keys()
            .iter()
            .find(|(_, key)| {
                key.purpose() == Purpose::AUTHENTICATION
                    && key.security_level() == SecurityLevel::MASTER
                    && key.key_type() == KeyType::ECDSA_HASH160
                    && key.data().as_slice() == public_key_hash160.as_slice()
            })
            .map(|(id, _)| *id)
            .ok_or_else(|| {
                WasmSdkError::invalid_argument("Provided private key does not match any master key")
            })?;
        let keys_to_add: Vec<IdentityPublicKey> = if let Some(keys_json) = add_public_keys {
            let keys_data: serde_json::Value = serde_json::from_str(&keys_json).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid JSON for add_public_keys: {}", e))
            })?;
            let keys_array = keys_data.as_array().ok_or_else(|| {
                WasmSdkError::invalid_argument("add_public_keys must be a JSON array")
            })?;
            let mut next_key_id = identity.public_keys().keys().max().copied().unwrap_or(0) + 1;
            keys_array
                .iter()
                .map(|key_data| {
                    let key_type_str = key_data["keyType"]
                        .as_str()
                        .ok_or_else(|| WasmSdkError::invalid_argument("keyType is required"))?;
                    let purpose_str = key_data["purpose"]
                        .as_str()
                        .ok_or_else(|| WasmSdkError::invalid_argument("purpose is required"))?;
                    let security_level_str = key_data["securityLevel"].as_str().unwrap_or("HIGH");
                    let data_str = key_data["data"]
                        .as_str()
                        .ok_or_else(|| WasmSdkError::invalid_argument("data is required"))?;
                    let key_type = match key_type_str {
                        "ECDSA_SECP256K1" => KeyType::ECDSA_SECP256K1,
                        "BLS12_381" => KeyType::BLS12_381,
                        "ECDSA_HASH160" => KeyType::ECDSA_HASH160,
                        "BIP13_SCRIPT_HASH" => KeyType::BIP13_SCRIPT_HASH,
                        "EDDSA_25519_HASH160" => KeyType::EDDSA_25519_HASH160,
                        _ => {
                            return Err(WasmSdkError::invalid_argument(format!(
                                "Unknown key type: {}",
                                key_type_str
                            )));
                        }
                    };
                    let purpose = match purpose_str {
                        "AUTHENTICATION" => Purpose::AUTHENTICATION,
                        "ENCRYPTION" => Purpose::ENCRYPTION,
                        "DECRYPTION" => Purpose::DECRYPTION,
                        "TRANSFER" => Purpose::TRANSFER,
                        "SYSTEM" => Purpose::SYSTEM,
                        "VOTING" => Purpose::VOTING,
                        _ => {
                            return Err(WasmSdkError::invalid_argument(format!(
                                "Unknown purpose: {}",
                                purpose_str
                            )));
                        }
                    };
                    let security_level = match security_level_str {
                        "MASTER" => SecurityLevel::MASTER,
                        "CRITICAL" => SecurityLevel::CRITICAL,
                        "HIGH" => SecurityLevel::HIGH,
                        "MEDIUM" => SecurityLevel::MEDIUM,
                        _ => SecurityLevel::HIGH,
                    };
                    let key_data =
                        dash_sdk::dpp::dashcore::base64::decode(data_str).map_err(|e| {
                            WasmSdkError::invalid_argument(format!(
                                "Invalid base64 key data: {}",
                                e
                            ))
                        })?;
                    use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
                    let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
                        id: next_key_id,
                        key_type,
                        purpose,
                        security_level,
                        contract_bounds: None,
                        read_only: false,
                        data: BinaryData::new(key_data),
                        disabled_at: None,
                    });
                    next_key_id += 1;
                    Ok(public_key)
                })
                .collect::<Result<Vec<_>, WasmSdkError>>()?
        } else {
            Vec::new()
        };
        let keys_to_disable = disable_public_keys.unwrap_or_default();
        let added_keys_count = keys_to_add.len();
        let disabled_keys_count = keys_to_disable.len();
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
        let identity_nonce = sdk
            .get_identity_nonce(identifier, true, None)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to get identity nonce: {}", e)))?;
        let signer = SingleKeySigner::from_string(&private_key_wif, self.network())
            .map_err(WasmSdkError::invalid_argument)?;
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
        use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
        let result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast update: {}", e)))?;
        let updated_revision = match result {
            StateTransitionProofResult::VerifiedIdentity(updated_identity) => {
                updated_identity.revision()
            }
            StateTransitionProofResult::VerifiedPartialIdentity(partial_identity) => {
                partial_identity.revision.unwrap_or(current_revision + 1)
            }
            _ => current_revision + 1,
        };
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
        let pro_tx_hash = identifier_from_js(&masternode_pro_tx_hash, "ProTxHash")?;
        let pro_tx_hash_base58 = pro_tx_hash.to_string(Encoding::Base58);
        let data_contract_id = identifier_from_js(&contract_id, "contract ID")?;
        let contract_id_base58 = data_contract_id.to_string(Encoding::Base58);
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
        let private_key = if voting_key_wif.len() == 64
            && voting_key_wif.chars().all(|c| c.is_ascii_hexdigit())
        {
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
            PrivateKey::from_wif(&voting_key_wif).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid WIF private key: {}", e))
            })?
        };
        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(
            &private_key.inner.secret_bytes(),
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid secret key: {}", e)))?;
        let public_key =
            dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize();
        let voting_key_hash = {
            use dash_sdk::dpp::dashcore::hashes::{hash160, Hash};
            hash160::Hash::hash(&public_key_bytes[..])
                .to_byte_array()
                .to_vec()
        };
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
        use dash_sdk::dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
        use dash_sdk::dpp::voting::votes::resource_vote::ResourceVote;
        let resource_vote = ResourceVote::V0(ResourceVoteV0 {
            vote_poll,
            resource_vote_choice,
        });
        use dash_sdk::dpp::voting::votes::Vote;
        let vote = Vote::ResourceVote(resource_vote);
        let signer = SingleKeySigner::from_string(&voting_key_wif, self.network())
            .map_err(WasmSdkError::invalid_argument)?;
        use dash_sdk::platform::transition::vote::PutVote;
        vote.put_to_platform(pro_tx_hash, &voting_public_key, &sdk, &signer, None)
            .await?;
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
