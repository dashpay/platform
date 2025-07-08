use crate::sdk::WasmSdk;
use dash_sdk::dpp::dashcore::PrivateKey;
use dash_sdk::dpp::identity::{IdentityPublicKey, KeyType, Purpose};
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::platform_value::{BinaryData, Identifier, string_encoding::Encoding};
use dash_sdk::dpp::prelude::UserFeeIncrease;
use dash_sdk::dpp::ProtocolError;
use dash_sdk::dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dash_sdk::dpp::state_transition::identity_credit_transfer_transition::methods::IdentityCreditTransferTransitionMethodsV0;
use dash_sdk::dpp::state_transition::StateTransition;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::Fetch;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use js_sys;

/// A simple signer for WASM that uses a single private key
#[derive(Debug)]
struct WasmSigner {
    private_key: PrivateKey,
}

impl WasmSigner {
    fn new(private_key_wif: &str) -> Result<Self, JsValue> {
        let private_key = PrivateKey::from_wif(private_key_wif)
            .map_err(|e| JsValue::from_str(&format!("Invalid WIF private key: {}", e)))?;
        Ok(Self { private_key })
    }
}

impl Signer for WasmSigner {
    fn sign(
        &self,
        _identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        use dash_sdk::dpp::dashcore::signer;
        let signature = signer::sign(data, &self.private_key.inner.secret_bytes())?;
        Ok(signature.to_vec().into())
    }

    fn can_sign_with(&self, _identity_public_key: &IdentityPublicKey) -> bool {
        // For simplicity, we assume the signer can sign with any key
        // In a real implementation, you'd check if the public key matches
        true
    }
}

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
        let signer = WasmSigner::new(&private_key_wif)?;
        
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
        let signer = WasmSigner::new(&private_key_wif)?;
        
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
}