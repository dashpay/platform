//! # DAPI Client Module
//!
//! This module provides a WASM-compatible DAPI client implementation that works
//! without platform_proto or gRPC dependencies.

pub mod endpoints;
pub mod error;
pub mod requests;
pub mod responses;
pub mod transport;
pub mod universal_transport;
pub mod universal_client;
pub mod types;

use crate::error::to_js_error;
use js_sys::{Array, Object, Reflect};
use std::time::Duration;
use wasm_bindgen::prelude::*;

pub use error::DapiClientError;
pub use transport::{Transport, TransportConfig};
pub use types::*;

/// DAPI Client configuration
#[wasm_bindgen]
#[derive(Clone)]
pub struct DapiClientConfig {
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
impl DapiClientConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(network: String) -> DapiClientConfig {
        let endpoints = match network.as_str() {
            "mainnet" => vec![
                "https://dapi.dash.org:443".to_string(),
                "https://dapi-1.dash.org:443".to_string(),
                "https://dapi-2.dash.org:443".to_string(),
            ],
            "testnet" => vec![
                "https://52.13.132.146:1443".to_string(),
                "https://52.89.154.48:1443".to_string(),
                "https://44.227.137.77:1443".to_string(),
                "https://52.40.219.41:1443".to_string(),
                "https://54.149.33.167:1443".to_string(),
                "https://54.187.14.232:1443".to_string(),
                "https://52.12.176.90:1443".to_string(),
                "https://52.34.144.50:1443".to_string(),
                "https://44.239.39.153:1443".to_string(),
            ],
            _ => vec!["http://localhost:3000".to_string()],
        };

        DapiClientConfig {
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

/// DAPI Client for making requests to Dash Platform
#[wasm_bindgen]
pub struct DapiClient {
    config: DapiClientConfig,
    transport: Transport,
}

#[wasm_bindgen]
impl DapiClient {
    /// Create a new DAPI client
    #[wasm_bindgen(constructor)]
    pub fn new(config: DapiClientConfig) -> Result<DapiClient, JsError> {
        let transport_config = TransportConfig {
            endpoints: config.endpoints.clone(),
            timeout: Duration::from_millis(config.timeout_ms as u64),
            retries: config.retries,
        };

        let transport = Transport::new(transport_config);

        Ok(DapiClient { config, transport })
    }

    /// Get the network type
    #[wasm_bindgen(getter)]
    pub fn network(&self) -> String {
        self.config.network.clone()
    }

    /// Get current endpoint
    #[wasm_bindgen(js_name = getCurrentEndpoint)]
    pub fn get_current_endpoint(&self) -> String {
        self.transport.get_current_endpoint()
    }

    /// Broadcast a state transition
    #[wasm_bindgen(js_name = broadcastStateTransition)]
    pub async fn broadcast_state_transition(
        &self,
        state_transition_bytes: Vec<u8>,
        wait: bool,
    ) -> Result<JsValue, JsError> {
        use requests::BroadcastRequest;

        let request = BroadcastRequest {
            state_transition: state_transition_bytes,
            wait,
        };

        let response = self
            .transport
            .request("/v0/broadcastStateTransition", &request)
            .await
            .map_err(to_js_error)?;

        Ok(response)
    }

    /// Get identity by ID
    #[wasm_bindgen(js_name = getIdentity)]
    pub async fn get_identity(&self, identity_id: String, prove: bool) -> Result<JsValue, JsError> {
        use requests::GetIdentityRequest;

        let request = GetIdentityRequest { identity_id, prove };

        let response = self
            .transport
            .request("/v0/getIdentity", &request)
            .await
            .map_err(to_js_error)?;

        Ok(response)
    }

    /// Get data contract by ID
    #[wasm_bindgen(js_name = getDataContract)]
    pub async fn get_data_contract(
        &self,
        contract_id: String,
        prove: bool,
    ) -> Result<JsValue, JsError> {
        use requests::GetDataContractRequest;

        let request = GetDataContractRequest { contract_id, prove };

        let response = self
            .transport
            .request("/v0/getDataContract", &request)
            .await
            .map_err(to_js_error)?;

        Ok(response)
    }

    /// Get identity balance
    #[wasm_bindgen(js_name = getIdentityBalance)]
    pub async fn get_identity_balance(
        &self,
        identity_id: String,
        prove: bool,
    ) -> Result<JsValue, JsError> {
        use requests::GetIdentityBalanceRequest;

        let request = GetIdentityBalanceRequest { identity_id, prove };

        let response = self
            .transport
            .request("/v0/getIdentityBalance", &request)
            .await
            .map_err(to_js_error)?;

        Ok(response)
    }

    /// Get documents
    #[wasm_bindgen(js_name = getDocuments)]
    pub async fn get_documents(
        &self,
        contract_id: String,
        document_type: String,
        where_clause: JsValue,
        order_by: JsValue,
        limit: u32,
        start_after: Option<String>,
        prove: bool,
    ) -> Result<JsValue, JsError> {
        use requests::GetDocumentsRequest;

        let where_obj = if where_clause.is_object() {
            serde_wasm_bindgen::from_value(where_clause)
                .map_err(|e| JsError::new(&format!("Invalid where clause: {}", e)))?
        } else {
            serde_json::Value::Null
        };

        let order_by_obj = if order_by.is_object() {
            serde_wasm_bindgen::from_value(order_by)
                .map_err(|e| JsError::new(&format!("Invalid order by: {}", e)))?
        } else {
            serde_json::Value::Null
        };

        let request = GetDocumentsRequest {
            contract_id,
            document_type,
            where_clause: where_obj,
            order_by: order_by_obj,
            limit,
            start_after,
            prove,
        };

        let response = self
            .transport
            .request("/v0/getDocuments", &request)
            .await
            .map_err(to_js_error)?;

        Ok(response)
    }

    /// Get epoch info
    #[wasm_bindgen(js_name = getEpochInfo)]
    pub async fn get_epoch_info(
        &self,
        epoch: Option<u32>,
        prove: bool,
    ) -> Result<JsValue, JsError> {
        use requests::GetEpochInfoRequest;

        let request = GetEpochInfoRequest { epoch, prove };

        let response = self
            .transport
            .request("/v0/getEpochInfo", &request)
            .await
            .map_err(to_js_error)?;

        Ok(response)
    }

    /// Subscribe to state transitions
    #[wasm_bindgen(js_name = subscribeToStateTransitions)]
    pub async fn subscribe_to_state_transitions(
        &self,
        _query: JsValue,
        _callback: js_sys::Function,
    ) -> Result<JsValue, JsError> {
        // Create subscription handle
        let handle = Object::new();
        Reflect::set(&handle, &"active".into(), &true.into())
            .map_err(|_| JsError::new("Failed to set active flag"))?;

        // Add unsubscribe method
        let unsubscribe_fn =
            js_sys::Function::new_no_args("this.active = false; return 'Unsubscribed';");
        Reflect::set(&handle, &"unsubscribe".into(), &unsubscribe_fn)
            .map_err(|_| JsError::new("Failed to set unsubscribe method"))?;

        // TODO: Implement actual WebSocket subscription when available
        // For now, return a mock subscription handle
        Ok(handle.into())
    }

    /// Get protocol version
    #[wasm_bindgen(js_name = getProtocolVersion)]
    pub async fn get_protocol_version(&self) -> Result<JsValue, JsError> {
        let response = self
            .transport
            .request("/v0/getProtocolVersion", &serde_json::json!({}))
            .await
            .map_err(to_js_error)?;

        Ok(response)
    }

    /// Wait for state transition result
    #[wasm_bindgen(js_name = waitForStateTransitionResult)]
    pub async fn wait_for_state_transition_result(
        &self,
        state_transition_hash: String,
        timeout_ms: Option<u32>,
    ) -> Result<JsValue, JsError> {
        use requests::WaitForStateTransitionRequest;

        let request = WaitForStateTransitionRequest {
            state_transition_hash,
            timeout_ms: timeout_ms.unwrap_or(60000),
        };

        let response = self
            .transport
            .request("/v0/waitForStateTransitionResult", &request)
            .await
            .map_err(to_js_error)?;

        Ok(response)
    }
}

impl DapiClient {
    /// Make a raw request to DAPI
    pub async fn raw_request(
        &self,
        path: &str,
        payload: &serde_json::Value,
    ) -> Result<JsValue, DapiClientError> {
        self.transport.request(path, payload).await
    }
}
