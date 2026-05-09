//! Generic state transition broadcast functionality.
//!
//! This module provides methods to broadcast any state transition
//! to the network and wait for the result.

use crate::error::WasmSdkError;
use crate::sdk::WasmSdk;
use crate::settings::{parse_put_settings, PutSettingsJs};
use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
use dash_sdk::dpp::state_transition::StateTransition;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_dpp2::state_transitions::proof_result::{
    convert_proof_result, StateTransitionProofResultTypeJs,
};
use wasm_dpp2::StateTransitionWasm;

#[wasm_bindgen(typescript_custom_section)]
const BROADCAST_AND_WAIT_RESULT_TS: &str = r#"
export interface BroadcastAndWaitResult {
  result: StateTransitionProofResultType;
  transitionHash: Uint8Array;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "BroadcastAndWaitResult")]
    pub type BroadcastAndWaitResultJs;
}

#[wasm_bindgen(js_name = "BroadcastAndWaitResult")]
pub struct BroadcastAndWaitResultWasm {
    result: JsValue,
    transition_hash: Vec<u8>,
}

impl BroadcastAndWaitResultWasm {
    fn new(result: StateTransitionProofResultTypeJs, transition_hash: [u8; 32]) -> Self {
        Self {
            result: JsValue::from(result),
            transition_hash: transition_hash.to_vec(),
        }
    }
}

#[wasm_bindgen(js_class = BroadcastAndWaitResult)]
impl BroadcastAndWaitResultWasm {
    #[wasm_bindgen(getter)]
    pub fn result(&self) -> StateTransitionProofResultTypeJs {
        self.result.clone().unchecked_into()
    }

    #[wasm_bindgen(getter, js_name = "transitionHash")]
    pub fn transition_hash(&self) -> Uint8Array {
        Uint8Array::from(self.transition_hash.as_slice())
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Broadcasts a state transition to the network.
    ///
    /// This method only broadcasts but does not wait for the result.
    /// Use `waitForResponse` to wait for confirmation after broadcasting,
    /// or use `broadcastAndWait` to do both in one call.
    ///
    /// @param stateTransition - The state transition to broadcast
    /// @param settings - Optional put settings (retries, timeout)
    #[wasm_bindgen(js_name = "broadcastStateTransition")]
    pub async fn broadcast_state_transition(
        &self,
        #[wasm_bindgen(js_name = "stateTransition")] state_transition: &StateTransitionWasm,
        settings: Option<PutSettingsJs>,
    ) -> Result<(), WasmSdkError> {
        let st: StateTransition = state_transition.into();
        let put_settings = parse_put_settings(settings)?;

        st.broadcast(self.as_ref(), put_settings)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast: {}", e)))?;

        Ok(())
    }

    /// Waits for a state transition response after it has been broadcast.
    ///
    /// Use this after calling `broadcastStateTransition` to wait for the transition
    /// to be processed by the network. This is useful when you want to broadcast
    /// and wait separately (e.g., for monitoring or progress tracking).
    ///
    /// Note: This differs from `waitForStateTransitionResult` which takes a hash string.
    /// This method takes the full state transition object and performs proof verification.
    ///
    /// @param stateTransition - The state transition that was broadcast
    /// @param settings - Optional put settings (retries, timeout, waitTimeoutMs)
    /// @returns The verified state transition result
    #[wasm_bindgen(js_name = "waitForResponse")]
    pub async fn wait_for_response(
        &self,
        #[wasm_bindgen(js_name = "stateTransition")] state_transition: &StateTransitionWasm,
        settings: Option<PutSettingsJs>,
    ) -> Result<StateTransitionProofResultTypeJs, WasmSdkError> {
        let st: StateTransition = state_transition.into();
        let put_settings = parse_put_settings(settings)?;

        let result = st
            .wait_for_response::<StateTransitionProofResult>(self.as_ref(), put_settings)
            .await
            .map_err(|e| {
                WasmSdkError::generic(format!("Failed to wait for state transition result: {}", e))
            })?;

        convert_proof_result(result).map_err(WasmSdkError::from)
    }

    /// Broadcasts a state transition and waits for the result.
    ///
    /// This method broadcasts the transition and waits for confirmation from the network.
    /// Returns once the transition has been processed or fails.
    /// This is equivalent to calling `broadcastStateTransition` followed by
    /// `waitForResponse`.
    ///
    /// @param stateTransition - The state transition to broadcast
    /// @param settings - Optional put settings (retries, timeout, waitTimeoutMs)
    /// @returns The verified state transition result
    #[wasm_bindgen(js_name = "broadcastAndWait")]
    pub async fn broadcast_and_wait(
        &self,
        #[wasm_bindgen(js_name = "stateTransition")] state_transition: &StateTransitionWasm,
        settings: Option<PutSettingsJs>,
    ) -> Result<BroadcastAndWaitResultWasm, WasmSdkError> {
        let st: StateTransition = state_transition.into();
        let put_settings = parse_put_settings(settings)?;

        let result = st
            .broadcast_and_wait::<StateTransitionProofResult>(self.as_ref(), put_settings)
            .await
            .map_err(WasmSdkError::from)?;

        let (proof_result, transition_hash) = result.into_parts();
        let converted = convert_proof_result(proof_result).map_err(WasmSdkError::from)?;

        Ok(BroadcastAndWaitResultWasm::new(converted, transition_hash))
    }
}
