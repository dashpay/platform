use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::shielded::orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
use crate::utils::try_vec_to_fixed_bytes;
use crate::{impl_wasm_conversions_inner, impl_wasm_type_info};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Options for constructing a ShieldedTransferTransition.
 */
export interface ShieldedTransferTransitionOptions {
    actions: SerializedOrchardAction[];
    valueBalance: bigint;
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
}

/**
 * ShieldedTransferTransition serialized as a plain object.
 */
export interface ShieldedTransferTransitionObject {
    $formatVersion: string;
    actions: SerializedOrchardActionObject[];
    valueBalance: bigint;
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
}

/**
 * ShieldedTransferTransition serialized as JSON (human-readable).
 */
export interface ShieldedTransferTransitionJSON {
    $formatVersion: string;
    actions: SerializedOrchardActionJSON[];
    valueBalance: number | string;
    anchor: string;
    proof: string;
    bindingSignature: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ShieldedTransferTransitionOptions")]
    pub type ShieldedTransferTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "ShieldedTransferTransitionObject")]
    pub type ShieldedTransferTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "ShieldedTransferTransitionJSON")]
    pub type ShieldedTransferTransitionJSONJs;
}

/// Non-WASM-instance fields extracted from the constructor options via serde.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShieldedTransferTransitionSimpleFields {
    value_balance: u64,
    anchor: Vec<u8>,
    proof: Vec<u8>,
    binding_signature: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
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
    #[wasm_bindgen(constructor)]
    pub fn new(
        options: ShieldedTransferTransitionOptionsJs,
    ) -> WasmDppResult<ShieldedTransferTransitionWasm> {
        let actions = actions_from_js_options(options.as_ref(), "actions")?;

        let fields: ShieldedTransferTransitionSimpleFields =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let anchor: [u8; 32] = try_vec_to_fixed_bytes(fields.anchor, "anchor")?;
        let binding_signature: [u8; 64] =
            try_vec_to_fixed_bytes(fields.binding_signature, "bindingSignature")?;

        Ok(ShieldedTransferTransitionWasm(
            ShieldedTransferTransition::V0(ShieldedTransferTransitionV0 {
                actions: actions.into_iter().map(Into::into).collect(),
                value_balance: fields.value_balance,
                anchor,
                proof: fields.proof,
                binding_signature,
            }),
        ))
    }

    /// Returns the serialized Orchard actions.
    #[wasm_bindgen(getter = "actions")]
    pub fn actions(&self) -> Vec<SerializedOrchardActionWasm> {
        match &self.0 {
            ShieldedTransferTransition::V0(v0) => v0
                .actions
                .iter()
                .cloned()
                .map(SerializedOrchardActionWasm::from)
                .collect(),
        }
    }

    /// Returns the value balance (fee amount leaving the pool).
    #[wasm_bindgen(getter = "valueBalance")]
    pub fn value_balance(&self) -> u64 {
        match &self.0 {
            ShieldedTransferTransition::V0(v0) => v0.value_balance,
        }
    }

    /// Returns the anchor (32-byte Merkle root).
    #[wasm_bindgen(getter = "anchor")]
    pub fn anchor(&self) -> Vec<u8> {
        match &self.0 {
            ShieldedTransferTransition::V0(v0) => v0.anchor.to_vec(),
        }
    }

    /// Returns the Halo2 proof bytes.
    #[wasm_bindgen(getter = "proof")]
    pub fn proof(&self) -> Vec<u8> {
        match &self.0 {
            ShieldedTransferTransition::V0(v0) => v0.proof.clone(),
        }
    }

    /// Returns the RedPallas binding signature (64 bytes).
    #[wasm_bindgen(getter = "bindingSignature")]
    pub fn binding_signature(&self) -> Vec<u8> {
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

    #[wasm_bindgen(js_name = toStateTransition)]
    pub fn to_state_transition(&self) -> crate::state_transitions::base::StateTransitionWasm {
        StateTransition::ShieldedTransfer(self.0.clone()).into()
    }
}

impl_wasm_conversions_inner!(
    ShieldedTransferTransitionWasm,
    ShieldedTransferTransition,
    ShieldedTransferTransition,
    ShieldedTransferTransitionObjectJs,
    ShieldedTransferTransitionJSONJs
);

impl_wasm_type_info!(ShieldedTransferTransitionWasm, ShieldedTransferTransition);
