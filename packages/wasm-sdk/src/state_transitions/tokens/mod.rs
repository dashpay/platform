//! Token state transition implementations for the WASM SDK.
//!
//! This module provides WASM bindings for token operations like mint, burn, transfer, etc.

use crate::sdk::{WasmSdk, MAINNET_TRUSTED_CONTEXT, TESTNET_TRUSTED_CONTEXT};
use dash_sdk::dpp::balances::credits::TokenAmount;
use dash_sdk::dpp::dashcore::{PrivateKey, Network};
use dash_sdk::dpp::data_contract::TokenContractPosition;
use dash_sdk::dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use platform_value::{platform_value, Value};
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::platform_value::{Identifier, BinaryData, string_encoding::Encoding};
use dash_sdk::dpp::prelude::UserFeeIncrease;
use dash_sdk::dpp::state_transition::batch_transition::BatchTransition;
use dash_sdk::dpp::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use dash_sdk::dpp::state_transition::StateTransition;
use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
use dash_sdk::dpp::tokens::calculate_token_id;
use dash_sdk::dpp::ProtocolError;
use dash_sdk::dpp::document::DocumentV0Getters;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::{Fetch, FetchMany};
use dash_sdk::Sdk;
use serde_wasm_bindgen::to_value;
use serde_json;
use std::sync::Arc;
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

/// A simple signer for WASM that uses a single private key
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

impl std::fmt::Debug for WasmSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmSigner").finish()
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Mint new tokens according to the token's configuration.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - The ID of the data contract containing the token
    /// * `token_position` - The position of the token in the contract (0-indexed)
    /// * `amount` - The amount of tokens to mint
    /// * `identity_id` - The identity ID of the minter
    /// * `private_key_wif` - The private key in WIF format for signing
    /// * `key_id` - The key ID to use for signing
    /// * `recipient_id` - Optional recipient identity ID (if None, mints to issuer)
    /// * `public_note` - Optional public note for the mint operation
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the state transition result
    #[wasm_bindgen(js_name = tokenMint)]
    pub async fn token_mint(
        &self,
        data_contract_id: String,
        token_position: u16,
        amount: String,
        identity_id: String,
        private_key_wif: String,
        key_id: u32,
        recipient_id: Option<String>,
        public_note: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();
        
        // Parse identifiers
        let contract_id = Identifier::from_string(&data_contract_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid contract ID: {}", e)))?;
        
        let issuer_id = Identifier::from_string(&identity_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid identity ID: {}", e)))?;
        
        let recipient = if let Some(recipient_str) = recipient_id {
            Some(Identifier::from_string(&recipient_str, Encoding::Base58)
                .map_err(|e| JsValue::from_str(&format!("Invalid recipient ID: {}", e)))?)
        } else {
            None
        };
        
        // Parse amount
        let token_amount = amount.parse::<TokenAmount>()
            .map_err(|e| JsValue::from_str(&format!("Invalid amount: {}", e)))?;
        
        // Fetch the data contract first to ensure it's in the cache
        let data_contract = dash_sdk::platform::DataContract::fetch(&sdk, contract_id)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch data contract: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Data contract not found"))?;
        
        // Add the contract to the context provider's cache if using trusted mode
        match sdk.network {
            dash_sdk::dpp::dashcore::Network::Testnet => {
                if let Some(ref context) = *crate::sdk::TESTNET_TRUSTED_CONTEXT.lock().unwrap() {
                    context.add_known_contract(data_contract.clone());
                }
            }
            dash_sdk::dpp::dashcore::Network::Dash => {
                if let Some(ref context) = *crate::sdk::MAINNET_TRUSTED_CONTEXT.lock().unwrap() {
                    context.add_known_contract(data_contract.clone());
                }
            }
            _ => {} // Other networks don't use trusted context
        }
        
        // Get identity to construct public key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, issuer_id)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;
        
        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(issuer_id, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;
        
        // Create public key from the private key and key ID
        let private_key_bytes = WasmSigner::new(&private_key_wif)?.private_key.to_bytes();
        let public_key_bytes = dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(
            &dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new(),
            &dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(&private_key_bytes)
                .map_err(|e| JsValue::from_str(&format!("Invalid private key: {}", e)))?
        ).serialize().to_vec();
        
        let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: key_id,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::CRITICAL,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(public_key_bytes),
            disabled_at: None,
        });
        
        // Create signer
        let signer = WasmSigner::new(&private_key_wif)?;
        
        // Calculate token ID
        let token_id = Identifier::from(calculate_token_id(
            contract_id.as_bytes(),
            token_position,
        ));
        
        // Create the state transition
        let platform_version = sdk.version();
        let state_transition = BatchTransition::new_token_mint_transition(
            token_id,
            issuer_id,
            contract_id,
            token_position,
            token_amount,
            recipient,
            public_note,
            None, // using_group_info
            &public_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            &signer,
            platform_version,
            None, // state_transition_creation_options
        ).map_err(|e| JsValue::from_str(&format!("Failed to create mint transition: {}", e)))?;
        
        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast transition: {}", e)))?;
        
        // Convert result to JsValue based on the type
        match proof_result {
            StateTransitionProofResult::VerifiedTokenBalance(recipient_id, new_balance) => {
                to_value(&serde_json::json!({
                    "type": "VerifiedTokenBalance",
                    "recipientId": recipient_id.to_string(Encoding::Base58),
                    "newBalance": new_balance.to_string()
                })).map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
            }
            StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
                // For now, just indicate that a document was created
                // TODO: Properly serialize the document once we figure out the correct trait import
                to_value(&serde_json::json!({
                    "type": "VerifiedTokenActionWithDocument",
                    "documentId": doc.id().to_string(Encoding::Base58),
                    "message": "Token mint operation recorded successfully"
                })).map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                to_value(&serde_json::json!({
                    "type": "VerifiedTokenGroupActionWithDocument",
                    "groupPower": power,
                    "document": doc.is_some()
                })).map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithTokenBalance(power, status, balance) => {
                to_value(&serde_json::json!({
                    "type": "VerifiedTokenGroupActionWithTokenBalance",
                    "groupPower": power,
                    "status": format!("{:?}", status),
                    "balance": balance.map(|b| b.to_string())
                })).map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
            }
            _ => Err(JsValue::from_str("Unexpected result type for mint transition"))
        }
    }

    /// Burn tokens, permanently removing them from circulation.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - The ID of the data contract containing the token
    /// * `token_position` - The position of the token in the contract (0-indexed)
    /// * `amount` - The amount of tokens to burn
    /// * `identity_id` - The identity ID of the burner
    /// * `private_key_wif` - The private key in WIF format for signing
    /// * `key_id` - The key ID to use for signing
    /// * `public_note` - Optional public note for the burn operation
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the state transition result
    #[wasm_bindgen(js_name = tokenBurn)]
    pub async fn token_burn(
        &self,
        data_contract_id: String,
        token_position: u16,
        amount: String,
        identity_id: String,
        private_key_wif: String,
        key_id: u32,
        public_note: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();
        
        // Parse identifiers
        let contract_id = Identifier::from_string(&data_contract_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid contract ID: {}", e)))?;
        
        let burner_id = Identifier::from_string(&identity_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid identity ID: {}", e)))?;
        
        // Parse amount
        let token_amount = amount.parse::<TokenAmount>()
            .map_err(|e| JsValue::from_str(&format!("Invalid amount: {}", e)))?;
        
        // Fetch the data contract first to ensure it's in the cache
        let data_contract = dash_sdk::platform::DataContract::fetch(&sdk, contract_id)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch data contract: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Data contract not found"))?;
        
        // Add the contract to the context provider's cache if using trusted mode
        match sdk.network {
            dash_sdk::dpp::dashcore::Network::Testnet => {
                if let Some(ref context) = *crate::sdk::TESTNET_TRUSTED_CONTEXT.lock().unwrap() {
                    context.add_known_contract(data_contract.clone());
                }
            }
            dash_sdk::dpp::dashcore::Network::Dash => {
                if let Some(ref context) = *crate::sdk::MAINNET_TRUSTED_CONTEXT.lock().unwrap() {
                    context.add_known_contract(data_contract.clone());
                }
            }
            _ => {} // Other networks don't use trusted context
        }
        
        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(burner_id, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;
        
        // Create public key from the private key and key ID
        let private_key_bytes = WasmSigner::new(&private_key_wif)?.private_key.to_bytes();
        let public_key_bytes = dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(
            &dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new(),
            &dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(&private_key_bytes)
                .map_err(|e| JsValue::from_str(&format!("Invalid private key: {}", e)))?
        ).serialize().to_vec();
        
        let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: key_id,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::CRITICAL,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(public_key_bytes),
            disabled_at: None,
        });
        
        // Create signer
        let signer = WasmSigner::new(&private_key_wif)?;
        
        // Calculate token ID
        let token_id = Identifier::from(calculate_token_id(
            contract_id.as_bytes(),
            token_position,
        ));
        
        // Create the state transition
        let platform_version = sdk.version();
        let state_transition = BatchTransition::new_token_burn_transition(
            token_id,
            burner_id,
            contract_id,
            token_position,
            token_amount,
            public_note,
            None, // using_group_info
            &public_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            &signer,
            platform_version,
            None, // state_transition_creation_options
        ).map_err(|e| JsValue::from_str(&format!("Failed to create burn transition: {}", e)))?;
        
        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast transition: {}", e)))?;
        
        // Convert result to JsValue based on the type
        match proof_result {
            StateTransitionProofResult::VerifiedTokenBalance(identity_id, new_balance) => {
                to_value(&serde_json::json!({
                    "type": "VerifiedTokenBalance",
                    "identityId": identity_id.to_string(Encoding::Base58),
                    "newBalance": new_balance.to_string()
                })).map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
            }
            StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
                // For now, just indicate that a document was created
                // TODO: Properly serialize the document once we figure out the correct trait import
                to_value(&serde_json::json!({
                    "type": "VerifiedTokenActionWithDocument",
                    "documentId": doc.id().to_string(Encoding::Base58),
                    "message": "Token mint operation recorded successfully"
                })).map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                to_value(&serde_json::json!({
                    "type": "VerifiedTokenGroupActionWithDocument",
                    "groupPower": power,
                    "document": doc.is_some()
                })).map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithTokenBalance(power, status, balance) => {
                to_value(&serde_json::json!({
                    "type": "VerifiedTokenGroupActionWithTokenBalance",
                    "groupPower": power,
                    "status": format!("{:?}", status),
                    "balance": balance.map(|b| b.to_string())
                })).map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
            }
            _ => Err(JsValue::from_str("Unexpected result type for burn transition"))
        }
    }

    /// Transfer tokens between identities.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - The ID of the data contract containing the token
    /// * `token_position` - The position of the token in the contract (0-indexed)
    /// * `amount` - The amount of tokens to transfer
    /// * `sender_id` - The identity ID of the sender
    /// * `recipient_id` - The identity ID of the recipient
    /// * `private_key_wif` - The private key in WIF format for signing
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the state transition result
    #[wasm_bindgen(js_name = tokenTransfer)]
    pub async fn token_transfer(
        &self,
        data_contract_id: String,
        token_position: u16,
        amount: String,
        sender_id: String,
        recipient_id: String,
        private_key_wif: String,
    ) -> Result<JsValue, JsValue> {
        Err(JsValue::from_str("Token transfer not yet implemented - similar pattern to mint/burn"))
    }

    /// Freeze tokens for a specific identity.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - The ID of the data contract containing the token
    /// * `token_position` - The position of the token in the contract (0-indexed)
    /// * `identity_to_freeze` - The identity ID whose tokens to freeze
    /// * `freezer_id` - The identity ID of the freezer (must have permission)
    /// * `private_key_wif` - The private key in WIF format for signing
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the state transition result
    #[wasm_bindgen(js_name = tokenFreeze)]
    pub async fn token_freeze(
        &self,
        data_contract_id: String,
        token_position: u16,
        identity_to_freeze: String,
        freezer_id: String,
        private_key_wif: String,
    ) -> Result<JsValue, JsValue> {
        Err(JsValue::from_str("Token freeze not yet implemented"))
    }

    /// Unfreeze tokens for a specific identity.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - The ID of the data contract containing the token
    /// * `token_position` - The position of the token in the contract (0-indexed)
    /// * `identity_to_unfreeze` - The identity ID whose tokens to unfreeze
    /// * `unfreezer_id` - The identity ID of the unfreezer (must have permission)
    /// * `private_key_wif` - The private key in WIF format for signing
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the state transition result
    #[wasm_bindgen(js_name = tokenUnfreeze)]
    pub async fn token_unfreeze(
        &self,
        data_contract_id: String,
        token_position: u16,
        identity_to_unfreeze: String,
        unfreezer_id: String,
        private_key_wif: String,
    ) -> Result<JsValue, JsValue> {
        Err(JsValue::from_str("Token unfreeze not yet implemented"))
    }

    /// Destroy frozen tokens.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - The ID of the data contract containing the token
    /// * `token_position` - The position of the token in the contract (0-indexed)
    /// * `identity_id` - The identity ID whose frozen tokens to destroy
    /// * `destroyer_id` - The identity ID of the destroyer (must have permission)
    /// * `private_key_wif` - The private key in WIF format for signing
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the state transition result
    #[wasm_bindgen(js_name = tokenDestroyFrozen)]
    pub async fn token_destroy_frozen(
        &self,
        data_contract_id: String,
        token_position: u16,
        identity_id: String,
        destroyer_id: String,
        private_key_wif: String,
    ) -> Result<JsValue, JsValue> {
        Err(JsValue::from_str("Token destroy frozen not yet implemented"))
    }
}