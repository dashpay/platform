use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::impl_wasm_type_info;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[wasm_bindgen(js_name = ShieldedTransferTransition)]
pub struct ShieldedTransferTransitionWasm(ShieldedTransferTransition);

impl From<ShieldedTransferTransition> for ShieldedTransferTransitionWasm {
    fn from(v: ShieldedTransferTransition) -> Self {
        ShieldedTransferTransitionWasm(v)
    }
}

impl From<ShieldedTransferTransitionWasm> for ShieldedTransferTransition {
    fn from(v: ShieldedTransferTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = ShieldedTransferTransition)]
impl ShieldedTransferTransitionWasm {
    #[wasm_bindgen(js_name = getType)]
    pub fn get_type(&self) -> u8 {
        self.0.state_transition_type() as u8
    }

    /// Returns the serialized Orchard actions as a JS array.
    #[wasm_bindgen(js_name = getActions)]
    pub fn get_actions(&self) -> WasmDppResult<JsValue> {
        let inner = match &self.0 {
            ShieldedTransferTransition::V0(v0) => &v0.actions,
        };
        serde_wasm_bindgen::to_value(inner).map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    /// Returns the value balance (fee amount leaving the pool).
    #[wasm_bindgen(js_name = getValueBalance)]
    pub fn get_value_balance(&self) -> u64 {
        match &self.0 {
            ShieldedTransferTransition::V0(v0) => v0.value_balance,
        }
    }

    /// Returns the anchor (32-byte Merkle root).
    #[wasm_bindgen(js_name = getAnchor)]
    pub fn get_anchor(&self) -> Vec<u8> {
        match &self.0 {
            ShieldedTransferTransition::V0(v0) => v0.anchor.to_vec(),
        }
    }

    /// Returns the Halo2 proof bytes.
    #[wasm_bindgen(js_name = getProof)]
    pub fn get_proof(&self) -> Vec<u8> {
        match &self.0 {
            ShieldedTransferTransition::V0(v0) => v0.proof.clone(),
        }
    }

    /// Returns the RedPallas binding signature (64 bytes).
    #[wasm_bindgen(js_name = getBindingSignature)]
    pub fn get_binding_signature(&self) -> Vec<u8> {
        match &self.0 {
            ShieldedTransferTransition::V0(v0) => v0.binding_signature.to_vec(),
        }
    }

    #[wasm_bindgen(js_name = getModifiedDataIds)]
    pub fn modified_data_ids(&self) -> Vec<IdentifierWasm> {
        self.0
            .modified_data_ids()
            .into_iter()
            .map(IdentifierWasm::from)
            .collect()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        serde_wasm_bindgen::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(PlatformSerializable::serialize_to_bytes(
            &StateTransition::ShieldedTransfer(self.0.clone()),
        )?)
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<ShieldedTransferTransitionWasm> {
        let st = StateTransition::deserialize_from_bytes(&bytes)?;
        match st {
            StateTransition::ShieldedTransfer(inner) => Ok(inner.into()),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type: expected ShieldedTransfer",
            )),
        }
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let json = serde_json::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        serde_wasm_bindgen::to_value(&json).map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = toStateTransition)]
    pub fn to_state_transition(&self) -> crate::state_transitions::base::StateTransitionWasm {
        StateTransition::ShieldedTransfer(self.0.clone()).into()
    }
}

impl_wasm_type_info!(ShieldedTransferTransitionWasm, ShieldedTransferTransition);
