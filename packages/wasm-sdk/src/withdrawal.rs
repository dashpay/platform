//! # Withdrawal Module
//!
//! This module provides functionality for withdrawing funds from identities on Dash Platform

use crate::sdk::WasmSdk;
use crate::dapi_client::{DapiClient, DapiClientConfig};
use dpp::prelude::Identifier;
use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

/// Options for withdrawal operations
#[wasm_bindgen]
#[derive(Clone, Default)]
pub struct WithdrawalOptions {
    retries: Option<u32>,
    timeout_ms: Option<u32>,
    fee_multiplier: Option<f64>,
}

#[wasm_bindgen]
impl WithdrawalOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WithdrawalOptions {
        WithdrawalOptions::default()
    }

    /// Set the number of retries
    #[wasm_bindgen(js_name = withRetries)]
    pub fn with_retries(mut self, retries: u32) -> WithdrawalOptions {
        self.retries = Some(retries);
        self
    }

    /// Set the timeout in milliseconds
    #[wasm_bindgen(js_name = withTimeout)]
    pub fn with_timeout(mut self, timeout_ms: u32) -> WithdrawalOptions {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Set the fee multiplier
    #[wasm_bindgen(js_name = withFeeMultiplier)]
    pub fn with_fee_multiplier(mut self, multiplier: f64) -> WithdrawalOptions {
        self.fee_multiplier = Some(multiplier);
        self
    }
}

/// Create a withdrawal from an identity
#[wasm_bindgen(js_name = withdrawFromIdentity)]
pub async fn withdraw_from_identity(
    sdk: &WasmSdk,
    identity_id: &str,
    amount: f64,
    to_address: &str,
    signature_public_key_id: u32,
    options: Option<WithdrawalOptions>,
) -> Result<JsValue, JsError> {
    let _identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    let _amount_duffs = (amount * 100_000_000.0) as u64;
    let _options = options.unwrap_or_default();

    // Validate the address format
    validate_dash_address(to_address)?;

    // Create withdrawal state transition
    let output_script = create_output_script_from_address(to_address)?;
    
    // Get current identity nonce from the platform
    let client_config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(client_config)?;
    let identity_info = client.get_identity(identity_id.to_string(), false).await?;
    let nonce = js_sys::Reflect::get(&identity_info, &"revision".into())
        .map_err(|_| JsError::new("Failed to get identity revision"))?
        .as_f64()
        .ok_or_else(|| JsError::new("Invalid revision type"))?;
    
    // Create the withdrawal transition
    let transition_bytes = create_withdrawal_transition(
        identity_id,
        amount,
        to_address,
        output_script,
        nonce + 1.0, // Increment nonce
        signature_public_key_id,
        None, // Use default fee
    )?;
    
    // Broadcast the transition
    let broadcast_result = client.broadcast_state_transition(
        transition_bytes,
        true, // wait for result
    ).await?;
    
    Ok(broadcast_result)
}

/// Create a withdrawal state transition
#[wasm_bindgen(js_name = createWithdrawalTransition)]
pub fn create_withdrawal_transition(
    identity_id: &str,
    amount: f64,
    to_address: &str,
    output_script: Vec<u8>,
    identity_nonce: f64,
    signature_public_key_id: u32,
    core_fee_per_byte: Option<u32>,
) -> Result<Vec<u8>, JsError> {
    let _identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    let _amount_duffs = (amount * 100_000_000.0) as u64;
    let _nonce = identity_nonce as u64;
    let _fee_per_byte = core_fee_per_byte.unwrap_or(1);

    if to_address.is_empty() {
        return Err(JsError::new("Withdrawal address cannot be empty"));
    }

    if output_script.is_empty() {
        return Err(JsError::new("Output script cannot be empty"));
    }

    
    // Create withdrawal state transition
    let mut st_bytes = Vec::new();
    
    // State transition type (0x0B = IdentityWithdrawal)
    st_bytes.push(0x0B);
    
    // Protocol version
    st_bytes.push(0x01);
    
    // Identity ID (32 bytes)
    st_bytes.extend_from_slice(&_identifier.to_buffer());
    
    // Amount (8 bytes, little-endian)
    st_bytes.extend_from_slice(&_amount_duffs.to_le_bytes());
    
    // Core fee per byte (2 bytes, little-endian)
    st_bytes.extend_from_slice(&(_fee_per_byte as u16).to_le_bytes());
    
    // Output script length (varint)
    if output_script.len() < 253 {
        st_bytes.push(output_script.len() as u8);
    } else {
        st_bytes.push(253);
        st_bytes.extend_from_slice(&(output_script.len() as u16).to_le_bytes());
    }
    
    // Output script
    st_bytes.extend_from_slice(&output_script);
    
    // Nonce (8 bytes, little-endian)
    st_bytes.extend_from_slice(&_nonce.to_le_bytes());
    
    // Signature public key ID (4 bytes, little-endian)
    st_bytes.extend_from_slice(&signature_public_key_id.to_le_bytes());
    
    // Note: Signature will be added by the signing process
    
    Ok(st_bytes)
}

/// Get withdrawal status
#[wasm_bindgen(js_name = getWithdrawalStatus)]
pub async fn get_withdrawal_status(
    sdk: &WasmSdk,
    withdrawal_id: &str,
    options: Option<WithdrawalOptions>,
) -> Result<JsValue, JsError> {
    let _withdrawal_identifier = Identifier::from_string(
        withdrawal_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid withdrawal ID: {}", e)))?;

    let _options = options.unwrap_or_default();

    // Query withdrawal document from the platform
    
    let config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(config)?;
    
    // Withdrawals are tracked as documents in a system contract
    let query = Object::new();
    Reflect::set(&query, &"where".into(), &js_sys::Array::new().into())
        .map_err(|_| JsError::new("Failed to create query"))?;
    
    let where_clause = js_sys::Array::new();
    let withdrawal_condition = js_sys::Array::of3(
        &"withdrawalId".into(),
        &"==".into(),
        &withdrawal_id.into()
    );
    where_clause.push(&withdrawal_condition);
    
    Reflect::set(&query, &"where".into(), &where_clause)
        .map_err(|_| JsError::new("Failed to set where clause"))?;
    
    // Query the withdrawal contract
    let withdrawals_contract_id = "HWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"; // System withdrawals contract
    let documents = client.get_documents(
        withdrawals_contract_id.to_string(),
        "withdrawal".to_string(),
        where_clause.into(),  // where clause  
        JsValue::null(),      // order_by
        100,                  // limit
        None,                 // start_after
        false                 // prove
    ).await?;
    
    // Parse the response
    if let Some(docs_array) = documents.dyn_ref::<js_sys::Array>() {
        if docs_array.length() > 0 {
            let withdrawal_doc = docs_array.get(0);
            return Ok(withdrawal_doc);
        }
    }
    
    // If not found, return not found status
    let response = Object::new();
    Reflect::set(&response, &"status".into(), &"not_found".into())
        .map_err(|_| JsError::new("Failed to set status"))?;
    Reflect::set(&response, &"withdrawalId".into(), &withdrawal_id.into())
        .map_err(|_| JsError::new("Failed to set withdrawal ID"))?;

    Ok(response.into())
}

/// Get all withdrawals for an identity
#[wasm_bindgen(js_name = getIdentityWithdrawals)]
pub async fn get_identity_withdrawals(
    sdk: &WasmSdk,
    identity_id: &str,
    limit: Option<u32>,
    offset: Option<u32>,
    options: Option<WithdrawalOptions>,
) -> Result<JsValue, JsError> {
    let _identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    let _limit = limit.unwrap_or(100);
    let _offset = offset.unwrap_or(0);
    let _options = options.unwrap_or_default();

    // Query withdrawals for this identity
    use crate::dapi_client::{DapiClient, DapiClientConfig};
    
    let config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(config)?;
    
    // Build query for withdrawals by identity
    let query = Object::new();
    
    let where_clause = js_sys::Array::new();
    let identity_condition = js_sys::Array::of3(
        &"identityId".into(),
        &"==".into(),
        &identity_id.into()
    );
    where_clause.push(&identity_condition);
    
    Reflect::set(&query, &"where".into(), &where_clause)
        .map_err(|_| JsError::new("Failed to set where clause"))?;
    Reflect::set(&query, &"limit".into(), &_limit.into())
        .map_err(|_| JsError::new("Failed to set limit"))?;
    Reflect::set(&query, &"startAt".into(), &_offset.into())
        .map_err(|_| JsError::new("Failed to set offset"))?;
    
    // Order by creation date descending
    let order_by = js_sys::Array::of2(
        &js_sys::Array::of2(&"createdAt".into(), &"desc".into()),
        &js_sys::Array::of2(&"$id".into(), &"asc".into())
    );
    Reflect::set(&query, &"orderBy".into(), &order_by)
        .map_err(|_| JsError::new("Failed to set orderBy"))?;
    
    // Query the withdrawal contract
    let withdrawals_contract_id = "HWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"; // System withdrawals contract
    let documents = client.get_documents(
        withdrawals_contract_id.to_string(),
        "withdrawal".to_string(),
        where_clause.into(),  // where clause
        order_by.into(),      // order by
        _limit,               // limit
        if _offset > 0 { Some(_offset.to_string()) } else { None }, // start_after
        false                 // prove
    ).await?;
    
    // Build response
    let response = Object::new();
    
    if let Some(docs_array) = documents.dyn_ref::<js_sys::Array>() {
        Reflect::set(&response, &"withdrawals".into(), &documents)
            .map_err(|_| JsError::new("Failed to set withdrawals"))?;
        Reflect::set(&response, &"totalCount".into(), &docs_array.length().into())
            .map_err(|_| JsError::new("Failed to set total count"))?;
    } else {
        Reflect::set(&response, &"withdrawals".into(), &js_sys::Array::new().into())
            .map_err(|_| JsError::new("Failed to set withdrawals"))?;
        Reflect::set(&response, &"totalCount".into(), &0.into())
            .map_err(|_| JsError::new("Failed to set total count"))?;
    }

    Ok(response.into())
}

/// Calculate withdrawal fee
#[wasm_bindgen(js_name = calculateWithdrawalFee)]
pub fn calculate_withdrawal_fee(
    amount: f64,
    output_script_size: u32,
    core_fee_per_byte: Option<u32>,
) -> Result<f64, JsError> {
    let _amount_duffs = (amount * 100_000_000.0) as u64;
    let fee_per_byte = core_fee_per_byte.unwrap_or(1);
    
    // Basic fee calculation based on transaction size
    // Withdrawal transactions have a base size plus the output script
    let base_size = 200; // Approximate base transaction size
    let total_size = base_size + output_script_size;
    let fee_duffs = total_size * fee_per_byte;
    
    Ok(fee_duffs as f64 / 100_000_000.0)
}

/// Broadcast a withdrawal transaction
#[wasm_bindgen(js_name = broadcastWithdrawal)]
pub async fn broadcast_withdrawal(
    sdk: &WasmSdk,
    withdrawal_transition: Vec<u8>,
    options: Option<WithdrawalOptions>,
) -> Result<JsValue, JsError> {
    if withdrawal_transition.is_empty() {
        return Err(JsError::new("Withdrawal transition cannot be empty"));
    }

    let _options = options.unwrap_or_default();

    // Create DAPI client and broadcast
    let config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(config)?;
    
    let broadcast_result = client.broadcast_state_transition(
        withdrawal_transition,
        true, // wait for result
    ).await?;
    
    // Check if broadcast was successful
    let success = js_sys::Reflect::get(&broadcast_result, &"success".into())
        .map_err(|_| JsError::new("Failed to get success status"))?
        .as_bool()
        .unwrap_or(false);
    
    if success {
        // Extract transaction ID from result
        let tx_id = js_sys::Reflect::get(&broadcast_result, &"transactionId".into())
            .unwrap_or(JsValue::null());
        
        let response = Object::new();
        Reflect::set(&response, &"success".into(), &true.into())
            .map_err(|_| JsError::new("Failed to set success"))?;
        Reflect::set(&response, &"transactionId".into(), &tx_id)
            .map_err(|_| JsError::new("Failed to set transaction ID"))?;
        Reflect::set(&response, &"message".into(), &"Withdrawal broadcast successfully".into())
            .map_err(|_| JsError::new("Failed to set message"))?;
        
        Ok(response.into())
    } else {
        // Extract error from result
        let error_msg = js_sys::Reflect::get(&broadcast_result, &"error".into())
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "Broadcast failed".to_string());
        
        let response = Object::new();
        Reflect::set(&response, &"success".into(), &false.into())
            .map_err(|_| JsError::new("Failed to set success"))?;
        Reflect::set(&response, &"transactionId".into(), &JsValue::null())
            .map_err(|_| JsError::new("Failed to set transaction ID"))?;
        Reflect::set(&response, &"error".into(), &error_msg.into())
            .map_err(|_| JsError::new("Failed to set error"))?;
        
        Ok(response.into())
    }
}

/// Estimate time until withdrawal is processed
#[wasm_bindgen(js_name = estimateWithdrawalTime)]
pub async fn estimate_withdrawal_time(
    sdk: &WasmSdk,
    options: Option<WithdrawalOptions>,
) -> Result<JsValue, JsError> {
    let _options = options.unwrap_or_default();

    let _sdk = sdk;

    // Estimate withdrawal time based on network conditions
    // Base time: 60 minutes (1 hour) for standard processing
    // Add 15 minutes for each 1000 withdrawals in queue
    let base_time_minutes = 60;
    let queue_factor = 15; // minutes per 1000 withdrawals
    
    // In production, these would come from actual network data
    let estimated_queue_length = 0; // Mock value
    let network_congestion_factor = 1.0; // 1.0 = normal, 2.0 = double time
    
    let queue_delay = (estimated_queue_length as f64 / 1000.0) * queue_factor as f64;
    let total_minutes = ((base_time_minutes as f64 + queue_delay) * network_congestion_factor) as u32;
    
    let response = Object::new();
    Reflect::set(&response, &"estimatedMinutes".into(), &total_minutes.into())
        .map_err(|_| JsError::new("Failed to set estimated minutes"))?;
    Reflect::set(&response, &"currentQueueLength".into(), &estimated_queue_length.into())
        .map_err(|_| JsError::new("Failed to set queue length"))?;
    Reflect::set(&response, &"networkCongestion".into(), &network_congestion_factor.into())
        .map_err(|_| JsError::new("Failed to set network congestion"))?;

    Ok(response.into())
}

/// Create output script from Dash address
fn create_output_script_from_address(address: &str) -> Result<Vec<u8>, JsError> {
    use dashcore::Address;
    use std::str::FromStr;
    
    // Parse the address
    let addr = Address::from_str(address)
        .map_err(|e| JsError::new(&format!("Invalid address: {}", e)))?;
    
    // Assume the network and get the script pubkey
    let script = addr.assume_checked().script_pubkey();
    
    Ok(script.to_bytes())
}

/// Validate a Dash address format
fn validate_dash_address(address: &str) -> Result<(), JsError> {
    use dashcore::Address;
    use std::str::FromStr;
    
    // Check if address is empty
    if address.is_empty() {
        return Err(JsError::new("Withdrawal address cannot be empty"));
    }
    
    // Use dashcore's Address parsing which includes checksum validation
    Address::from_str(address)
        .map_err(|e| JsError::new(&format!("Invalid address: {}", e)))?;
    
    Ok(())
}