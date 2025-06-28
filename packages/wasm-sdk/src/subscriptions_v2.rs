//! Enhanced WebSocket Subscription Module with proper memory management
//!
//! This module provides real-time subscription functionality for monitoring
//! blockchain events and state changes through WebSocket connections,
//! with proper cleanup to prevent memory leaks.

use js_sys::Function;
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CloseEvent, MessageEvent, WebSocket};

// Global registry to track active subscriptions and their closures
thread_local! {
    static ACTIVE_SUBSCRIPTIONS: RefCell<HashMap<String, SubscriptionData>> = RefCell::new(HashMap::new());
}

struct SubscriptionData {
    websocket: WebSocket,
    _onmessage: Closure<dyn FnMut(MessageEvent)>,
    _onerror: Closure<dyn FnMut(web_sys::Event)>,
    _onclose: Closure<dyn FnMut(CloseEvent)>,
    _onopen: Closure<dyn FnMut()>,
}

/// Extract subscription ID from JSON value
fn extract_subscription_id(msg: &serde_json::Value) -> Result<String, JsError> {
    msg["id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| JsError::new("Failed to get subscription ID"))
}

/// Enhanced WebSocket subscription handle with automatic cleanup
#[wasm_bindgen]
pub struct SubscriptionHandleV2 {
    id: String,
}

#[wasm_bindgen]
impl SubscriptionHandleV2 {
    /// Get the subscription ID
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    /// Close the subscription and clean up resources
    #[wasm_bindgen]
    pub fn close(&self) -> Result<(), JsError> {
        ACTIVE_SUBSCRIPTIONS.with(|subs| {
            let mut subs = subs.borrow_mut();
            if let Some(data) = subs.remove(&self.id) {
                // Close WebSocket
                data.websocket
                    .close()
                    .map_err(|_| JsError::new("Failed to close WebSocket connection"))?;

                // Clear event handlers
                data.websocket.set_onmessage(None);
                data.websocket.set_onerror(None);
                data.websocket.set_onclose(None);
                data.websocket.set_onopen(None);

                Ok(())
            } else {
                Err(JsError::new("Subscription not found"))
            }
        })
    }

    /// Check if the subscription is active
    #[wasm_bindgen(getter, js_name = isActive)]
    pub fn is_active(&self) -> bool {
        ACTIVE_SUBSCRIPTIONS.with(|subs| {
            let subs = subs.borrow();
            if let Some(data) = subs.get(&self.id) {
                data.websocket.ready_state() == WebSocket::OPEN
            } else {
                false
            }
        })
    }
}

impl Drop for SubscriptionHandleV2 {
    fn drop(&mut self) {
        // Ensure cleanup when handle is dropped
        let _ = self.close();
    }
}

/// Create a subscription with proper cleanup
fn create_subscription(
    endpoint: &str,
    subscribe_msg: serde_json::Value,
    on_message: Function,
    on_error: Option<Function>,
    on_close: Option<Function>,
) -> Result<SubscriptionHandleV2, JsError> {
    let ws = WebSocket::new(endpoint)
        .map_err(|_| JsError::new("Failed to create WebSocket connection"))?;

    let subscription_id = extract_subscription_id(&subscribe_msg)?;
    let id_clone = subscription_id.clone();

    // Create message handler
    let on_message_clone = on_message.clone();
    let onmessage = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
        if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
            if let Some(string) = text.as_string() {
                if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&string) {
                    // Handle subscription confirmation
                    if msg.get("id").is_some() && msg.get("result").is_some() {
                        return;
                    }

                    // Handle subscription update
                    if let Some(params) = msg.get("params") {
                        if let Ok(js_params) = serde_wasm_bindgen::to_value(params) {
                            let _ = on_message_clone.call1(&JsValue::null(), &js_params);
                        }
                    }
                }
            }
        }
    });

    // Create error handler
    let onerror = {
        let on_error_fn = on_error.clone();
        Closure::<dyn FnMut(_)>::new(move |_e: web_sys::Event| {
            if let Some(ref error_fn) = on_error_fn {
                let error = JsError::new("WebSocket error occurred");
                let _ = error_fn.call1(&JsValue::null(), &error.into());
            }
        })
    };

    // Create close handler
    let onclose = {
        let on_close_fn = on_close.clone();
        let id_for_close = subscription_id.clone();
        Closure::<dyn FnMut(_)>::new(move |_e: CloseEvent| {
            // Clean up from registry
            ACTIVE_SUBSCRIPTIONS.with(|subs| {
                subs.borrow_mut().remove(&id_for_close);
            });

            if let Some(ref close_fn) = on_close_fn {
                let _ = close_fn.call0(&JsValue::null());
            }
        })
    };

    // Create open handler
    let subscribe_msg_str = serde_json::to_string(&subscribe_msg)
        .map_err(|e| JsError::new(&format!("Failed to serialize subscription: {}", e)))?;

    let onopen = {
        let ws_clone = ws.clone();
        let msg = subscribe_msg_str.clone();
        Closure::<dyn FnMut()>::new(move || {
            let _ = ws_clone.send_with_str(&msg);
        })
    };

    // Set event handlers
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));

    // Store subscription data
    let subscription_data = SubscriptionData {
        websocket: ws,
        _onmessage: onmessage,
        _onerror: onerror,
        _onclose: onclose,
        _onopen: onopen,
    };

    ACTIVE_SUBSCRIPTIONS.with(|subs| {
        subs.borrow_mut()
            .insert(subscription_id.clone(), subscription_data);
    });

    Ok(SubscriptionHandleV2 { id: id_clone })
}

/// Subscribe to identity balance updates with automatic cleanup
#[wasm_bindgen(js_name = subscribeToIdentityBalanceUpdatesV2)]
pub fn subscribe_to_identity_balance_updates_v2(
    identity_id: &str,
    callback: &Function,
    endpoint: Option<String>,
) -> Result<SubscriptionHandleV2, JsError> {
    let endpoint = endpoint.unwrap_or_else(|| "wss://api.platform.dash.org/ws".to_string());

    let subscribe_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "subscribe",
        "params": {
            "type": "identityBalance",
            "identityId": identity_id,
        },
        "id": uuid::Uuid::new_v4().to_string(),
    });

    create_subscription(&endpoint, subscribe_msg, callback.clone(), None, None)
}

/// Subscribe to data contract updates with automatic cleanup
#[wasm_bindgen(js_name = subscribeToDataContractUpdatesV2)]
pub fn subscribe_to_data_contract_updates_v2(
    contract_id: &str,
    callback: &Function,
    endpoint: Option<String>,
) -> Result<SubscriptionHandleV2, JsError> {
    let endpoint = endpoint.unwrap_or_else(|| "wss://api.platform.dash.org/ws".to_string());

    let subscribe_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "subscribe",
        "params": {
            "type": "dataContract",
            "contractId": contract_id,
        },
        "id": uuid::Uuid::new_v4().to_string(),
    });

    create_subscription(&endpoint, subscribe_msg, callback.clone(), None, None)
}

/// Subscribe to document updates with automatic cleanup
#[wasm_bindgen(js_name = subscribeToDocumentUpdatesV2)]
pub fn subscribe_to_document_updates_v2(
    contract_id: &str,
    document_type: &str,
    where_clause: JsValue,
    callback: &Function,
    endpoint: Option<String>,
) -> Result<SubscriptionHandleV2, JsError> {
    let endpoint = endpoint.unwrap_or_else(|| "wss://api.platform.dash.org/ws".to_string());

    let mut params = serde_json::json!({
        "type": "documents",
        "contractId": contract_id,
        "documentType": document_type,
    });

    if !where_clause.is_null() && !where_clause.is_undefined() {
        params["where"] = serde_wasm_bindgen::from_value(where_clause)
            .map_err(|e| JsError::new(&format!("Invalid where clause: {}", e)))?;
    }

    let subscribe_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "subscribe",
        "params": params,
        "id": uuid::Uuid::new_v4().to_string(),
    });

    create_subscription(&endpoint, subscribe_msg, callback.clone(), None, None)
}

/// Subscribe with custom error and close handlers
#[wasm_bindgen(js_name = subscribeWithHandlersV2)]
pub fn subscribe_with_handlers_v2(
    subscription_type: &str,
    params: JsValue,
    on_message: &Function,
    on_error: Option<Function>,
    on_close: Option<Function>,
    endpoint: Option<String>,
) -> Result<SubscriptionHandleV2, JsError> {
    let endpoint = endpoint.unwrap_or_else(|| "wss://api.platform.dash.org/ws".to_string());

    let mut subscription_params = serde_json::json!({
        "type": subscription_type,
    });

    if !params.is_null() && !params.is_undefined() {
        let additional_params: serde_json::Value = serde_wasm_bindgen::from_value(params)
            .map_err(|e| JsError::new(&format!("Invalid params: {}", e)))?;

        if let serde_json::Value::Object(map) = additional_params {
            for (key, value) in map {
                subscription_params[key] = value;
            }
        }
    }

    let subscribe_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "subscribe",
        "params": subscription_params,
        "id": uuid::Uuid::new_v4().to_string(),
    });

    create_subscription(
        &endpoint,
        subscribe_msg,
        on_message.clone(),
        on_error,
        on_close,
    )
}

/// Clean up all active subscriptions
#[wasm_bindgen(js_name = cleanupAllSubscriptions)]
pub fn cleanup_all_subscriptions() {
    ACTIVE_SUBSCRIPTIONS.with(|subs| {
        let mut subs = subs.borrow_mut();

        // Close all WebSockets
        for (_, data) in subs.drain() {
            let _ = data.websocket.close();
            data.websocket.set_onmessage(None);
            data.websocket.set_onerror(None);
            data.websocket.set_onclose(None);
            data.websocket.set_onopen(None);
        }
    });
}

/// Get count of active subscriptions
#[wasm_bindgen(js_name = getActiveSubscriptionCount)]
pub fn get_active_subscription_count() -> usize {
    ACTIVE_SUBSCRIPTIONS.with(|subs| subs.borrow().len())
}

/// Connection options for subscriptions
#[wasm_bindgen]
pub struct SubscriptionOptions {
    /// Reconnect automatically on disconnect
    pub auto_reconnect: bool,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Reconnection delay in milliseconds
    pub reconnect_delay_ms: u32,
    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u32,
}

#[wasm_bindgen]
impl SubscriptionOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            auto_reconnect: true,
            max_reconnect_attempts: 5,
            reconnect_delay_ms: 1000,
            connection_timeout_ms: 30000,
        }
    }
}
