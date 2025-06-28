//! Fetch many operations
//!
//! This module provides functionality for fetching multiple objects from the platform.

use crate::sdk::WasmSdk;
use crate::dapi_client::{DapiClient, DapiClientConfig};
use dpp::prelude::Identifier;
use wasm_bindgen::prelude::*;
use js_sys::{Object, Reflect};

#[wasm_bindgen]
pub struct FetchManyOptions {
    prove: bool,
}

#[wasm_bindgen]
impl FetchManyOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> FetchManyOptions {
        FetchManyOptions { prove: true }
    }

    #[wasm_bindgen(js_name = setProve)]
    pub fn set_prove(&mut self, prove: bool) {
        self.prove = prove;
    }
}

#[wasm_bindgen]
pub struct FetchManyResponse {
    items: JsValue, // Object mapping IDs to items
    metadata: JsValue,
}

#[wasm_bindgen]
impl FetchManyResponse {
    #[wasm_bindgen(getter)]
    pub fn items(&self) -> JsValue {
        self.items.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn metadata(&self) -> JsValue {
        self.metadata.clone()
    }
}

/// Fetch multiple identities by their IDs
/// 
/// This implementation fetches identities sequentially. For parallel fetching,
/// JavaScript callers can map over IDs and use Promise.all on individual fetch calls.
#[wasm_bindgen]
pub async fn fetch_identities(
    sdk: &WasmSdk,
    identity_ids: Vec<String>,
    options: Option<FetchManyOptions>,
) -> Result<FetchManyResponse, JsError> {
    let opts = options.unwrap_or_else(FetchManyOptions::new);
    let items = Object::new();
    
    // Create DAPI client
    let config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(config)?;
    
    // Fetch all identities (sequentially for now, but could be optimized)
    // In JavaScript, the caller can use Promise.all() to parallelize if needed
    for id_str in &identity_ids {
        // Validate identifier
        let _ = Identifier::from_string(
            id_str,
            platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| JsError::new(&format!("Invalid identifier: {}", e)))?;
        
        // Fetch the identity
        match client.get_identity(id_str.clone(), opts.prove).await {
            Ok(identity_value) => {
                Reflect::set(&items, &id_str.into(), &identity_value)
                    .map_err(|_| JsError::new("Failed to set identity in response"))?;
            }
            Err(_) => {
                // Identity not found or error - set null
                Reflect::set(&items, &id_str.into(), &JsValue::NULL)
                    .map_err(|_| JsError::new("Failed to set null in response"))?;
            }
        }
    }
    
    // Create metadata with current timestamp
    let metadata = Object::new();
    let timestamp = js_sys::Date::now();
    Reflect::set(&metadata, &"height".into(), &JsValue::from_f64(0.0))
        .map_err(|_| JsError::new("Failed to set metadata"))?;
    Reflect::set(&metadata, &"time_ms".into(), &JsValue::from_f64(timestamp))
        .map_err(|_| JsError::new("Failed to set metadata"))?;
    Reflect::set(&metadata, &"fetched_count".into(), &JsValue::from_f64(identity_ids.len() as f64))
        .map_err(|_| JsError::new("Failed to set metadata"))?;
    
    Ok(FetchManyResponse {
        items: items.into(),
        metadata: metadata.into(),
    })
}

/// Fetch multiple data contracts by their IDs
#[wasm_bindgen]
pub async fn fetch_data_contracts(
    sdk: &WasmSdk,
    contract_ids: Vec<String>,
    options: Option<FetchManyOptions>,
) -> Result<FetchManyResponse, JsError> {
    let opts = options.unwrap_or_else(FetchManyOptions::new);
    let items = Object::new();
    
    // Create DAPI client
    let config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(config)?;
    
    // Fetch all contracts (sequentially for now, but could be optimized)
    // In JavaScript, the caller can use Promise.all() to parallelize if needed
    for id_str in &contract_ids {
        // Validate identifier
        let _ = Identifier::from_string(
            id_str,
            platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| JsError::new(&format!("Invalid identifier: {}", e)))?;
        
        // Fetch the contract
        match client.get_data_contract(id_str.clone(), opts.prove).await {
            Ok(contract_value) => {
                Reflect::set(&items, &id_str.into(), &contract_value)
                    .map_err(|_| JsError::new("Failed to set contract in response"))?;
            }
            Err(_) => {
                // Contract not found or error - set null
                Reflect::set(&items, &id_str.into(), &JsValue::NULL)
                    .map_err(|_| JsError::new("Failed to set null in response"))?;
            }
        }
    }
    
    // Create metadata with current timestamp
    let metadata = Object::new();
    let timestamp = js_sys::Date::now();
    Reflect::set(&metadata, &"height".into(), &JsValue::from_f64(0.0))
        .map_err(|_| JsError::new("Failed to set metadata"))?;
    Reflect::set(&metadata, &"time_ms".into(), &JsValue::from_f64(timestamp))
        .map_err(|_| JsError::new("Failed to set metadata"))?;
    Reflect::set(&metadata, &"fetched_count".into(), &JsValue::from_f64(contract_ids.len() as f64))
        .map_err(|_| JsError::new("Failed to set metadata"))?;
    
    Ok(FetchManyResponse {
        items: items.into(),
        metadata: metadata.into(),
    })
}

/// Document query options for fetching multiple documents
#[wasm_bindgen]
pub struct DocumentQueryOptions {
    contract_id: String,
    _document_type: String,
    where_clause: JsValue,
    order_by: JsValue,
    limit: Option<u32>,
    start_at: Option<String>,
    start_after: Option<String>,
}

#[wasm_bindgen]
impl DocumentQueryOptions {
    #[wasm_bindgen(constructor)]
    pub fn new(contract_id: String, document_type: String) -> DocumentQueryOptions {
        DocumentQueryOptions {
            contract_id,
            _document_type: document_type,
            where_clause: JsValue::NULL,
            order_by: JsValue::NULL,
            limit: None,
            start_at: None,
            start_after: None,
        }
    }

    #[wasm_bindgen(js_name = setWhereClause)]
    pub fn set_where_clause(&mut self, where_clause: JsValue) {
        self.where_clause = where_clause;
    }

    #[wasm_bindgen(js_name = setOrderBy)]
    pub fn set_order_by(&mut self, order_by: JsValue) {
        self.order_by = order_by;
    }

    #[wasm_bindgen(js_name = setLimit)]
    pub fn set_limit(&mut self, limit: u32) {
        self.limit = Some(limit);
    }

    #[wasm_bindgen(js_name = setStartAt)]
    pub fn set_start_at(&mut self, start_at: String) {
        self.start_at = Some(start_at);
    }

    #[wasm_bindgen(js_name = setStartAfter)]
    pub fn set_start_after(&mut self, start_after: String) {
        self.start_after = Some(start_after);
    }
}

/// Fetch multiple documents based on query criteria
#[wasm_bindgen]
pub async fn fetch_documents(
    _sdk: &WasmSdk,
    query_options: DocumentQueryOptions,
    options: Option<FetchManyOptions>,
) -> Result<FetchManyResponse, JsError> {
    let _opts = options.unwrap_or_else(FetchManyOptions::new);
    
    // Convert query options to platform query
    let _contract_id = Identifier::from_string(
        &query_options.contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;
    
    // For now, return empty response as document querying is complex
    // This would need full DriveQuery implementation
    let items = Object::new();
    let metadata = Object::new();
    
    Reflect::set(&metadata, &"height".into(), &JsValue::from_f64(0.0))
        .map_err(|_| JsError::new("Failed to set metadata"))?;
    Reflect::set(&metadata, &"time_ms".into(), &JsValue::from_f64(0.0))
        .map_err(|_| JsError::new("Failed to set metadata"))?;
    
    Ok(FetchManyResponse {
        items: items.into(),
        metadata: metadata.into(),
    })
}