use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::platform_address::PlatformAddressWasm;
use crate::shielded::orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
use crate::utils::try_from_options;
use crate::utils::try_vec_to_fixed_bytes;
use crate::{impl_wasm_conversions_inner, impl_wasm_type_info};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::unshield_transition::UnshieldTransition;
use dpp::state_transition::unshield_transition::v0::UnshieldTransitionV0;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Options for constructing an UnshieldTransition.
 */
export interface UnshieldTransitionOptions {
    outputAddress: PlatformAddressLike;
    actions: SerializedOrchardAction[];
    unshieldingAmount: bigint;
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
}

/**
 * UnshieldTransition serialized as a plain object.
 *
 * `outputAddress` is the raw 21 bytes of a PlatformAddress (type byte + 20-byte hash);
 * the JSON form (below) carries the same value as a hex string.
 */
export interface UnshieldTransitionObject {
    $formatVersion: string;
    outputAddress: Uint8Array;
    actions: SerializedOrchardActionObject[];
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
    outputAddress: string;
    actions: SerializedOrchardActionJSON[];
    unshieldingAmount: number | string;
    anchor: string;
    proof: string;
    bindingSignature: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "UnshieldTransitionOptions")]
    pub type UnshieldTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "UnshieldTransitionObject")]
    pub type UnshieldTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "UnshieldTransitionJSON")]
    pub type UnshieldTransitionJSONJs;
}

/// Non-WASM-instance fields extracted from the constructor options via serde.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnshieldTransitionSimpleFields {
    unshielding_amount: u64,
    anchor: Vec<u8>,
    proof: Vec<u8>,
    binding_signature: Vec<u8>,
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
    pub fn new(options: UnshieldTransitionOptionsJs) -> WasmDppResult<UnshieldTransitionWasm> {
        let js_opts: &JsValue = options.as_ref();

        let output_address: PlatformAddressWasm = try_from_options(js_opts, "outputAddress")?;
        let actions = actions_from_js_options(js_opts, "actions")?;

        let fields: UnshieldTransitionSimpleFields = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let anchor: [u8; 32] = try_vec_to_fixed_bytes(fields.anchor, "anchor")?;
        let binding_signature: [u8; 64] =
            try_vec_to_fixed_bytes(fields.binding_signature, "bindingSignature")?;

        Ok(UnshieldTransitionWasm(UnshieldTransition::V0(
            UnshieldTransitionV0 {
                output_address: output_address.into(),
                actions: actions.into_iter().map(Into::into).collect(),
                unshielding_amount: fields.unshielding_amount,
                anchor,
                proof: fields.proof,
                binding_signature,
            },
        )))
    }

    /// Returns the output address receiving the unshielded funds.
    #[wasm_bindgen(getter = "outputAddress")]
    pub fn output_address(&self) -> PlatformAddressWasm {
        match &self.0 {
            UnshieldTransition::V0(v0) => PlatformAddressWasm::from(v0.output_address),
        }
    }

    /// Returns the serialized Orchard actions.
    #[wasm_bindgen(getter = "actions")]
    pub fn actions(&self) -> Vec<SerializedOrchardActionWasm> {
        match &self.0 {
            UnshieldTransition::V0(v0) => v0
                .actions
                .iter()
                .cloned()
                .map(SerializedOrchardActionWasm::from)
                .collect(),
        }
    }

    /// Returns the unshielding amount.
    #[wasm_bindgen(getter = "unshieldingAmount")]
    pub fn unshielding_amount(&self) -> u64 {
        match &self.0 {
            UnshieldTransition::V0(v0) => v0.unshielding_amount,
        }
    }

    /// Returns the anchor (32-byte Merkle root).
    #[wasm_bindgen(getter = "anchor")]
    pub fn anchor(&self) -> Vec<u8> {
        match &self.0 {
            UnshieldTransition::V0(v0) => v0.anchor.to_vec(),
        }
    }

    /// Returns the Halo2 proof bytes.
    #[wasm_bindgen(getter = "proof")]
    pub fn proof(&self) -> Vec<u8> {
        match &self.0 {
            UnshieldTransition::V0(v0) => v0.proof.clone(),
        }
    }

    /// Returns the RedPallas binding signature (64 bytes).
    #[wasm_bindgen(getter = "bindingSignature")]
    pub fn binding_signature(&self) -> Vec<u8> {
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

impl_wasm_conversions_inner!(
    UnshieldTransitionWasm,
    UnshieldTransition,
    UnshieldTransition,
    UnshieldTransitionObjectJs,
    UnshieldTransitionJSONJs
);

impl_wasm_type_info!(UnshieldTransitionWasm, UnshieldTransition);
