use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::{impl_wasm_conversions_serde, impl_wasm_type_info};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * A serialized Orchard action (spend-output pair) in binary/Object form.
 */
export interface SerializedOrchardAction {
    nullifier: Uint8Array;
    rk: Uint8Array;
    cmx: Uint8Array;
    encryptedNote: Uint8Array;
    cvNet: Uint8Array;
    spendAuthSig: Uint8Array;
}

/**
 * A serialized Orchard action (spend-output pair) in JSON form.
 */
export interface SerializedOrchardActionJSON {
    nullifier: string;
    rk: string;
    cmx: string;
    encryptedNote: string;
    cvNet: string;
    spendAuthSig: string;
}

/**
 * ShieldFromAssetLockTransition serialized as a plain object.
 */
export interface ShieldFromAssetLockTransitionObject {
    $formatVersion: string;
    assetLockProof: AssetLockProofObject;
    actions: SerializedOrchardAction[];
    valueBalance: bigint;
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
    signature: Uint8Array;
}

/**
 * ShieldFromAssetLockTransition serialized as JSON (human-readable).
 */
export interface ShieldFromAssetLockTransitionJSON {
    $formatVersion: string;
    assetLockProof: AssetLockProofJSON;
    actions: SerializedOrchardActionJSON[];
    valueBalance: number | string;
    anchor: string;
    proof: string;
    bindingSignature: string;
    signature: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ShieldFromAssetLockTransitionObject")]
    pub type ShieldFromAssetLockTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "ShieldFromAssetLockTransitionJSON")]
    pub type ShieldFromAssetLockTransitionJSONJs;
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = ShieldFromAssetLockTransition)]
pub struct ShieldFromAssetLockTransitionWasm(ShieldFromAssetLockTransition);

impl From<ShieldFromAssetLockTransition> for ShieldFromAssetLockTransitionWasm {
    fn from(v: ShieldFromAssetLockTransition) -> Self {
        ShieldFromAssetLockTransitionWasm(v)
    }
}

impl From<ShieldFromAssetLockTransitionWasm> for ShieldFromAssetLockTransition {
    fn from(v: ShieldFromAssetLockTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = ShieldFromAssetLockTransition)]
impl ShieldFromAssetLockTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(value: ShieldFromAssetLockTransitionObjectJs) -> WasmDppResult<ShieldFromAssetLockTransitionWasm> {
        let inner: ShieldFromAssetLockTransition = serde_wasm_bindgen::from_value(value.into())
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Ok(ShieldFromAssetLockTransitionWasm(inner))
    }

    #[wasm_bindgen(js_name = getType)]
    pub fn get_type(&self) -> u8 {
        self.0.state_transition_type() as u8
    }

    /// Returns the asset lock proof as a JS value.
    #[wasm_bindgen(js_name = getAssetLockProof)]
    pub fn get_asset_lock_proof(&self) -> WasmDppResult<JsValue> {
        let proof = match &self.0 {
            ShieldFromAssetLockTransition::V0(v0) => &v0.asset_lock_proof,
        };
        serde_wasm_bindgen::to_value(proof).map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    /// Returns the serialized Orchard actions as a JS array.
    #[wasm_bindgen(js_name = getActions)]
    pub fn get_actions(&self) -> WasmDppResult<JsValue> {
        let inner = match &self.0 {
            ShieldFromAssetLockTransition::V0(v0) => &v0.actions,
        };
        serde_wasm_bindgen::to_value(inner).map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    /// Returns the net value balance.
    #[wasm_bindgen(js_name = getValueBalance)]
    pub fn get_value_balance(&self) -> u64 {
        match &self.0 {
            ShieldFromAssetLockTransition::V0(v0) => v0.value_balance,
        }
    }

    /// Returns the anchor (32-byte Merkle root).
    #[wasm_bindgen(js_name = getAnchor)]
    pub fn get_anchor(&self) -> Vec<u8> {
        match &self.0 {
            ShieldFromAssetLockTransition::V0(v0) => v0.anchor.to_vec(),
        }
    }

    /// Returns the Halo2 proof bytes.
    #[wasm_bindgen(js_name = getProof)]
    pub fn get_proof(&self) -> Vec<u8> {
        match &self.0 {
            ShieldFromAssetLockTransition::V0(v0) => v0.proof.clone(),
        }
    }

    /// Returns the RedPallas binding signature (64 bytes).
    #[wasm_bindgen(js_name = getBindingSignature)]
    pub fn get_binding_signature(&self) -> Vec<u8> {
        match &self.0 {
            ShieldFromAssetLockTransition::V0(v0) => v0.binding_signature.to_vec(),
        }
    }

    /// Returns the ECDSA signature.
    #[wasm_bindgen(js_name = getSignature)]
    pub fn get_signature(&self) -> Vec<u8> {
        match &self.0 {
            ShieldFromAssetLockTransition::V0(v0) => v0.signature.to_vec(),
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
            &StateTransition::ShieldFromAssetLock(self.0.clone()),
        )?)
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<ShieldFromAssetLockTransitionWasm> {
        let st = StateTransition::deserialize_from_bytes(&bytes)?;
        match st {
            StateTransition::ShieldFromAssetLock(inner) => Ok(inner.into()),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type: expected ShieldFromAssetLock",
            )),
        }
    }

    #[wasm_bindgen(js_name = toStateTransition)]
    pub fn to_state_transition(&self) -> crate::state_transitions::base::StateTransitionWasm {
        StateTransition::ShieldFromAssetLock(self.0.clone()).into()
    }
}

impl_wasm_conversions_serde!(
    ShieldFromAssetLockTransitionWasm,
    ShieldFromAssetLockTransition,
    ShieldFromAssetLockTransitionObjectJs,
    ShieldFromAssetLockTransitionJSONJs
);

impl_wasm_type_info!(
    ShieldFromAssetLockTransitionWasm,
    ShieldFromAssetLockTransition
);
