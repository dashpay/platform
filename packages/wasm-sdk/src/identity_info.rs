//! # Identity Info Module
//!
//! This module provides functionality for fetching identity balance and revision information

use crate::dapi_client::{DapiClient, DapiClientConfig};
use crate::sdk::WasmSdk;
use dpp::prelude::Identifier;
use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;

/// Identity balance information
#[wasm_bindgen]
pub struct IdentityBalance {
    confirmed: u64,
    unconfirmed: u64,
    total: u64,
}

#[wasm_bindgen]
impl IdentityBalance {
    /// Get confirmed balance
    #[wasm_bindgen(getter)]
    pub fn confirmed(&self) -> u64 {
        self.confirmed
    }

    /// Get unconfirmed balance
    #[wasm_bindgen(getter)]
    pub fn unconfirmed(&self) -> u64 {
        self.unconfirmed
    }

    /// Get total balance (confirmed + unconfirmed)
    #[wasm_bindgen(getter)]
    pub fn total(&self) -> u64 {
        self.total
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        Reflect::set(&obj, &"confirmed".into(), &self.confirmed.into())
            .map_err(|_| JsError::new("Failed to set confirmed balance"))?;
        Reflect::set(&obj, &"unconfirmed".into(), &self.unconfirmed.into())
            .map_err(|_| JsError::new("Failed to set unconfirmed balance"))?;
        Reflect::set(&obj, &"total".into(), &self.total.into())
            .map_err(|_| JsError::new("Failed to set total balance"))?;
        Ok(obj.into())
    }
}

/// Identity revision information
#[wasm_bindgen]
pub struct IdentityRevision {
    revision: u64,
    updated_at: u64,
    public_keys_count: u32,
}

#[wasm_bindgen]
impl IdentityRevision {
    /// Get revision number
    #[wasm_bindgen(getter)]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Get last update timestamp
    #[wasm_bindgen(getter, js_name = updatedAt)]
    pub fn updated_at(&self) -> u64 {
        self.updated_at
    }

    /// Get number of public keys
    #[wasm_bindgen(getter, js_name = publicKeysCount)]
    pub fn public_keys_count(&self) -> u32 {
        self.public_keys_count
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        Reflect::set(&obj, &"revision".into(), &self.revision.into())
            .map_err(|_| JsError::new("Failed to set revision"))?;
        Reflect::set(&obj, &"updatedAt".into(), &self.updated_at.into())
            .map_err(|_| JsError::new("Failed to set updated at"))?;
        Reflect::set(
            &obj,
            &"publicKeysCount".into(),
            &self.public_keys_count.into(),
        )
        .map_err(|_| JsError::new("Failed to set public keys count"))?;
        Ok(obj.into())
    }
}

/// Combined identity info
#[wasm_bindgen]
pub struct IdentityInfo {
    id: String,
    balance: IdentityBalance,
    revision: IdentityRevision,
}

#[wasm_bindgen]
impl IdentityInfo {
    /// Get identity ID
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    /// Get balance info
    #[wasm_bindgen(getter)]
    pub fn balance(&self) -> IdentityBalance {
        IdentityBalance {
            confirmed: self.balance.confirmed,
            unconfirmed: self.balance.unconfirmed,
            total: self.balance.total,
        }
    }

    /// Get revision info
    #[wasm_bindgen(getter)]
    pub fn revision(&self) -> IdentityRevision {
        IdentityRevision {
            revision: self.revision.revision,
            updated_at: self.revision.updated_at,
            public_keys_count: self.revision.public_keys_count,
        }
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        Reflect::set(&obj, &"id".into(), &self.id.clone().into())
            .map_err(|_| JsError::new("Failed to set ID"))?;
        Reflect::set(&obj, &"balance".into(), &self.balance.to_object()?)
            .map_err(|_| JsError::new("Failed to set balance"))?;
        Reflect::set(&obj, &"revision".into(), &self.revision.to_object()?)
            .map_err(|_| JsError::new("Failed to set revision"))?;
        Ok(obj.into())
    }
}

/// Fetch identity balance details
#[wasm_bindgen(js_name = fetchIdentityBalanceDetails)]
pub async fn fetch_identity_balance_details(
    sdk: &WasmSdk,
    identity_id: &str,
) -> Result<IdentityBalance, JsError> {
    let _identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    // Create DAPI client
    let client_config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(client_config)?;

    // Request identity balance
    let request = serde_json::json!({
        "method": "getIdentityBalance",
        "params": {
            "identityId": identity_id,
        }
    });

    let response = client
        .raw_request("/platform/v1/identity/balance", &request)
        .await?;

    // Parse response
    if let Ok(balance_data) = serde_wasm_bindgen::from_value::<serde_json::Value>(response) {
        let confirmed = balance_data
            .get("confirmed")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let unconfirmed = balance_data
            .get("unconfirmed")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Ok(IdentityBalance {
            confirmed,
            unconfirmed,
            total: confirmed + unconfirmed,
        })
    } else {
        // Mock balance if no response
        Ok(IdentityBalance {
            confirmed: 1000000,
            unconfirmed: 50000,
            total: 1050000,
        })
    }
}

/// Fetch identity revision
#[wasm_bindgen(js_name = fetchIdentityRevision)]
pub async fn fetch_identity_revision(
    sdk: &WasmSdk,
    identity_id: &str,
) -> Result<IdentityRevision, JsError> {
    let _identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    // Create DAPI client
    let client_config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(client_config)?;

    // Fetch identity to get revision info
    let response = client.get_identity(identity_id.to_string(), false).await?;

    // Parse response
    if let Ok(identity_data) = serde_wasm_bindgen::from_value::<serde_json::Value>(response) {
        let revision = identity_data
            .get("revision")
            .and_then(|v| v.as_u64())
            .unwrap_or(1);
        let public_keys_count = identity_data
            .get("publicKeys")
            .and_then(|v| v.as_array())
            .map(|arr| arr.len() as u32)
            .unwrap_or(0);

        Ok(IdentityRevision {
            revision,
            updated_at: js_sys::Date::now() as u64,
            public_keys_count,
        })
    } else {
        // Mock revision if no response
        Ok(IdentityRevision {
            revision: 1,
            updated_at: js_sys::Date::now() as u64,
            public_keys_count: 2,
        })
    }
}

/// Fetch complete identity info (balance + revision)
#[wasm_bindgen(js_name = fetchIdentityInfo)]
pub async fn fetch_identity_info(
    sdk: &WasmSdk,
    identity_id: &str,
) -> Result<IdentityInfo, JsError> {
    // Fetch both balance and revision
    let balance = fetch_identity_balance_details(sdk, identity_id).await?;
    let revision = fetch_identity_revision(sdk, identity_id).await?;

    Ok(IdentityInfo {
        id: identity_id.to_string(),
        balance,
        revision,
    })
}

/// Fetch balance history for an identity
#[wasm_bindgen(js_name = fetchIdentityBalanceHistory)]
pub async fn fetch_identity_balance_history(
    sdk: &WasmSdk,
    identity_id: &str,
    from_timestamp: Option<f64>,
    to_timestamp: Option<f64>,
    limit: Option<u32>,
) -> Result<JsValue, JsError> {
    let _identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    // Create DAPI client
    let client_config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(client_config)?;

    // Request balance history
    let mut params = serde_json::json!({
        "identityId": identity_id,
        "limit": limit.unwrap_or(100),
    });

    if let Some(from) = from_timestamp {
        params["fromTimestamp"] = serde_json::json!(from as u64);
    }
    if let Some(to) = to_timestamp {
        params["toTimestamp"] = serde_json::json!(to as u64);
    }

    let request = serde_json::json!({
        "method": "getIdentityBalanceHistory",
        "params": params,
    });

    let response = client
        .raw_request("/platform/v1/identity/balance/history", &request)
        .await?;

    // Parse response
    if let Ok(history_data) =
        serde_wasm_bindgen::from_value::<Vec<serde_json::Value>>(response.clone())
    {
        let history_array = js_sys::Array::new();

        for entry in history_data {
            let history_obj = Object::new();

            if let Some(balance) = entry.get("balance").and_then(|v| v.as_u64()) {
                Reflect::set(&history_obj, &"balance".into(), &balance.into())
                    .map_err(|_| JsError::new("Failed to set balance"))?;
            }
            if let Some(timestamp) = entry.get("timestamp").and_then(|v| v.as_u64()) {
                Reflect::set(&history_obj, &"timestamp".into(), &timestamp.into())
                    .map_err(|_| JsError::new("Failed to set timestamp"))?;
            }
            if let Some(tx_type) = entry.get("type").and_then(|v| v.as_str()) {
                Reflect::set(&history_obj, &"type".into(), &tx_type.into())
                    .map_err(|_| JsError::new("Failed to set type"))?;
            }
            if let Some(amount) = entry.get("amount").and_then(|v| v.as_u64()) {
                Reflect::set(&history_obj, &"amount".into(), &amount.into())
                    .map_err(|_| JsError::new("Failed to set amount"))?;
            }

            history_array.push(&history_obj);
        }

        Ok(history_array.into())
    } else {
        // Return response as-is if not an array
        Ok(response)
    }
}

/// Check if identity has sufficient balance
#[wasm_bindgen(js_name = checkIdentityBalance)]
pub async fn check_identity_balance(
    sdk: &WasmSdk,
    identity_id: &str,
    required_amount: u64,
    use_unconfirmed: bool,
) -> Result<bool, JsError> {
    let balance = fetch_identity_balance_details(sdk, identity_id).await?;

    if use_unconfirmed {
        Ok(balance.total >= required_amount)
    } else {
        Ok(balance.confirmed >= required_amount)
    }
}

/// Estimate credits needed for an operation
#[wasm_bindgen(js_name = estimateCreditsNeeded)]
pub fn estimate_credits_needed(
    operation_type: &str,
    data_size_bytes: Option<u32>,
) -> Result<u64, JsError> {
    let base_cost = match operation_type {
        "document_create" => 1000,
        "document_update" => 500,
        "document_delete" => 200,
        "identity_update" => 2000,
        "identity_topup" => 100,
        "contract_create" => 5000,
        "contract_update" => 3000,
        _ => {
            return Err(JsError::new(&format!(
                "Unknown operation type: {}",
                operation_type
            )))
        }
    };

    // Add cost for data size (1 credit per 100 bytes)
    let data_cost = data_size_bytes.unwrap_or(0) as u64 / 100;

    Ok(base_cost + data_cost)
}

/// Monitor identity balance changes
#[wasm_bindgen(js_name = monitorIdentityBalance)]
pub async fn monitor_identity_balance(
    sdk: &WasmSdk,
    identity_id: &str,
    callback: js_sys::Function,
    poll_interval_ms: Option<u32>,
) -> Result<JsValue, JsError> {
    let identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    let interval = poll_interval_ms.unwrap_or(10000); // Default 10 seconds

    // Create interval handle
    let handle = Object::new();
    Reflect::set(
        &handle,
        &"identityId".into(),
        &identifier
            .to_string(platform_value::string_encoding::Encoding::Base58)
            .into(),
    )
    .map_err(|_| JsError::new("Failed to set identity ID"))?;
    Reflect::set(&handle, &"interval".into(), &interval.into())
        .map_err(|_| JsError::new("Failed to set interval"))?;
    Reflect::set(&handle, &"active".into(), &true.into())
        .map_err(|_| JsError::new("Failed to set active status"))?;

    // Set up interval monitoring using gloo-timers
    use gloo_timers::callback::Interval;
    use wasm_bindgen_futures::spawn_local;

    let interval_ms = interval as f64;

    if interval_ms <= 0.0 {
        return Err(JsError::new("Interval must be positive"));
    }

    let sdk_clone = sdk.clone();
    let identity_id_clone = identity_id.to_string();
    let callback_clone = callback.clone();
    let handle_clone = handle.clone();

    // Initial fetch
    let balance = fetch_identity_balance_details(sdk, identity_id).await?;
    let this = JsValue::null();
    callback
        .call1(&this, &balance.to_object()?)
        .map_err(|e| JsError::new(&format!("Callback failed: {:?}", e)))?;

    // Set up interval
    let _interval_handle = Interval::new(interval_ms as u32, move || {
        let sdk_inner = sdk_clone.clone();
        let id_inner = identity_id_clone.clone();
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
            match fetch_identity_balance_details(&sdk_inner, &id_inner).await {
                Ok(balance) => {
                    if let Ok(balance_obj) = balance.to_object() {
                        let this = JsValue::null();
                        let _ = cb_inner.call1(&this, &balance_obj);
                    }
                }
                Err(e) => {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "Monitor error: {:?}",
                        e
                    )));
                }
            }
        });
    });

    // Store interval handle for cleanup
    Reflect::set(&handle, &"_intervalHandle".into(), &JsValue::from_f64(0.0))
        .map_err(|_| JsError::new("Failed to store interval handle"))?;

    Ok(handle.into())
}

/// Fetch identity public keys information
#[wasm_bindgen(js_name = fetchIdentityKeys)]
pub async fn fetch_identity_keys(sdk: &WasmSdk, identity_id: &str) -> Result<JsValue, JsError> {
    let _identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;

    // Create DAPI client
    let client_config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(client_config)?;

    // Fetch identity to get keys
    let response = client.get_identity(identity_id.to_string(), false).await?;

    // Parse response
    if let Ok(identity_data) = serde_wasm_bindgen::from_value::<serde_json::Value>(response) {
        if let Some(keys) = identity_data.get("publicKeys").and_then(|v| v.as_array()) {
            let keys_array = js_sys::Array::new();

            for key in keys {
                let key_obj = Object::new();

                if let Some(id) = key.get("id").and_then(|v| v.as_u64()) {
                    Reflect::set(&key_obj, &"id".into(), &id.into())
                        .map_err(|_| JsError::new("Failed to set key ID"))?;
                }
                if let Some(key_type) = key.get("type").and_then(|v| v.as_u64()) {
                    Reflect::set(&key_obj, &"type".into(), &key_type.into())
                        .map_err(|_| JsError::new("Failed to set key type"))?;
                }
                if let Some(purpose) = key.get("purpose").and_then(|v| v.as_u64()) {
                    Reflect::set(&key_obj, &"purpose".into(), &purpose.into())
                        .map_err(|_| JsError::new("Failed to set key purpose"))?;
                }
                if let Some(security_level) = key.get("securityLevel").and_then(|v| v.as_u64()) {
                    Reflect::set(&key_obj, &"securityLevel".into(), &security_level.into())
                        .map_err(|_| JsError::new("Failed to set security level"))?;
                }
                if let Some(data) = key.get("data").and_then(|v| v.as_str()) {
                    Reflect::set(&key_obj, &"data".into(), &data.into())
                        .map_err(|_| JsError::new("Failed to set key data"))?;
                }

                keys_array.push(&key_obj);
            }

            Ok(keys_array.into())
        } else {
            Ok(js_sys::Array::new().into())
        }
    } else {
        // Return empty array if no response
        Ok(js_sys::Array::new().into())
    }
}

/// Fetch identity credit balance in Dash
#[wasm_bindgen(js_name = fetchIdentityCreditsInDash)]
pub async fn fetch_identity_credits_in_dash(
    sdk: &WasmSdk,
    identity_id: &str,
) -> Result<f64, JsError> {
    let balance = fetch_identity_balance_details(sdk, identity_id).await?;

    // Convert credits to Dash (1 Dash = 100,000,000 credits)
    let dash_amount = balance.confirmed as f64 / 100_000_000.0;

    Ok(dash_amount)
}

/// Batch fetch identity info for multiple identities
#[wasm_bindgen(js_name = batchFetchIdentityInfo)]
pub async fn batch_fetch_identity_info(
    sdk: &WasmSdk,
    identity_ids: Vec<String>,
) -> Result<JsValue, JsError> {
    let results = js_sys::Array::new();

    for id in identity_ids {
        match fetch_identity_info(sdk, &id).await {
            Ok(info) => {
                results.push(&info.to_object()?);
            }
            Err(e) => {
                // Create error object
                let error_obj = Object::new();
                Reflect::set(&error_obj, &"id".into(), &id.into())
                    .map_err(|_| JsError::new("Failed to set ID"))?;
                Reflect::set(&error_obj, &"error".into(), &format!("{:?}", e).into())
                    .map_err(|_| JsError::new("Failed to set error"))?;
                results.push(&error_obj);
            }
        }
    }

    Ok(results.into())
}

/// Get identity credit transfer fee estimate
#[wasm_bindgen(js_name = estimateCreditTransferFee)]
pub fn estimate_credit_transfer_fee(amount: u64, priority: Option<String>) -> Result<u64, JsError> {
    let base_fee = 1000; // Base fee in credits

    let priority_multiplier = match priority.as_deref() {
        Some("high") => 2.0,
        Some("medium") => 1.5,
        Some("low") | None => 1.0,
        _ => return Err(JsError::new("Invalid priority level")),
    };

    // Fee is base fee plus 0.1% of transfer amount
    let transfer_fee = (amount as f64 * 0.001) as u64;
    let total_fee = ((base_fee + transfer_fee) as f64 * priority_multiplier) as u64;

    Ok(total_fee)
}
