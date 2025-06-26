//! Identity nonce management
//!
//! This module provides functionality for managing identity nonces and identity contract nonces.

use crate::error::to_js_error;
use crate::sdk::WasmSdk;
use dpp::prelude::Identifier;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
use web_sys::js_sys::{Date, Object, Reflect};

/// Options for fetching nonces
#[wasm_bindgen]
pub struct NonceOptions {
    cached: bool,
    prove: bool,
}

#[wasm_bindgen]
impl NonceOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> NonceOptions {
        NonceOptions {
            cached: true,
            prove: true,
        }
    }

    #[wasm_bindgen(js_name = setCached)]
    pub fn set_cached(&mut self, cached: bool) {
        self.cached = cached;
    }

    #[wasm_bindgen(js_name = setProve)]
    pub fn set_prove(&mut self, prove: bool) {
        self.prove = prove;
    }
}

/// Response containing nonce information
#[wasm_bindgen]
pub struct NonceResponse {
    nonce: u64,
    metadata: JsValue,
}

#[wasm_bindgen]
impl NonceResponse {
    #[wasm_bindgen(getter)]
    pub fn nonce(&self) -> u64 {
        self.nonce
    }

    #[wasm_bindgen(getter)]
    pub fn metadata(&self) -> JsValue {
        self.metadata.clone()
    }
}

/// Cache entry for nonce values
#[derive(Clone)]
struct NonceCacheEntry {
    nonce: u64,
    last_fetch_time_ms: f64,
}

/// Global cache for identity nonces
static IDENTITY_NONCE_CACHE: std::sync::OnceLock<Arc<Mutex<HashMap<Identifier, NonceCacheEntry>>>> = 
    std::sync::OnceLock::new();

/// Global cache for identity contract nonces
static CONTRACT_NONCE_CACHE: std::sync::OnceLock<Arc<Mutex<HashMap<(Identifier, Identifier), NonceCacheEntry>>>> = 
    std::sync::OnceLock::new();

/// Default cache staleness time (5 seconds)
const DEFAULT_CACHE_STALE_TIME_MS: f64 = 5000.0;

/// Get the identity nonce cache
fn get_identity_nonce_cache() -> Arc<Mutex<HashMap<Identifier, NonceCacheEntry>>> {
    IDENTITY_NONCE_CACHE.get_or_init(|| Arc::new(Mutex::new(HashMap::new()))).clone()
}

/// Get the contract nonce cache
fn get_contract_nonce_cache() -> Arc<Mutex<HashMap<(Identifier, Identifier), NonceCacheEntry>>> {
    CONTRACT_NONCE_CACHE.get_or_init(|| Arc::new(Mutex::new(HashMap::new()))).clone()
}

/// Check if identity nonce is cached and fresh
#[wasm_bindgen(js_name = checkIdentityNonceCache)]
pub fn check_identity_nonce_cache(
    identity_id: &str,
) -> Result<Option<u64>, JsError> {
    let identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;
    
    let current_time = Date::now();
    let cache = get_identity_nonce_cache();
    let cache_guard = cache.lock().unwrap();
    
    if let Some(entry) = cache_guard.get(&identifier) {
        if current_time - entry.last_fetch_time_ms < DEFAULT_CACHE_STALE_TIME_MS {
            return Ok(Some(entry.nonce));
        }
    }
    
    Ok(None)
}

/// Update identity nonce cache
#[wasm_bindgen(js_name = updateIdentityNonceCache)]
pub fn update_identity_nonce_cache(
    identity_id: &str,
    nonce: u64,
) -> Result<(), JsError> {
    let identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;
    
    let current_time = Date::now();
    let cache = get_identity_nonce_cache();
    let mut cache_guard = cache.lock().unwrap();
    
    cache_guard.insert(identifier, NonceCacheEntry {
        nonce,
        last_fetch_time_ms: current_time,
    });
    
    Ok(())
}

/// Check if identity contract nonce is cached and fresh
#[wasm_bindgen(js_name = checkIdentityContractNonceCache)]
pub fn check_identity_contract_nonce_cache(
    identity_id: &str,
    contract_id: &str,
) -> Result<Option<u64>, JsError> {
    let identity_identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;
    
    let contract_identifier = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;
    
    let current_time = Date::now();
    let cache = get_contract_nonce_cache();
    let cache_guard = cache.lock().unwrap();
    let cache_key = (identity_identifier, contract_identifier);
    
    if let Some(entry) = cache_guard.get(&cache_key) {
        if current_time - entry.last_fetch_time_ms < DEFAULT_CACHE_STALE_TIME_MS {
            return Ok(Some(entry.nonce));
        }
    }
    
    Ok(None)
}

/// Update identity contract nonce cache
#[wasm_bindgen(js_name = updateIdentityContractNonceCache)]
pub fn update_identity_contract_nonce_cache(
    identity_id: &str,
    contract_id: &str,
    nonce: u64,
) -> Result<(), JsError> {
    let identity_identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;
    
    let contract_identifier = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;
    
    let current_time = Date::now();
    let cache = get_contract_nonce_cache();
    let mut cache_guard = cache.lock().unwrap();
    let cache_key = (identity_identifier, contract_identifier);
    
    cache_guard.insert(cache_key, NonceCacheEntry {
        nonce,
        last_fetch_time_ms: current_time,
    });
    
    Ok(())
}

/// Increment identity nonce in cache
#[wasm_bindgen(js_name = incrementIdentityNonceCache)]
pub fn increment_identity_nonce_cache(
    identity_id: &str,
    increment: Option<u32>,
) -> Result<u64, JsError> {
    let identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;
    
    let increment_by = increment.unwrap_or(1) as u64;
    let current_time = Date::now();
    let cache = get_identity_nonce_cache();
    let mut cache_guard = cache.lock().unwrap();
    
    let new_nonce = if let Some(entry) = cache_guard.get_mut(&identifier) {
        entry.nonce = entry.nonce.saturating_add(increment_by);
        entry.last_fetch_time_ms = current_time;
        entry.nonce
    } else {
        // If not in cache, return 0 and let JavaScript fetch it
        return Err(JsError::new("Nonce not in cache, please fetch from network first"));
    };
    
    Ok(new_nonce)
}

/// Increment identity contract nonce in cache
#[wasm_bindgen(js_name = incrementIdentityContractNonceCache)]
pub fn increment_identity_contract_nonce_cache(
    identity_id: &str,
    contract_id: &str,
    increment: Option<u32>,
) -> Result<u64, JsError> {
    let identity_identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;
    
    let contract_identifier = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;
    
    let increment_by = increment.unwrap_or(1) as u64;
    let current_time = Date::now();
    let cache = get_contract_nonce_cache();
    let mut cache_guard = cache.lock().unwrap();
    let cache_key = (identity_identifier, contract_identifier);
    
    let new_nonce = if let Some(entry) = cache_guard.get_mut(&cache_key) {
        entry.nonce = entry.nonce.saturating_add(increment_by);
        entry.last_fetch_time_ms = current_time;
        entry.nonce
    } else {
        // If not in cache, return error and let JavaScript fetch it
        return Err(JsError::new("Nonce not in cache, please fetch from network first"));
    };
    
    Ok(new_nonce)
}

/// Clear identity nonce cache
#[wasm_bindgen(js_name = clearIdentityNonceCache)]
pub fn clear_identity_nonce_cache() {
    let cache = get_identity_nonce_cache();
    let mut cache_guard = cache.lock().unwrap();
    cache_guard.clear();
}

/// Clear identity contract nonce cache
#[wasm_bindgen(js_name = clearIdentityContractNonceCache)]
pub fn clear_identity_contract_nonce_cache() {
    let cache = get_contract_nonce_cache();
    let mut cache_guard = cache.lock().unwrap();
    cache_guard.clear();
}