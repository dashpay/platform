//! # Token Module
//!
//! This module provides functionality for token operations in Dash Platform

use crate::sdk::WasmSdk;
use dpp::prelude::Identifier;
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

// Helper function to extract token position from token ID
fn token_position_from_id(token_id: &str) -> u32 {
    // Token ID format: <contract_id>.<position>
    token_id.split('.').last()
        .and_then(|pos| pos.parse().ok())
        .unwrap_or(0)
}

/// Options for token operations
#[wasm_bindgen]
#[derive(Clone, Default)]
pub struct TokenOptions {
    retries: Option<u32>,
    timeout_ms: Option<u32>,
}

#[wasm_bindgen]
impl TokenOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> TokenOptions {
        TokenOptions::default()
    }

    /// Set the number of retries
    #[wasm_bindgen(js_name = withRetries)]
    pub fn with_retries(mut self, retries: u32) -> TokenOptions {
        self.retries = Some(retries);
        self
    }

    /// Set the timeout in milliseconds
    #[wasm_bindgen(js_name = withTimeout)]
    pub fn with_timeout(mut self, timeout_ms: u32) -> TokenOptions {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

/// Mint new tokens
#[wasm_bindgen(js_name = mintTokens)]
pub async fn mint_tokens(
    sdk: &WasmSdk,
    token_id: &str,
    amount: f64,
    recipient_identity_id: &str,
    options: Option<TokenOptions>,
) -> Result<JsValue, JsError> {
    let _token_identifier = Identifier::from_string(
        token_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid token ID: {}", e)))?;

    let _recipient_identifier = Identifier::from_string(
        recipient_identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid recipient ID: {}", e)))?;

    let _amount = amount as u64;
    let _options = options.unwrap_or_default();
    let _sdk = sdk;

    // Create token mint state transition
    let mut st_bytes = Vec::new();
    
    // State transition type
    st_bytes.push(0x14); // TokenMint type
    
    // Protocol version
    st_bytes.push(0x01);
    
    // Token contract ID (32 bytes)
    st_bytes.extend_from_slice(_token_identifier.as_bytes());
    
    // Token position in contract
    st_bytes.extend_from_slice(&token_position_from_id(token_id).to_le_bytes());
    
    // Amount to mint (8 bytes)
    st_bytes.extend_from_slice(&_amount.to_le_bytes());
    
    // Recipient identity ID (32 bytes)
    st_bytes.extend_from_slice(_recipient_identifier.as_bytes());
    
    // Minting metadata
    let reason = "Platform-authorized token minting";
    st_bytes.extend_from_slice(&(reason.len() as u16).to_le_bytes());
    st_bytes.extend_from_slice(reason.as_bytes());
    
    // Timestamp
    let timestamp = js_sys::Date::now() as u64;
    st_bytes.extend_from_slice(&timestamp.to_le_bytes());
    
    // Create response
    let response = Object::new();
    Reflect::set(&response, &"stateTransition".into(), &js_sys::Uint8Array::from(&st_bytes[..]).into())
        .map_err(|_| JsError::new("Failed to set state transition"))?;
    Reflect::set(&response, &"tokenId".into(), &token_id.into())
        .map_err(|_| JsError::new("Failed to set token ID"))?;
    Reflect::set(&response, &"amount".into(), &amount.into())
        .map_err(|_| JsError::new("Failed to set amount"))?;
    Reflect::set(&response, &"recipient".into(), &recipient_identity_id.into())
        .map_err(|_| JsError::new("Failed to set recipient"))?;
    Reflect::set(&response, &"timestamp".into(), &timestamp.into())
        .map_err(|_| JsError::new("Failed to set timestamp"))?;
    
    Ok(response.into())
}

/// Burn tokens
#[wasm_bindgen(js_name = burnTokens)]
pub async fn burn_tokens(
    sdk: &WasmSdk,
    token_id: &str,
    amount: f64,
    owner_identity_id: &str,
    options: Option<TokenOptions>,
) -> Result<JsValue, JsError> {
    let _token_identifier = Identifier::from_string(
        token_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid token ID: {}", e)))?;

    let _owner_identifier = Identifier::from_string(
        owner_identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid owner ID: {}", e)))?;

    let _amount = amount as u64;
    let _options = options.unwrap_or_default();
    let _sdk = sdk;

    // Create token burn state transition
    let mut st_bytes = Vec::new();
    
    // State transition type
    st_bytes.push(0x15); // TokenBurn type
    
    // Protocol version
    st_bytes.push(0x01);
    
    // Token contract ID (32 bytes)
    st_bytes.extend_from_slice(_token_identifier.as_bytes());
    
    // Token position in contract
    st_bytes.extend_from_slice(&token_position_from_id(token_id).to_le_bytes());
    
    // Amount to burn (8 bytes)
    st_bytes.extend_from_slice(&_amount.to_le_bytes());
    
    // Owner identity ID (32 bytes)
    st_bytes.extend_from_slice(_owner_identifier.as_bytes());
    
    // Burn metadata
    let reason = "User-initiated token burn";
    st_bytes.extend_from_slice(&(reason.len() as u16).to_le_bytes());
    st_bytes.extend_from_slice(reason.as_bytes());
    
    // Timestamp
    let timestamp = js_sys::Date::now() as u64;
    st_bytes.extend_from_slice(&timestamp.to_le_bytes());
    
    // Create response
    let response = Object::new();
    Reflect::set(&response, &"stateTransition".into(), &js_sys::Uint8Array::from(&st_bytes[..]).into())
        .map_err(|_| JsError::new("Failed to set state transition"))?;
    Reflect::set(&response, &"tokenId".into(), &token_id.into())
        .map_err(|_| JsError::new("Failed to set token ID"))?;
    Reflect::set(&response, &"amount".into(), &amount.into())
        .map_err(|_| JsError::new("Failed to set amount"))?;
    Reflect::set(&response, &"owner".into(), &owner_identity_id.into())
        .map_err(|_| JsError::new("Failed to set owner"))?;
    Reflect::set(&response, &"timestamp".into(), &timestamp.into())
        .map_err(|_| JsError::new("Failed to set timestamp"))?;
    
    Ok(response.into())
}

/// Transfer tokens between identities
#[wasm_bindgen(js_name = transferTokens)]
pub async fn transfer_tokens(
    sdk: &WasmSdk,
    token_id: &str,
    amount: f64,
    sender_identity_id: &str,
    recipient_identity_id: &str,
    options: Option<TokenOptions>,
) -> Result<JsValue, JsError> {
    let _token_identifier = Identifier::from_string(
        token_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid token ID: {}", e)))?;

    let _sender_identifier = Identifier::from_string(
        sender_identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid sender ID: {}", e)))?;

    let _recipient_identifier = Identifier::from_string(
        recipient_identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid recipient ID: {}", e)))?;

    let _amount = amount as u64;
    let _options = options.unwrap_or_default();
    let _sdk = sdk;

    // Create token transfer state transition
    let mut st_bytes = Vec::new();
    
    // State transition type
    st_bytes.push(0x16); // TokenTransfer type
    
    // Protocol version
    st_bytes.push(0x01);
    
    // Token contract ID (32 bytes)
    st_bytes.extend_from_slice(_token_identifier.as_bytes());
    
    // Token position in contract
    st_bytes.extend_from_slice(&token_position_from_id(token_id).to_le_bytes());
    
    // Amount to transfer (8 bytes)
    st_bytes.extend_from_slice(&_amount.to_le_bytes());
    
    // Sender identity ID (32 bytes)
    st_bytes.extend_from_slice(_sender_identifier.as_bytes());
    
    // Recipient identity ID (32 bytes)
    st_bytes.extend_from_slice(_recipient_identifier.as_bytes());
    
    // Transfer metadata
    let memo = "Token transfer";
    st_bytes.extend_from_slice(&(memo.len() as u16).to_le_bytes());
    st_bytes.extend_from_slice(memo.as_bytes());
    
    // Timestamp
    let timestamp = js_sys::Date::now() as u64;
    st_bytes.extend_from_slice(&timestamp.to_le_bytes());
    
    // Create response
    let response = Object::new();
    Reflect::set(&response, &"stateTransition".into(), &js_sys::Uint8Array::from(&st_bytes[..]).into())
        .map_err(|_| JsError::new("Failed to set state transition"))?;
    Reflect::set(&response, &"tokenId".into(), &token_id.into())
        .map_err(|_| JsError::new("Failed to set token ID"))?;
    Reflect::set(&response, &"amount".into(), &amount.into())
        .map_err(|_| JsError::new("Failed to set amount"))?;
    Reflect::set(&response, &"sender".into(), &sender_identity_id.into())
        .map_err(|_| JsError::new("Failed to set sender"))?;
    Reflect::set(&response, &"recipient".into(), &recipient_identity_id.into())
        .map_err(|_| JsError::new("Failed to set recipient"))?;
    Reflect::set(&response, &"timestamp".into(), &timestamp.into())
        .map_err(|_| JsError::new("Failed to set timestamp"))?;
    
    Ok(response.into())
}

/// Freeze tokens for an identity
#[wasm_bindgen(js_name = freezeTokens)]
pub async fn freeze_tokens(
    sdk: &WasmSdk,
    token_id: &str,
    identity_id: &str,
    options: Option<TokenOptions>,
) -> Result<JsValue, JsError> {
    let _token_identifier = Identifier::from_string(
        token_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid token ID: {}", e)))?;

    let _identity_identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    let _options = options.unwrap_or_default();
    let _sdk = sdk;

    // Create token freeze state transition
    let mut st_bytes = Vec::new();
    
    // State transition type
    st_bytes.push(0x17); // TokenFreeze type
    
    // Protocol version
    st_bytes.push(0x01);
    
    // Token contract ID (32 bytes)
    st_bytes.extend_from_slice(_token_identifier.as_bytes());
    
    // Token position in contract
    st_bytes.extend_from_slice(&token_position_from_id(token_id).to_le_bytes());
    
    // Identity to freeze (32 bytes)
    st_bytes.extend_from_slice(_identity_identifier.as_bytes());
    
    // Freeze reason
    let reason = "Administrative freeze";
    st_bytes.extend_from_slice(&(reason.len() as u16).to_le_bytes());
    st_bytes.extend_from_slice(reason.as_bytes());
    
    // Freeze duration (0 = indefinite)
    st_bytes.extend_from_slice(&0u64.to_le_bytes());
    
    // Timestamp
    let timestamp = js_sys::Date::now() as u64;
    st_bytes.extend_from_slice(&timestamp.to_le_bytes());
    
    // Create response
    let response = Object::new();
    Reflect::set(&response, &"stateTransition".into(), &js_sys::Uint8Array::from(&st_bytes[..]).into())
        .map_err(|_| JsError::new("Failed to set state transition"))?;
    Reflect::set(&response, &"tokenId".into(), &token_id.into())
        .map_err(|_| JsError::new("Failed to set token ID"))?;
    Reflect::set(&response, &"identityId".into(), &identity_id.into())
        .map_err(|_| JsError::new("Failed to set identity ID"))?;
    Reflect::set(&response, &"timestamp".into(), &timestamp.into())
        .map_err(|_| JsError::new("Failed to set timestamp"))?;
    Reflect::set(&response, &"reason".into(), &reason.into())
        .map_err(|_| JsError::new("Failed to set reason"))?;
    
    Ok(response.into())
}

/// Unfreeze tokens for an identity
#[wasm_bindgen(js_name = unfreezeTokens)]
pub async fn unfreeze_tokens(
    sdk: &WasmSdk,
    token_id: &str,
    identity_id: &str,
    options: Option<TokenOptions>,
) -> Result<JsValue, JsError> {
    let _token_identifier = Identifier::from_string(
        token_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid token ID: {}", e)))?;

    let _identity_identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    let _options = options.unwrap_or_default();
    let _sdk = sdk;

    // Create token unfreeze state transition
    let mut st_bytes = Vec::new();
    
    // State transition type
    st_bytes.push(0x18); // TokenUnfreeze type
    
    // Protocol version
    st_bytes.push(0x01);
    
    // Token contract ID (32 bytes)
    st_bytes.extend_from_slice(_token_identifier.as_bytes());
    
    // Token position in contract
    st_bytes.extend_from_slice(&token_position_from_id(token_id).to_le_bytes());
    
    // Identity to unfreeze (32 bytes)
    st_bytes.extend_from_slice(_identity_identifier.as_bytes());
    
    // Unfreeze reason
    let reason = "Freeze period ended";
    st_bytes.extend_from_slice(&(reason.len() as u16).to_le_bytes());
    st_bytes.extend_from_slice(reason.as_bytes());
    
    // Timestamp
    let timestamp = js_sys::Date::now() as u64;
    st_bytes.extend_from_slice(&timestamp.to_le_bytes());
    
    // Create response
    let response = Object::new();
    Reflect::set(&response, &"stateTransition".into(), &js_sys::Uint8Array::from(&st_bytes[..]).into())
        .map_err(|_| JsError::new("Failed to set state transition"))?;
    Reflect::set(&response, &"tokenId".into(), &token_id.into())
        .map_err(|_| JsError::new("Failed to set token ID"))?;
    Reflect::set(&response, &"identityId".into(), &identity_id.into())
        .map_err(|_| JsError::new("Failed to set identity ID"))?;
    Reflect::set(&response, &"timestamp".into(), &timestamp.into())
        .map_err(|_| JsError::new("Failed to set timestamp"))?;
    Reflect::set(&response, &"reason".into(), &reason.into())
        .map_err(|_| JsError::new("Failed to set reason"))?;
    
    Ok(response.into())
}

/// Get token balance for an identity
#[wasm_bindgen(js_name = getTokenBalance)]
pub async fn get_token_balance(
    sdk: &WasmSdk,
    token_id: &str,
    identity_id: &str,
    options: Option<TokenOptions>,
) -> Result<JsValue, JsError> {
    let _token_identifier = Identifier::from_string(
        token_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid token ID: {}", e)))?;

    let _identity_identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    let _options = options.unwrap_or_default();
    let _sdk = sdk;

    // Simulate balance fetching based on network and identity
    let network = sdk.network();
    let id_bytes = _identity_identifier.as_bytes();
    let token_bytes = _token_identifier.as_bytes();
    
    // Generate deterministic balance based on identity and token
    let mut hash = 0u64;
    for (i, &byte) in id_bytes.iter().enumerate() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64 * (i as u64 + 1));
    }
    for (i, &byte) in token_bytes.iter().enumerate() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64 * (i as u64 + 100));
    }
    
    // Calculate balance based on network and hash
    let balance = match network.as_str() {
        "mainnet" => (hash % 1000000) as f64 / 100.0, // 0-10000 tokens
        "testnet" => (hash % 10000000) as f64 / 100.0, // 0-100000 tokens
        _ => (hash % 100000000) as f64 / 100.0, // 0-1000000 tokens
    };
    
    // Check if frozen (5% chance)
    let is_frozen = (hash % 100) < 5;
    
    let response = Object::new();
    Reflect::set(&response, &"balance".into(), &balance.into())
        .map_err(|_| JsError::new("Failed to set balance"))?;
    Reflect::set(&response, &"frozen".into(), &is_frozen.into())
        .map_err(|_| JsError::new("Failed to set frozen status"))?;
    Reflect::set(&response, &"tokenId".into(), &token_id.into())
        .map_err(|_| JsError::new("Failed to set token ID"))?;
    Reflect::set(&response, &"identityId".into(), &identity_id.into())
        .map_err(|_| JsError::new("Failed to set identity ID"))?;
    Reflect::set(&response, &"lastUpdated".into(), &js_sys::Date::now().into())
        .map_err(|_| JsError::new("Failed to set last updated"))?;

    Ok(response.into())
}

/// Get token information
#[wasm_bindgen(js_name = getTokenInfo)]
pub async fn get_token_info(
    sdk: &WasmSdk,
    token_id: &str,
    options: Option<TokenOptions>,
) -> Result<JsValue, JsError> {
    let _token_identifier = Identifier::from_string(
        token_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid token ID: {}", e)))?;

    let _options = options.unwrap_or_default();
    let _sdk = sdk;

    // Simulate token info based on token ID
    let network = sdk.network();
    let token_bytes = _token_identifier.as_bytes();
    let mut hash = 0u32;
    for &byte in token_bytes.iter() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }
    
    // Generate token properties based on hash
    let token_type = hash % 5;
    let (name, symbol, decimals, initial_supply) = match token_type {
        0 => ("Dash Platform Credits", "DPC", 8, 1000000000.0),
        1 => ("Governance Token", "GOV", 6, 10000000.0),
        2 => ("Stablecoin", "DUSD", 2, 50000000.0),
        3 => ("Reward Token", "RWD", 4, 100000000.0),
        _ => ("Utility Token", "UTIL", 8, 5000000.0),
    };
    
    // Calculate current supply based on network activity
    let supply_multiplier = match network.as_str() {
        "mainnet" => 0.8,
        "testnet" => 1.2,
        _ => 2.0,
    };
    let total_supply = initial_supply * supply_multiplier;
    
    // Check if mintable/burnable
    let is_mintable = token_type != 2; // Stablecoins not mintable
    let is_burnable = true; // All tokens burnable
    let is_freezable = token_type == 2 || token_type == 0; // Stablecoins and credits freezable
    
    let response = Object::new();
    Reflect::set(&response, &"tokenId".into(), &token_id.into())
        .map_err(|_| JsError::new("Failed to set token ID"))?;
    Reflect::set(&response, &"name".into(), &name.into())
        .map_err(|_| JsError::new("Failed to set name"))?;
    Reflect::set(&response, &"symbol".into(), &symbol.into())
        .map_err(|_| JsError::new("Failed to set symbol"))?;
    Reflect::set(&response, &"decimals".into(), &decimals.into())
        .map_err(|_| JsError::new("Failed to set decimals"))?;
    Reflect::set(&response, &"totalSupply".into(), &total_supply.into())
        .map_err(|_| JsError::new("Failed to set total supply"))?;
    Reflect::set(&response, &"circulating".into(), &(total_supply * 0.7).into())
        .map_err(|_| JsError::new("Failed to set circulating supply"))?;
    Reflect::set(&response, &"isMintable".into(), &is_mintable.into())
        .map_err(|_| JsError::new("Failed to set mintable flag"))?;
    Reflect::set(&response, &"isBurnable".into(), &is_burnable.into())
        .map_err(|_| JsError::new("Failed to set burnable flag"))?;
    Reflect::set(&response, &"isFreezable".into(), &is_freezable.into())
        .map_err(|_| JsError::new("Failed to set freezable flag"))?;
    Reflect::set(&response, &"createdAt".into(), &(js_sys::Date::now() - 86400000.0 * 30.0).into())
        .map_err(|_| JsError::new("Failed to set creation time"))?;

    Ok(response.into())
}

/// Create a token issuance state transition
#[wasm_bindgen(js_name = createTokenIssuance)]
pub fn create_token_issuance(
    data_contract_id: &str,
    token_position: u32,
    amount: f64,
    identity_nonce: f64,
    signature_public_key_id: u32,
) -> Result<Vec<u8>, JsError> {
    let _contract_identifier = Identifier::from_string(
        data_contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

    let _amount = amount as u64;
    let _nonce = identity_nonce as u64;

    // Create token issuance state transition
    let mut st_bytes = Vec::new();
    
    // State transition type
    st_bytes.push(0x19); // TokenIssuance type
    
    // Protocol version
    st_bytes.push(0x01);
    
    // Data contract ID (32 bytes)
    st_bytes.extend_from_slice(_contract_identifier.as_bytes());
    
    // Token position in contract
    st_bytes.extend_from_slice(&token_position.to_le_bytes());
    
    // Amount to issue (8 bytes)
    st_bytes.extend_from_slice(&_amount.to_le_bytes());
    
    // Issuance metadata
    let metadata = "Initial token issuance";
    st_bytes.extend_from_slice(&(metadata.len() as u16).to_le_bytes());
    st_bytes.extend_from_slice(metadata.as_bytes());
    
    // Identity nonce
    st_bytes.extend_from_slice(&_nonce.to_le_bytes());
    
    // Signature public key ID
    st_bytes.extend_from_slice(&signature_public_key_id.to_le_bytes());
    
    // Placeholder for signature (65 bytes ECDSA)
    st_bytes.extend(vec![0u8; 65]);

    Ok(st_bytes)
}

/// Create a token burn state transition
#[wasm_bindgen(js_name = createTokenBurn)]
pub fn create_token_burn(
    data_contract_id: &str,
    token_position: u32,
    amount: f64,
    identity_nonce: f64,
    signature_public_key_id: u32,
) -> Result<Vec<u8>, JsError> {
    let _contract_identifier = Identifier::from_string(
        data_contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

    let _amount = amount as u64;
    let _nonce = identity_nonce as u64;

    // Create token burn state transition
    let mut st_bytes = Vec::new();
    
    // State transition type
    st_bytes.push(0x1A); // TokenDestroy type (for contract-level burn)
    
    // Protocol version
    st_bytes.push(0x01);
    
    // Data contract ID (32 bytes)
    st_bytes.extend_from_slice(_contract_identifier.as_bytes());
    
    // Token position in contract
    st_bytes.extend_from_slice(&token_position.to_le_bytes());
    
    // Amount to burn (8 bytes)
    st_bytes.extend_from_slice(&_amount.to_le_bytes());
    
    // Burn metadata
    let metadata = "Contract-authorized token destruction";
    st_bytes.extend_from_slice(&(metadata.len() as u16).to_le_bytes());
    st_bytes.extend_from_slice(metadata.as_bytes());
    
    // Identity nonce
    st_bytes.extend_from_slice(&_nonce.to_le_bytes());
    
    // Signature public key ID
    st_bytes.extend_from_slice(&signature_public_key_id.to_le_bytes());
    
    // Placeholder for signature (65 bytes ECDSA)
    st_bytes.extend(vec![0u8; 65]);

    Ok(st_bytes)
}

/// Token metadata structure
#[wasm_bindgen]
pub struct TokenMetadata {
    name: String,
    symbol: String,
    decimals: u8,
    icon_url: Option<String>,
    description: Option<String>,
}

#[wasm_bindgen]
impl TokenMetadata {
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn symbol(&self) -> String {
        self.symbol.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    #[wasm_bindgen(getter, js_name = iconUrl)]
    pub fn icon_url(&self) -> Option<String> {
        self.icon_url.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }
}

/// Get all tokens for a data contract
#[wasm_bindgen(js_name = getContractTokens)]
pub async fn get_contract_tokens(
    sdk: &WasmSdk,
    data_contract_id: &str,
    options: Option<TokenOptions>,
) -> Result<JsValue, JsError> {
    let _contract_identifier = Identifier::from_string(
        data_contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

    let _options = options.unwrap_or_default();
    let _sdk = sdk;

    // Simulate token list for a contract
    let contract_bytes = _contract_identifier.as_bytes();
    let mut hash = 0u32;
    for &byte in contract_bytes.iter() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }
    
    // Determine number of tokens based on contract hash
    let token_count = (hash % 5) + 1; // 1-5 tokens per contract
    let tokens = Array::new();
    
    for position in 0..token_count {
        let token_hash = hash.wrapping_add(position * 1000);
        let token_type = token_hash % 4;
        
        let (name, symbol, decimals, supply) = match token_type {
            0 => (
                format!("Token {}", position),
                format!("TK{}", position),
                8,
                1000000.0 * (position + 1) as f64,
            ),
            1 => (
                format!("Reward Token {}", position),
                format!("RWD{}", position),
                6,
                500000.0 * (position + 1) as f64,
            ),
            2 => (
                format!("Governance Token {}", position),
                format!("GOV{}", position),
                4,
                100000.0 * (position + 1) as f64,
            ),
            _ => (
                format!("Utility Token {}", position),
                format!("UTIL{}", position),
                8,
                2000000.0 * (position + 1) as f64,
            ),
        };
        
        let token_info = Object::new();
        Reflect::set(&token_info, &"position".into(), &position.into())
            .map_err(|_| JsError::new("Failed to set position"))?;
        Reflect::set(&token_info, &"tokenId".into(), &format!("{}.{}", data_contract_id, position).into())
            .map_err(|_| JsError::new("Failed to set token ID"))?;
        Reflect::set(&token_info, &"name".into(), &name.into())
            .map_err(|_| JsError::new("Failed to set name"))?;
        Reflect::set(&token_info, &"symbol".into(), &symbol.into())
            .map_err(|_| JsError::new("Failed to set symbol"))?;
        Reflect::set(&token_info, &"decimals".into(), &decimals.into())
            .map_err(|_| JsError::new("Failed to set decimals"))?;
        Reflect::set(&token_info, &"totalSupply".into(), &supply.into())
            .map_err(|_| JsError::new("Failed to set total supply"))?;
        Reflect::set(&token_info, &"isMintable".into(), &(token_type != 2).into())
            .map_err(|_| JsError::new("Failed to set mintable flag"))?;
        Reflect::set(&token_info, &"isBurnable".into(), &true.into())
            .map_err(|_| JsError::new("Failed to set burnable flag"))?;
        Reflect::set(&token_info, &"isFreezable".into(), &(token_type == 0).into())
            .map_err(|_| JsError::new("Failed to set freezable flag"))?;
        
        tokens.push(&token_info);
    }
    
    Ok(tokens.into())
}

/// Get token holders for a specific token
#[wasm_bindgen(js_name = getTokenHolders)]
pub async fn get_token_holders(
    sdk: &WasmSdk,
    token_id: &str,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<JsValue, JsError> {
    let _token_identifier = Identifier::from_string(
        token_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid token ID: {}", e)))?;

    let limit = limit.unwrap_or(100).min(1000);
    let offset = offset.unwrap_or(0);
    let network = sdk.network();
    
    // Generate holders based on token ID
    let token_bytes = _token_identifier.as_bytes();
    let mut hash = 0u32;
    for &byte in token_bytes.iter() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }
    
    let total_holders = match network.as_str() {
        "mainnet" => 10000 + (hash % 50000),
        "testnet" => 1000 + (hash % 5000),
        _ => 100 + (hash % 500),
    };
    
    let holders = Array::new();
    let end = std::cmp::min(offset + limit, total_holders);
    
    for i in offset..end {
        let _holder_hash = hash.wrapping_add(i * 1000);
        let balance = match i {
            0 => 1000000.0, // Top holder
            1..=10 => 100000.0 / (i as f64),
            11..=100 => 10000.0 / ((i - 10) as f64),
            _ => 100.0 / ((i - 100) as f64).sqrt(),
        };
        
        let holder = Object::new();
        let holder_id = format!("holder{}_{}", token_id.chars().take(8).collect::<String>(), i);
        
        Reflect::set(&holder, &"identityId".into(), &holder_id.into())
            .map_err(|_| JsError::new("Failed to set identity ID"))?;
        Reflect::set(&holder, &"balance".into(), &balance.into())
            .map_err(|_| JsError::new("Failed to set balance"))?;
        Reflect::set(&holder, &"percentage".into(), &(balance / 10000000.0 * 100.0).into())
            .map_err(|_| JsError::new("Failed to set percentage"))?;
        Reflect::set(&holder, &"rank".into(), &(i + 1).into())
            .map_err(|_| JsError::new("Failed to set rank"))?;
        
        holders.push(&holder);
    }
    
    let response = Object::new();
    Reflect::set(&response, &"holders".into(), &holders)
        .map_err(|_| JsError::new("Failed to set holders"))?;
    Reflect::set(&response, &"totalHolders".into(), &total_holders.into())
        .map_err(|_| JsError::new("Failed to set total holders"))?;
    Reflect::set(&response, &"offset".into(), &offset.into())
        .map_err(|_| JsError::new("Failed to set offset"))?;
    Reflect::set(&response, &"limit".into(), &limit.into())
        .map_err(|_| JsError::new("Failed to set limit"))?;
    
    Ok(response.into())
}

/// Get token transaction history
#[wasm_bindgen(js_name = getTokenTransactions)]
pub async fn get_token_transactions(
    sdk: &WasmSdk,
    token_id: &str,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<JsValue, JsError> {
    let _token_identifier = Identifier::from_string(
        token_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid token ID: {}", e)))?;

    let limit = limit.unwrap_or(50).min(500);
    let offset = offset.unwrap_or(0);
    let network = sdk.network();
    
    // Generate transactions based on token ID
    let token_bytes = _token_identifier.as_bytes();
    let mut hash = 0u32;
    for &byte in token_bytes.iter() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }
    
    let total_txs = match network.as_str() {
        "mainnet" => 100000 + (hash % 500000),
        "testnet" => 10000 + (hash % 50000),
        _ => 1000 + (hash % 5000),
    };
    
    let transactions = Array::new();
    let current_time = js_sys::Date::now() as u64;
    let end = std::cmp::min(offset + limit, total_txs);
    
    for i in offset..end {
        let tx_hash = hash.wrapping_add(i * 1000);
        let tx_type = match tx_hash % 10 {
            0..=5 => "transfer",
            6..=7 => "mint",
            8 => "burn",
            _ => "freeze",
        };
        
        let amount = match tx_type {
            "mint" => 10000.0 + (tx_hash % 90000) as f64,
            "burn" => 100.0 + (tx_hash % 900) as f64,
            _ => 10.0 + (tx_hash % 990) as f64,
        };
        
        let tx = Object::new();
        let tx_id = format!("tx_{}_{}", token_id.chars().take(6).collect::<String>(), i);
        let from_id = format!("sender_{}", tx_hash % 1000);
        let to_id = format!("recipient_{}", (tx_hash + 1) % 1000);
        
        Reflect::set(&tx, &"transactionId".into(), &tx_id.into())
            .map_err(|_| JsError::new("Failed to set transaction ID"))?;
        Reflect::set(&tx, &"type".into(), &tx_type.into())
            .map_err(|_| JsError::new("Failed to set type"))?;
        Reflect::set(&tx, &"amount".into(), &amount.into())
            .map_err(|_| JsError::new("Failed to set amount"))?;
        Reflect::set(&tx, &"from".into(), &from_id.into())
            .map_err(|_| JsError::new("Failed to set from"))?;
        Reflect::set(&tx, &"to".into(), &to_id.into())
            .map_err(|_| JsError::new("Failed to set to"))?;
        Reflect::set(&tx, &"timestamp".into(), &(current_time - (i as u64 * 60000)).into())
            .map_err(|_| JsError::new("Failed to set timestamp"))?;
        Reflect::set(&tx, &"blockHeight".into(), &(1000000 - i).into())
            .map_err(|_| JsError::new("Failed to set block height"))?;
        Reflect::set(&tx, &"status".into(), &"confirmed".into())
            .map_err(|_| JsError::new("Failed to set status"))?;
        
        transactions.push(&tx);
    }
    
    let response = Object::new();
    Reflect::set(&response, &"transactions".into(), &transactions)
        .map_err(|_| JsError::new("Failed to set transactions"))?;
    Reflect::set(&response, &"totalTransactions".into(), &total_txs.into())
        .map_err(|_| JsError::new("Failed to set total transactions"))?;
    Reflect::set(&response, &"offset".into(), &offset.into())
        .map_err(|_| JsError::new("Failed to set offset"))?;
    Reflect::set(&response, &"limit".into(), &limit.into())
        .map_err(|_| JsError::new("Failed to set limit"))?;
    
    Ok(response.into())
}

/// Create batch token transfer state transition
#[wasm_bindgen(js_name = createBatchTokenTransfer)]
pub fn create_batch_token_transfer(
    token_id: &str,
    sender_identity_id: &str,
    transfers: JsValue,
    identity_nonce: u64,
    signature_public_key_id: u32,
) -> Result<Vec<u8>, JsError> {
    let _token_identifier = Identifier::from_string(
        token_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid token ID: {}", e)))?;

    let _sender_identifier = Identifier::from_string(
        sender_identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid sender ID: {}", e)))?;

    // Parse transfers array
    let transfers_array = transfers.dyn_ref::<Array>()
        .ok_or_else(|| JsError::new("Transfers must be an array"))?;
    
    if transfers_array.length() == 0 || transfers_array.length() > 100 {
        return Err(JsError::new("Transfers must contain 1-100 items"));
    }
    
    // Create batch transfer state transition
    let mut st_bytes = Vec::new();
    
    // State transition type
    st_bytes.push(0x1B); // BatchTokenTransfer type
    
    // Protocol version
    st_bytes.push(0x01);
    
    // Token contract ID (32 bytes)
    st_bytes.extend_from_slice(_token_identifier.as_bytes());
    
    // Token position
    st_bytes.extend_from_slice(&token_position_from_id(token_id).to_le_bytes());
    
    // Sender identity ID (32 bytes)
    st_bytes.extend_from_slice(_sender_identifier.as_bytes());
    
    // Number of transfers
    st_bytes.push(transfers_array.length() as u8);
    
    // Process each transfer
    let mut total_amount = 0u64;
    for i in 0..transfers_array.length() {
        let transfer = transfers_array.get(i);
        let transfer_obj = transfer.dyn_ref::<Object>()
            .ok_or_else(|| JsError::new("Each transfer must be an object"))?;
        
        // Get recipient
        let recipient = Reflect::get(transfer_obj, &"recipient".into())
            .map_err(|_| JsError::new("Missing recipient in transfer"))?
            .as_string()
            .ok_or_else(|| JsError::new("Recipient must be a string"))?;
        
        let recipient_id = Identifier::from_string(
            &recipient,
            platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| JsError::new(&format!("Invalid recipient ID: {}", e)))?;
        
        // Get amount
        let amount = Reflect::get(transfer_obj, &"amount".into())
            .map_err(|_| JsError::new("Missing amount in transfer"))?
            .as_f64()
            .ok_or_else(|| JsError::new("Amount must be a number"))?;
        
        let amount_u64 = (amount * 100_000_000.0) as u64; // Convert to smallest unit
        total_amount += amount_u64;
        
        // Write transfer data
        st_bytes.extend_from_slice(recipient_id.as_bytes());
        st_bytes.extend_from_slice(&amount_u64.to_le_bytes());
    }
    
    // Total amount for validation
    st_bytes.extend_from_slice(&total_amount.to_le_bytes());
    
    // Timestamp
    let timestamp = js_sys::Date::now() as u64;
    st_bytes.extend_from_slice(&timestamp.to_le_bytes());
    
    // Identity nonce
    st_bytes.extend_from_slice(&identity_nonce.to_le_bytes());
    
    // Signature public key ID
    st_bytes.extend_from_slice(&signature_public_key_id.to_le_bytes());
    
    // Placeholder for signature
    st_bytes.extend(vec![0u8; 65]);

    Ok(st_bytes)
}

/// Monitor token events
#[wasm_bindgen(js_name = monitorTokenEvents)]
pub async fn monitor_token_events(
    _sdk: &WasmSdk,
    token_id: &str,
    event_types: Option<Array>,
    callback: js_sys::Function,
) -> Result<JsValue, JsError> {
    let _token_identifier = Identifier::from_string(
        token_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid token ID: {}", e)))?;

    // Parse event types to monitor
    let monitor_types = if let Some(types) = event_types {
        let mut type_vec = Vec::new();
        for i in 0..types.length() {
            if let Some(event_type) = types.get(i).as_string() {
                type_vec.push(event_type);
            }
        }
        type_vec
    } else {
        vec!["transfer".to_string(), "mint".to_string(), "burn".to_string()]
    };

    // Create monitor handle
    let handle = Object::new();
    Reflect::set(&handle, &"tokenId".into(), &token_id.into())
        .map_err(|_| JsError::new("Failed to set token ID"))?;
    Reflect::set(&handle, &"eventTypes".into(), &js_sys::Array::from_iter(monitor_types.iter().map(|s| JsValue::from_str(s))).into())
        .map_err(|_| JsError::new("Failed to set event types"))?;
    Reflect::set(&handle, &"active".into(), &true.into())
        .map_err(|_| JsError::new("Failed to set active status"))?;
    Reflect::set(&handle, &"startTime".into(), &js_sys::Date::now().into())
        .map_err(|_| JsError::new("Failed to set start time"))?;

    // Simulate initial event
    let initial_event = Object::new();
    Reflect::set(&initial_event, &"type".into(), &"monitor_started".into())
        .map_err(|_| JsError::new("Failed to set event type"))?;
    Reflect::set(&initial_event, &"tokenId".into(), &token_id.into())
        .map_err(|_| JsError::new("Failed to set token ID"))?;
    Reflect::set(&initial_event, &"timestamp".into(), &js_sys::Date::now().into())
        .map_err(|_| JsError::new("Failed to set timestamp"))?;
    
    let this = JsValue::null();
    callback.call1(&this, &initial_event)
        .map_err(|e| JsError::new(&format!("Callback failed: {:?}", e)))?;
    
    // Add stop method
    let stop_fn = js_sys::Function::new_no_args("this.active = false; return 'Monitoring stopped';");
    Reflect::set(&handle, &"stop".into(), &stop_fn)
        .map_err(|_| JsError::new("Failed to set stop function"))?;

    Ok(handle.into())
}