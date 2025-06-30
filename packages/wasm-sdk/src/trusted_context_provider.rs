//! Trusted HTTP-based context provider for WASM environments
//! 
//! This module provides a context provider that fetches quorum information
//! from trusted HTTP endpoints instead of requiring Core RPC access.

use crate::context_provider::{ContextProvider, ContextProviderError};
use dpp::dashcore::Network;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

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

/// A trusted HTTP-based context provider for WASM environments
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct TrustedHttpContextProvider {
    network: Network,
    devnet_name: Option<String>,
    base_url: String,
    llmq_type: u32,
    
    // Use RefCell for interior mutability in WASM
    current_quorums_cache: Rc<RefCell<HashMap<String, QuorumData>>>,
    previous_quorums_cache: Rc<RefCell<HashMap<String, QuorumData>>>,
}

#[wasm_bindgen]
impl TrustedHttpContextProvider {
    /// Create a new trusted HTTP context provider
    #[wasm_bindgen(constructor)]
    pub fn new(network: &str, devnet_name: Option<String>) -> Result<TrustedHttpContextProvider, JsValue> {
        let network = match network {
            "mainnet" => Network::Dash,
            "testnet" => Network::Testnet,
            "devnet" => Network::Devnet,
            _ => return Err(JsValue::from_str("Invalid network")),
        };
        
        let base_url = get_quorum_base_url(network, devnet_name.as_deref());
        let llmq_type = get_llmq_type_for_network(network);
        
        Ok(TrustedHttpContextProvider {
            network,
            devnet_name,
            base_url,
            llmq_type,
            current_quorums_cache: Rc::new(RefCell::new(HashMap::new())),
            previous_quorums_cache: Rc::new(RefCell::new(HashMap::new())),
        })
    }
    
    /// Fetch quorums from the HTTP endpoint
    async fn fetch_quorums_internal(&self, endpoint: &str) -> Result<QuorumsResponse, String> {
        let window = web_sys::window().ok_or("No window object")?;
        
        let url = format!("{}/{}?quorumType={}", self.base_url, endpoint, self.llmq_type);
        
        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);
        
        let request = Request::new_with_str_and_init(&url, &opts)
            .map_err(|e| format!("Failed to create request: {:?}", e))?;
        
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| format!("Fetch failed: {:?}", e))?;
        
        let resp: Response = resp_value.dyn_into()
            .map_err(|_| "Response is not a Response object")?;
        
        if !resp.ok() {
            return Err(format!("HTTP error: {}", resp.status()));
        }
        
        let json = JsFuture::from(resp.json().map_err(|e| format!("Failed to get JSON: {:?}", e))?)
            .await
            .map_err(|e| format!("Failed to parse JSON: {:?}", e))?;
        
        let response: QuorumsResponse = from_value(json)
            .map_err(|e| format!("Failed to deserialize: {:?}", e))?;
        
        Ok(response)
    }
    
    /// Fetch current quorums
    pub async fn fetch_current_quorums(&self) -> Result<JsValue, JsValue> {
        match self.fetch_quorums_internal("quorums").await {
            Ok(response) => {
                // Update cache
                let mut cache = self.current_quorums_cache.borrow_mut();
                for quorum in &response.data {
                    cache.insert(quorum.quorum_hash.clone(), quorum.clone());
                }
                
                to_value(&response)
                    .map_err(|e| JsValue::from_str(&format!("Serialization error: {:?}", e)))
            }
            Err(e) => Err(JsValue::from_str(&e)),
        }
    }
    
    /// Fetch previous quorums
    pub async fn fetch_previous_quorums(&self) -> Result<JsValue, JsValue> {
        let window = web_sys::window().ok_or(JsValue::from_str("No window object"))?;
        
        let url = format!("{}/previous?quorumType={}", self.base_url, self.llmq_type);
        
        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);
        
        let request = Request::new_with_str_and_init(&url, &opts)
            .map_err(|e| JsValue::from_str(&format!("Failed to create request: {:?}", e)))?;
        
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| JsValue::from_str(&format!("Fetch failed: {:?}", e)))?;
        
        let resp: Response = resp_value.dyn_into()
            .map_err(|_| JsValue::from_str("Response is not a Response object"))?;
        
        if !resp.ok() {
            return Err(JsValue::from_str(&format!("HTTP error: {}", resp.status())));
        }
        
        let json = JsFuture::from(resp.json().map_err(|e| JsValue::from_str(&format!("Failed to get JSON: {:?}", e)))?)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to parse JSON: {:?}", e)))?;
        
        let response: PreviousQuorumsResponse = from_value(json)
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize: {:?}", e)))?;
        
        // Update cache
        let mut cache = self.previous_quorums_cache.borrow_mut();
        for quorum in &response.data.quorums {
            cache.insert(quorum.quorum_hash.clone(), quorum.clone());
        }
        
        to_value(&response)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {:?}", e)))
    }
}

// Implement the ContextProvider trait for TrustedHttpContextProvider
impl ContextProvider for TrustedHttpContextProvider {
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
        // For now, return None as we don't cache data contracts
        // This could be extended to fetch from Platform if needed
        Ok(None)
    }
    
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        // Return the L1 locked height for each network
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
        // For now, return None as we don't cache token configurations
        // This could be extended to fetch from Platform if needed
        Ok(None)
    }
}