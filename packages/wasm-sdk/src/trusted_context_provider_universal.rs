//! Universal trusted context provider that works in both browser and Node.js
//! 
//! This module provides a context provider that fetches quorum information
//! from trusted HTTP endpoints and works in both browser and Node.js environments.

use crate::context_provider::{ContextProvider, ContextProviderError};
use dpp::dashcore::Network;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use js_sys::{global, Reflect};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// Response from the quorums endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumsResponse {
    pub success: bool,
    pub data: Vec<QuorumData>,
}

/// Data about a specific quorum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumData {
    pub quorum_hash: String,
    pub key: String,
    pub height: u64,
    pub members: Vec<String>,
    pub threshold_signature: String,
    pub mining_members_count: u32,
    pub valid_members_count: u32,
}

/// Response from the previous quorums endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousQuorumsResponse {
    pub success: bool,
    pub data: PreviousQuorumsData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousQuorumsData {
    pub height: u64,
    pub quorums: Vec<QuorumData>,
}

/// Get the base URL for quorum endpoints based on the network
fn get_quorum_base_url(network: Network, devnet_name: Option<&str>) -> String {
    match network {
        Network::Dash => "https://quorums.mainnet.networks.dash.org".to_string(),
        Network::Testnet => "https://quorums.testnet.networks.dash.org".to_string(),
        Network::Devnet => {
            if let Some(name) = devnet_name {
                format!("https://quorums.devnet.{}.networks.dash.org", name)
            } else {
                panic!("Devnet name must be provided for devnet network")
            }
        }
        Network::Regtest => panic!("Regtest network is not supported by trusted context provider"),
        _ => panic!("Unknown network type"),
    }
}

/// Get the LLMQ type for the network
fn get_llmq_type_for_network(network: Network) -> u32 {
    match network {
        Network::Dash => 4,     // Mainnet uses LLMQ type 4
        Network::Testnet => 6,  // Testnet uses LLMQ type 6
        Network::Devnet => 107, // Devnet uses LLMQ type 107
        _ => 6,                 // Default to testnet type
    }
}

/// Detect if we're running in Node.js
fn is_nodejs() -> bool {
    // Check if global.process exists (Node.js specific)
    let process_exists = Reflect::has(&global(), &JsValue::from_str("process")).unwrap_or(false);
    
    // Additional check for process.versions.node
    if process_exists {
        if let Ok(process) = Reflect::get(&global(), &JsValue::from_str("process")) {
            if let Ok(versions) = Reflect::get(&process, &JsValue::from_str("versions")) {
                return Reflect::has(&versions, &JsValue::from_str("node")).unwrap_or(false);
            }
        }
    }
    
    false
}

/// Universal fetch function that works in both browser and Node.js
async fn universal_fetch(url: &str) -> Result<JsValue, JsValue> {
    if is_nodejs() {
        // Node.js environment - use global fetch if available
        let global_obj = global();
        
        // Check if fetch is available in global scope
        if let Ok(fetch_fn) = Reflect::get(&global_obj, &JsValue::from_str("fetch")) {
            if fetch_fn.is_function() {
                // Use global fetch (requires node-fetch or Node 18+)
                let promise = js_sys::Function::from(fetch_fn)
                    .call1(&JsValue::NULL, &JsValue::from_str(url))
                    .map_err(|e| JsValue::from_str(&format!("Fetch failed: {:?}", e)))?;
                
                return JsFuture::from(js_sys::Promise::from(promise)).await;
            }
        }
        
        // If fetch is not available, try to use the built-in https module
        return Err(JsValue::from_str(
            "Node.js fetch not available. Please ensure global.fetch is set (e.g., using node-fetch)"
        ));
    } else {
        // Browser environment - use window.fetch
        let window = web_sys::window()
            .ok_or_else(|| JsValue::from_str("No window object"))?;
        
        let promise = window.fetch_with_str(url);
        JsFuture::from(promise).await
    }
}

/// Parse response as JSON
async fn parse_json(response: JsValue) -> Result<JsValue, JsValue> {
    // Check if it's a Response object with a json() method
    if let Ok(json_fn) = Reflect::get(&response, &JsValue::from_str("json")) {
        if json_fn.is_function() {
            let json_promise = js_sys::Function::from(json_fn)
                .call0(&response)
                .map_err(|e| JsValue::from_str(&format!("Failed to call json(): {:?}", e)))?;
            
            return JsFuture::from(js_sys::Promise::from(json_promise)).await;
        }
    }
    
    // If not, assume it's already parsed
    Ok(response)
}

/// A universal trusted HTTP-based context provider
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct UniversalTrustedHttpContextProvider {
    network: Network,
    devnet_name: Option<String>,
    base_url: String,
    llmq_type: u32,
    
    // Use RefCell for interior mutability in WASM
    current_quorums_cache: Rc<RefCell<HashMap<String, QuorumData>>>,
    previous_quorums_cache: Rc<RefCell<HashMap<String, QuorumData>>>,
}

#[wasm_bindgen]
impl UniversalTrustedHttpContextProvider {
    /// Create a new universal trusted HTTP context provider
    #[wasm_bindgen(constructor)]
    pub fn new(network: &str, devnet_name: Option<String>) -> Result<UniversalTrustedHttpContextProvider, JsValue> {
        let network = match network {
            "mainnet" => Network::Dash,
            "testnet" => Network::Testnet,
            "devnet" => Network::Devnet,
            _ => return Err(JsValue::from_str("Invalid network")),
        };
        
        let base_url = get_quorum_base_url(network, devnet_name.as_deref());
        let llmq_type = get_llmq_type_for_network(network);
        
        Ok(UniversalTrustedHttpContextProvider {
            network,
            devnet_name,
            base_url,
            llmq_type,
            current_quorums_cache: Rc::new(RefCell::new(HashMap::new())),
            previous_quorums_cache: Rc::new(RefCell::new(HashMap::new())),
        })
    }
    
    /// Check if running in Node.js
    #[wasm_bindgen(js_name = isNodeJs)]
    pub fn is_nodejs(&self) -> bool {
        is_nodejs()
    }
    
    /// Fetch quorums from the HTTP endpoint
    async fn fetch_quorums_internal(&self, endpoint: &str) -> Result<QuorumsResponse, String> {
        let url = format!("{}/{}?quorumType={}", self.base_url, endpoint, self.llmq_type);
        
        // Use universal fetch
        let response = universal_fetch(&url)
            .await
            .map_err(|e| format!("Fetch failed: {:?}", e))?;
        
        // Check if response is ok
        if let Ok(ok) = Reflect::get(&response, &JsValue::from_str("ok")) {
            if !ok.as_bool().unwrap_or(true) {
                let status = Reflect::get(&response, &JsValue::from_str("status"))
                    .ok()
                    .and_then(|s| s.as_f64())
                    .unwrap_or(0.0);
                return Err(format!("HTTP error: {}", status));
            }
        }
        
        // Parse JSON
        let json = parse_json(response)
            .await
            .map_err(|e| format!("Failed to parse JSON: {:?}", e))?;
        
        // Convert to QuorumsResponse
        let response: QuorumsResponse = serde_wasm_bindgen::from_value(json)
            .map_err(|e| format!("Failed to deserialize: {:?}", e))?;
        
        Ok(response)
    }
    
    /// Fetch current quorums
    #[wasm_bindgen(js_name = fetchCurrentQuorums)]
    pub async fn fetch_current_quorums(&self) -> Result<JsValue, JsValue> {
        match self.fetch_quorums_internal("quorums").await {
            Ok(response) => {
                // Update cache
                let mut cache = self.current_quorums_cache.borrow_mut();
                for quorum in &response.data {
                    cache.insert(quorum.quorum_hash.clone(), quorum.clone());
                }
                
                serde_wasm_bindgen::to_value(&response)
                    .map_err(|e| JsValue::from_str(&format!("Serialization error: {:?}", e)))
            }
            Err(e) => Err(JsValue::from_str(&e)),
        }
    }
    
    /// Fetch previous quorums
    #[wasm_bindgen(js_name = fetchPreviousQuorums)]
    pub async fn fetch_previous_quorums(&self) -> Result<JsValue, JsValue> {
        let url = format!("{}/previous?quorumType={}", self.base_url, self.llmq_type);
        
        // Use universal fetch
        let response = universal_fetch(&url)
            .await
            .map_err(|e| JsValue::from_str(&format!("Fetch failed: {:?}", e)))?;
        
        // Check if response is ok
        if let Ok(ok) = Reflect::get(&response, &JsValue::from_str("ok")) {
            if !ok.as_bool().unwrap_or(true) {
                let status = Reflect::get(&response, &JsValue::from_str("status"))
                    .ok()
                    .and_then(|s| s.as_f64())
                    .unwrap_or(0.0);
                return Err(JsValue::from_str(&format!("HTTP error: {}", status)));
            }
        }
        
        // Parse JSON
        let json = parse_json(response)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to parse JSON: {:?}", e)))?;
        
        let response: PreviousQuorumsResponse = serde_wasm_bindgen::from_value(json)
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize: {:?}", e)))?;
        
        // Update cache
        let mut cache = self.previous_quorums_cache.borrow_mut();
        for quorum in &response.data.quorums {
            cache.insert(quorum.quorum_hash.clone(), quorum.clone());
        }
        
        serde_wasm_bindgen::to_value(&response)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {:?}", e)))
    }
}

// Implement the ContextProvider trait
impl ContextProvider for UniversalTrustedHttpContextProvider {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        let expected_type = get_llmq_type_for_network(self.network);
        if quorum_type != expected_type {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Quorum type {} doesn't match network type {}",
                quorum_type, expected_type
            )));
        }
        
        let quorum_hash_hex = hex::encode(quorum_hash);
        
        // Check caches first
        {
            let cache = self.current_quorums_cache.borrow();
            if let Some(quorum) = cache.get(&quorum_hash_hex) {
                let pubkey_hex = quorum.key.trim_start_matches("0x");
                let pubkey_bytes = hex::decode(pubkey_hex)
                    .map_err(|e| ContextProviderError::Other(format!("Invalid hex in public key: {}", e)))?;
                
                if pubkey_bytes.len() != 48 {
                    return Err(ContextProviderError::Other(
                        format!("Invalid public key length: {} bytes, expected 48", pubkey_bytes.len())
                    ));
                }
                
                let pubkey_array: [u8; 48] = pubkey_bytes.try_into()
                    .map_err(|_| ContextProviderError::Other("Failed to convert public key to array".to_string()))?;
                
                return Ok(pubkey_array);
            }
        }
        
        {
            let cache = self.previous_quorums_cache.borrow();
            if let Some(quorum) = cache.get(&quorum_hash_hex) {
                let pubkey_hex = quorum.key.trim_start_matches("0x");
                let pubkey_bytes = hex::decode(pubkey_hex)
                    .map_err(|e| ContextProviderError::Other(format!("Invalid hex in public key: {}", e)))?;
                
                if pubkey_bytes.len() != 48 {
                    return Err(ContextProviderError::Other(
                        format!("Invalid public key length: {} bytes, expected 48", pubkey_bytes.len())
                    ));
                }
                
                let pubkey_array: [u8; 48] = pubkey_bytes.try_into()
                    .map_err(|_| ContextProviderError::Other("Failed to convert public key to array".to_string()))?;
                
                return Ok(pubkey_array);
            }
        }
        
        Err(ContextProviderError::InvalidQuorum(format!(
            "Quorum not found for type {} and hash {}",
            quorum_type, quorum_hash_hex
        )))
    }
    
    fn get_data_contract(
        &self,
        _id: &Identifier,
        _platform_version: &dpp::version::PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok(None)
    }
    
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        match self.network {
            Network::Dash => Ok(2132092), // Mainnet L1 locked height
            Network::Testnet => Ok(1090319), // Testnet L1 locked height
            Network::Devnet => Ok(1), // Devnet activation height
            _ => Err(ContextProviderError::Other("Unsupported network".to_string())),
        }
    }
    
    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }
}