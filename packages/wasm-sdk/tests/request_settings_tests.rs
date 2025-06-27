//! Request settings module tests

use wasm_bindgen_test::*;
use wasm_sdk::{
    request_settings::{RequestSettings, RetryHandler, RequestSettingsBuilder, execute_with_retry},
    start,
};
use wasm_bindgen::prelude::*;
use js_sys::{Object, Function, Promise};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_request_settings_defaults() {
    start().await.expect("Failed to start WASM");
    
    let settings = RequestSettings::new();
    
    // Convert to object and check defaults
    let obj = settings.to_object().expect("Failed to convert to object");
    
    let max_retries = js_sys::Reflect::get(&obj, &"maxRetries".into())
        .unwrap().as_f64().unwrap();
    let timeout = js_sys::Reflect::get(&obj, &"timeoutMs".into())
        .unwrap().as_f64().unwrap();
    let use_backoff = js_sys::Reflect::get(&obj, &"useExponentialBackoff".into())
        .unwrap().as_bool().unwrap();
    
    assert_eq!(max_retries, 3.0);
    assert_eq!(timeout, 30000.0);
    assert!(use_backoff);
}

#[wasm_bindgen_test]
async fn test_request_settings_modification() {
    start().await.expect("Failed to start WASM");
    
    let mut settings = RequestSettings::new();
    
    settings.set_max_retries(5);
    settings.set_timeout(60000);
    settings.set_use_exponential_backoff(false);
    settings.set_retry_on_timeout(false);
    settings.set_retry_on_network_error(false);
    
    let obj = settings.to_object().unwrap();
    
    let max_retries = js_sys::Reflect::get(&obj, &"maxRetries".into())
        .unwrap().as_f64().unwrap();
    let timeout = js_sys::Reflect::get(&obj, &"timeoutMs".into())
        .unwrap().as_f64().unwrap();
    let use_backoff = js_sys::Reflect::get(&obj, &"useExponentialBackoff".into())
        .unwrap().as_bool().unwrap();
    let retry_timeout = js_sys::Reflect::get(&obj, &"retryOnTimeout".into())
        .unwrap().as_bool().unwrap();
    
    assert_eq!(max_retries, 5.0);
    assert_eq!(timeout, 60000.0);
    assert!(!use_backoff);
    assert!(!retry_timeout);
}

#[wasm_bindgen_test]
async fn test_retry_delay_calculation() {
    start().await.expect("Failed to start WASM");
    
    let settings = RequestSettings::new();
    
    // Test exponential backoff
    assert_eq!(settings.get_retry_delay(0), 1000);
    assert_eq!(settings.get_retry_delay(1), 2000);
    assert_eq!(settings.get_retry_delay(2), 4000);
    assert_eq!(settings.get_retry_delay(3), 8000);
    
    // Test max delay cap
    assert_eq!(settings.get_retry_delay(10), 30000); // Should be capped at max
}

#[wasm_bindgen_test]
async fn test_retry_delay_without_backoff() {
    start().await.expect("Failed to start WASM");
    
    let mut settings = RequestSettings::new();
    settings.set_use_exponential_backoff(false);
    
    // Should always return initial delay
    assert_eq!(settings.get_retry_delay(0), 1000);
    assert_eq!(settings.get_retry_delay(1), 1000);
    assert_eq!(settings.get_retry_delay(5), 1000);
}

#[wasm_bindgen_test]
async fn test_retry_handler_should_retry() {
    start().await.expect("Failed to start WASM");
    
    let settings = RequestSettings::new();
    let mut handler = RetryHandler::new(settings);
    
    // Create timeout error
    let error = Object::new();
    js_sys::Reflect::set(&error, &"isTimeout".into(), &true.into()).unwrap();
    
    assert!(handler.should_retry(&error.into()));
    
    // Increment attempts
    handler.increment_attempt();
    handler.increment_attempt();
    handler.increment_attempt();
    
    // Should not retry after max attempts
    assert!(!handler.should_retry(&error.into()));
}

#[wasm_bindgen_test]
async fn test_retry_handler_network_error() {
    start().await.expect("Failed to start WASM");
    
    let settings = RequestSettings::new();
    let handler = RetryHandler::new(settings);
    
    // Create network error
    let error = Object::new();
    js_sys::Reflect::set(&error, &"isNetworkError".into(), &true.into()).unwrap();
    
    assert!(handler.should_retry(&error.into()));
}

#[wasm_bindgen_test]
async fn test_retry_handler_error_codes() {
    start().await.expect("Failed to start WASM");
    
    let settings = RequestSettings::new();
    let handler = RetryHandler::new(settings);
    
    // Test retryable error codes
    for code in &["NETWORK_ERROR", "TIMEOUT", "UNAVAILABLE"] {
        let error = Object::new();
        js_sys::Reflect::set(&error, &"code".into(), &(*code).into()).unwrap();
        assert!(handler.should_retry(&error.into()), "Should retry on {}", code);
    }
    
    // Test non-retryable error code
    let error = Object::new();
    js_sys::Reflect::set(&error, &"code".into(), &"INVALID_ARGUMENT".into()).unwrap();
    assert!(!handler.should_retry(&error.into()));
}

#[wasm_bindgen_test]
async fn test_request_settings_builder() {
    start().await.expect("Failed to start WASM");
    
    let settings = RequestSettingsBuilder::new()
        .with_max_retries(10)
        .with_timeout(5000)
        .with_initial_retry_delay(500)
        .with_backoff_multiplier(1.5)
        .build();
    
    let obj = settings.to_object().unwrap();
    
    let max_retries = js_sys::Reflect::get(&obj, &"maxRetries".into())
        .unwrap().as_f64().unwrap();
    let timeout = js_sys::Reflect::get(&obj, &"timeoutMs".into())
        .unwrap().as_f64().unwrap();
    let initial_delay = js_sys::Reflect::get(&obj, &"initialRetryDelayMs".into())
        .unwrap().as_f64().unwrap();
    let multiplier = js_sys::Reflect::get(&obj, &"backoffMultiplier".into())
        .unwrap().as_f64().unwrap();
    
    assert_eq!(max_retries, 10.0);
    assert_eq!(timeout, 5000.0);
    assert_eq!(initial_delay, 500.0);
    assert_eq!(multiplier, 1.5);
}

#[wasm_bindgen_test]
async fn test_request_settings_builder_without_retries() {
    start().await.expect("Failed to start WASM");
    
    let settings = RequestSettingsBuilder::new()
        .without_retries()
        .build();
    
    let obj = settings.to_object().unwrap();
    let max_retries = js_sys::Reflect::get(&obj, &"maxRetries".into())
        .unwrap().as_f64().unwrap();
    
    assert_eq!(max_retries, 0.0);
}

#[wasm_bindgen_test]
async fn test_custom_headers() {
    start().await.expect("Failed to start WASM");
    
    let mut settings = RequestSettings::new();
    
    let headers = Object::new();
    js_sys::Reflect::set(&headers, &"Authorization".into(), &"Bearer token".into()).unwrap();
    js_sys::Reflect::set(&headers, &"X-Custom-Header".into(), &"value".into()).unwrap();
    
    settings.set_custom_headers(headers);
    
    let obj = settings.to_object().unwrap();
    let custom_headers = js_sys::Reflect::get(&obj, &"customHeaders".into()).unwrap();
    
    assert!(custom_headers.is_object());
    
    let auth = js_sys::Reflect::get(&custom_headers, &"Authorization".into())
        .unwrap().as_string().unwrap();
    assert_eq!(auth, "Bearer token");
}

#[wasm_bindgen_test]
async fn test_retry_handler_timing() {
    start().await.expect("Failed to start WASM");
    
    let settings = RequestSettings::new();
    let handler = RetryHandler::new(settings.clone());
    
    // Check initial state
    assert_eq!(handler.current_attempt(), 0);
    assert!(!handler.is_timeout_exceeded());
    
    // Get next retry delay
    let delay = handler.get_next_retry_delay();
    assert_eq!(delay, settings.get_retry_delay(0));
    
    // Elapsed time should be small
    let elapsed = handler.get_elapsed_time();
    assert!(elapsed < 1000.0); // Less than 1 second
}

#[wasm_bindgen_test] 
async fn test_execute_with_retry_success() {
    start().await.expect("Failed to start WASM");
    
    // Create a function that succeeds immediately
    let success_fn = Function::new_no_args("
        return Promise.resolve('success');
    ");
    
    let settings = RequestSettings::new();
    let result = execute_with_retry(success_fn, settings).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_string().unwrap(), "success");
}

#[wasm_bindgen_test]
async fn test_execute_with_retry_eventual_success() {
    start().await.expect("Failed to start WASM");
    
    // Create a function that fails twice then succeeds
    let eventual_success_fn = Function::new_no_args("
        if (!window.retryTestCounter) window.retryTestCounter = 0;
        window.retryTestCounter++;
        
        if (window.retryTestCounter < 3) {
            const error = new Error('Temporary failure');
            error.isNetworkError = true;
            return Promise.reject(error);
        }
        
        return Promise.resolve('success after retries');
    ");
    
    let mut settings = RequestSettings::new();
    settings.set_initial_retry_delay(10); // Fast retry for testing
    
    let result = execute_with_retry(eventual_success_fn, settings).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_string().unwrap(), "success after retries");
    
    // Clean up
    js_sys::Reflect::delete_property(&js_sys::global(), &"retryTestCounter".into()).unwrap();
}