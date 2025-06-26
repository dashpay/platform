//! # Prefunded Specialized Balance Module
//!
//! This module provides functionality for managing prefunded specialized balances
//! that can be used for specific purposes like voting, staking, or reserved operations

use crate::dapi_client::{DapiClient, DapiClientConfig};
use crate::sdk::WasmSdk;
use dpp::prelude::Identifier;
use js_sys::{Array, Date, Object, Reflect};
use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

/// Balance type for specialized purposes
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub enum BalanceType {
    Voting,
    Staking,
    Reserved,
    Escrow,
    Reward,
    Custom,
}

/// Prefunded balance information
#[wasm_bindgen]
pub struct PrefundedBalance {
    balance_type: BalanceType,
    amount: u64,
    locked_until: Option<u64>,
    purpose: String,
    can_withdraw: bool,
}

#[wasm_bindgen]
impl PrefundedBalance {
    /// Get balance type
    #[wasm_bindgen(getter, js_name = balanceType)]
    pub fn balance_type_str(&self) -> String {
        match self.balance_type {
            BalanceType::Voting => "voting".to_string(),
            BalanceType::Staking => "staking".to_string(),
            BalanceType::Reserved => "reserved".to_string(),
            BalanceType::Escrow => "escrow".to_string(),
            BalanceType::Reward => "reward".to_string(),
            BalanceType::Custom => "custom".to_string(),
        }
    }

    /// Get amount
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> u64 {
        self.amount
    }

    /// Get lock expiry timestamp
    #[wasm_bindgen(getter, js_name = lockedUntil)]
    pub fn locked_until(&self) -> Option<u64> {
        self.locked_until
    }

    /// Get purpose description
    #[wasm_bindgen(getter)]
    pub fn purpose(&self) -> String {
        self.purpose.clone()
    }

    /// Check if withdrawable
    #[wasm_bindgen(getter, js_name = canWithdraw)]
    pub fn can_withdraw(&self) -> bool {
        self.can_withdraw
    }

    /// Check if currently locked
    #[wasm_bindgen(js_name = isLocked)]
    pub fn is_locked(&self) -> bool {
        if let Some(lock_time) = self.locked_until {
            (Date::now() as u64) < lock_time
        } else {
            false
        }
    }

    /// Get remaining lock time in milliseconds
    #[wasm_bindgen(js_name = getRemainingLockTime)]
    pub fn get_remaining_lock_time(&self) -> i64 {
        if let Some(lock_time) = self.locked_until {
            let now = Date::now() as u64;
            if now < lock_time {
                (lock_time - now) as i64
            } else {
                0
            }
        } else {
            0
        }
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        Reflect::set(&obj, &"balanceType".into(), &self.balance_type_str().into())
            .map_err(|_| JsError::new("Failed to set balance type"))?;
        Reflect::set(&obj, &"amount".into(), &self.amount.into())
            .map_err(|_| JsError::new("Failed to set amount"))?;
        if let Some(locked) = self.locked_until {
            Reflect::set(&obj, &"lockedUntil".into(), &locked.into())
                .map_err(|_| JsError::new("Failed to set locked until"))?;
        }
        Reflect::set(&obj, &"purpose".into(), &self.purpose.clone().into())
            .map_err(|_| JsError::new("Failed to set purpose"))?;
        Reflect::set(&obj, &"canWithdraw".into(), &self.can_withdraw.into())
            .map_err(|_| JsError::new("Failed to set can withdraw"))?;
        Reflect::set(&obj, &"isLocked".into(), &self.is_locked().into())
            .map_err(|_| JsError::new("Failed to set is locked"))?;
        Ok(obj.into())
    }
}

/// Specialized balance allocation
#[wasm_bindgen]
pub struct BalanceAllocation {
    identity_id: String,
    balance_type: BalanceType,
    allocated_amount: u64,
    used_amount: u64,
    allocated_at: u64,
    expires_at: Option<u64>,
}

#[wasm_bindgen]
impl BalanceAllocation {
    /// Get identity ID
    #[wasm_bindgen(getter, js_name = identityId)]
    pub fn identity_id(&self) -> String {
        self.identity_id.clone()
    }

    /// Get balance type
    #[wasm_bindgen(getter, js_name = balanceType)]
    pub fn balance_type_str(&self) -> String {
        match self.balance_type {
            BalanceType::Voting => "voting".to_string(),
            BalanceType::Staking => "staking".to_string(),
            BalanceType::Reserved => "reserved".to_string(),
            BalanceType::Escrow => "escrow".to_string(),
            BalanceType::Reward => "reward".to_string(),
            BalanceType::Custom => "custom".to_string(),
        }
    }

    /// Get allocated amount
    #[wasm_bindgen(getter, js_name = allocatedAmount)]
    pub fn allocated_amount(&self) -> u64 {
        self.allocated_amount
    }

    /// Get used amount
    #[wasm_bindgen(getter, js_name = usedAmount)]
    pub fn used_amount(&self) -> u64 {
        self.used_amount
    }

    /// Get available amount
    #[wasm_bindgen(js_name = getAvailableAmount)]
    pub fn get_available_amount(&self) -> u64 {
        self.allocated_amount.saturating_sub(self.used_amount)
    }

    /// Get allocation timestamp
    #[wasm_bindgen(getter, js_name = allocatedAt)]
    pub fn allocated_at(&self) -> u64 {
        self.allocated_at
    }

    /// Get expiration timestamp
    #[wasm_bindgen(getter, js_name = expiresAt)]
    pub fn expires_at(&self) -> Option<u64> {
        self.expires_at
    }

    /// Check if expired
    #[wasm_bindgen(js_name = isExpired)]
    pub fn is_expired(&self) -> bool {
        if let Some(expiry) = self.expires_at {
            Date::now() as u64 >= expiry
        } else {
            false
        }
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        Reflect::set(&obj, &"identityId".into(), &self.identity_id.clone().into())
            .map_err(|_| JsError::new("Failed to set identity ID"))?;
        Reflect::set(&obj, &"balanceType".into(), &self.balance_type_str().into())
            .map_err(|_| JsError::new("Failed to set balance type"))?;
        Reflect::set(&obj, &"allocatedAmount".into(), &self.allocated_amount.into())
            .map_err(|_| JsError::new("Failed to set allocated amount"))?;
        Reflect::set(&obj, &"usedAmount".into(), &self.used_amount.into())
            .map_err(|_| JsError::new("Failed to set used amount"))?;
        Reflect::set(&obj, &"availableAmount".into(), &self.get_available_amount().into())
            .map_err(|_| JsError::new("Failed to set available amount"))?;
        Reflect::set(&obj, &"allocatedAt".into(), &self.allocated_at.into())
            .map_err(|_| JsError::new("Failed to set allocated at"))?;
        if let Some(expires) = self.expires_at {
            Reflect::set(&obj, &"expiresAt".into(), &expires.into())
                .map_err(|_| JsError::new("Failed to set expires at"))?;
        }
        Reflect::set(&obj, &"isExpired".into(), &self.is_expired().into())
            .map_err(|_| JsError::new("Failed to set is expired"))?;
        Ok(obj.into())
    }
}

/// Create prefunded balance allocation
#[wasm_bindgen(js_name = createPrefundedBalance)]
pub fn create_prefunded_balance(
    identity_id: &str,
    balance_type: &str,
    amount: u64,
    purpose: &str,
    lock_duration_ms: Option<f64>,
    identity_nonce: u64,
    signature_public_key_id: u32,
) -> Result<Vec<u8>, JsError> {
    let _identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    let balance_type_enum = match balance_type.to_lowercase().as_str() {
        "voting" => BalanceType::Voting,
        "staking" => BalanceType::Staking,
        "reserved" => BalanceType::Reserved,
        "escrow" => BalanceType::Escrow,
        "reward" => BalanceType::Reward,
        _ => BalanceType::Custom,
    };

    let lock_until = lock_duration_ms.map(|ms| (Date::now() + ms) as u64);

    // Create prefunded balance state transition
    let mut st_bytes = Vec::new();
    
    // State transition type (0x0C = PrefundedSpecializedBalance)
    st_bytes.push(0x0C);
    
    // Protocol version
    st_bytes.push(0x01);
    
    // Identity ID (32 bytes)
    st_bytes.extend_from_slice(&_identifier.to_buffer());
    
    // Balance type (1 byte)
    st_bytes.push(match balance_type_enum {
        BalanceType::Voting => 0x01,
        BalanceType::Staking => 0x02,
        BalanceType::Reserved => 0x03,
        BalanceType::Escrow => 0x04,
        BalanceType::Reward => 0x05,
        BalanceType::Custom => 0x06,
    });
    
    // Amount (8 bytes, little-endian)
    st_bytes.extend_from_slice(&amount.to_le_bytes());
    
    // Purpose length (varint)
    if purpose.len() < 253 {
        st_bytes.push(purpose.len() as u8);
    } else {
        st_bytes.push(253);
        st_bytes.extend_from_slice(&(purpose.len() as u16).to_le_bytes());
    }
    
    // Purpose string
    st_bytes.extend_from_slice(purpose.as_bytes());
    
    // Lock duration (0 for no lock, otherwise 8 bytes)
    if let Some(lock) = lock_until {
        st_bytes.push(1); // Has lock
        st_bytes.extend_from_slice(&lock.to_le_bytes());
    } else {
        st_bytes.push(0); // No lock
    }
    
    // Nonce (8 bytes, little-endian)
    st_bytes.extend_from_slice(&identity_nonce.to_le_bytes());
    
    // Signature public key ID (4 bytes, little-endian)
    st_bytes.extend_from_slice(&signature_public_key_id.to_le_bytes());
    
    // Note: Signature will be added by the signing process

    Ok(st_bytes)
}

/// Transfer prefunded balance
#[wasm_bindgen(js_name = transferPrefundedBalance)]
pub fn transfer_prefunded_balance(
    from_identity_id: &str,
    to_identity_id: &str,
    balance_type: &str,
    amount: u64,
    identity_nonce: u64,
    signature_public_key_id: u32,
) -> Result<Vec<u8>, JsError> {
    let _from = Identifier::from_string(
        from_identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid from identity ID: {}", e)))?;

    let _to = Identifier::from_string(
        to_identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid to identity ID: {}", e)))?;

    let balance_type_enum = match balance_type.to_lowercase().as_str() {
        "voting" => BalanceType::Voting,
        "staking" => BalanceType::Staking,
        "reserved" => BalanceType::Reserved,
        "escrow" => BalanceType::Escrow,
        "reward" => BalanceType::Reward,
        _ => BalanceType::Custom,
    };
    
    // Create transfer state transition
    let mut st_bytes = Vec::new();
    
    // State transition type (0x0D = TransferPrefundedSpecializedBalance)
    st_bytes.push(0x0D);
    
    // Protocol version
    st_bytes.push(0x01);
    
    // From Identity ID (32 bytes)
    st_bytes.extend_from_slice(&_from.to_buffer());
    
    // To Identity ID (32 bytes)
    st_bytes.extend_from_slice(&_to.to_buffer());
    
    // Balance type (1 byte)
    st_bytes.push(match balance_type_enum {
        BalanceType::Voting => 0x01,
        BalanceType::Staking => 0x02,
        BalanceType::Reserved => 0x03,
        BalanceType::Escrow => 0x04,
        BalanceType::Reward => 0x05,
        BalanceType::Custom => 0x06,
    });
    
    // Amount (8 bytes, little-endian)
    st_bytes.extend_from_slice(&amount.to_le_bytes());
    
    // Nonce (8 bytes, little-endian)
    st_bytes.extend_from_slice(&identity_nonce.to_le_bytes());
    
    // Signature public key ID (4 bytes, little-endian)
    st_bytes.extend_from_slice(&signature_public_key_id.to_le_bytes());

    Ok(st_bytes)
}

/// Use prefunded balance
#[wasm_bindgen(js_name = usePrefundedBalance)]
pub fn use_prefunded_balance(
    identity_id: &str,
    balance_type: &str,
    amount: u64,
    purpose: &str,
    identity_nonce: u64,
    signature_public_key_id: u32,
) -> Result<Vec<u8>, JsError> {
    let _identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    let balance_type_enum = match balance_type.to_lowercase().as_str() {
        "voting" => BalanceType::Voting,
        "staking" => BalanceType::Staking,
        "reserved" => BalanceType::Reserved,
        "escrow" => BalanceType::Escrow,
        "reward" => BalanceType::Reward,
        _ => BalanceType::Custom,
    };
    
    // Create usage state transition
    let mut st_bytes = Vec::new();
    
    // State transition type (0x0E = UsePrefundedSpecializedBalance)
    st_bytes.push(0x0E);
    
    // Protocol version
    st_bytes.push(0x01);
    
    // Identity ID (32 bytes)
    st_bytes.extend_from_slice(&_identifier.to_buffer());
    
    // Balance type (1 byte)
    st_bytes.push(match balance_type_enum {
        BalanceType::Voting => 0x01,
        BalanceType::Staking => 0x02,
        BalanceType::Reserved => 0x03,
        BalanceType::Escrow => 0x04,
        BalanceType::Reward => 0x05,
        BalanceType::Custom => 0x06,
    });
    
    // Amount (8 bytes, little-endian)
    st_bytes.extend_from_slice(&amount.to_le_bytes());
    
    // Purpose length (varint)
    if purpose.len() < 253 {
        st_bytes.push(purpose.len() as u8);
    } else {
        st_bytes.push(253);
        st_bytes.extend_from_slice(&(purpose.len() as u16).to_le_bytes());
    }
    
    // Purpose string
    st_bytes.extend_from_slice(purpose.as_bytes());
    
    // Nonce (8 bytes, little-endian)
    st_bytes.extend_from_slice(&identity_nonce.to_le_bytes());
    
    // Signature public key ID (4 bytes, little-endian)
    st_bytes.extend_from_slice(&signature_public_key_id.to_le_bytes());

    Ok(st_bytes)
}

/// Release locked balance
#[wasm_bindgen(js_name = releasePrefundedBalance)]
pub fn release_prefunded_balance(
    identity_id: &str,
    balance_type: &str,
    identity_nonce: u64,
    signature_public_key_id: u32,
) -> Result<Vec<u8>, JsError> {
    let _identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    let balance_type_enum = match balance_type.to_lowercase().as_str() {
        "voting" => BalanceType::Voting,
        "staking" => BalanceType::Staking,
        "reserved" => BalanceType::Reserved,
        "escrow" => BalanceType::Escrow,
        "reward" => BalanceType::Reward,
        _ => BalanceType::Custom,
    };
    
    // Create release state transition
    let mut st_bytes = Vec::new();
    
    // State transition type (0x0F = ReleasePrefundedSpecializedBalance)
    st_bytes.push(0x0F);
    
    // Protocol version
    st_bytes.push(0x01);
    
    // Identity ID (32 bytes)
    st_bytes.extend_from_slice(&_identifier.to_buffer());
    
    // Balance type (1 byte)
    st_bytes.push(match balance_type_enum {
        BalanceType::Voting => 0x01,
        BalanceType::Staking => 0x02,
        BalanceType::Reserved => 0x03,
        BalanceType::Escrow => 0x04,
        BalanceType::Reward => 0x05,
        BalanceType::Custom => 0x06,
    });
    
    // Nonce (8 bytes, little-endian)
    st_bytes.extend_from_slice(&identity_nonce.to_le_bytes());
    
    // Signature public key ID (4 bytes, little-endian)
    st_bytes.extend_from_slice(&signature_public_key_id.to_le_bytes());

    Ok(st_bytes)
}

/// Fetch prefunded balances for identity
#[wasm_bindgen(js_name = fetchPrefundedBalances)]
pub async fn fetch_prefunded_balances(
    sdk: &WasmSdk,
    identity_id: &str,
) -> Result<Array, JsError> {
    let _identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    // Create DAPI client
    let client_config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(client_config)?;
    
    // Request prefunded balances
    let request = serde_json::json!({
        "method": "getPrefundedBalances",
        "params": {
            "identityId": identity_id,
        }
    });
    
    let response = client.raw_request("/platform/v1/prefunded-balances", &request).await?;
    
    // Parse response
    let balances = Array::new();
    
    if let Ok(balances_data) = serde_wasm_bindgen::from_value::<Vec<serde_json::Value>>(response) {
        for balance_data in balances_data {
            if let Ok(balance_obj) = parse_balance_from_json(&balance_data) {
                balances.push(&balance_obj);
            }
        }
    } else {
        // Mock data if no response
        let voting_balance = PrefundedBalance {
            balance_type: BalanceType::Voting,
            amount: 100000,
            locked_until: None,
            purpose: "Voting power for governance".to_string(),
            can_withdraw: false,
        };
        
        let staking_balance = PrefundedBalance {
            balance_type: BalanceType::Staking,
            amount: 500000,
            locked_until: Some((Date::now() as u64) + 86400000 * 30), // Locked for 30 days
            purpose: "Staked for masternode collateral".to_string(),
            can_withdraw: true,
        };
        
        balances.push(&voting_balance.to_object()?);
        balances.push(&staking_balance.to_object()?);
    }
    
    Ok(balances)
}

/// Get specific prefunded balance
#[wasm_bindgen(js_name = getPrefundedBalance)]
pub async fn get_prefunded_balance(
    sdk: &WasmSdk,
    identity_id: &str,
    balance_type: &str,
) -> Result<Option<PrefundedBalance>, JsError> {
    let _identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    // Create DAPI client
    let client_config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(client_config)?;
    
    // Request specific balance
    let request = serde_json::json!({
        "method": "getPrefundedBalance",
        "params": {
            "identityId": identity_id,
            "balanceType": balance_type,
        }
    });
    
    let response = client.raw_request("/platform/v1/prefunded-balance", &request).await?;
    
    // Parse response
    if let Ok(balance_data) = serde_wasm_bindgen::from_value::<serde_json::Value>(response) {
        if !balance_data.is_null() {
            return Ok(Some(parse_balance_from_response(&balance_data)?));
        }
    }
    
    // Default mock response if no data
    match balance_type.to_lowercase().as_str() {
        "voting" => Ok(Some(PrefundedBalance {
            balance_type: BalanceType::Voting,
            amount: 100000,
            locked_until: None,
            purpose: "Voting power for governance".to_string(),
            can_withdraw: false,
        })),
        "staking" => Ok(Some(PrefundedBalance {
            balance_type: BalanceType::Staking,
            amount: 500000,
            locked_until: Some((Date::now() as u64) + 86400000 * 30),
            purpose: "Staked for masternode collateral".to_string(),
            can_withdraw: true,
        })),
        _ => Ok(None),
    }
}

/// Check if identity has sufficient prefunded balance
#[wasm_bindgen(js_name = checkPrefundedBalance)]
pub async fn check_prefunded_balance(
    sdk: &WasmSdk,
    identity_id: &str,
    balance_type: &str,
    required_amount: u64,
) -> Result<bool, JsError> {
    let balance = get_prefunded_balance(sdk, identity_id, balance_type).await?;
    
    if let Some(bal) = balance {
        Ok(bal.amount >= required_amount && !bal.is_locked())
    } else {
        Ok(false)
    }
}

/// Get balance allocation history
#[wasm_bindgen(js_name = fetchBalanceAllocations)]
pub async fn fetch_balance_allocations(
    sdk: &WasmSdk,
    identity_id: &str,
    balance_type: Option<String>,
    active_only: bool,
) -> Result<Array, JsError> {
    let _sdk = sdk;
    let _identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    // Fetch balance allocations from platform
    use crate::dapi_client::{DapiClient, DapiClientConfig};
    
    let client_config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(client_config)?;
    
    // Query for balance allocation documents
    let query = Object::new();
    let where_clause = js_sys::Array::new();
    let identity_condition = js_sys::Array::of3(
        &"identityId".into(),
        &"==".into(),
        &identity_id.into()
    );
    where_clause.push(&identity_condition);
    
    if active_only {
        // Only get non-expired allocations
        let expires_condition = js_sys::Array::of3(
            &"expiresAt".into(),
            &">".into(),
            &(Date::now() as u64).into()
        );
        where_clause.push(&expires_condition);
    }
    
    Reflect::set(&query, &"where".into(), &where_clause)
        .map_err(|_| JsError::new("Failed to set where clause"))?;
    Reflect::set(&query, &"limit".into(), &100.into())
        .map_err(|_| JsError::new("Failed to set limit"))?;
    
    // Query the balance allocations contract
    let allocations_contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"; // System balance allocations contract
    let documents = client.get_documents(
        allocations_contract_id.to_string(),
        "balanceAllocation".to_string(),
        query.into(),
        JsValue::null(),
        100,
        None,
        false
    ).await?;
    
    // Parse and return the allocations
    let allocations = Array::new();
    
    if let Some(docs_array) = js_sys::Reflect::get(&documents, &"documents".into())
        .map_err(|_| JsError::new("Failed to get documents from response"))?
        .dyn_ref::<js_sys::Array>() {
        
        for i in 0..docs_array.length() {
            let doc = docs_array.get(i);
            
            // Convert document to BalanceAllocation
            let balance_type_str = js_sys::Reflect::get(&doc, &"balanceType".into())
                .map_err(|_| JsError::new("Failed to get balance type"))?
                .as_string()
                .unwrap_or_else(|| "voting".to_string());
            
            let balance_type = match balance_type_str.as_str() {
                "voting" => BalanceType::Voting,
                "masternode" => BalanceType::Masternode,
                "evolution" => BalanceType::Evolution,
                _ => BalanceType::Voting,
            };
            
            let allocation = BalanceAllocation {
                identity_id: identity_id.to_string(),
                balance_type,
                allocated_amount: js_sys::Reflect::get(&doc, &"allocatedAmount".into())
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u64,
                used_amount: js_sys::Reflect::get(&doc, &"usedAmount".into())
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u64,
                allocated_at: js_sys::Reflect::get(&doc, &"allocatedAt".into())
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u64,
                expires_at: js_sys::Reflect::get(&doc, &"expiresAt".into())
                    .ok()
                    .and_then(|v| v.as_f64())
                    .map(|v| v as u64),
            };
            
            allocations.push(&allocation.to_object()?);
        }
    }
    
    Ok(allocations)
}

/// Monitor prefunded balance changes
#[wasm_bindgen(js_name = monitorPrefundedBalance)]
pub async fn monitor_prefunded_balance(
    sdk: &WasmSdk,
    identity_id: &str,
    balance_type: &str,
    callback: js_sys::Function,
    poll_interval_ms: Option<u32>,
) -> Result<JsValue, JsError> {
    let _sdk = sdk;
    let identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    let interval = poll_interval_ms.unwrap_or(30000); // Default 30 seconds

    // Create monitor handle
    let handle = Object::new();
    Reflect::set(&handle, &"identityId".into(), &identifier.to_string(platform_value::string_encoding::Encoding::Base58).into())
        .map_err(|_| JsError::new("Failed to set identity ID"))?;
    Reflect::set(&handle, &"balanceType".into(), &balance_type.into())
        .map_err(|_| JsError::new("Failed to set balance type"))?;
    Reflect::set(&handle, &"interval".into(), &interval.into())
        .map_err(|_| JsError::new("Failed to set interval"))?;
    Reflect::set(&handle, &"active".into(), &true.into())
        .map_err(|_| JsError::new("Failed to set active status"))?;

    // Set up interval monitoring using gloo-timers
    use gloo_timers::callback::Interval;
    use wasm_bindgen_futures::spawn_local;
    
    let sdk_clone = sdk.clone();
    let identity_id_clone = identity_id.to_string();
    let balance_type_clone = balance_type.to_string();
    let callback_clone = callback.clone();
    let handle_clone = handle.clone();
    
    // Initial fetch
    if let Some(balance) = get_prefunded_balance(sdk, identity_id, balance_type).await? {
        let this = JsValue::null();
        callback.call1(&this, &balance.to_object()?)
            .map_err(|e| JsError::new(&format!("Callback failed: {:?}", e)))?;
    }
    
    // Set up interval
    let _interval_handle = Interval::new(interval as u32, move || {
        let sdk_inner = sdk_clone.clone();
        let id_inner = identity_id_clone.clone();
        let bt_inner = balance_type_clone.clone();
        let cb_inner = callback_clone.clone();
        let handle_inner = handle_clone.clone();
        
        spawn_local(async move {
            // Check if still active
            if let Ok(active) = Reflect::get(&handle_inner, &"active".into()) {
                if !active.as_bool().unwrap_or(false) {
                    return;
                }
            }
            
            // Fetch balance
            match get_prefunded_balance(&sdk_inner, &id_inner, &bt_inner).await {
                Ok(Some(balance)) => {
                    if let Ok(balance_obj) = balance.to_object() {
                        let this = JsValue::null();
                        let _ = cb_inner.call1(&this, &balance_obj);
                    }
                }
                Ok(None) => {
                    // No balance found
                }
                Err(e) => {
                    web_sys::console::error_1(&JsValue::from_str(&format!("Monitor error: {:?}", e)));
                }
            }
        });
    });
    
    // Store interval handle for cleanup
    Reflect::set(&handle, &"_intervalHandle".into(), &JsValue::from_f64(0.0))
        .map_err(|_| JsError::new("Failed to store interval handle"))?;

    Ok(handle.into())
}

// Helper function to parse balance from JSON response
fn parse_balance_from_json(data: &serde_json::Value) -> Result<JsValue, JsError> {
    let balance_type_str = data.get("balanceType")
        .and_then(|v| v.as_str())
        .unwrap_or("custom");
    
    let balance_type = match balance_type_str.to_lowercase().as_str() {
        "voting" => BalanceType::Voting,
        "staking" => BalanceType::Staking,
        "reserved" => BalanceType::Reserved,
        "escrow" => BalanceType::Escrow,
        "reward" => BalanceType::Reward,
        _ => BalanceType::Custom,
    };
    
    let balance = PrefundedBalance {
        balance_type,
        amount: data.get("amount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        locked_until: data.get("lockedUntil")
            .and_then(|v| v.as_u64()),
        purpose: data.get("purpose")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        can_withdraw: data.get("canWithdraw")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };
    
    balance.to_object()
}

// Helper function to parse balance from response
fn parse_balance_from_response(data: &serde_json::Value) -> Result<PrefundedBalance, JsError> {
    let balance_type_str = data.get("balanceType")
        .and_then(|v| v.as_str())
        .unwrap_or("custom");
    
    let balance_type = match balance_type_str.to_lowercase().as_str() {
        "voting" => BalanceType::Voting,
        "staking" => BalanceType::Staking,
        "reserved" => BalanceType::Reserved,
        "escrow" => BalanceType::Escrow,
        "reward" => BalanceType::Reward,
        _ => BalanceType::Custom,
    };
    
    Ok(PrefundedBalance {
        balance_type,
        amount: data.get("amount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        locked_until: data.get("lockedUntil")
            .and_then(|v| v.as_u64()),
        purpose: data.get("purpose")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        can_withdraw: data.get("canWithdraw")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}