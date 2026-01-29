//! Common settings types for SDK operations.
//!
//! This module provides WASM bindings for settings used across various SDK methods
//! including queries, broadcasts, and state transitions.

use crate::error::WasmSdkError;
use dash_sdk::platform::transition::put_settings::PutSettings;
use rs_dapi_client::RequestSettings;
use serde::Deserialize;
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_dpp2::serialization::conversions::from_object;

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
    max_decoding_message_size: Option<u64>,
}

impl From<RequestSettingsInput> for RequestSettings {
    fn from(input: RequestSettingsInput) -> Self {
        RequestSettings {
            retries: input.retries.map(|r| r as usize),
            timeout: input.timeout_ms.map(Duration::from_millis),
            connect_timeout: input.connect_timeout_ms.map(Duration::from_millis),
            ban_failed_address: input.ban_failed_address,
            max_decoding_message_size: input.max_decoding_message_size.map(|v| v as usize),
        }
    }
}

/// Parse request settings from JavaScript into RequestSettings.
///
/// Used for query operations.
///
/// Returns:
/// - `Ok(None)` if no settings were provided
/// - `Ok(Some(settings))` if valid settings were parsed
/// - `Err(...)` if settings were provided but malformed
pub fn parse_request_settings(
    settings: Option<RequestSettingsJs>,
) -> Result<Option<RequestSettings>, WasmSdkError> {
    let Some(settings_js) = settings else {
        return Ok(None);
    };

    if settings_js.is_undefined() || settings_js.is_null() {
        return Ok(None);
    }

    let input: RequestSettingsInput = from_object(settings_js.into())?;
    Ok(Some(input.into()))
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

/// Struct for deserializing PutSettings from JavaScript.
/// Used with `try_from_options_optional::<PutSettingsInput>(...).map(Into::into)`.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PutSettingsInput {
    #[serde(flatten)]
    request: RequestSettingsInput,
    // Put-specific fields
    wait_timeout_ms: Option<u64>,
    user_fee_increase: Option<u16>,
    identity_nonce_stale_time_s: Option<u64>,
    state_transition_creation_options: Option<StateTransitionCreationOptionsInput>,
}

impl TryFrom<&JsValue> for PutSettingsInput {
    type Error = wasm_dpp2::error::WasmDppError;

    fn try_from(value: &JsValue) -> Result<Self, Self::Error> {
        from_object(value.clone())
    }
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
///
/// Returns:
/// - `Ok(None)` if no settings were provided
/// - `Ok(Some(settings))` if valid settings were parsed
/// - `Err(...)` if settings were provided but malformed
pub fn parse_put_settings(
    settings: Option<PutSettingsJs>,
) -> Result<Option<PutSettings>, WasmSdkError> {
    let Some(settings_js) = settings else {
        return Ok(None);
    };

    if settings_js.is_undefined() || settings_js.is_null() {
        return Ok(None);
    }

    let input: PutSettingsInput = from_object(settings_js.into())?;
    Ok(Some(input.into()))
}

/// Extracts user_fee_increase from optional PutSettings, defaulting to 0.
///
/// This helper simplifies the common pattern of extracting user_fee_increase
/// from settings for state transition creation.
pub fn get_user_fee_increase(
    settings: Option<&PutSettings>,
) -> dash_sdk::dpp::prelude::UserFeeIncrease {
    settings
        .and_then(|s| s.user_fee_increase)
        .unwrap_or_default()
}
