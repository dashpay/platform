//! Common settings types for SDK operations.
//!
//! This module provides WASM bindings for settings used across various SDK methods
//! including queries, broadcasts, and state transitions.

use dash_sdk::platform::transition::put_settings::PutSettings;
use rs_dapi_client::RequestSettings;
use serde::Deserialize;
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

/// Internal struct for deserializing RequestSettings from JavaScript.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RequestSettingsInput {
    retries: Option<u32>,
    timeout_ms: Option<u64>,
    connect_timeout_ms: Option<u64>,
    ban_failed_address: Option<bool>,
}

impl From<RequestSettingsInput> for RequestSettings {
    fn from(input: RequestSettingsInput) -> Self {
        RequestSettings {
            retries: input.retries.map(|r| r as usize),
            timeout: input.timeout_ms.map(Duration::from_millis),
            connect_timeout: input.connect_timeout_ms.map(Duration::from_millis),
            ban_failed_address: input.ban_failed_address,
        }
    }
}

/// Parse request settings from JavaScript into RequestSettings.
///
/// Used for query operations.
pub fn parse_request_settings(settings: Option<RequestSettingsJs>) -> Option<RequestSettings> {
    let js_value: JsValue = settings?.into();

    if js_value.is_undefined() || js_value.is_null() {
        return None;
    }

    let input: RequestSettingsInput = serde_wasm_bindgen::from_value(js_value).ok()?;
    Some(input.into())
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

/// Internal struct for deserializing state transition creation options.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct StateTransitionCreationOptionsInput {
    #[serde(default)]
    allow_signing_with_any_security_level: bool,
    #[serde(default)]
    allow_signing_with_any_purpose: bool,
}

/// Internal struct for deserializing PutSettings from JavaScript.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PutSettingsInput {
    #[serde(flatten)]
    request: RequestSettingsInput,
    // Put-specific fields
    wait_timeout_ms: Option<u64>,
    user_fee_increase: Option<u16>,
    identity_nonce_stale_time_s: Option<u64>,
    state_transition_creation_options: Option<StateTransitionCreationOptionsInput>,
}

impl From<PutSettingsInput> for PutSettings {
    fn from(input: PutSettingsInput) -> Self {
        use dash_sdk::dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;

        let state_transition_creation_options =
            input.state_transition_creation_options.map(|opts| {
                let mut creation_options = StateTransitionCreationOptions::default();
                creation_options
                    .signing_options
                    .allow_signing_with_any_security_level =
                    opts.allow_signing_with_any_security_level;
                creation_options
                    .signing_options
                    .allow_signing_with_any_purpose = opts.allow_signing_with_any_purpose;
                creation_options
            });

        PutSettings {
            request_settings: input.request.into(),
            wait_timeout: input.wait_timeout_ms.map(Duration::from_millis),
            user_fee_increase: input.user_fee_increase,
            identity_nonce_stale_time_s: input.identity_nonce_stale_time_s,
            state_transition_creation_options,
        }
    }
}

/// Parse put settings from JavaScript into PutSettings.
///
/// Used for state transition broadcast operations.
pub fn parse_put_settings(settings: Option<PutSettingsJs>) -> Option<PutSettings> {
    let js_value: JsValue = settings?.into();

    if js_value.is_undefined() || js_value.is_null() {
        return None;
    }

    let input: PutSettingsInput = serde_wasm_bindgen::from_value(js_value).ok()?;
    Some(input.into())
}
