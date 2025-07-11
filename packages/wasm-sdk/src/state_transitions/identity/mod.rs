use crate::sdk::WasmSdk;
use dash_sdk::dpp::dashcore::PrivateKey;
use dash_sdk::dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::platform_value::{BinaryData, Identifier, string_encoding::Encoding};
use dash_sdk::dpp::prelude::UserFeeIncrease;
use dash_sdk::dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dash_sdk::dpp::state_transition::identity_credit_transfer_transition::methods::IdentityCreditTransferTransitionMethodsV0;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::Fetch;
use simple_signer::SingleKeySigner;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use js_sys;

#[wasm_bindgen]
impl WasmSdk {
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
        sender_id: String,
        recipient_id: String,
        amount: u64,
        private_key_wif: String,
        key_id: Option<u32>,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();
        
        // Parse identifiers
        let sender_identifier = Identifier::from_string(&sender_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid sender ID: {}", e)))?;
            
        let recipient_identifier = Identifier::from_string(&recipient_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid recipient ID: {}", e)))?;
        
        // Validate not sending to self
        if sender_identifier == recipient_identifier {
            return Err(JsValue::from_str("Cannot transfer credits to yourself"));
        }
        
        // Validate amount
        if amount == 0 {
            return Err(JsValue::from_str("Transfer amount must be greater than 0"));
        }
        
        // Fetch sender identity
        let sender_identity = dash_sdk::platform::Identity::fetch(&sdk, sender_identifier)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch sender identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Sender identity not found"))?;
        
        // Parse private key and find matching public key
        let private_key_bytes = dash_sdk::dpp::dashcore::PrivateKey::from_wif(&private_key_wif)
            .map_err(|e| JsValue::from_str(&format!("Invalid private key: {}", e)))?
            .inner
            .secret_bytes();
        
        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(&private_key_bytes)
            .map_err(|e| JsValue::from_str(&format!("Invalid secret key: {}", e)))?;
        let public_key = dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize();
        
        // Create public key hash using hash160
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{Hash, hash160};
            hash160::Hash::hash(&public_key_bytes[..]).to_byte_array().to_vec()
        };
        
        // Find matching key - prioritize key_id if provided, otherwise find any matching key
        let matching_key = if let Some(requested_key_id) = key_id {
            // Find specific key by ID
            sender_identity.public_keys()
                .get(&requested_key_id)
                .filter(|key| {
                    key.purpose() == Purpose::TRANSFER &&
                    key.key_type() == KeyType::ECDSA_HASH160 &&
                    key.data().as_slice() == public_key_hash160.as_slice()
                })
                .ok_or_else(|| JsValue::from_str(&format!("Key with ID {} not found or doesn't match private key", requested_key_id)))?
        } else {
            // Find any matching transfer key
            sender_identity.public_keys().iter()
                .find(|(_, key)| {
                    key.purpose() == Purpose::TRANSFER &&
                    key.key_type() == KeyType::ECDSA_HASH160 &&
                    key.data().as_slice() == public_key_hash160.as_slice()
                })
                .map(|(_, key)| key)
                .ok_or_else(|| JsValue::from_str("No matching transfer key found for the provided private key"))?
        };
        
        // Get identity nonce
        let identity_nonce = sdk
            .get_identity_nonce(sender_identifier, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to get identity nonce: {}", e)))?;
        
        // Create signer
        let signer = SingleKeySigner::from_string(&private_key_wif, dash_sdk::dpp::dashcore::Network::Testnet)
            .map_err(|e| JsValue::from_str(&e))?;
        
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
            None, // No version override
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create transfer transition: {}", e)))?;
        
        // Broadcast the transition
        use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
        let _result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast transfer: {}", e)))?;
        
        
        // Create JavaScript result object
        let result_obj = js_sys::Object::new();
        
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("status"), &JsValue::from_str("success"))
            .map_err(|e| JsValue::from_str(&format!("Failed to set status: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("senderId"), &JsValue::from_str(&sender_id))
            .map_err(|e| JsValue::from_str(&format!("Failed to set senderId: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("recipientId"), &JsValue::from_str(&recipient_id))
            .map_err(|e| JsValue::from_str(&format!("Failed to set recipientId: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("amount"), &JsValue::from_f64(amount as f64))
            .map_err(|e| JsValue::from_str(&format!("Failed to set amount: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("message"), &JsValue::from_str("Credits transferred successfully"))
            .map_err(|e| JsValue::from_str(&format!("Failed to set message: {:?}", e)))?;
        
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
        identity_id: String,
        to_address: String,
        amount: u64,
        core_fee_per_byte: Option<u32>,
        private_key_wif: String,
        key_id: Option<u32>,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();
        
        // Parse identity identifier
        let identifier = Identifier::from_string(&identity_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid identity ID: {}", e)))?;
        
        // Parse the Dash address
        use dash_sdk::dpp::dashcore::Address;
        use std::str::FromStr;
        let address = Address::from_str(&to_address)
            .map_err(|e| JsValue::from_str(&format!("Invalid Dash address: {}", e)))?
            .assume_checked();
        
        // Validate amount
        if amount == 0 {
            return Err(JsValue::from_str("Withdrawal amount must be greater than 0"));
        }
        
        // Fetch the identity
        let identity = dash_sdk::platform::Identity::fetch(&sdk, identifier)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;
        
        // Parse private key and find matching public key
        let private_key_bytes = dash_sdk::dpp::dashcore::PrivateKey::from_wif(&private_key_wif)
            .map_err(|e| JsValue::from_str(&format!("Invalid private key: {}", e)))?
            .inner
            .secret_bytes();
        
        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(&private_key_bytes)
            .map_err(|e| JsValue::from_str(&format!("Invalid secret key: {}", e)))?;
        let public_key = dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize();
        
        // Create public key hash using hash160
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{Hash, hash160};
            hash160::Hash::hash(&public_key_bytes[..]).to_byte_array().to_vec()
        };
        
        // Find matching key - prioritize key_id if provided, otherwise find any matching key
        // For withdrawals, we can use either TRANSFER or OWNER keys
        let matching_key = if let Some(requested_key_id) = key_id {
            // Find specific key by ID
            identity.public_keys()
                .get(&requested_key_id)
                .filter(|key| {
                    (key.purpose() == Purpose::TRANSFER || key.purpose() == Purpose::OWNER) &&
                    key.key_type() == KeyType::ECDSA_HASH160 &&
                    key.data().as_slice() == public_key_hash160.as_slice()
                })
                .ok_or_else(|| JsValue::from_str(&format!("Key with ID {} not found or doesn't match private key", requested_key_id)))?
        } else {
            // Find any matching withdrawal-capable key (prefer TRANSFER keys)
            identity.public_keys().iter()
                .find(|(_, key)| {
                    key.purpose() == Purpose::TRANSFER &&
                    key.key_type() == KeyType::ECDSA_HASH160 &&
                    key.data().as_slice() == public_key_hash160.as_slice()
                })
                .or_else(|| {
                    identity.public_keys().iter()
                        .find(|(_, key)| {
                            key.purpose() == Purpose::OWNER &&
                            key.key_type() == KeyType::ECDSA_HASH160 &&
                            key.data().as_slice() == public_key_hash160.as_slice()
                        })
                })
                .map(|(_, key)| key)
                .ok_or_else(|| JsValue::from_str("No matching withdrawal key found for the provided private key"))?
        };
        
        // Create signer
        let signer = SingleKeySigner::from_string(&private_key_wif, dash_sdk::dpp::dashcore::Network::Testnet)
            .map_err(|e| JsValue::from_str(&e))?;
        
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
                None, // No special settings
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("Withdrawal failed: {}", e)))?;
        
        // Create JavaScript result object
        let result_obj = js_sys::Object::new();
        
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("status"), &JsValue::from_str("success"))
            .map_err(|e| JsValue::from_str(&format!("Failed to set status: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("identityId"), &JsValue::from_str(&identity_id))
            .map_err(|e| JsValue::from_str(&format!("Failed to set identityId: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("toAddress"), &JsValue::from_str(&to_address))
            .map_err(|e| JsValue::from_str(&format!("Failed to set toAddress: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("amount"), &JsValue::from_f64(amount as f64))
            .map_err(|e| JsValue::from_str(&format!("Failed to set amount: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("remainingBalance"), &JsValue::from_f64(remaining_balance as f64))
            .map_err(|e| JsValue::from_str(&format!("Failed to set remainingBalance: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("message"), &JsValue::from_str("Credits withdrawn successfully"))
            .map_err(|e| JsValue::from_str(&format!("Failed to set message: {:?}", e)))?;
        
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
        identity_id: String,
        add_public_keys: Option<String>,
        disable_public_keys: Option<Vec<u32>>,
        private_key_wif: String,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();
        
        // Parse identity identifier
        let identifier = Identifier::from_string(&identity_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid identity ID: {}", e)))?;
        
        // Fetch the identity
        let identity = dash_sdk::platform::Identity::fetch(&sdk, identifier)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;
        
        // Get current revision
        let current_revision = identity.revision();
        
        // Parse private key and verify it's a master key
        let private_key = PrivateKey::from_wif(&private_key_wif)
            .map_err(|e| JsValue::from_str(&format!("Invalid private key: {}", e)))?;
        
        // Create public key hash to find matching master key
        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(&private_key.inner.secret_bytes())
            .map_err(|e| JsValue::from_str(&format!("Invalid secret key: {}", e)))?;
        let public_key = dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize();
        
        // Create public key hash using hash160
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{Hash, hash160};
            hash160::Hash::hash(&public_key_bytes[..]).to_byte_array().to_vec()
        };
        
        // Find matching master key
        let master_key = identity.public_keys().iter()
            .find(|(_, key)| {
                key.purpose() == Purpose::AUTHENTICATION &&
                key.security_level() == SecurityLevel::MASTER &&
                key.key_type() == KeyType::ECDSA_HASH160 &&
                key.data().as_slice() == public_key_hash160.as_slice()
            })
            .map(|(id, _)| *id)
            .ok_or_else(|| JsValue::from_str("Provided private key does not match any master key"))?;
        
        // Parse and prepare keys to add
        let keys_to_add: Vec<IdentityPublicKey> = if let Some(keys_json) = add_public_keys {
            // Parse JSON array of keys
            let keys_data: serde_json::Value = serde_json::from_str(&keys_json)
                .map_err(|e| JsValue::from_str(&format!("Invalid JSON for add_public_keys: {}", e)))?;
            
            let keys_array = keys_data.as_array()
                .ok_or_else(|| JsValue::from_str("add_public_keys must be a JSON array"))?;
            
            // Get the current max key ID
            let mut next_key_id = identity.public_keys().keys().max().copied().unwrap_or(0) + 1;
            
            keys_array.iter()
                .map(|key_data| {
                    let key_type_str = key_data["keyType"].as_str()
                        .ok_or_else(|| JsValue::from_str("keyType is required"))?;
                    let purpose_str = key_data["purpose"].as_str()
                        .ok_or_else(|| JsValue::from_str("purpose is required"))?;
                    let security_level_str = key_data["securityLevel"].as_str()
                        .unwrap_or("HIGH");
                    let data_str = key_data["data"].as_str()
                        .ok_or_else(|| JsValue::from_str("data is required"))?;
                    
                    // Parse key type
                    let key_type = match key_type_str {
                        "ECDSA_SECP256K1" => KeyType::ECDSA_SECP256K1,
                        "BLS12_381" => KeyType::BLS12_381,
                        "ECDSA_HASH160" => KeyType::ECDSA_HASH160,
                        "BIP13_SCRIPT_HASH" => KeyType::BIP13_SCRIPT_HASH,
                        "EDDSA_25519_HASH160" => KeyType::EDDSA_25519_HASH160,
                        _ => return Err(JsValue::from_str(&format!("Unknown key type: {}", key_type_str)))
                    };
                    
                    // Parse purpose
                    let purpose = match purpose_str {
                        "AUTHENTICATION" => Purpose::AUTHENTICATION,
                        "ENCRYPTION" => Purpose::ENCRYPTION,
                        "DECRYPTION" => Purpose::DECRYPTION,
                        "TRANSFER" => Purpose::TRANSFER,
                        "SYSTEM" => Purpose::SYSTEM,
                        "VOTING" => Purpose::VOTING,
                        _ => return Err(JsValue::from_str(&format!("Unknown purpose: {}", purpose_str)))
                    };
                    
                    // Parse security level
                    let security_level = match security_level_str {
                        "MASTER" => SecurityLevel::MASTER,
                        "CRITICAL" => SecurityLevel::CRITICAL,
                        "HIGH" => SecurityLevel::HIGH,
                        "MEDIUM" => SecurityLevel::MEDIUM,
                        _ => SecurityLevel::HIGH
                    };
                    
                    // Decode key data from base64
                    let key_data = dash_sdk::dpp::dashcore::base64::decode(data_str)
                        .map_err(|e| JsValue::from_str(&format!("Invalid base64 key data: {}", e)))?;
                    
                    // Create the identity public key
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
                .collect::<Result<Vec<_>, _>>()?
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
                    return Err(JsValue::from_str(&format!("Cannot disable master key {}", key_id)));
                }
                if key.purpose() == Purpose::AUTHENTICATION && 
                   key.security_level() == SecurityLevel::CRITICAL &&
                   key.key_type() == KeyType::ECDSA_SECP256K1 {
                    return Err(JsValue::from_str(&format!("Cannot disable critical authentication key {}", key_id)));
                }
                if key.purpose() == Purpose::TRANSFER {
                    return Err(JsValue::from_str(&format!("Cannot disable transfer key {}", key_id)));
                }
            } else {
                return Err(JsValue::from_str(&format!("Key {} not found", key_id)));
            }
        }
        
        // Get identity nonce
        let identity_nonce = sdk
            .get_identity_nonce(identifier, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to get identity nonce: {}", e)))?;
        
        // Create signer
        let signer = SingleKeySigner::from_string(&private_key_wif, dash_sdk::dpp::dashcore::Network::Testnet)
            .map_err(|e| JsValue::from_str(&e))?;
        
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
            None, // No version override
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create update transition: {}", e)))?;
        
        // Broadcast the transition
        use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
        let result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast update: {}", e)))?;
        
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
        
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("status"), &JsValue::from_str("success"))
            .map_err(|e| JsValue::from_str(&format!("Failed to set status: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("identityId"), &JsValue::from_str(&identity_id))
            .map_err(|e| JsValue::from_str(&format!("Failed to set identityId: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("revision"), &JsValue::from_f64(updated_revision as f64))
            .map_err(|e| JsValue::from_str(&format!("Failed to set revision: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("addedKeys"), &JsValue::from_f64(added_keys_count as f64))
            .map_err(|e| JsValue::from_str(&format!("Failed to set addedKeys: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("disabledKeys"), &JsValue::from_f64(disabled_keys_count as f64))
            .map_err(|e| JsValue::from_str(&format!("Failed to set disabledKeys: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("message"), &JsValue::from_str("Identity updated successfully"))
            .map_err(|e| JsValue::from_str(&format!("Failed to set message: {:?}", e)))?;
        
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
    #[wasm_bindgen(js_name = masternodeVote)]
    pub async fn masternode_vote(
        &self,
        masternode_pro_tx_hash: String,
        contract_id: String,
        document_type_name: String,
        index_name: String,
        index_values: String,
        vote_choice: String,
        voting_key_wif: String,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();
        
        // Parse ProTxHash (try hex first, then base58)
        let pro_tx_hash = if masternode_pro_tx_hash.len() == 64 && masternode_pro_tx_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            // Looks like hex
            Identifier::from_string(&masternode_pro_tx_hash, Encoding::Hex)
                .map_err(|e| JsValue::from_str(&format!("Invalid ProTxHash (hex): {}", e)))?
        } else {
            // Try base58
            Identifier::from_string(&masternode_pro_tx_hash, Encoding::Base58)
                .map_err(|e| JsValue::from_str(&format!("Invalid ProTxHash (base58): {}", e)))?
        };
        
        // Parse contract ID
        let data_contract_id = Identifier::from_string(&contract_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid contract ID: {}", e)))?;
        
        // Parse index values from JSON
        let index_values_json: serde_json::Value = serde_json::from_str(&index_values)
            .map_err(|e| JsValue::from_str(&format!("Invalid index values JSON: {}", e)))?;
        
        let index_values_array = index_values_json.as_array()
            .ok_or_else(|| JsValue::from_str("index_values must be a JSON array"))?;
        
        let index_values_vec: Vec<dash_sdk::dpp::platform_value::Value> = index_values_array.iter()
            .map(|v| {
                match v {
                    serde_json::Value::String(s) => Ok(dash_sdk::dpp::platform_value::Value::Text(s.clone())),
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            Ok(dash_sdk::dpp::platform_value::Value::I64(i))
                        } else if let Some(u) = n.as_u64() {
                            Ok(dash_sdk::dpp::platform_value::Value::U64(u))
                        } else {
                            Ok(dash_sdk::dpp::platform_value::Value::Float(n.as_f64().unwrap()))
                        }
                    }
                    serde_json::Value::Bool(b) => Ok(dash_sdk::dpp::platform_value::Value::Bool(*b)),
                    _ => Err(JsValue::from_str("Unsupported index value type"))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        
        // Parse vote choice
        use dash_sdk::dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
        let resource_vote_choice = if vote_choice == "abstain" {
            ResourceVoteChoice::Abstain
        } else if vote_choice == "lock" {
            ResourceVoteChoice::Lock
        } else if vote_choice.starts_with("towardsIdentity:") {
            let identity_id_str = vote_choice.strip_prefix("towardsIdentity:")
                .ok_or_else(|| JsValue::from_str("Invalid vote choice format"))?;
            let identity_id = Identifier::from_string(identity_id_str, Encoding::Base58)
                .map_err(|e| JsValue::from_str(&format!("Invalid identity ID in vote choice: {}", e)))?;
            ResourceVoteChoice::TowardsIdentity(identity_id)
        } else {
            return Err(JsValue::from_str("Invalid vote choice. Must be 'abstain', 'lock', or 'towardsIdentity:<identity_id>'"));
        };
        
        // Parse private key (try WIF first, then hex)
        let private_key = if voting_key_wif.len() == 64 && voting_key_wif.chars().all(|c| c.is_ascii_hexdigit()) {
            // Looks like hex
            let key_bytes = hex::decode(&voting_key_wif)
                .map_err(|e| JsValue::from_str(&format!("Invalid hex private key: {}", e)))?;
            if key_bytes.len() != 32 {
                return Err(JsValue::from_str("Private key must be 32 bytes"));
            }
            PrivateKey::from_slice(&key_bytes, dash_sdk::dpp::dashcore::Network::Testnet)
                .map_err(|e| JsValue::from_str(&format!("Invalid private key bytes: {}", e)))?
        } else {
            // Try WIF
            PrivateKey::from_wif(&voting_key_wif)
                .map_err(|e| JsValue::from_str(&format!("Invalid WIF private key: {}", e)))?
        };
        
        // Create the voting public key from the private key
        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(&private_key.inner.secret_bytes())
            .map_err(|e| JsValue::from_str(&format!("Invalid secret key: {}", e)))?;
        let public_key = dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize();
        
        // Create voting public key hash using hash160
        let voting_key_hash = {
            use dash_sdk::dpp::dashcore::hashes::{Hash, hash160};
            hash160::Hash::hash(&public_key_bytes[..]).to_byte_array().to_vec()
        };
        
        // Create the voting identity public key
        use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        let voting_public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0, // The ID doesn't matter for voting keys
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::VOTING,
            security_level: SecurityLevel::HIGH, // Voting keys should be HIGH, not MASTER
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(voting_key_hash),
            disabled_at: None,
        });
        
        // Create the contested document resource vote poll
        use dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
        let vote_poll = dash_sdk::dpp::voting::vote_polls::VotePoll::ContestedDocumentResourceVotePoll(
            ContestedDocumentResourceVotePoll {
                contract_id: data_contract_id,
                document_type_name: document_type_name.clone(),
                index_name: index_name.clone(),
                index_values: index_values_vec,
            }
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
        let signer = SingleKeySigner::from_string(&voting_key_wif, dash_sdk::dpp::dashcore::Network::Testnet)
            .map_err(|e| JsValue::from_str(&e))?;
        
        // Submit the vote using PutVote trait
        use dash_sdk::platform::transition::vote::PutVote;
        
        vote.put_to_platform(
            pro_tx_hash,
            &voting_public_key,
            &sdk,
            &signer,
            None,
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to submit vote: {}", e)))?;
        
        // Create JavaScript result object
        let result_obj = js_sys::Object::new();
        
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("status"), &JsValue::from_str("success"))
            .map_err(|e| JsValue::from_str(&format!("Failed to set status: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("proTxHash"), &JsValue::from_str(&masternode_pro_tx_hash))
            .map_err(|e| JsValue::from_str(&format!("Failed to set proTxHash: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("contractId"), &JsValue::from_str(&contract_id))
            .map_err(|e| JsValue::from_str(&format!("Failed to set contractId: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("documentType"), &JsValue::from_str(&document_type_name))
            .map_err(|e| JsValue::from_str(&format!("Failed to set documentType: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("voteChoice"), &JsValue::from_str(&vote_choice))
            .map_err(|e| JsValue::from_str(&format!("Failed to set voteChoice: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("message"), &JsValue::from_str("Vote submitted successfully"))
            .map_err(|e| JsValue::from_str(&format!("Failed to set message: {:?}", e)))?;
        
        Ok(result_obj.into())
    }
}