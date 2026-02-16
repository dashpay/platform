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
use wasm_bindgen::prelude::*;
use wasm_dpp2::state_transitions::proof_result::{
    convert_proof_result, StateTransitionProofResultTypeJs,
};
use wasm_dpp2::StateTransitionWasm;

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
    ) -> Result<StateTransitionProofResultTypeJs, WasmSdkError> {
        let st: StateTransition = state_transition.into();
        let put_settings = parse_put_settings(settings)?;

        let result = st
            .broadcast_and_wait::<StateTransitionProofResult>(self.as_ref(), put_settings)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast: {}", e)))?;

        convert_proof_result(result).map_err(WasmSdkError::from)
    }
}
