//! # Request Settings Module
//!
//! This module provides request configuration and retry logic for WASM environment

use js_sys::{Date, Object, Promise, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// Request settings for DAPI calls
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct RequestSettings {
    /// Maximum number of retries
    max_retries: u32,
    /// Initial retry delay in milliseconds
    initial_retry_delay_ms: u32,
    /// Maximum retry delay in milliseconds
    max_retry_delay_ms: u32,
    /// Backoff multiplier for exponential backoff
    backoff_multiplier: f64,
    /// Request timeout in milliseconds
    timeout_ms: u32,
    /// Whether to use exponential backoff
    use_exponential_backoff: bool,
    /// Whether to retry on timeout
    retry_on_timeout: bool,
    /// Whether to retry on network errors
    retry_on_network_error: bool,
    /// Custom headers to include
    custom_headers: Option<Object>,
}

#[wasm_bindgen]
impl RequestSettings {
    /// Create default request settings
    #[wasm_bindgen(constructor)]
    pub fn new() -> RequestSettings {
        RequestSettings {
            max_retries: 3,
            initial_retry_delay_ms: 1000,
            max_retry_delay_ms: 30000,
            backoff_multiplier: 2.0,
            timeout_ms: 30000,
            use_exponential_backoff: true,
            retry_on_timeout: true,
            retry_on_network_error: true,
            custom_headers: None,
        }
    }

    /// Set maximum retries
    #[wasm_bindgen(js_name = setMaxRetries)]
    pub fn set_max_retries(&mut self, retries: u32) {
        self.max_retries = retries;
    }

    /// Set initial retry delay
    #[wasm_bindgen(js_name = setInitialRetryDelay)]
    pub fn set_initial_retry_delay(&mut self, delay_ms: u32) {
        self.initial_retry_delay_ms = delay_ms;
    }

    /// Set maximum retry delay
    #[wasm_bindgen(js_name = setMaxRetryDelay)]
    pub fn set_max_retry_delay(&mut self, delay_ms: u32) {
        self.max_retry_delay_ms = delay_ms;
    }

    /// Set backoff multiplier
    #[wasm_bindgen(js_name = setBackoffMultiplier)]
    pub fn set_backoff_multiplier(&mut self, multiplier: f64) {
        self.backoff_multiplier = multiplier;
    }

    /// Set request timeout
    #[wasm_bindgen(js_name = setTimeout)]
    pub fn set_timeout(&mut self, timeout_ms: u32) {
        self.timeout_ms = timeout_ms;
    }

    /// Enable/disable exponential backoff
    #[wasm_bindgen(js_name = setUseExponentialBackoff)]
    pub fn set_use_exponential_backoff(&mut self, use_backoff: bool) {
        self.use_exponential_backoff = use_backoff;
    }

    /// Enable/disable retry on timeout
    #[wasm_bindgen(js_name = setRetryOnTimeout)]
    pub fn set_retry_on_timeout(&mut self, retry: bool) {
        self.retry_on_timeout = retry;
    }

    /// Enable/disable retry on network error
    #[wasm_bindgen(js_name = setRetryOnNetworkError)]
    pub fn set_retry_on_network_error(&mut self, retry: bool) {
        self.retry_on_network_error = retry;
    }

    /// Set custom headers
    #[wasm_bindgen(js_name = setCustomHeaders)]
    pub fn set_custom_headers(&mut self, headers: Object) {
        self.custom_headers = Some(headers);
    }

    /// Get the delay for a specific retry attempt
    #[wasm_bindgen(js_name = getRetryDelay)]
    pub fn get_retry_delay(&self, attempt: u32) -> u32 {
        if !self.use_exponential_backoff {
            return self.initial_retry_delay_ms;
        }

        let delay = self.initial_retry_delay_ms as f64 * self.backoff_multiplier.powi(attempt as i32);
        delay.min(self.max_retry_delay_ms as f64) as u32
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        Reflect::set(&obj, &"maxRetries".into(), &self.max_retries.into())
            .map_err(|_| JsError::new("Failed to set max retries"))?;
        Reflect::set(&obj, &"initialRetryDelayMs".into(), &self.initial_retry_delay_ms.into())
            .map_err(|_| JsError::new("Failed to set initial retry delay"))?;
        Reflect::set(&obj, &"maxRetryDelayMs".into(), &self.max_retry_delay_ms.into())
            .map_err(|_| JsError::new("Failed to set max retry delay"))?;
        Reflect::set(&obj, &"backoffMultiplier".into(), &self.backoff_multiplier.into())
            .map_err(|_| JsError::new("Failed to set backoff multiplier"))?;
        Reflect::set(&obj, &"timeoutMs".into(), &self.timeout_ms.into())
            .map_err(|_| JsError::new("Failed to set timeout"))?;
        Reflect::set(&obj, &"useExponentialBackoff".into(), &self.use_exponential_backoff.into())
            .map_err(|_| JsError::new("Failed to set exponential backoff"))?;
        Reflect::set(&obj, &"retryOnTimeout".into(), &self.retry_on_timeout.into())
            .map_err(|_| JsError::new("Failed to set retry on timeout"))?;
        Reflect::set(&obj, &"retryOnNetworkError".into(), &self.retry_on_network_error.into())
            .map_err(|_| JsError::new("Failed to set retry on network error"))?;
        
        if let Some(ref headers) = self.custom_headers {
            Reflect::set(&obj, &"customHeaders".into(), headers)
                .map_err(|_| JsError::new("Failed to set custom headers"))?;
        }
        
        Ok(obj.into())
    }
}

impl Default for RequestSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Retry handler for WASM environment
#[wasm_bindgen]
pub struct RetryHandler {
    settings: RequestSettings,
    current_attempt: u32,
    start_time: f64,
}

#[wasm_bindgen]
impl RetryHandler {
    /// Create a new retry handler
    #[wasm_bindgen(constructor)]
    pub fn new(settings: RequestSettings) -> RetryHandler {
        RetryHandler {
            settings,
            current_attempt: 0,
            start_time: Date::now(),
        }
    }

    /// Check if we should retry
    #[wasm_bindgen(js_name = shouldRetry)]
    pub fn should_retry(&self, error: &JsValue) -> bool {
        if self.current_attempt >= self.settings.max_retries {
            return false;
        }

        // Check error type
        if let Some(error_obj) = error.dyn_ref::<Object>() {
            // Check for timeout error
            if self.settings.retry_on_timeout {
                if let Ok(is_timeout) = Reflect::get(error_obj, &"isTimeout".into()) {
                    if is_timeout.is_truthy() {
                        return true;
                    }
                }
            }

            // Check for network error
            if self.settings.retry_on_network_error {
                if let Ok(is_network) = Reflect::get(error_obj, &"isNetworkError".into()) {
                    if is_network.is_truthy() {
                        return true;
                    }
                }
            }

            // Check error code
            if let Ok(code) = Reflect::get(error_obj, &"code".into()) {
                if let Some(code_str) = code.as_string() {
                    // Retry on specific error codes
                    match code_str.as_str() {
                        "NETWORK_ERROR" | "TIMEOUT" | "UNAVAILABLE" => return true,
                        _ => {}
                    }
                }
            }
        }

        false
    }

    /// Get the next retry delay
    #[wasm_bindgen(js_name = getNextRetryDelay)]
    pub fn get_next_retry_delay(&self) -> u32 {
        self.settings.get_retry_delay(self.current_attempt)
    }

    /// Increment attempt counter
    #[wasm_bindgen(js_name = incrementAttempt)]
    pub fn increment_attempt(&mut self) {
        self.current_attempt += 1;
    }

    /// Get current attempt number
    #[wasm_bindgen(getter, js_name = currentAttempt)]
    pub fn current_attempt(&self) -> u32 {
        self.current_attempt
    }

    /// Get elapsed time in milliseconds
    #[wasm_bindgen(js_name = getElapsedTime)]
    pub fn get_elapsed_time(&self) -> f64 {
        Date::now() - self.start_time
    }

    /// Check if timeout exceeded
    #[wasm_bindgen(js_name = isTimeoutExceeded)]
    pub fn is_timeout_exceeded(&self) -> bool {
        self.get_elapsed_time() > self.settings.timeout_ms as f64
    }
}

/// Execute a request with retry logic
#[wasm_bindgen(js_name = executeWithRetry)]
pub async fn execute_with_retry(
    request_fn: js_sys::Function,
    settings: RequestSettings,
) -> Result<JsValue, JsError> {
    let mut retry_handler = RetryHandler::new(settings.clone());
    let this = JsValue::null();
    
    loop {
        // Call the request function
        let result = request_fn.call0(&this)
            .map_err(|e| JsError::new(&format!("Failed to call request function: {:?}", e)))?;
        
        // Check if it's a promise
        if js_sys::Promise::is_type_of(&result) {
            let promise = result.dyn_into::<Promise>()
                .map_err(|_| JsError::new("Failed to convert to Promise"))?;
            match JsFuture::from(promise).await {
                Ok(value) => return Ok(value),
                Err(error) => {
                    if !retry_handler.should_retry(&error) {
                        return Err(JsError::new(&format!("Request failed: {:?}", error)));
                    }
                    
                    // Wait before retrying
                    let delay = retry_handler.get_next_retry_delay();
                    sleep_ms(delay).await;
                    retry_handler.increment_attempt();
                }
            }
        } else {
            // Not a promise, return directly
            return Ok(result);
        }
        
        // Check overall timeout
        if retry_handler.is_timeout_exceeded() {
            return Err(JsError::new("Overall timeout exceeded"));
        }
    }
}

/// Sleep for specified milliseconds (browser-compatible)
async fn sleep_ms(ms: u32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let closure = Closure::once(move || {
            let _ = resolve.call0(&JsValue::undefined());
        });
        
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                ms as i32,
            );
        }
        
        closure.forget();
    });
    
    let _ = JsFuture::from(promise).await;
}

/// Builder for creating customized request settings
#[wasm_bindgen]
pub struct RequestSettingsBuilder {
    settings: RequestSettings,
}

#[wasm_bindgen]
impl RequestSettingsBuilder {
    /// Create a new builder
    #[wasm_bindgen(constructor)]
    pub fn new() -> RequestSettingsBuilder {
        RequestSettingsBuilder {
            settings: RequestSettings::new(),
        }
    }

    /// Set max retries
    #[wasm_bindgen(js_name = withMaxRetries)]
    pub fn with_max_retries(mut self, retries: u32) -> RequestSettingsBuilder {
        self.settings.max_retries = retries;
        self
    }

    /// Set timeout
    #[wasm_bindgen(js_name = withTimeout)]
    pub fn with_timeout(mut self, timeout_ms: u32) -> RequestSettingsBuilder {
        self.settings.timeout_ms = timeout_ms;
        self
    }

    /// Set initial retry delay
    #[wasm_bindgen(js_name = withInitialRetryDelay)]
    pub fn with_initial_retry_delay(mut self, delay_ms: u32) -> RequestSettingsBuilder {
        self.settings.initial_retry_delay_ms = delay_ms;
        self
    }

    /// Set backoff multiplier
    #[wasm_bindgen(js_name = withBackoffMultiplier)]
    pub fn with_backoff_multiplier(mut self, multiplier: f64) -> RequestSettingsBuilder {
        self.settings.backoff_multiplier = multiplier;
        self
    }

    /// Disable retries
    #[wasm_bindgen(js_name = withoutRetries)]
    pub fn without_retries(mut self) -> RequestSettingsBuilder {
        self.settings.max_retries = 0;
        self
    }

    /// Build the settings
    pub fn build(self) -> RequestSettings {
        self.settings
    }
}

impl Default for RequestSettingsBuilder {
    fn default() -> Self {
        Self::new()
    }
}