//! # Fetch Unproved Module
//!
//! This module provides functionality to fetch data from Platform without proof verification.
//! This is useful for faster queries when proof verification is not required.

use crate::dapi_client::{DapiClient, DapiClientConfig};
use crate::fetch::FetchOptions;
use crate::sdk::WasmSdk;
use platform_value::Identifier;
use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

/// Fetch an identity without proof verification
#[wasm_bindgen(js_name = fetchIdentityUnproved)]
pub async fn fetch_identity_unproved(
    sdk: &WasmSdk,
    identity_id: &str,
    options: Option<FetchOptions>,
) -> Result<JsValue, JsError> {
    let _identifier = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identifier: {}", e)))?;

    let options = options.unwrap_or_default();
    
    // Create DAPI client
    let client_config = DapiClientConfig::new(sdk.network());
    if let Some(timeout) = options.timeout {
        client_config.clone().set_timeout(timeout);
    }
    if let Some(retries) = options.retries {
        client_config.clone().set_retries(retries);
    }
    
    let client = DapiClient::new(client_config)?;
    
    // Fetch identity without proof
    let response = client.get_identity(identity_id.to_string(), false).await?;
    
    Ok(response)
}

/// Fetch a data contract without proof verification
#[wasm_bindgen(js_name = fetchDataContractUnproved)]
pub async fn fetch_data_contract_unproved(
    sdk: &WasmSdk,
    contract_id: &str,
    options: Option<FetchOptions>,
) -> Result<JsValue, JsError> {
    let _identifier = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identifier: {}", e)))?;

    let options = options.unwrap_or_default();
    
    // Create DAPI client
    let client_config = DapiClientConfig::new(sdk.network());
    if let Some(timeout) = options.timeout {
        client_config.clone().set_timeout(timeout);
    }
    if let Some(retries) = options.retries {
        client_config.clone().set_retries(retries);
    }
    
    let client = DapiClient::new(client_config)?;
    
    // Fetch data contract without proof
    let response = client.get_data_contract(contract_id.to_string(), false).await?;
    
    Ok(response)
}

/// Fetch documents without proof verification
#[wasm_bindgen(js_name = fetchDocumentsUnproved)]
pub async fn fetch_documents_unproved(
    sdk: &WasmSdk,
    contract_id: &str,
    document_type: &str,
    where_clause: JsValue,
    order_by: JsValue,
    limit: Option<u32>,
    start_at: Option<Vec<u8>>,
    options: Option<FetchOptions>,
) -> Result<JsValue, JsError> {
    let _contract_identifier = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract identifier: {}", e)))?;

    let options = options.unwrap_or_default();
    
    // Create DAPI client
    let client_config = DapiClientConfig::new(sdk.network());
    if let Some(timeout) = options.timeout {
        client_config.clone().set_timeout(timeout);
    }
    if let Some(retries) = options.retries {
        client_config.clone().set_retries(retries);
    }
    
    let client = DapiClient::new(client_config)?;
    
    // Convert start_at to base64 string if present
    let start_after = start_at.map(|bytes| {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(bytes)
    });
    
    // Fetch documents without proof
    let response = client.get_documents(
        contract_id.to_string(),
        document_type.to_string(),
        where_clause,
        order_by,
        limit.unwrap_or(100),
        start_after,
        false,
    ).await?;
    
    Ok(response)
}

/// Fetch identity by public key hash without proof
#[wasm_bindgen(js_name = fetchIdentityByKeyUnproved)]
pub async fn fetch_identity_by_key_unproved(
    sdk: &WasmSdk,
    public_key_hash: Vec<u8>,
    options: Option<FetchOptions>,
) -> Result<JsValue, JsError> {
    if public_key_hash.len() != 20 {
        return Err(JsError::new("Public key hash must be 20 bytes"));
    }

    let _options = options.unwrap_or_default();

    // Create DAPI client
    let client_config = DapiClientConfig::new(sdk.network());
    if let Some(timeout) = _options.timeout {
        client_config.clone().set_timeout(timeout);
    }
    if let Some(retries) = _options.retries {
        client_config.clone().set_retries(retries);
    }
    
    let client = DapiClient::new(client_config)?;
    
    // Convert public key hash to hex string for query
    let hash_hex = hex::encode(&public_key_hash);
    
    // Query identities by public key hash
    // This requires querying the identity index by public key hash
    let query = Object::new();
    let where_clause = js_sys::Array::new();
    let condition = js_sys::Array::of3(
        &"publicKeyHashes".into(),
        &"contains".into(),
        &hash_hex.into()
    );
    where_clause.push(&condition);
    
    Reflect::set(&query, &"where".into(), &where_clause)
        .map_err(|_| JsError::new("Failed to set where clause"))?;
    Reflect::set(&query, &"limit".into(), &100.into())
        .map_err(|_| JsError::new("Failed to set limit"))?;
    
    // Query the identities contract for identities with this public key hash
    let identities_contract_id = "11c70af56a763b05943888fa3719ef56b3e826615fdda2d463c63f4034cb861c"; // System identities contract
    let response = client.get_documents(
        identities_contract_id.to_string(),
        "identity".to_string(),
        query.into(),
        JsValue::null(),
        100,
        None,
        false, // unproved
    ).await?;
    
    Ok(response)
}

/// Fetch data contract history without proof
#[wasm_bindgen(js_name = fetchDataContractHistoryUnproved)]
pub async fn fetch_data_contract_history_unproved(
    sdk: &WasmSdk,
    contract_id: &str,
    start_at_ms: Option<f64>,
    limit: Option<u32>,
    offset: Option<u32>,
    options: Option<FetchOptions>,
) -> Result<JsValue, JsError> {
    let identifier = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identifier: {}", e)))?;

    // Execute the request (placeholder)
    let _options = options.unwrap_or_default();
    let _identifier = identifier;
    let _limit = limit;
    let _offset = offset;
    let _start_at_ms = start_at_ms;
    let _sdk = sdk;

    // Create DAPI client
    let client_config = DapiClientConfig::new(sdk.network());
    if let Some(timeout) = _options.timeout {
        client_config.clone().set_timeout(timeout);
    }
    if let Some(retries) = _options.retries {
        client_config.clone().set_retries(retries);
    }
    
    let client = DapiClient::new(client_config)?;
    
    // Query contract history documents
    let query = Object::new();
    let where_clause = js_sys::Array::new();
    
    // Add contract ID condition
    let contract_condition = js_sys::Array::of3(
        &"contractId".into(),
        &"==".into(),
        &contract_id.into()
    );
    where_clause.push(&contract_condition);
    
    // Add timestamp condition if provided
    if let Some(start_ms) = start_at_ms {
        let timestamp_condition = js_sys::Array::of3(
            &"updatedAt".into(),
            &">=".into(),
            &start_ms.into()
        );
        where_clause.push(&timestamp_condition);
    }
    
    Reflect::set(&query, &"where".into(), &where_clause)
        .map_err(|_| JsError::new("Failed to set where clause"))?;
    
    // Order by timestamp descending
    let order_by = js_sys::Array::of2(
        &js_sys::Array::of2(&"updatedAt".into(), &"desc".into()),
        &js_sys::Array::of2(&"$id".into(), &"asc".into())
    );
    Reflect::set(&query, &"orderBy".into(), &order_by)
        .map_err(|_| JsError::new("Failed to set orderBy"))?;
    
    // Set limit and offset
    Reflect::set(&query, &"limit".into(), &_limit.unwrap_or(100).into())
        .map_err(|_| JsError::new("Failed to set limit"))?;
    
    if let Some(offset_val) = _offset {
        Reflect::set(&query, &"startAt".into(), &offset_val.into())
            .map_err(|_| JsError::new("Failed to set offset"))?;
    }
    
    // Query the contract history from system contract
    let history_contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"; // System contract history contract
    let documents = client.get_documents(
        history_contract_id.to_string(),
        "contractHistory".to_string(),
        query.into(),
        JsValue::null(),
        _limit.unwrap_or(100),
        None,
        false, // unproved
    ).await?;
    
    // Build response with history array
    let response = Object::new();
    Reflect::set(&response, &"history".into(), &documents)
        .map_err(|_| JsError::new("Failed to set history"))?;
    Reflect::set(&response, &"contractId".into(), &contract_id.into())
        .map_err(|_| JsError::new("Failed to set contract ID"))?;

    Ok(response.into())
}

/// Batch fetch multiple items without proof
#[wasm_bindgen(js_name = fetchBatchUnproved)]
pub async fn fetch_batch_unproved(
    sdk: &WasmSdk,
    requests: JsValue,
    options: Option<FetchOptions>,
) -> Result<JsValue, JsError> {
    // Parse requests array from JS
    let requests_array = js_sys::Array::from(&requests);
    let results = js_sys::Array::new();

    for i in 0..requests_array.length() {
        let request = requests_array.get(i);
        
        // Parse request type
        let request_type = Reflect::get(&request, &"type".into())
            .map_err(|_| JsError::new("Failed to get request type"))?
            .as_string()
            .ok_or_else(|| JsError::new("Request type must be a string"))?;

        let result = match request_type.as_str() {
            "identity" => {
                let id = Reflect::get(&request, &"id".into())
                    .map_err(|_| JsError::new("Failed to get identity ID"))?
                    .as_string()
                    .ok_or_else(|| JsError::new("Identity ID must be a string"))?;
                
                fetch_identity_unproved(sdk, &id, options.clone()).await?
            }
            "dataContract" => {
                let id = Reflect::get(&request, &"id".into())
                    .map_err(|_| JsError::new("Failed to get contract ID"))?
                    .as_string()
                    .ok_or_else(|| JsError::new("Contract ID must be a string"))?;
                
                fetch_data_contract_unproved(sdk, &id, options.clone()).await?
            }
            _ => return Err(JsError::new(&format!("Unknown request type: {}", request_type))),
        };

        results.push(&result);
    }

    Ok(results.into())
}