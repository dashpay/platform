//! Common settings types for SDK operations.
//!
//! This module provides WASM bindings for settings used across various SDK methods
//! including queries, broadcasts, and state transitions.

use dash_sdk::platform::transition::put_settings::PutSettings;
use rs_dapi_client::RequestSettings;
use std::time::Duration;
use wasm_bindgen::prelude::*;

// ============================================================================
// RequestSettings - for queries and basic requests
// ============================================================================

/// TypeScript interface for request settings (queries)
#[wasm_bindgen(typescript_custom_section)]
const REQUEST_SETTINGS_TS: &'static str = r#"
/**
 * Settings for SDK query/request operations.
 */
export interface RequestSettings {
  /**
   * Number of retries for the request.
   * @default 5
   */
  retries?: number;

  /**
   * Request timeout in milliseconds.
   * @default 10000
   */
  timeoutMs?: number;

  /**
   * Connection timeout in milliseconds.
   */
  connectTimeoutMs?: number;

  /**
   * Whether to ban failed addresses.
   * @default true
   */
  banFailedAddress?: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "RequestSettings")]
    pub type RequestSettingsJs;
}

/// Parse request settings from JavaScript into RequestSettings.
///
/// Used for query operations.
pub fn parse_request_settings(settings: Option<RequestSettingsJs>) -> Option<RequestSettings> {
    let settings_js = settings?;
    let settings_value: JsValue = settings_js.into();

    if settings_value.is_undefined() || settings_value.is_null() {
        return None;
    }

    let mut request_settings = RequestSettings::default();

    // Parse retries
    if let Ok(retries_js) = js_sys::Reflect::get(&settings_value, &JsValue::from_str("retries")) {
        if let Some(retries) = retries_js.as_f64() {
            request_settings.retries = Some(retries as usize);
        }
    }

    // Parse timeoutMs
    if let Ok(timeout_js) = js_sys::Reflect::get(&settings_value, &JsValue::from_str("timeoutMs")) {
        if let Some(ms) = timeout_js.as_f64() {
            request_settings.timeout = Some(Duration::from_millis(ms as u64));
        }
    }

    // Parse connectTimeoutMs
    if let Ok(connect_timeout_js) =
        js_sys::Reflect::get(&settings_value, &JsValue::from_str("connectTimeoutMs"))
    {
        if let Some(ms) = connect_timeout_js.as_f64() {
            request_settings.connect_timeout = Some(Duration::from_millis(ms as u64));
        }
    }

    // Parse banFailedAddress
    if let Ok(ban_js) =
        js_sys::Reflect::get(&settings_value, &JsValue::from_str("banFailedAddress"))
    {
        if let Some(ban) = ban_js.as_bool() {
            request_settings.ban_failed_address = Some(ban);
        }
    }

    Some(request_settings)
}

// ============================================================================
// PutSettings - for state transitions (broadcasts)
// ============================================================================

/// TypeScript interface for put/broadcast settings (state transitions)
#[wasm_bindgen(typescript_custom_section)]
const PUT_SETTINGS_TS: &'static str = r#"
/**
 * Settings for state transition broadcast operations.
 * Extends RequestSettings with additional options for waiting.
 */
export interface PutSettings {
  /**
   * Number of retries for the request.
   * @default 5
   */
  retries?: number;

  /**
   * Request timeout in milliseconds.
   * @default 10000
   */
  timeoutMs?: number;

  /**
   * Connection timeout in milliseconds.
   */
  connectTimeoutMs?: number;

  /**
   * Whether to ban failed addresses.
   * @default true
   */
  banFailedAddress?: boolean;

  /**
   * Timeout in milliseconds for waiting for the state transition result.
   * Only applies to broadcast and wait operations.
   */
  waitTimeoutMs?: number;

  /**
   * Fee increase multiplier (0-65535) to prioritize transaction processing.
   * Higher values result in higher fees and faster processing.
   * @default 0
   */
  userFeeIncrease?: number;

  /**
   * Time in seconds after which identity nonces are considered stale.
   * Used for nonce management in state transitions.
   */
  identityNonceStaleTimeS?: number;

  /**
   * Options for state transition creation (debugging).
   */
  stateTransitionCreationOptions?: {
    /**
     * Allow signing with any security level (debugging only).
     */
    allowSigningWithAnySecurityLevel?: boolean;
    /**
     * Allow signing with any purpose (debugging only).
     */
    allowSigningWithAnyPurpose?: boolean;
  };
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "PutSettings")]
    pub type PutSettingsJs;
}

/// Parse put settings from JavaScript into PutSettings.
///
/// Used for state transition broadcast operations.
pub fn parse_put_settings(settings: Option<PutSettingsJs>) -> Option<PutSettings> {
    let settings_js = settings?;
    let settings_value: JsValue = settings_js.into();

    if settings_value.is_undefined() || settings_value.is_null() {
        return None;
    }

    let mut put_settings = PutSettings::default();

    // Parse retries
    if let Ok(retries_js) = js_sys::Reflect::get(&settings_value, &JsValue::from_str("retries")) {
        if let Some(retries) = retries_js.as_f64() {
            put_settings.request_settings.retries = Some(retries as usize);
        }
    }

    // Parse timeoutMs
    if let Ok(timeout_js) = js_sys::Reflect::get(&settings_value, &JsValue::from_str("timeoutMs")) {
        if let Some(ms) = timeout_js.as_f64() {
            put_settings.request_settings.timeout = Some(Duration::from_millis(ms as u64));
        }
    }

    // Parse connectTimeoutMs
    if let Ok(connect_timeout_js) =
        js_sys::Reflect::get(&settings_value, &JsValue::from_str("connectTimeoutMs"))
    {
        if let Some(ms) = connect_timeout_js.as_f64() {
            put_settings.request_settings.connect_timeout = Some(Duration::from_millis(ms as u64));
        }
    }

    // Parse banFailedAddress
    if let Ok(ban_js) =
        js_sys::Reflect::get(&settings_value, &JsValue::from_str("banFailedAddress"))
    {
        if let Some(ban) = ban_js.as_bool() {
            put_settings.request_settings.ban_failed_address = Some(ban);
        }
    }

    // Parse waitTimeoutMs
    if let Ok(wait_timeout_js) =
        js_sys::Reflect::get(&settings_value, &JsValue::from_str("waitTimeoutMs"))
    {
        if let Some(ms) = wait_timeout_js.as_f64() {
            put_settings.wait_timeout = Some(Duration::from_millis(ms as u64));
        }
    }

    // Parse userFeeIncrease
    if let Ok(fee_increase_js) =
        js_sys::Reflect::get(&settings_value, &JsValue::from_str("userFeeIncrease"))
    {
        if let Some(fee) = fee_increase_js.as_f64() {
            put_settings.user_fee_increase = Some(fee as u16);
        }
    }

    // Parse identityNonceStaleTimeS
    if let Ok(nonce_stale_js) =
        js_sys::Reflect::get(&settings_value, &JsValue::from_str("identityNonceStaleTimeS"))
    {
        if let Some(secs) = nonce_stale_js.as_f64() {
            put_settings.identity_nonce_stale_time_s = Some(secs as u64);
        }
    }

    // Parse stateTransitionCreationOptions
    if let Ok(creation_options_js) = js_sys::Reflect::get(
        &settings_value,
        &JsValue::from_str("stateTransitionCreationOptions"),
    ) {
        if !creation_options_js.is_undefined() && !creation_options_js.is_null() {
            use dash_sdk::dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;

            let mut creation_options = StateTransitionCreationOptions::default();

            // Parse allowSigningWithAnySecurityLevel
            if let Ok(allow_sec_level_js) = js_sys::Reflect::get(
                &creation_options_js,
                &JsValue::from_str("allowSigningWithAnySecurityLevel"),
            ) {
                if let Some(allow) = allow_sec_level_js.as_bool() {
                    creation_options.signing_options.allow_signing_with_any_security_level = allow;
                }
            }

            // Parse allowSigningWithAnyPurpose
            if let Ok(allow_purpose_js) = js_sys::Reflect::get(
                &creation_options_js,
                &JsValue::from_str("allowSigningWithAnyPurpose"),
            ) {
                if let Some(allow) = allow_purpose_js.as_bool() {
                    creation_options.signing_options.allow_signing_with_any_purpose = allow;
                }
            }

            put_settings.state_transition_creation_options = Some(creation_options);
        }
    }

    Some(put_settings)
}
