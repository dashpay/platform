use crate::core::core_script::CoreScriptWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::identity::transitions::pooling::PoolingWasm;
use crate::shielded::orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
use crate::utils::try_from_options;
use crate::utils::try_vec_to_fixed_bytes;
use crate::{impl_wasm_conversions_inner, impl_wasm_type_info};
use dpp::identity::core_script::CoreScript;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use dpp::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Options for constructing a ShieldedWithdrawalTransition.
 *
 * `pooling` accepts the `Pooling` enum, the lower-case name string
 * ("never" / "ifavailable" / "standard"), or the numeric value (0/1/2) — same
 * shape as IdentityCreditWithdrawalTransition.
 */
export interface ShieldedWithdrawalTransitionOptions {
    actions: SerializedOrchardAction[];
    unshieldingAmount: bigint;
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
    coreFeePerByte: number;
    pooling: CreditWithdrawalTransitionPoolingLike;
    outputScript: Uint8Array;
}

/**
 * ShieldedWithdrawalTransition serialized as a plain object.
 */
export interface ShieldedWithdrawalTransitionObject {
    $formatVersion: string;
    actions: SerializedOrchardActionObject[];
    unshieldingAmount: bigint;
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
    coreFeePerByte: number;
    pooling: PoolingWasm;
    outputScript: Uint8Array;
}

/**
 * ShieldedWithdrawalTransition serialized as JSON (human-readable).
 */
export interface ShieldedWithdrawalTransitionJSON {
    $formatVersion: string;
    actions: SerializedOrchardActionJSON[];
    unshieldingAmount: number | string;
    anchor: string;
    proof: string;
    bindingSignature: string;
    coreFeePerByte: number;
    pooling: string;
    outputScript: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ShieldedWithdrawalTransitionOptions")]
    pub type ShieldedWithdrawalTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "ShieldedWithdrawalTransitionObject")]
    pub type ShieldedWithdrawalTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "ShieldedWithdrawalTransitionJSON")]
    pub type ShieldedWithdrawalTransitionJSONJs;
}

/// Non-WASM-instance fields extracted from the constructor options via serde.
/// `pooling` is extracted separately via `try_from_options` so it accepts the
/// flexible `PoolingLikeJs` shape (enum / string / number).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShieldedWithdrawalTransitionSimpleFields {
    unshielding_amount: u64,
    anchor: Vec<u8>,
    proof: Vec<u8>,
    binding_signature: Vec<u8>,
    core_fee_per_byte: u32,
    output_script: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
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
    #[wasm_bindgen(constructor)]
    pub fn new(
        options: ShieldedWithdrawalTransitionOptionsJs,
    ) -> WasmDppResult<ShieldedWithdrawalTransitionWasm> {
        let actions = actions_from_js_options(options.as_ref(), "actions")?;
        let pooling: PoolingWasm = try_from_options(options.as_ref(), "pooling")?;

        let fields: ShieldedWithdrawalTransitionSimpleFields =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let anchor: [u8; 32] = try_vec_to_fixed_bytes(fields.anchor, "anchor")?;
        let binding_signature: [u8; 64] =
            try_vec_to_fixed_bytes(fields.binding_signature, "bindingSignature")?;

        Ok(ShieldedWithdrawalTransitionWasm(
            ShieldedWithdrawalTransition::V0(ShieldedWithdrawalTransitionV0 {
                actions: actions.into_iter().map(Into::into).collect(),
                unshielding_amount: fields.unshielding_amount,
                anchor,
                proof: fields.proof,
                binding_signature,
                core_fee_per_byte: fields.core_fee_per_byte,
                pooling: pooling.into(),
                output_script: CoreScript::from(fields.output_script),
            }),
        ))
    }

    /// Returns the serialized Orchard actions.
    #[wasm_bindgen(getter = "actions")]
    pub fn actions(&self) -> Vec<SerializedOrchardActionWasm> {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0
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
            ShieldedWithdrawalTransition::V0(v0) => v0.unshielding_amount,
        }
    }

    /// Returns the anchor (32-byte Merkle root).
    #[wasm_bindgen(getter = "anchor")]
    pub fn anchor(&self) -> Vec<u8> {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0.anchor.to_vec(),
        }
    }

    /// Returns the Halo2 proof bytes.
    #[wasm_bindgen(getter = "proof")]
    pub fn proof(&self) -> Vec<u8> {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0.proof.clone(),
        }
    }

    /// Returns the RedPallas binding signature (64 bytes).
    #[wasm_bindgen(getter = "bindingSignature")]
    pub fn binding_signature(&self) -> Vec<u8> {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0.binding_signature.to_vec(),
        }
    }

    /// Returns the core fee per byte.
    #[wasm_bindgen(getter = "coreFeePerByte")]
    pub fn core_fee_per_byte(&self) -> u32 {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0.core_fee_per_byte,
        }
    }

    /// Returns the pooling strategy as a name string ("never" / "ifavailable" / "standard").
    /// Matches the shape of `IdentityCreditWithdrawalTransition.pooling`.
    #[wasm_bindgen(getter = "pooling")]
    pub fn pooling(&self) -> String {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => PoolingWasm::from(v0.pooling).into(),
        }
    }

    /// Returns the output script (core address).
    #[wasm_bindgen(getter = "outputScript")]
    pub fn output_script(&self) -> CoreScriptWasm {
        match &self.0 {
            ShieldedWithdrawalTransition::V0(v0) => v0.output_script.clone().into(),
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

    #[wasm_bindgen(js_name = toStateTransition)]
    pub fn to_state_transition(&self) -> crate::state_transitions::base::StateTransitionWasm {
        StateTransition::ShieldedWithdrawal(self.0.clone()).into()
    }
}

impl_wasm_conversions_inner!(
    ShieldedWithdrawalTransitionWasm,
    ShieldedWithdrawalTransition,
    ShieldedWithdrawalTransition,
    ShieldedWithdrawalTransitionObjectJs,
    ShieldedWithdrawalTransitionJSONJs
);

impl_wasm_type_info!(
    ShieldedWithdrawalTransitionWasm,
    ShieldedWithdrawalTransition
);
