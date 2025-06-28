//! WebSocket Subscription Module
//!
//! This module provides real-time subscription functionality for monitoring
//! blockchain events and state changes through WebSocket connections.

use js_sys::Function;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

/// Extract subscription ID from JSON value
fn extract_subscription_id(msg: &serde_json::Value) -> Result<String, JsError> {
    msg["id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| JsError::new("Failed to get subscription ID"))
}

/// WebSocket subscription handle
#[wasm_bindgen]
#[derive(Clone)]
pub struct SubscriptionHandle {
    id: String,
    websocket: WebSocket,
    _callbacks: Rc<RefCell<SubscriptionCallbacks>>,
}

struct SubscriptionCallbacks {
    on_message: Option<Function>,
    on_error: Option<Function>,
    on_close: Option<Function>,
}

#[wasm_bindgen]
impl SubscriptionHandle {
    /// Get the subscription ID
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    /// Close the subscription
    #[wasm_bindgen]
    pub fn close(&self) -> Result<(), JsError> {
        self.websocket
            .close()
            .map_err(|_| JsError::new("Failed to close WebSocket connection"))
    }

    /// Check if the subscription is active
    #[wasm_bindgen(getter, js_name = isActive)]
    pub fn is_active(&self) -> bool {
        self.websocket.ready_state() == WebSocket::OPEN
    }
}

/// Subscribe to identity balance updates
#[wasm_bindgen(js_name = subscribeToIdentityBalanceUpdates)]
pub fn subscribe_to_identity_balance_updates(
    identity_id: &str,
    callback: &Function,
    endpoint: Option<String>,
) -> Result<SubscriptionHandle, JsError> {
    let endpoint = endpoint.unwrap_or_else(|| "wss://api.platform.dash.org/ws".to_string());

    // Create WebSocket connection
    let ws = WebSocket::new(&endpoint)
        .map_err(|_| JsError::new("Failed to create WebSocket connection"))?;

    // Create subscription request
    let subscribe_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "subscribe",
        "params": {
            "type": "identityBalance",
            "identityId": identity_id,
        },
        "id": uuid::Uuid::new_v4().to_string(),
    });

    let callbacks = Rc::new(RefCell::new(SubscriptionCallbacks {
        on_message: Some(callback.clone()),
        on_error: None,
        on_close: None,
    }));

    let subscription_id = extract_subscription_id(&subscribe_msg)?;

    let handle = SubscriptionHandle {
        id: subscription_id,
        websocket: ws.clone(),
        _callbacks: callbacks.clone(),
    };

    // Setup message handler
    let onmessage_callback = {
        let callbacks = callbacks.clone();
        Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                if let Some(string) = text.as_string() {
                    if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&string) {
                        if let Some(result) = msg.get("result") {
                            if let Some(callback) = callbacks.borrow().on_message.as_ref() {
                                if let Ok(js_result) = serde_wasm_bindgen::to_value(result) {
                                    let _ = callback.call1(&JsValue::null(), &js_result);
                                }
                            }
                        }
                    }
                }
            }
        })
    };

    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    // Setup open handler to send subscription
    let subscribe_msg_str = serde_json::to_string(&subscribe_msg)
        .map_err(|e| JsError::new(&format!("Failed to serialize subscription: {}", e)))?;

    let onopen_callback = {
        let ws = ws.clone();
        let msg = subscribe_msg_str.clone();
        Closure::<dyn FnMut()>::new(move || {
            let _ = ws.send_with_str(&msg);
        })
    };

    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    Ok(handle)
}

/// Subscribe to data contract updates
#[wasm_bindgen(js_name = subscribeToDataContractUpdates)]
pub fn subscribe_to_data_contract_updates(
    contract_id: &str,
    callback: &Function,
    endpoint: Option<String>,
) -> Result<SubscriptionHandle, JsError> {
    let endpoint = endpoint.unwrap_or_else(|| "wss://api.platform.dash.org/ws".to_string());

    let ws = WebSocket::new(&endpoint)
        .map_err(|_| JsError::new("Failed to create WebSocket connection"))?;

    let subscribe_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "subscribe",
        "params": {
            "type": "dataContract",
            "contractId": contract_id,
        },
        "id": uuid::Uuid::new_v4().to_string(),
    });

    let callbacks = Rc::new(RefCell::new(SubscriptionCallbacks {
        on_message: Some(callback.clone()),
        on_error: None,
        on_close: None,
    }));

    let subscription_id = extract_subscription_id(&subscribe_msg)?;

    let handle = SubscriptionHandle {
        id: subscription_id,
        websocket: ws.clone(),
        _callbacks: callbacks.clone(),
    };

    setup_websocket_handlers(&ws, callbacks, &subscribe_msg)?;

    Ok(handle)
}

/// Subscribe to document updates
#[wasm_bindgen(js_name = subscribeToDocumentUpdates)]
pub fn subscribe_to_document_updates(
    contract_id: &str,
    document_type: &str,
    where_clause: JsValue,
    callback: &Function,
    endpoint: Option<String>,
) -> Result<SubscriptionHandle, JsError> {
    let endpoint = endpoint.unwrap_or_else(|| "wss://api.platform.dash.org/ws".to_string());

    let ws = WebSocket::new(&endpoint)
        .map_err(|_| JsError::new("Failed to create WebSocket connection"))?;

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

    let callbacks = Rc::new(RefCell::new(SubscriptionCallbacks {
        on_message: Some(callback.clone()),
        on_error: None,
        on_close: None,
    }));

    let subscription_id = extract_subscription_id(&subscribe_msg)?;

    let handle = SubscriptionHandle {
        id: subscription_id,
        websocket: ws.clone(),
        _callbacks: callbacks.clone(),
    };

    setup_websocket_handlers(&ws, callbacks, &subscribe_msg)?;

    Ok(handle)
}

/// Subscribe to block headers
#[wasm_bindgen(js_name = subscribeToBlockHeaders)]
pub fn subscribe_to_block_headers(
    callback: &Function,
    endpoint: Option<String>,
) -> Result<SubscriptionHandle, JsError> {
    let endpoint = endpoint.unwrap_or_else(|| "wss://api.platform.dash.org/ws".to_string());

    let ws = WebSocket::new(&endpoint)
        .map_err(|_| JsError::new("Failed to create WebSocket connection"))?;

    let subscribe_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "subscribe",
        "params": {
            "type": "blockHeaders",
        },
        "id": uuid::Uuid::new_v4().to_string(),
    });

    let callbacks = Rc::new(RefCell::new(SubscriptionCallbacks {
        on_message: Some(callback.clone()),
        on_error: None,
        on_close: None,
    }));

    let subscription_id = extract_subscription_id(&subscribe_msg)?;

    let handle = SubscriptionHandle {
        id: subscription_id,
        websocket: ws.clone(),
        _callbacks: callbacks.clone(),
    };

    setup_websocket_handlers(&ws, callbacks, &subscribe_msg)?;

    Ok(handle)
}

/// Subscribe to state transition results
#[wasm_bindgen(js_name = subscribeToStateTransitionResults)]
pub fn subscribe_to_state_transition_results(
    state_transition_hash: &str,
    callback: &Function,
    endpoint: Option<String>,
) -> Result<SubscriptionHandle, JsError> {
    let endpoint = endpoint.unwrap_or_else(|| "wss://api.platform.dash.org/ws".to_string());

    let ws = WebSocket::new(&endpoint)
        .map_err(|_| JsError::new("Failed to create WebSocket connection"))?;

    let subscribe_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "subscribe",
        "params": {
            "type": "stateTransitionResult",
            "stateTransitionHash": state_transition_hash,
        },
        "id": uuid::Uuid::new_v4().to_string(),
    });

    let callbacks = Rc::new(RefCell::new(SubscriptionCallbacks {
        on_message: Some(callback.clone()),
        on_error: None,
        on_close: None,
    }));

    let subscription_id = extract_subscription_id(&subscribe_msg)?;

    let handle = SubscriptionHandle {
        id: subscription_id,
        websocket: ws.clone(),
        _callbacks: callbacks.clone(),
    };

    setup_websocket_handlers(&ws, callbacks, &subscribe_msg)?;

    Ok(handle)
}

// Helper function to setup WebSocket handlers
fn setup_websocket_handlers(
    ws: &WebSocket,
    callbacks: Rc<RefCell<SubscriptionCallbacks>>,
    subscribe_msg: &serde_json::Value,
) -> Result<(), JsError> {
    // Setup message handler
    let onmessage_callback = {
        let callbacks = callbacks.clone();
        Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                if let Some(string) = text.as_string() {
                    if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&string) {
                        // Handle subscription confirmation
                        if msg.get("id").is_some() && msg.get("result").is_some() {
                            // Subscription confirmed
                            return;
                        }

                        // Handle subscription update
                        if let Some(params) = msg.get("params") {
                            if let Some(callback) = callbacks.borrow().on_message.as_ref() {
                                let js_params = serde_wasm_bindgen::to_value(params).unwrap();
                                let _ = callback.call1(&JsValue::null(), &js_params);
                            }
                        }
                    }
                }
            }
        })
    };

    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    // Setup error handler
    let onerror_callback = {
        let callbacks = callbacks.clone();
        Closure::<dyn FnMut(_)>::new(move |_e: web_sys::Event| {
            if let Some(callback) = callbacks.borrow().on_error.as_ref() {
                let error = JsError::new("WebSocket error occurred");
                let _ = callback.call1(&JsValue::null(), &error.into());
            }
        })
    };

    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    // Setup close handler
    let onclose_callback = {
        let callbacks = callbacks.clone();
        Closure::<dyn FnMut(_)>::new(move |_e: web_sys::CloseEvent| {
            if let Some(callback) = callbacks.borrow().on_close.as_ref() {
                let _ = callback.call0(&JsValue::null());
            }
        })
    };

    ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
    onclose_callback.forget();

    // Setup open handler to send subscription
    let subscribe_msg_str = serde_json::to_string(subscribe_msg)
        .map_err(|e| JsError::new(&format!("Failed to serialize subscription: {}", e)))?;

    let onopen_callback = {
        let ws = ws.clone();
        let msg = subscribe_msg_str.clone();
        Closure::<dyn FnMut()>::new(move || {
            let _ = ws.send_with_str(&msg);
        })
    };

    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    Ok(())
}
