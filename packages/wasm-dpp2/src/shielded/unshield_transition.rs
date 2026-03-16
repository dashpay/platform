use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::{impl_wasm_conversions_serde, impl_wasm_type_info};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::unshield_transition::UnshieldTransition;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * UnshieldTransition serialized as a plain object.
 */
export interface UnshieldTransitionObject {
    $formatVersion: string;
    outputAddress: object;
    actions: SerializedOrchardAction[];
    unshieldingAmount: bigint;
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
}

/**
 * UnshieldTransition serialized as JSON (human-readable).
 */
export interface UnshieldTransitionJSON {
    $formatVersion: string;
    outputAddress: object;
    actions: SerializedOrchardActionJSON[];
    unshieldingAmount: number | string;
    anchor: string;
    proof: string;
    bindingSignature: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "UnshieldTransitionObject")]
    pub type UnshieldTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "UnshieldTransitionJSON")]
    pub type UnshieldTransitionJSONJs;
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = UnshieldTransition)]
pub struct UnshieldTransitionWasm(UnshieldTransition);

impl From<UnshieldTransition> for UnshieldTransitionWasm {
    fn from(v: UnshieldTransition) -> Self {
        UnshieldTransitionWasm(v)
    }
}

impl From<UnshieldTransitionWasm> for UnshieldTransition {
    fn from(v: UnshieldTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = UnshieldTransition)]
impl UnshieldTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(value: JsValue) -> WasmDppResult<UnshieldTransitionWasm> {
        let inner: UnshieldTransition = serde_wasm_bindgen::from_value(value)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Ok(UnshieldTransitionWasm(inner))
    }

    #[wasm_bindgen(js_name = getType)]
    pub fn get_type(&self) -> u8 {
        self.0.state_transition_type() as u8
    }

    /// Returns the output address as a JS value (serialized PlatformAddress).
    #[wasm_bindgen(js_name = getOutputAddress)]
    pub fn get_output_address(&self) -> WasmDppResult<JsValue> {
        let addr = match &self.0 {
            UnshieldTransition::V0(v0) => &v0.output_address,
        };
        serde_wasm_bindgen::to_value(addr).map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    /// Returns the serialized Orchard actions as a JS array.
    #[wasm_bindgen(js_name = getActions)]
    pub fn get_actions(&self) -> WasmDppResult<JsValue> {
        let inner = match &self.0 {
            UnshieldTransition::V0(v0) => &v0.actions,
        };
        serde_wasm_bindgen::to_value(inner).map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    /// Returns the unshielding amount.
    #[wasm_bindgen(js_name = getUnshieldingAmount)]
    pub fn get_unshielding_amount(&self) -> u64 {
        match &self.0 {
            UnshieldTransition::V0(v0) => v0.unshielding_amount,
        }
    }

    /// Returns the anchor (32-byte Merkle root).
    #[wasm_bindgen(js_name = getAnchor)]
    pub fn get_anchor(&self) -> Vec<u8> {
        match &self.0 {
            UnshieldTransition::V0(v0) => v0.anchor.to_vec(),
        }
    }

    /// Returns the Halo2 proof bytes.
    #[wasm_bindgen(js_name = getProof)]
    pub fn get_proof(&self) -> Vec<u8> {
        match &self.0 {
            UnshieldTransition::V0(v0) => v0.proof.clone(),
        }
    }

    /// Returns the RedPallas binding signature (64 bytes).
    #[wasm_bindgen(js_name = getBindingSignature)]
    pub fn get_binding_signature(&self) -> Vec<u8> {
        match &self.0 {
            UnshieldTransition::V0(v0) => v0.binding_signature.to_vec(),
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

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(PlatformSerializable::serialize_to_bytes(
            &StateTransition::Unshield(self.0.clone()),
        )?)
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<UnshieldTransitionWasm> {
        let st = StateTransition::deserialize_from_bytes(&bytes)?;
        match st {
            StateTransition::Unshield(inner) => Ok(inner.into()),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type: expected Unshield",
            )),
        }
    }

    #[wasm_bindgen(js_name = toStateTransition)]
    pub fn to_state_transition(&self) -> crate::state_transitions::base::StateTransitionWasm {
        StateTransition::Unshield(self.0.clone()).into()
    }
}

impl_wasm_conversions_serde!(
    UnshieldTransitionWasm,
    UnshieldTransition,
    UnshieldTransitionObjectJs,
    UnshieldTransitionJSONJs
);

impl_wasm_type_info!(UnshieldTransitionWasm, UnshieldTransition);
