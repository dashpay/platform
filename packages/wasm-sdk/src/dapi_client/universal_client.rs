//! Universal DAPI Client that works in both browser and Node.js

use super::universal_transport::UniversalTransport;
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;

/// Configuration for Universal DAPI Client
#[wasm_bindgen]
#[derive(Clone)]
pub struct UniversalDapiClientConfig {
    /// List of DAPI endpoints
    endpoints: Vec<String>,
    /// Request timeout in milliseconds
    timeout_ms: u32,
    /// Number of retries for failed requests
    retries: u32,
    /// Network type (mainnet, testnet, devnet)
    network: String,
}

#[wasm_bindgen]
impl UniversalDapiClientConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(network: String) -> UniversalDapiClientConfig {
        let endpoints = match network.as_str() {
            "mainnet" => vec![
                "dapi.dash.org:443".to_string(),
                "dapi-1.dash.org:443".to_string(),
                "dapi-2.dash.org:443".to_string(),
            ],
            "testnet" => vec![
                "52.13.132.146:1443".to_string(),
                "52.89.154.48:1443".to_string(),
                "44.227.137.77:1443".to_string(),
                "52.40.219.41:1443".to_string(),
                "54.149.33.167:1443".to_string(),
                "54.187.14.232:1443".to_string(),
                "52.12.176.90:1443".to_string(),
                "52.34.144.50:1443".to_string(),
                "44.239.39.153:1443".to_string(),
            ],
            _ => vec!["localhost:3000".to_string()],
        };

        UniversalDapiClientConfig {
            endpoints,
            timeout_ms: 30000,
            retries: 3,
            network,
        }
    }

    /// Add a custom endpoint
    #[wasm_bindgen(js_name = addEndpoint)]
    pub fn add_endpoint(&mut self, endpoint: String) {
        self.endpoints.push(endpoint);
    }

    /// Set timeout in milliseconds
    #[wasm_bindgen(js_name = setTimeout)]
    pub fn set_timeout(&mut self, timeout_ms: u32) {
        self.timeout_ms = timeout_ms;
    }

    /// Set number of retries
    #[wasm_bindgen(js_name = setRetries)]
    pub fn set_retries(&mut self, retries: u32) {
        self.retries = retries;
    }

    /// Get endpoints as JavaScript array
    #[wasm_bindgen(getter)]
    pub fn endpoints(&self) -> Array {
        let arr = Array::new();
        for endpoint in &self.endpoints {
            arr.push(&endpoint.into());
        }
        arr
    }
}

/// Universal DAPI Client that works in both browser and Node.js
#[wasm_bindgen]
pub struct UniversalDapiClient {
    config: UniversalDapiClientConfig,
    transport: UniversalTransport,
}

#[wasm_bindgen]
impl UniversalDapiClient {
    /// Create a new universal DAPI client
    #[wasm_bindgen(constructor)]
    pub fn new(config: UniversalDapiClientConfig) -> Result<UniversalDapiClient, JsError> {
        // Convert endpoints to JsValue array for UniversalTransport
        let js_endpoints: Vec<JsValue> = config.endpoints
            .iter()
            .map(|e| JsValue::from_str(e))
            .collect();
        
        let mut transport = UniversalTransport::new(js_endpoints)
            .map_err(|e| JsError::new(&format!("Failed to create transport: {:?}", e)))?;
        
        transport.set_timeout(config.timeout_ms);
        transport.set_retries(config.retries);
        
        Ok(UniversalDapiClient { config, transport })
    }

    /// Check if running in Node.js
    #[wasm_bindgen(js_name = isNodeJs)]
    pub fn is_nodejs(&self) -> bool {
        self.transport.is_nodejs()
    }

    /// Get the network type
    #[wasm_bindgen(getter)]
    pub fn network(&self) -> String {
        self.config.network.clone()
    }

    /// Get identity by ID
    #[wasm_bindgen(js_name = getIdentity)]
    pub async fn get_identity(&mut self, identity_id: String, prove: bool) -> Result<JsValue, JsError> {
        let request = Object::new();
        Reflect::set(&request, &JsValue::from_str("id"), &JsValue::from_str(&identity_id))
            .map_err(|e| JsError::new(&format!("Failed to set id: {:?}", e)))?;
        Reflect::set(&request, &JsValue::from_str("prove"), &JsValue::from_bool(prove))
            .map_err(|e| JsError::new(&format!("Failed to set prove: {:?}", e)))?;

        self.transport
            .request("/v0/getIdentity", &request.into())
            .await
    }

    /// Get identity balance
    #[wasm_bindgen(js_name = getIdentityBalance)]
    pub async fn get_identity_balance(
        &mut self,
        identity_id: String,
        prove: bool,
    ) -> Result<JsValue, JsError> {
        let request = Object::new();
        Reflect::set(&request, &JsValue::from_str("id"), &JsValue::from_str(&identity_id))
            .map_err(|e| JsError::new(&format!("Failed to set id: {:?}", e)))?;
        Reflect::set(&request, &JsValue::from_str("prove"), &JsValue::from_bool(prove))
            .map_err(|e| JsError::new(&format!("Failed to set prove: {:?}", e)))?;

        self.transport
            .request("/v0/getIdentityBalance", &request.into())
            .await
    }

    /// Get data contract by ID
    #[wasm_bindgen(js_name = getDataContract)]
    pub async fn get_data_contract(
        &mut self,
        contract_id: String,
        prove: bool,
    ) -> Result<JsValue, JsError> {
        let request = Object::new();
        Reflect::set(&request, &JsValue::from_str("id"), &JsValue::from_str(&contract_id))
            .map_err(|e| JsError::new(&format!("Failed to set id: {:?}", e)))?;
        Reflect::set(&request, &JsValue::from_str("prove"), &JsValue::from_bool(prove))
            .map_err(|e| JsError::new(&format!("Failed to set prove: {:?}", e)))?;

        self.transport
            .request("/v0/getDataContract", &request.into())
            .await
    }

    /// Get documents
    #[wasm_bindgen(js_name = getDocuments)]
    pub async fn get_documents(
        &mut self,
        contract_id: String,
        document_type: String,
        where_clause: JsValue,
        order_by: JsValue,
        limit: u32,
        start_after: Option<String>,
        prove: bool,
    ) -> Result<JsValue, JsError> {
        let request = Object::new();
        
        Reflect::set(&request, &JsValue::from_str("contractId"), &JsValue::from_str(&contract_id))
            .map_err(|e| JsError::new(&format!("Failed to set contractId: {:?}", e)))?;
        Reflect::set(&request, &JsValue::from_str("documentType"), &JsValue::from_str(&document_type))
            .map_err(|e| JsError::new(&format!("Failed to set documentType: {:?}", e)))?;
        
        if !where_clause.is_null() && !where_clause.is_undefined() {
            Reflect::set(&request, &JsValue::from_str("where"), &where_clause)
                .map_err(|e| JsError::new(&format!("Failed to set where: {:?}", e)))?;
        }
        
        if !order_by.is_null() && !order_by.is_undefined() {
            Reflect::set(&request, &JsValue::from_str("orderBy"), &order_by)
                .map_err(|e| JsError::new(&format!("Failed to set orderBy: {:?}", e)))?;
        }
        
        Reflect::set(&request, &JsValue::from_str("limit"), &JsValue::from_f64(limit as f64))
            .map_err(|e| JsError::new(&format!("Failed to set limit: {:?}", e)))?;
        
        if let Some(start) = start_after {
            Reflect::set(&request, &JsValue::from_str("startAfter"), &JsValue::from_str(&start))
                .map_err(|e| JsError::new(&format!("Failed to set startAfter: {:?}", e)))?;
        }
        
        Reflect::set(&request, &JsValue::from_str("prove"), &JsValue::from_bool(prove))
            .map_err(|e| JsError::new(&format!("Failed to set prove: {:?}", e)))?;

        self.transport
            .request("/v0/getDocuments", &request.into())
            .await
    }

    /// Broadcast a state transition
    #[wasm_bindgen(js_name = broadcastStateTransition)]
    pub async fn broadcast_state_transition(
        &mut self,
        state_transition_bytes: Vec<u8>,
        wait: bool,
    ) -> Result<JsValue, JsError> {
        let request = Object::new();
        
        // Convert bytes to base64 for JSON transport
        let base64 = base64::encode(&state_transition_bytes);
        Reflect::set(&request, &JsValue::from_str("stateTransition"), &JsValue::from_str(&base64))
            .map_err(|e| JsError::new(&format!("Failed to set stateTransition: {:?}", e)))?;
        Reflect::set(&request, &JsValue::from_str("wait"), &JsValue::from_bool(wait))
            .map_err(|e| JsError::new(&format!("Failed to set wait: {:?}", e)))?;

        self.transport
            .request("/v0/broadcastStateTransition", &request.into())
            .await
    }

    /// Get epochs
    #[wasm_bindgen(js_name = getEpochs)]
    pub async fn get_epochs(
        &mut self,
        start_epoch: Option<u32>,
        count: u32,
        ascending: bool,
        prove: bool,
    ) -> Result<JsValue, JsError> {
        let request = Object::new();
        
        if let Some(start) = start_epoch {
            Reflect::set(&request, &JsValue::from_str("startEpoch"), &JsValue::from_f64(start as f64))
                .map_err(|e| JsError::new(&format!("Failed to set startEpoch: {:?}", e)))?;
        }
        
        Reflect::set(&request, &JsValue::from_str("count"), &JsValue::from_f64(count as f64))
            .map_err(|e| JsError::new(&format!("Failed to set count: {:?}", e)))?;
        Reflect::set(&request, &JsValue::from_str("ascending"), &JsValue::from_bool(ascending))
            .map_err(|e| JsError::new(&format!("Failed to set ascending: {:?}", e)))?;
        Reflect::set(&request, &JsValue::from_str("prove"), &JsValue::from_bool(prove))
            .map_err(|e| JsError::new(&format!("Failed to set prove: {:?}", e)))?;

        self.transport
            .request("/v0/getEpochs", &request.into())
            .await
    }
}

