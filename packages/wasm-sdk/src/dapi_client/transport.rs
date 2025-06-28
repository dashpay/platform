//! Transport layer for DAPI client
//!
//! This module provides a flexible transport implementation that works in both
//! browser and Node.js environments without gRPC dependencies.

use super::error::DapiClientError;
use serde::Serialize;
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response, Headers};

/// Transport configuration
#[derive(Clone)]
pub struct TransportConfig {
    pub endpoints: Vec<String>,
    pub timeout: Duration,
    pub retries: u32,
}

/// Transport implementation for DAPI requests
pub struct Transport {
    config: TransportConfig,
    current_endpoint_index: std::cell::Cell<usize>,
}

impl Transport {
    /// Create a new transport instance
    pub fn new(config: TransportConfig) -> Self {
        Transport {
            config,
            current_endpoint_index: std::cell::Cell::new(0),
        }
    }

    /// Get the current endpoint
    pub fn get_current_endpoint(&self) -> String {
        let index = self.current_endpoint_index.get();
        self.config.endpoints.get(index)
            .cloned()
            .unwrap_or_else(|| self.config.endpoints[0].clone())
    }

    /// Rotate to the next endpoint
    fn rotate_endpoint(&self) {
        let current = self.current_endpoint_index.get();
        let next = (current + 1) % self.config.endpoints.len();
        self.current_endpoint_index.set(next);
    }

    /// Make a request to DAPI
    pub async fn request<T: Serialize>(
        &self,
        path: &str,
        payload: &T,
    ) -> Result<JsValue, DapiClientError> {
        let mut last_error = None;
        
        // Try each endpoint with retries
        for _ in 0..self.config.endpoints.len() {
            let endpoint = self.get_current_endpoint();
            
            // Try retries on current endpoint
            for attempt in 0..=self.config.retries {
                match self.make_single_request(&endpoint, path, payload).await {
                    Ok(response) => return Ok(response),
                    Err(e) => {
                        last_error = Some(e);
                        if attempt < self.config.retries {
                            // Exponential backoff
                            let delay = 100 * (2_u32.pow(attempt));
                            gloo_timers::future::TimeoutFuture::new(delay).await;
                        }
                    }
                }
            }
            
            // Rotate to next endpoint after all retries failed
            self.rotate_endpoint();
        }

        Err(last_error.unwrap_or_else(|| 
            DapiClientError::Transport("All endpoints failed".to_string())
        ))
    }

    /// Make a single HTTP request
    async fn make_single_request<T: Serialize>(
        &self,
        endpoint: &str,
        path: &str,
        payload: &T,
    ) -> Result<JsValue, DapiClientError> {
        let url = format!("{}{}", endpoint, path);

        // Create headers
        let headers = Headers::new()
            .map_err(|_| DapiClientError::Transport("Failed to create headers".to_string()))?;
        
        headers.set("Content-Type", "application/json")
            .map_err(|_| DapiClientError::Transport("Failed to set content type".to_string()))?;

        // Serialize payload
        let body = serde_json::to_string(payload)
            .map_err(|e| DapiClientError::Serialization(e.to_string()))?;

        // Create request options
        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_headers(&headers);
        opts.set_body(&body.into());

        // Create request
        let request = Request::new_with_str_and_init(&url, &opts)
            .map_err(|_| DapiClientError::Transport("Failed to create request".to_string()))?;

        // Add timeout using AbortController
        let window = web_sys::window()
            .ok_or_else(|| DapiClientError::Transport("No window object".to_string()))?;
        
        let abort_controller = web_sys::AbortController::new()
            .map_err(|_| DapiClientError::Transport("Failed to create abort controller".to_string()))?;
        
        opts.set_signal(Some(&abort_controller.signal()));

        // Set timeout
        let timeout_ms = self.config.timeout.as_millis() as i32;
        let abort_controller_clone = abort_controller.clone();
        let timeout_handle = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            &Closure::<dyn Fn()>::new(move || {
                abort_controller_clone.abort();
            }).into_js_value().unchecked_into(),
            timeout_ms,
        ).map_err(|_| DapiClientError::Transport("Failed to set timeout".to_string()))?;

        // Make the request
        let response_promise = window.fetch_with_request(&request);
        let response_result = JsFuture::from(response_promise).await;

        // Clear timeout
        window.clear_timeout_with_handle(timeout_handle);

        // Handle response
        match response_result {
            Ok(response_value) => {
                let response: Response = response_value.dyn_into()
                    .map_err(|_| DapiClientError::Transport("Invalid response type".to_string()))?;

                if response.ok() {
                    let json_promise = response.json()
                        .map_err(|_| DapiClientError::Transport("Failed to get JSON".to_string()))?;
                    
                    let json_value = JsFuture::from(json_promise).await
                        .map_err(|e| DapiClientError::Response(format!("Failed to parse JSON: {:?}", e)))?;

                    Ok(json_value)
                } else {
                    let status = response.status();
                    let status_text = response.status_text();
                    
                    // Try to get error body
                    if let Ok(text_promise) = response.text() {
                        if let Ok(error_text) = JsFuture::from(text_promise).await {
                            if let Some(text) = error_text.as_string() {
                                return Err(DapiClientError::Response(
                                    format!("HTTP {}: {} - {}", status, status_text, text)
                                ));
                            }
                        }
                    }
                    
                    Err(DapiClientError::Response(
                        format!("HTTP {}: {}", status, status_text)
                    ))
                }
            }
            Err(e) => {
                // Check if it was aborted (timeout)
                if let Some(error) = e.dyn_ref::<js_sys::Error>() {
                    let name = error.name();
                    if name == "AbortError" {
                        return Err(DapiClientError::Timeout);
                    }
                }
                
                Err(DapiClientError::Transport(format!("Request failed: {:?}", e)))
            }
        }
    }
}

// Required for Closure to work
use wasm_bindgen::closure::Closure;