use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::impl_wasm_type_info;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::shield_transition::ShieldTransition;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[wasm_bindgen(js_name = ShieldTransition)]
pub struct ShieldTransitionWasm(ShieldTransition);

impl From<ShieldTransition> for ShieldTransitionWasm {
    fn from(v: ShieldTransition) -> Self {
        ShieldTransitionWasm(v)
    }
}

impl From<ShieldTransitionWasm> for ShieldTransition {
    fn from(v: ShieldTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = ShieldTransition)]
impl ShieldTransitionWasm {
    #[wasm_bindgen(js_name = getType)]
    pub fn get_type(&self) -> u8 {
        self.0.state_transition_type() as u8
    }

    /// Returns the inputs map as a JS object.
    #[wasm_bindgen(js_name = getInputs)]
    pub fn get_inputs(&self) -> WasmDppResult<JsValue> {
        let inner = match &self.0 {
            ShieldTransition::V0(v0) => &v0.inputs,
        };
        serde_wasm_bindgen::to_value(inner).map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    /// Returns the serialized Orchard actions as a JS array.
    #[wasm_bindgen(js_name = getActions)]
    pub fn get_actions(&self) -> WasmDppResult<JsValue> {
        let inner = match &self.0 {
            ShieldTransition::V0(v0) => &v0.actions,
        };
        serde_wasm_bindgen::to_value(inner).map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    /// Returns the shield amount (credits entering the pool).
    #[wasm_bindgen(js_name = getAmount)]
    pub fn get_amount(&self) -> u64 {
        match &self.0 {
            ShieldTransition::V0(v0) => v0.amount,
        }
    }

    /// Returns the anchor (32-byte Merkle root).
    #[wasm_bindgen(js_name = getAnchor)]
    pub fn get_anchor(&self) -> Vec<u8> {
        match &self.0 {
            ShieldTransition::V0(v0) => v0.anchor.to_vec(),
        }
    }

    /// Returns the Halo2 proof bytes.
    #[wasm_bindgen(js_name = getProof)]
    pub fn get_proof(&self) -> Vec<u8> {
        match &self.0 {
            ShieldTransition::V0(v0) => v0.proof.clone(),
        }
    }

    /// Returns the RedPallas binding signature (64 bytes).
    #[wasm_bindgen(js_name = getBindingSignature)]
    pub fn get_binding_signature(&self) -> Vec<u8> {
        match &self.0 {
            ShieldTransition::V0(v0) => v0.binding_signature.to_vec(),
        }
    }

    /// Returns the fee strategy as a JS value.
    #[wasm_bindgen(js_name = getFeeStrategy)]
    pub fn get_fee_strategy(&self) -> WasmDppResult<JsValue> {
        let strategy = match &self.0 {
            ShieldTransition::V0(v0) => &v0.fee_strategy,
        };
        serde_wasm_bindgen::to_value(strategy)
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    /// Returns the user fee increase multiplier.
    #[wasm_bindgen(js_name = getUserFeeIncrease)]
    pub fn get_user_fee_increase(&self) -> u16 {
        match &self.0 {
            ShieldTransition::V0(v0) => v0.user_fee_increase,
        }
    }

    /// Returns the input witnesses as a JS value.
    #[wasm_bindgen(js_name = getInputWitnesses)]
    pub fn get_input_witnesses(&self) -> WasmDppResult<JsValue> {
        let witnesses = match &self.0 {
            ShieldTransition::V0(v0) => &v0.input_witnesses,
        };
        serde_wasm_bindgen::to_value(witnesses)
            .map_err(|e| WasmDppError::serialization(e.to_string()))
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
            &StateTransition::Shield(self.0.clone()),
        )?)
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<ShieldTransitionWasm> {
        let st = StateTransition::deserialize_from_bytes(&bytes)?;
        match st {
            StateTransition::Shield(inner) => Ok(inner.into()),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type: expected Shield",
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
        StateTransition::Shield(self.0.clone()).into()
    }
}

impl_wasm_type_info!(ShieldTransitionWasm, ShieldTransition);
