use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::impl_wasm_type_info;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[wasm_bindgen(js_name = ShieldedWithdrawalTransition)]
pub struct ShieldedWithdrawalTransitionWasm(ShieldedWithdrawalTransition);

impl From<ShieldedWithdrawalTransition> for ShieldedWithdrawalTransitionWasm {
    fn from(v: ShieldedWithdrawalTransition) -> Self {
        ShieldedWithdrawalTransitionWasm(v)
    }
}

impl From<ShieldedWithdrawalTransitionWasm> for ShieldedWithdrawalTransition {
    fn from(v: ShieldedWithdrawalTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = ShieldedWithdrawalTransition)]
impl ShieldedWithdrawalTransitionWasm {
    #[wasm_bindgen(js_name = getType)]
    pub fn get_type(&self) -> u8 {
        self.0.state_transition_type() as u8
    }

    /// Returns the serialized Orchard actions as a JS array.
    #[wasm_bindgen(js_name = getActions)]
    pub fn get_actions(&self) -> WasmDppResult<JsValue> {
        let inner = match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => &v0.actions,
        };
        serde_wasm_bindgen::to_value(inner).map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    /// Returns the unshielding amount.
    #[wasm_bindgen(js_name = getUnshieldingAmount)]
    pub fn get_unshielding_amount(&self) -> u64 {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0.unshielding_amount,
        }
    }

    /// Returns the anchor (32-byte Merkle root).
    #[wasm_bindgen(js_name = getAnchor)]
    pub fn get_anchor(&self) -> Vec<u8> {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0.anchor.to_vec(),
        }
    }

    /// Returns the Halo2 proof bytes.
    #[wasm_bindgen(js_name = getProof)]
    pub fn get_proof(&self) -> Vec<u8> {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0.proof.clone(),
        }
    }

    /// Returns the RedPallas binding signature (64 bytes).
    #[wasm_bindgen(js_name = getBindingSignature)]
    pub fn get_binding_signature(&self) -> Vec<u8> {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0.binding_signature.to_vec(),
        }
    }

    /// Returns the core fee per byte.
    #[wasm_bindgen(js_name = getCoreFeePerByte)]
    pub fn get_core_fee_per_byte(&self) -> u32 {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0.core_fee_per_byte,
        }
    }

    /// Returns the pooling strategy as a u8.
    #[wasm_bindgen(js_name = getPooling)]
    pub fn get_pooling(&self) -> u8 {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0.pooling as u8,
        }
    }

    /// Returns the output script (core address).
    #[wasm_bindgen(js_name = getOutputScript)]
    pub fn get_output_script(&self) -> Vec<u8> {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0.output_script.as_bytes().to_vec(),
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
            &StateTransition::ShieldedWithdrawal(self.0.clone()),
        )?)
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<ShieldedWithdrawalTransitionWasm> {
        let st = StateTransition::deserialize_from_bytes(&bytes)?;
        match st {
            StateTransition::ShieldedWithdrawal(inner) => Ok(inner.into()),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type: expected ShieldedWithdrawal",
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
        StateTransition::ShieldedWithdrawal(self.0.clone()).into()
    }
}

impl_wasm_type_info!(
    ShieldedWithdrawalTransitionWasm,
    ShieldedWithdrawalTransition
);
