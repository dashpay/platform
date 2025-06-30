//! Universal transport layer for DAPI client that works in both browser and Node.js
//!
//! This module provides a transport implementation that automatically detects
//! the environment and uses the appropriate fetch mechanism.

use js_sys::{global, Array, Object, Promise, Reflect, Function};
use serde_json::Value;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// Configuration for the universal transport
#[derive(Clone, Debug)]
pub struct UniversalTransportConfig {
    pub timeout_ms: u32,
    pub retries: u32,
    pub headers: HashMap<String, String>,
}

impl Default for UniversalTransportConfig {
    fn default() -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Accept".to_string(), "application/json".to_string());
        
        Self {
            timeout_ms: 30000,
            retries: 3,
            headers,
        }
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

/// Create a Headers object for the request
fn create_headers(headers: &HashMap<String, String>) -> Result<JsValue, JsValue> {
    if is_nodejs() {
        // In Node.js, headers can be a plain object
        let headers_obj = Object::new();
        for (key, value) in headers {
            Reflect::set(&headers_obj, &JsValue::from_str(key), &JsValue::from_str(value))?;
        }
        Ok(headers_obj.into())
    } else {
        // In browser, use the Headers constructor
        let headers_class = Reflect::get(&global(), &JsValue::from_str("Headers"))?;
        let headers_obj = Reflect::construct(&Function::from(headers_class), &Array::new())?;
        
        for (key, value) in headers {
            let append_fn = Reflect::get(&headers_obj, &JsValue::from_str("append"))?;
            let args = Array::new();
            args.push(&JsValue::from_str(key));
            args.push(&JsValue::from_str(value));
            Reflect::apply(&Function::from(append_fn), &headers_obj, &args)?;
        }
        
        Ok(headers_obj)
    }
}

/// Create request options
fn create_request_options(
    method: &str,
    headers: &HashMap<String, String>,
    body: Option<&str>,
) -> Result<JsValue, JsValue> {
    let opts = Object::new();
    
    // Set method
    Reflect::set(&opts, &JsValue::from_str("method"), &JsValue::from_str(method))?;
    
    // Set headers
    let headers_obj = create_headers(headers)?;
    Reflect::set(&opts, &JsValue::from_str("headers"), &headers_obj)?;
    
    // Set body if provided
    if let Some(body_str) = body {
        Reflect::set(&opts, &JsValue::from_str("body"), &JsValue::from_str(body_str))?;
    }
    
    // For Node.js, we might need to handle self-signed certificates
    if is_nodejs() {
        // Create an agent that ignores certificate errors (for development)
        if let Ok(https) = Reflect::get(&global(), &JsValue::from_str("require"))
            .and_then(|require_fn| {
                let args = Array::new();
                args.push(&JsValue::from_str("https"));
                Reflect::apply(&require_fn.into(), &JsValue::NULL, &args)
            }) {
            // Try to create an agent with rejectUnauthorized: false
            if let Ok(agent_class) = Reflect::get(&https, &JsValue::from_str("Agent")) {
                let agent_opts = Object::new();
                Reflect::set(&agent_opts, &JsValue::from_str("rejectUnauthorized"), &JsValue::FALSE)?;
                
                if let Ok(agent) = Reflect::construct(&Function::from(agent_class), &Array::of1(&agent_opts)) {
                    Reflect::set(&opts, &JsValue::from_str("agent"), &agent)?;
                }
            }
        }
    }
    
    Ok(opts.into())
}

/// Universal fetch function that works in both browser and Node.js
async fn universal_fetch(url: &str, options: JsValue) -> Result<JsValue, JsValue> {
    let global_obj = global();
    
    // Check if fetch is available in global scope
    let fetch_fn = Reflect::get(&global_obj, &JsValue::from_str("fetch"))
        .map_err(|_| JsValue::from_str("fetch is not available"))?;
    
    if !fetch_fn.is_function() {
        return Err(JsValue::from_str(
            "fetch is not a function. In Node.js, ensure global.fetch is set (e.g., using node-fetch)"
        ));
    }
    
    // Call fetch
    let args = Array::new();
    args.push(&JsValue::from_str(url));
    args.push(&options);
    
    let promise = Reflect::apply(&fetch_fn.into(), &JsValue::NULL, &args)?;
    
    JsFuture::from(Promise::from(promise)).await
}

/// Parse response
async fn parse_response(response: JsValue) -> Result<(u16, String), JsValue> {
    // Get status
    let status = Reflect::get(&response, &JsValue::from_str("status"))?
        .as_f64()
        .unwrap_or(0.0) as u16;
    
    // Get response text
    let text_fn = Reflect::get(&response, &JsValue::from_str("text"))?;
    if !text_fn.is_function() {
        return Err(JsValue::from_str("Response.text is not a function"));
    }
    
    let text_promise = Reflect::apply(&text_fn.into(), &response, &Array::new())?;
    let text_future = JsFuture::from(Promise::from(text_promise)).await?;
    let text = text_future.as_string().unwrap_or_default();
    
    Ok((status, text))
}

/// Universal transport for DAPI client
#[wasm_bindgen]
#[derive(Clone)]
pub struct UniversalTransport {
    endpoints: Vec<String>,
    current_endpoint: usize,
    config: UniversalTransportConfig,
}

#[wasm_bindgen]
impl UniversalTransport {
    /// Create a new universal transport
    #[wasm_bindgen(constructor)]
    pub fn new(endpoints: Vec<JsValue>) -> Result<UniversalTransport, JsValue> {
        let endpoints: Vec<String> = endpoints
            .into_iter()
            .filter_map(|e| e.as_string())
            .collect();
        
        if endpoints.is_empty() {
            return Err(JsValue::from_str("No endpoints provided"));
        }
        
        Ok(UniversalTransport {
            endpoints,
            current_endpoint: 0,
            config: UniversalTransportConfig::default(),
        })
    }
    
    /// Set timeout in milliseconds
    #[wasm_bindgen(js_name = setTimeout)]
    pub fn set_timeout(&mut self, timeout_ms: u32) {
        self.config.timeout_ms = timeout_ms;
    }
    
    /// Set number of retries
    #[wasm_bindgen(js_name = setRetries)]
    pub fn set_retries(&mut self, retries: u32) {
        self.config.retries = retries;
    }
    
    /// Add a custom header
    #[wasm_bindgen(js_name = addHeader)]
    pub fn add_header(&mut self, key: String, value: String) {
        self.config.headers.insert(key, value);
    }
    
    /// Check if running in Node.js
    #[wasm_bindgen(js_name = isNodeJs)]
    pub fn is_nodejs(&self) -> bool {
        is_nodejs()
    }
    
    /// Get the next endpoint to try
    fn get_next_endpoint(&mut self) -> String {
        let endpoint = self.endpoints[self.current_endpoint].clone();
        self.current_endpoint = (self.current_endpoint + 1) % self.endpoints.len();
        
        // Ensure the endpoint has a protocol
        if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
            format!("https://{}", endpoint)
        } else {
            endpoint
        }
    }
    
    /// Make a request
    pub async fn request(
        &mut self,
        path: &str,
        request: &JsValue,
    ) -> Result<JsValue, JsError> {
        let body = if request.is_null() || request.is_undefined() {
            None
        } else {
            Some(serde_wasm_bindgen::from_value::<Value>(request.clone())
                .map_err(|e| JsError::new(&format!("Serialization error: {}", e)))?)
        };
        
        let body_str = body
            .map(|v| serde_json::to_string(&v))
            .transpose()
            .map_err(|e| JsError::new(&format!("Serialization error: {}", e)))?;
        
        // Try each endpoint with retries
        let mut last_error = None;
        
        for retry in 0..self.config.retries {
            let endpoint = self.get_next_endpoint();
            let url = format!("{}{}", endpoint, path);
            
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Attempting request to {} (retry {})", url, retry
            )));
            
            // Create request options
            let options = match create_request_options("POST", &self.config.headers, body_str.as_deref()) {
                Ok(opts) => opts,
                Err(e) => {
                    last_error = Some(JsError::new(&format!("Failed to create request: {:?}", e)));
                    continue;
                }
            };
            
            // Make request
            match universal_fetch(&url, options).await {
                Ok(response) => {
                    match parse_response(response).await {
                        Ok((status, text)) => {
                            if status >= 200 && status < 300 {
                                // Try to parse as JSON
                                match serde_json::from_str::<Value>(&text) {
                                    Ok(json) => {
                                        return serde_wasm_bindgen::to_value(&json)
                                            .map_err(|e| JsError::new(&format!("Serialization error: {}", e)));
                                    }
                                    Err(e) => {
                                        // If not JSON, return as string
                                        web_sys::console::warn_1(&JsValue::from_str(&format!(
                                            "Response is not JSON: {}", e
                                        )));
                                        return Ok(JsValue::from_str(&text));
                                    }
                                }
                            } else {
                                last_error = Some(JsError::new(&format!("HTTP {}: {}", status, text)));
                            }
                        }
                        Err(e) => {
                            last_error = Some(JsError::new(&format!("Failed to parse response: {:?}", e)));
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(JsError::new(&format!("Fetch failed: {:?}", e)));
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| JsError::new("All retries failed")))
    }
}

// Re-export the transport as the default
pub use UniversalTransport as Transport;