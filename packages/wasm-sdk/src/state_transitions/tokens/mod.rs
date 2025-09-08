//! Token state transition implementations for the WASM SDK.
//!
//! This module provides WASM bindings for token operations like mint, burn, transfer, etc.

use crate::sdk::WasmSdk;
use dash_sdk::dpp::balances::credits::TokenAmount;
use dash_sdk::dpp::platform_value::{Identifier, string_encoding::Encoding};
use dash_sdk::dpp::prelude::UserFeeIncrease;
use dash_sdk::dpp::state_transition::batch_transition::BatchTransition;
use dash_sdk::dpp::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
use dash_sdk::dpp::tokens::calculate_token_id;
use dash_sdk::dpp::document::DocumentV0Getters;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::Fetch;
use serde_wasm_bindgen::to_value;
use serde_json;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

// WasmSigner has been replaced with SingleKeySigner from simple-signer crate

// Helper functions for token operations
impl WasmSdk {
    /// Parse and validate token operation parameters
    async fn parse_token_params(
        &self,
        data_contract_id: &str,
        identity_id: &str,
        amount: &str,
        recipient_id: Option<String>,
    ) -> Result<(Identifier, Identifier, TokenAmount, Option<Identifier>), JsValue> {
        // Parse identifiers
        let contract_id = Identifier::from_string(data_contract_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid contract ID: {}", e)))?;

        let identity_identifier = Identifier::from_string(identity_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid identity ID: {}", e)))?;

        let recipient = if let Some(recipient_str) = recipient_id {
            Some(
                Identifier::from_string(&recipient_str, Encoding::Base58)
                    .map_err(|e| JsValue::from_str(&format!("Invalid recipient ID: {}", e)))?,
            )
        } else {
            None
        };

        // Parse amount
        let token_amount = amount
            .parse::<TokenAmount>()
            .map_err(|e| JsValue::from_str(&format!("Invalid amount: {}", e)))?;

        Ok((contract_id, identity_identifier, token_amount, recipient))
    }

    /// Fetch and cache data contract in trusted context
    async fn fetch_and_cache_token_contract(
        &self,
        contract_id: Identifier,
    ) -> Result<dash_sdk::platform::DataContract, JsValue> {
        let sdk = self.inner_clone();

        // Fetch the data contract
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

        Ok(data_contract)
    }

    /// Convert state transition proof result to JsValue
    fn format_token_result(
        &self,
        proof_result: StateTransitionProofResult,
    ) -> Result<JsValue, JsValue> {
        match proof_result {
            StateTransitionProofResult::VerifiedTokenBalance(recipient_id, new_balance) => {
                to_value(&serde_json::json!({
                    "type": "VerifiedTokenBalance",
                    "recipientId": recipient_id.to_string(Encoding::Base58),
                    "newBalance": new_balance.to_string()
                }))
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
            }
            StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
                to_value(&serde_json::json!({
                    "type": "VerifiedTokenActionWithDocument",
                    "documentId": doc.id().to_string(Encoding::Base58),
                    "message": "Token operation recorded successfully"
                }))
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                to_value(&serde_json::json!({
                    "type": "VerifiedTokenGroupActionWithDocument",
                    "groupPower": power,
                    "document": doc.is_some()
                }))
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithTokenBalance(
                power,
                status,
                balance,
            ) => to_value(&serde_json::json!({
                "type": "VerifiedTokenGroupActionWithTokenBalance",
                "groupPower": power,
                "status": format!("{:?}", status),
                "balance": balance.map(|b| b.to_string())
            }))
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e))),
            _ => Err(JsValue::from_str(
                "Unexpected result type for token transition",
            )),
        }
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
        recipient_id: Option<String>,
        public_note: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();

        // Parse and validate parameters
        let (contract_id, issuer_id, token_amount, recipient) = self
            .parse_token_params(&data_contract_id, &identity_id, &amount, recipient_id)
            .await?;

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Get identity to find matching authentication key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, issuer_id)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(issuer_id, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;

        // Find matching authentication key and create signer
        let (_, matching_key) =
            crate::sdk::WasmSdk::find_authentication_key(&identity, &private_key_wif)?;
        let signer = crate::sdk::WasmSdk::create_signer_from_wif(&private_key_wif, sdk.network)?;
        let public_key = matching_key.clone();

        // Calculate token ID
        let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), token_position));

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
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create mint transition: {}", e)))?;

        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast transition: {}", e)))?;

        // Format and return result
        self.format_token_result(proof_result)
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
        public_note: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();

        // Parse and validate parameters (no recipient for burn)
        let (contract_id, burner_id, token_amount, _) = self
            .parse_token_params(&data_contract_id, &identity_id, &amount, None)
            .await?;

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Get identity to find matching authentication key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, burner_id)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(burner_id, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;

        // Find matching authentication key and create signer
        let (_, matching_key) =
            crate::sdk::WasmSdk::find_authentication_key(&identity, &private_key_wif)?;
        let signer = crate::sdk::WasmSdk::create_signer_from_wif(&private_key_wif, sdk.network)?;
        let public_key = matching_key.clone();

        // Calculate token ID
        let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), token_position));

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
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create burn transition: {}", e)))?;

        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast transition: {}", e)))?;

        // Format and return result
        self.format_token_result(proof_result)
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
    /// * `public_note` - Optional public note for the transfer
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
        public_note: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();

        // Parse and validate parameters
        let (contract_id, sender_identifier, token_amount, _) = self
            .parse_token_params(&data_contract_id, &sender_id, &amount, None)
            .await?;

        // Parse recipient ID
        let recipient_identifier = Identifier::from_string(&recipient_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid recipient ID: {}", e)))?;

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Get identity to find matching authentication key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, sender_identifier)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(sender_identifier, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;

        // Find matching authentication key and create signer
        let (_, matching_key) =
            crate::sdk::WasmSdk::find_authentication_key(&identity, &private_key_wif)?;
        let signer = crate::sdk::WasmSdk::create_signer_from_wif(&private_key_wif, sdk.network)?;
        let public_key = matching_key.clone();

        // Calculate token ID
        let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), token_position));

        // Create the state transition
        let platform_version = sdk.version();
        let state_transition = BatchTransition::new_token_transfer_transition(
            token_id,
            sender_identifier,
            contract_id,
            token_position,
            token_amount,
            recipient_identifier,
            public_note,
            None, // shared_encrypted_note
            None, // private_encrypted_note
            &public_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            &signer,
            platform_version,
            None, // state_transition_creation_options
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create transfer transition: {}", e)))?;

        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast transition: {}", e)))?;

        // Format and return result
        self.format_token_result(proof_result)
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
    /// * `public_note` - Optional public note for the freeze operation
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
        public_note: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();

        // Parse and validate parameters
        let (contract_id, freezer_identifier, _, _) = self
            .parse_token_params(
                &data_contract_id,
                &freezer_id,
                "0", // Amount not needed for freeze
                None,
            )
            .await?;

        // Parse identity to freeze
        let frozen_identity_id = Identifier::from_string(&identity_to_freeze, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid identity to freeze: {}", e)))?;

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Get identity to find matching authentication key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, freezer_identifier)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(freezer_identifier, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;

        // Find matching authentication key and create signer
        let (_, matching_key) =
            crate::sdk::WasmSdk::find_authentication_key(&identity, &private_key_wif)?;
        let signer = crate::sdk::WasmSdk::create_signer_from_wif(&private_key_wif, sdk.network)?;
        let public_key = matching_key.clone();

        // Calculate token ID
        let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), token_position));

        // Create the state transition
        let platform_version = sdk.version();
        let state_transition = BatchTransition::new_token_freeze_transition(
            token_id,
            freezer_identifier,
            contract_id,
            token_position,
            frozen_identity_id,
            public_note,
            None, // using_group_info
            &public_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            &signer,
            platform_version,
            None, // state_transition_creation_options
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create freeze transition: {}", e)))?;

        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast transition: {}", e)))?;

        // Format and return result
        self.format_token_result(proof_result)
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
    /// * `public_note` - Optional public note for the unfreeze operation
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
        public_note: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();

        // Parse and validate parameters
        let (contract_id, unfreezer_identifier, _, _) = self
            .parse_token_params(
                &data_contract_id,
                &unfreezer_id,
                "0", // Amount not needed for unfreeze
                None,
            )
            .await?;

        // Parse identity to unfreeze
        let frozen_identity_id =
            Identifier::from_string(&identity_to_unfreeze, Encoding::Base58)
                .map_err(|e| JsValue::from_str(&format!("Invalid identity to unfreeze: {}", e)))?;

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Get identity to find matching authentication key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, unfreezer_identifier)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(unfreezer_identifier, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;

        // Find matching authentication key and create signer
        let (_, matching_key) =
            crate::sdk::WasmSdk::find_authentication_key(&identity, &private_key_wif)?;
        let signer = crate::sdk::WasmSdk::create_signer_from_wif(&private_key_wif, sdk.network)?;
        let public_key = matching_key.clone();

        // Calculate token ID
        let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), token_position));

        // Create the state transition
        let platform_version = sdk.version();
        let state_transition = BatchTransition::new_token_unfreeze_transition(
            token_id,
            unfreezer_identifier,
            contract_id,
            token_position,
            frozen_identity_id,
            public_note,
            None, // using_group_info
            &public_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            &signer,
            platform_version,
            None, // state_transition_creation_options
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create unfreeze transition: {}", e)))?;

        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast transition: {}", e)))?;

        // Format and return result
        self.format_token_result(proof_result)
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
    /// * `public_note` - Optional public note for the destroy operation
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
        public_note: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();

        // Parse and validate parameters
        let (contract_id, destroyer_identifier, _, _) = self
            .parse_token_params(
                &data_contract_id,
                &destroyer_id,
                "0", // Amount not needed for destroy frozen
                None,
            )
            .await?;

        // Parse identity whose frozen tokens to destroy
        let frozen_identity_id =
            Identifier::from_string(&identity_id, Encoding::Base58).map_err(|e| {
                JsValue::from_str(&format!("Invalid identity to destroy frozen funds: {}", e))
            })?;

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Get identity to find matching authentication key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, destroyer_identifier)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(destroyer_identifier, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;

        // Find matching authentication key and create signer
        let (_, matching_key) =
            crate::sdk::WasmSdk::find_authentication_key(&identity, &private_key_wif)?;
        let signer = crate::sdk::WasmSdk::create_signer_from_wif(&private_key_wif, sdk.network)?;
        let public_key = matching_key.clone();

        // Calculate token ID
        let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), token_position));

        // Create the state transition
        let platform_version = sdk.version();
        let state_transition = BatchTransition::new_token_destroy_frozen_funds_transition(
            token_id,
            destroyer_identifier,
            contract_id,
            token_position,
            frozen_identity_id,
            public_note,
            None, // using_group_info
            &public_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            &signer,
            platform_version,
            None, // state_transition_creation_options
        )
        .map_err(|e| {
            JsValue::from_str(&format!(
                "Failed to create destroy frozen transition: {}",
                e
            ))
        })?;

        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast transition: {}", e)))?;

        // Format and return result
        self.format_token_result(proof_result)
    }

    /// Set or update the price for direct token purchases.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - The ID of the data contract containing the token
    /// * `token_position` - The position of the token in the contract (0-indexed)
    /// * `identity_id` - The identity ID of the actor setting the price
    /// * `price_type` - The pricing type: "single" or "tiered"
    /// * `price_data` - JSON string with pricing data (single price or tiered pricing map)
    /// * `private_key_wif` - The private key in WIF format for signing
    /// * `key_id` - The key ID to use for signing
    /// * `public_note` - Optional public note for the price change
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the state transition result
    #[wasm_bindgen(js_name = tokenSetPriceForDirectPurchase)]
    pub async fn token_set_price_for_direct_purchase(
        &self,
        data_contract_id: String,
        token_position: u16,
        identity_id: String,
        price_type: String,
        price_data: String,
        private_key_wif: String,
        public_note: Option<String>,
    ) -> Result<JsValue, JsValue> {
        use dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
        use dash_sdk::dpp::fee::Credits;
        use std::collections::BTreeMap;

        let sdk = self.inner_clone();

        // Parse identifiers
        let (contract_id, actor_id, _, _) = self
            .parse_token_params(
                &data_contract_id,
                &identity_id,
                "0", // Amount not needed for setting price
                None,
            )
            .await?;

        // Fetch and cache the contract
        self.fetch_and_cache_token_contract(contract_id).await?;

        // Parse pricing schedule
        let pricing_schedule = if price_data.is_empty() || price_data == "null" {
            // Empty price_data means remove pricing (make not purchasable)
            None
        } else {
            match price_type.to_lowercase().as_str() {
                "single" => {
                    // Parse single price
                    let price_credits: Credits = price_data
                        .parse::<u64>()
                        .map_err(|e| JsValue::from_str(&format!("Invalid price credits: {}", e)))?;
                    Some(TokenPricingSchedule::SinglePrice(price_credits))
                }
                "tiered" | "set" => {
                    // Parse tiered pricing map from JSON
                    let price_map: std::collections::HashMap<String, u64> =
                        serde_json::from_str(&price_data).map_err(|e| {
                            JsValue::from_str(&format!("Invalid tiered pricing JSON: {}", e))
                        })?;

                    // Convert to BTreeMap<TokenAmount, Credits>
                    let mut btree_map = BTreeMap::new();
                    for (amount_str, credits) in price_map {
                        let amount: TokenAmount = amount_str.parse().map_err(|e| {
                            JsValue::from_str(&format!(
                                "Invalid token amount '{}': {}",
                                amount_str, e
                            ))
                        })?;
                        btree_map.insert(amount, credits);
                    }

                    if btree_map.is_empty() {
                        return Err(JsValue::from_str("Tiered pricing map cannot be empty"));
                    }

                    Some(TokenPricingSchedule::SetPrices(btree_map))
                }
                _ => {
                    return Err(JsValue::from_str(
                        "Invalid price type. Use 'single' or 'tiered'",
                    ))
                }
            }
        };

        // Get identity to find matching authentication key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, actor_id)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;

        // Find matching authentication key and create signer
        let (_, matching_key) =
            crate::sdk::WasmSdk::find_authentication_key(&identity, &private_key_wif)?;
        let signer = crate::sdk::WasmSdk::create_signer_from_wif(&private_key_wif, sdk.network)?;
        let public_key = matching_key.clone();

        // Calculate token ID
        let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), token_position));

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(actor_id, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;

        // Create the state transition
        let platform_version = sdk.version();
        let state_transition = BatchTransition::new_token_change_direct_purchase_price_transition(
            token_id,
            actor_id,
            contract_id,
            token_position,
            pricing_schedule,
            public_note,
            None, // using_group_info
            &public_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            &signer,
            platform_version,
            None, // state_transition_creation_options
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create set price transition: {}", e)))?;

        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast transition: {}", e)))?;

        // Format and return result based on the proof result type
        match proof_result {
            StateTransitionProofResult::VerifiedTokenPricingSchedule(owner_id, schedule) => {
                to_value(&serde_json::json!({
                    "type": "VerifiedTokenPricingSchedule",
                    "ownerId": owner_id.to_string(Encoding::Base58),
                    "pricingSchedule": schedule.map(|s| match s {
                        TokenPricingSchedule::SinglePrice(credits) => serde_json::json!({
                            "type": "single",
                            "price": credits
                        }),
                        TokenPricingSchedule::SetPrices(prices) => {
                            let price_map: std::collections::HashMap<String, u64> = prices
                                .into_iter()
                                .map(|(amount, credits)| (amount.to_string(), credits))
                                .collect();
                            serde_json::json!({
                                "type": "tiered",
                                "prices": price_map
                            })
                        }
                    })
                }))
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
            }
            StateTransitionProofResult::VerifiedTokenGroupActionWithTokenPricingSchedule(
                power,
                status,
                schedule,
            ) => to_value(&serde_json::json!({
                "type": "VerifiedTokenGroupActionWithTokenPricingSchedule",
                "groupPower": power,
                "status": format!("{:?}", status),
                "pricingSchedule": schedule.map(|s| match s {
                    TokenPricingSchedule::SinglePrice(credits) => serde_json::json!({
                        "type": "single",
                        "price": credits
                    }),
                    TokenPricingSchedule::SetPrices(prices) => {
                        let price_map: std::collections::HashMap<String, u64> = prices
                            .into_iter()
                            .map(|(amount, credits)| (amount.to_string(), credits))
                            .collect();
                        serde_json::json!({
                            "type": "tiered",
                            "prices": price_map
                        })
                    }
                })
            }))
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e))),
            _ => self.format_token_result(proof_result),
        }
    }

    /// Purchase tokens directly at the configured price.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - The ID of the data contract containing the token
    /// * `token_position` - The position of the token in the contract (0-indexed)
    /// * `amount` - The amount of tokens to purchase
    /// * `identity_id` - The identity ID of the purchaser
    /// * `total_agreed_price` - The total price in credits for the purchase
    /// * `private_key_wif` - The private key in WIF format for signing
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the state transition result
    #[wasm_bindgen(js_name = tokenDirectPurchase)]
    pub async fn token_direct_purchase(
        &self,
        data_contract_id: String,
        token_position: u16,
        amount: String,
        identity_id: String,
        total_agreed_price: Option<String>,
        private_key_wif: String,
    ) -> Result<JsValue, JsValue> {
        use dash_sdk::dpp::fee::Credits;

        let sdk = self.inner_clone();

        // Parse and validate parameters
        let (contract_id, purchaser_id, token_amount, _) = self
            .parse_token_params(&data_contract_id, &identity_id, &amount, None)
            .await?;

        // Get total price - either from parameter or fetch from pricing schedule
        let price_credits: Credits = match total_agreed_price {
            Some(price_str) => {
                // Use provided price
                price_str
                    .parse::<u64>()
                    .map_err(|e| JsValue::from_str(&format!("Invalid total agreed price: {}", e)))?
            }
            None => {
                // Fetch price from pricing schedule
                let token_id = crate::queries::token::calculate_token_id_from_contract(
                    &data_contract_id,
                    token_position,
                )
                .map_err(|e| {
                    JsValue::from_str(&format!("Failed to calculate token ID: {:?}", e))
                })?;

                let token_ids = vec![token_id];
                let prices =
                    crate::queries::token::get_token_direct_purchase_prices(self, token_ids)
                        .await
                        .map_err(|e| {
                            JsValue::from_str(&format!("Failed to fetch token price: {:?}", e))
                        })?;

                // Use js_sys to work with JavaScript objects
                use js_sys::{Reflect, Array};

                // Get the prices array from the result
                let prices_prop = Reflect::get(&prices, &JsValue::from_str("prices"))
                    .map_err(|_| JsValue::from_str("Failed to get prices property"))?;

                // Convert to array and get first element
                let prices_array = Array::from(&prices_prop);
                if prices_array.length() == 0 {
                    return Err(JsValue::from_str("No prices found for token"));
                }

                let first_price = prices_array.get(0);

                // Get current price from the price object
                let current_price_prop =
                    Reflect::get(&first_price, &JsValue::from_str("currentPrice"))
                        .map_err(|_| JsValue::from_str("Failed to get currentPrice property"))?;

                // Convert to string and parse
                let price_str = current_price_prop
                    .as_string()
                    .ok_or_else(|| JsValue::from_str("Current price is not a string"))?;

                let price_per_token = price_str.parse::<u64>().map_err(|e| {
                    JsValue::from_str(&format!("Invalid current price format: {}", e))
                })?;

                price_per_token * token_amount
            }
        };

        // Fetch and cache the contract
        self.fetch_and_cache_token_contract(contract_id).await?;

        // Get identity to find matching authentication key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, purchaser_id)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;

        // Find matching authentication key and create signer
        let (_, matching_key) =
            crate::sdk::WasmSdk::find_authentication_key(&identity, &private_key_wif)?;
        let signer = crate::sdk::WasmSdk::create_signer_from_wif(&private_key_wif, sdk.network)?;
        let public_key = matching_key.clone();

        // Calculate token ID
        let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), token_position));

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(purchaser_id, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;

        // Create the state transition
        let platform_version = sdk.version();
        let state_transition = BatchTransition::new_token_direct_purchase_transition(
            token_id,
            purchaser_id,
            contract_id,
            token_position,
            token_amount,
            price_credits,
            &public_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            &signer,
            platform_version,
            None, // state_transition_creation_options
        )
        .map_err(|e| {
            JsValue::from_str(&format!(
                "Failed to create direct purchase transition: {}",
                e
            ))
        })?;

        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast transition: {}", e)))?;

        // Format and return result
        self.format_token_result(proof_result)
    }

    /// Claim tokens from a distribution
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - ID of the data contract containing the token
    /// * `token_position` - Position of the token within the contract
    /// * `distribution_type` - Type of distribution: "perpetual" or "preprogrammed"
    /// * `identity_id` - Identity ID of the claimant
    /// * `private_key_wif` - Private key in WIF format
    /// * `public_note` - Optional public note
    ///
    /// Returns a Promise that resolves to a JsValue containing the state transition result
    #[wasm_bindgen(js_name = tokenClaim)]
    pub async fn token_claim(
        &self,
        data_contract_id: String,
        token_position: u16,
        distribution_type: String,
        identity_id: String,
        private_key_wif: String,
        public_note: Option<String>,
    ) -> Result<JsValue, JsValue> {
        use dash_sdk::dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;

        let sdk = self.inner_clone();

        // Parse identifiers
        let (contract_id, identity_identifier, _, _) = self
            .parse_token_params(
                &data_contract_id,
                &identity_id,
                "0", // Amount not needed for claim
                None,
            )
            .await?;

        // Fetch and cache the contract
        self.fetch_and_cache_token_contract(contract_id).await?;

        // Parse distribution type
        let dist_type = match distribution_type.to_lowercase().as_str() {
            "perpetual" => TokenDistributionType::Perpetual,
            "preprogrammed" | "pre-programmed" | "scheduled" => {
                TokenDistributionType::PreProgrammed
            }
            _ => {
                return Err(JsValue::from_str(
                    "Invalid distribution type. Use 'perpetual' or 'preprogrammed'",
                ))
            }
        };

        // Get identity to find matching authentication key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, identity_identifier)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;

        // Find matching authentication key and create signer
        let (_, matching_key) =
            crate::sdk::WasmSdk::find_authentication_key(&identity, &private_key_wif)?;
        let signer = crate::sdk::WasmSdk::create_signer_from_wif(&private_key_wif, sdk.network)?;
        let public_key = matching_key.clone();

        // Calculate token ID
        let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), token_position));

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(identity_identifier, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;

        // Create the state transition directly as a token claim transition
        let platform_version = sdk.version();
        // Create state transition using BatchTransition's token claim method
        let state_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity_identifier,
            contract_id,
            token_position,
            dist_type,
            public_note,
            &public_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            &signer,
            platform_version,
            None, // state_transition_creation_options
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create claim transition: {}", e)))?;

        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast transition: {}", e)))?;

        // Format and return result
        self.format_token_result(proof_result)
    }

    /// Update token configuration settings.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - The ID of the data contract containing the token
    /// * `token_position` - The position of the token in the contract (0-indexed)
    /// * `config_item_type` - The type of configuration to update
    /// * `config_value` - The new configuration value (JSON string)
    /// * `identity_id` - The identity ID of the owner/admin
    /// * `private_key_wif` - The private key in WIF format for signing
    /// * `public_note` - Optional public note for the configuration change
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the state transition result
    #[wasm_bindgen(js_name = tokenConfigUpdate)]
    pub async fn token_config_update(
        &self,
        data_contract_id: String,
        token_position: u16,
        config_item_type: String,
        config_value: String,
        identity_id: String,
        private_key_wif: String,
        public_note: Option<String>,
    ) -> Result<JsValue, JsValue> {
        use dash_sdk::dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
        use dash_sdk::dpp::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
        use dash_sdk::dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
        use dash_sdk::dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;

        let sdk = self.inner_clone();

        // Parse identifiers
        let (contract_id, owner_id, _, _) = self
            .parse_token_params(
                &data_contract_id,
                &identity_id,
                "0", // Amount not needed for config update
                None,
            )
            .await?;

        // Fetch and cache the contract
        self.fetch_and_cache_token_contract(contract_id).await?;

        // Parse configuration change item based on type
        let config_change_item = match config_item_type.as_str() {
            "conventions" => {
                // Parse JSON for conventions
                let convention: TokenConfigurationConvention = serde_json::from_str(&config_value)
                    .map_err(|e| JsValue::from_str(&format!("Invalid conventions JSON: {}", e)))?;
                TokenConfigurationChangeItem::Conventions(convention)
            }
            "max_supply" => {
                if config_value.is_empty() || config_value == "null" {
                    TokenConfigurationChangeItem::MaxSupply(None)
                } else {
                    let max_supply: TokenAmount = config_value
                        .parse()
                        .map_err(|e| JsValue::from_str(&format!("Invalid max supply: {}", e)))?;
                    TokenConfigurationChangeItem::MaxSupply(Some(max_supply))
                }
            }
            "perpetual_distribution" => {
                if config_value.is_empty() || config_value == "null" {
                    TokenConfigurationChangeItem::PerpetualDistribution(None)
                } else {
                    // Parse JSON for perpetual distribution config
                    let distribution: TokenPerpetualDistribution =
                        serde_json::from_str(&config_value).map_err(|e| {
                            JsValue::from_str(&format!(
                                "Invalid perpetual distribution JSON: {}",
                                e
                            ))
                        })?;
                    TokenConfigurationChangeItem::PerpetualDistribution(Some(distribution))
                }
            }
            "new_tokens_destination_identity" => {
                if config_value.is_empty() || config_value == "null" {
                    TokenConfigurationChangeItem::NewTokensDestinationIdentity(None)
                } else {
                    let dest_id = Identifier::from_string(&config_value, Encoding::Base58)
                        .map_err(|e| {
                            JsValue::from_str(&format!("Invalid destination identity ID: {}", e))
                        })?;
                    TokenConfigurationChangeItem::NewTokensDestinationIdentity(Some(dest_id))
                }
            }
            "minting_allow_choosing_destination" => {
                let allow: bool = config_value
                    .parse()
                    .map_err(|_| JsValue::from_str("Invalid boolean value"))?;
                TokenConfigurationChangeItem::MintingAllowChoosingDestination(allow)
            }
            "manual_minting"
            | "manual_burning"
            | "conventions_control_group"
            | "conventions_admin_group"
            | "max_supply_control_group"
            | "max_supply_admin_group"
            | "perpetual_distribution_control_group"
            | "perpetual_distribution_admin_group"
            | "new_tokens_destination_identity_control_group"
            | "new_tokens_destination_identity_admin_group"
            | "minting_allow_choosing_destination_control_group"
            | "minting_allow_choosing_destination_admin_group"
            | "manual_minting_admin_group"
            | "manual_burning_admin_group" => {
                // Parse AuthorizedActionTakers from JSON
                let action_takers: AuthorizedActionTakers = serde_json::from_str(&config_value)
                    .map_err(|e| {
                        JsValue::from_str(&format!("Invalid authorized action takers JSON: {}", e))
                    })?;

                match config_item_type.as_str() {
                    "manual_minting" => TokenConfigurationChangeItem::ManualMinting(action_takers),
                    "manual_burning" => TokenConfigurationChangeItem::ManualBurning(action_takers),
                    "conventions_control_group" => {
                        TokenConfigurationChangeItem::ConventionsControlGroup(action_takers)
                    }
                    "conventions_admin_group" => {
                        TokenConfigurationChangeItem::ConventionsAdminGroup(action_takers)
                    }
                    "max_supply_control_group" => {
                        TokenConfigurationChangeItem::MaxSupplyControlGroup(action_takers)
                    }
                    "max_supply_admin_group" => {
                        TokenConfigurationChangeItem::MaxSupplyAdminGroup(action_takers)
                    }
                    "perpetual_distribution_control_group" => {
                        TokenConfigurationChangeItem::PerpetualDistributionControlGroup(
                            action_takers,
                        )
                    }
                    "perpetual_distribution_admin_group" => {
                        TokenConfigurationChangeItem::PerpetualDistributionAdminGroup(action_takers)
                    }
                    "new_tokens_destination_identity_control_group" => {
                        TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(
                            action_takers,
                        )
                    }
                    "new_tokens_destination_identity_admin_group" => {
                        TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(
                            action_takers,
                        )
                    }
                    "minting_allow_choosing_destination_control_group" => {
                        TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                            action_takers,
                        )
                    }
                    "minting_allow_choosing_destination_admin_group" => {
                        TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(
                            action_takers,
                        )
                    }
                    "manual_minting_admin_group" => {
                        TokenConfigurationChangeItem::ManualMintingAdminGroup(action_takers)
                    }
                    "manual_burning_admin_group" => {
                        TokenConfigurationChangeItem::ManualBurningAdminGroup(action_takers)
                    }
                    _ => unreachable!(),
                }
            }
            _ => {
                return Err(JsValue::from_str(&format!(
                    "Invalid config item type: {}",
                    config_item_type
                )))
            }
        };

        // Get identity to find matching authentication key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, owner_id)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;

        // Find matching authentication key and create signer
        let (_, matching_key) =
            crate::sdk::WasmSdk::find_authentication_key(&identity, &private_key_wif)?;
        let signer = crate::sdk::WasmSdk::create_signer_from_wif(&private_key_wif, sdk.network)?;
        let public_key = matching_key.clone();

        // Calculate token ID
        let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), token_position));

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(owner_id, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;

        // Create the state transition
        let platform_version = sdk.version();
        let state_transition = BatchTransition::new_token_config_update_transition(
            token_id,
            owner_id,
            contract_id,
            token_position,
            config_change_item,
            public_note,
            None, // using_group_info
            &public_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            &signer,
            platform_version,
            None, // state_transition_creation_options
        )
        .map_err(|e| {
            JsValue::from_str(&format!("Failed to create config update transition: {}", e))
        })?;

        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast transition: {}", e)))?;

        // Format and return result
        self.format_token_result(proof_result)
    }
}
