use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::platform_address::{
    FeeStrategyStepWasm, PlatformAddressInputWasm, fee_strategy_from_js_options,
    fee_strategy_from_steps_or_default, inputs_from_js_options,
};
use crate::shielded::address_witness::{AddressWitnessWasm, input_witnesses_from_js_options};
use crate::shielded::orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
use crate::utils::try_vec_to_fixed_bytes;
use crate::utils::{try_from_options_optional_with, try_to_u16};
use crate::{impl_wasm_conversions_inner, impl_wasm_type_info};
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::shield_transition::ShieldTransition;
use dpp::state_transition::shield_transition::v0::ShieldTransitionV0;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Options for constructing a ShieldTransition.
 * Uses WASM instance types for complex fields.
 */
export interface ShieldTransitionOptions {
    inputs: PlatformAddressInput[];
    actions: SerializedOrchardAction[];
    amount: bigint;
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
    feeStrategy?: FeeStrategyStep[];
    userFeeIncrease?: number;
    inputWitnesses: AddressWitness[];
}

/**
 * ShieldTransition serialized as a plain object.
 */
export interface ShieldTransitionObject {
    $formatVersion: string;
    inputs: PlatformAddressInputObject[];
    actions: SerializedOrchardActionObject[];
    amount: bigint;
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
    feeStrategy: FeeStrategyStepObject[];
    userFeeIncrease: number;
    inputWitnesses: AddressWitnessObject[];
}

/**
 * ShieldTransition serialized as JSON (human-readable).
 */
export interface ShieldTransitionJSON {
    $formatVersion: string;
    inputs: PlatformAddressInputJSON[];
    actions: SerializedOrchardActionJSON[];
    amount: number | string;
    anchor: string;
    proof: string;
    bindingSignature: string;
    feeStrategy: FeeStrategyStepJSON[];
    userFeeIncrease: number;
    inputWitnesses: AddressWitnessJSON[];
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ShieldTransitionOptions")]
    pub type ShieldTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "ShieldTransitionObject")]
    pub type ShieldTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "ShieldTransitionJSON")]
    pub type ShieldTransitionJSONJs;
}

/// Non-WASM-instance fields extracted from the constructor options via serde.
///
/// The complex fields (`inputs`, `actions`, `feeStrategy`, `inputWitnesses`) are
/// extracted separately as WASM class instances.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShieldTransitionSimpleFields {
    amount: u64,
    anchor: Vec<u8>,
    proof: Vec<u8>,
    binding_signature: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
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
    #[wasm_bindgen(constructor)]
    pub fn new(options: ShieldTransitionOptionsJs) -> WasmDppResult<ShieldTransitionWasm> {
        let js_opts: &JsValue = options.as_ref();

        // Extract WASM class instances (borrow &options)
        let inputs = inputs_from_js_options(js_opts, "inputs")?;
        let actions = actions_from_js_options(js_opts, "actions")?;
        let input_witnesses = input_witnesses_from_js_options(js_opts, "inputWitnesses")?;
        let fee_strategy = fee_strategy_from_js_options(js_opts, "feeStrategy")?;
        let user_fee_increase: UserFeeIncrease =
            try_from_options_optional_with(js_opts, "userFeeIncrease", |v| {
                try_to_u16(v, "userFeeIncrease")
            })?
            .unwrap_or(0);

        // Extract simple fields via serde (consumes options)
        let fields: ShieldTransitionSimpleFields =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let anchor: [u8; 32] = try_vec_to_fixed_bytes(fields.anchor, "anchor")?;
        let binding_signature: [u8; 64] =
            try_vec_to_fixed_bytes(fields.binding_signature, "bindingSignature")?;

        let inputs_map = crate::platform_address::inputs_to_btree_map(inputs)?;
        let fee_strategy = fee_strategy_from_steps_or_default(fee_strategy);

        Ok(ShieldTransitionWasm(ShieldTransition::V0(
            ShieldTransitionV0 {
                inputs: inputs_map,
                actions: actions.into_iter().map(Into::into).collect(),
                amount: fields.amount,
                anchor,
                proof: fields.proof,
                binding_signature,
                fee_strategy,
                user_fee_increase,
                input_witnesses: input_witnesses.into_iter().map(Into::into).collect(),
            },
        )))
    }

    /// Returns the input addresses funding the shield (with their nonces and amounts).
    #[wasm_bindgen(getter = "inputs")]
    pub fn inputs(&self) -> Vec<PlatformAddressInputWasm> {
        match &self.0 {
            ShieldTransition::V0(v0) => v0
                .inputs
                .iter()
                .map(|(address, (nonce, amount))| {
                    PlatformAddressInputWasm::new(*address, *nonce, *amount)
                })
                .collect(),
        }
    }

    /// Returns the serialized Orchard actions.
    #[wasm_bindgen(getter = "actions")]
    pub fn actions(&self) -> Vec<SerializedOrchardActionWasm> {
        match &self.0 {
            ShieldTransition::V0(v0) => v0
                .actions
                .iter()
                .cloned()
                .map(SerializedOrchardActionWasm::from)
                .collect(),
        }
    }

    /// Returns the shield amount (credits entering the pool).
    #[wasm_bindgen(getter = "amount")]
    pub fn amount(&self) -> u64 {
        match &self.0 {
            ShieldTransition::V0(v0) => v0.amount,
        }
    }

    /// Returns the anchor (32-byte Merkle root).
    #[wasm_bindgen(getter = "anchor")]
    pub fn anchor(&self) -> Vec<u8> {
        match &self.0 {
            ShieldTransition::V0(v0) => v0.anchor.to_vec(),
        }
    }

    /// Returns the Halo2 proof bytes.
    #[wasm_bindgen(getter = "proof")]
    pub fn proof(&self) -> Vec<u8> {
        match &self.0 {
            ShieldTransition::V0(v0) => v0.proof.clone(),
        }
    }

    /// Returns the RedPallas binding signature (64 bytes).
    #[wasm_bindgen(getter = "bindingSignature")]
    pub fn binding_signature(&self) -> Vec<u8> {
        match &self.0 {
            ShieldTransition::V0(v0) => v0.binding_signature.to_vec(),
        }
    }

    /// Returns the fee strategy steps.
    #[wasm_bindgen(getter = "feeStrategy")]
    pub fn fee_strategy(&self) -> Vec<FeeStrategyStepWasm> {
        match &self.0 {
            ShieldTransition::V0(v0) => v0
                .fee_strategy
                .iter()
                .cloned()
                .map(FeeStrategyStepWasm::from)
                .collect(),
        }
    }

    /// Returns the user fee increase multiplier.
    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> u16 {
        match &self.0 {
            ShieldTransition::V0(v0) => v0.user_fee_increase,
        }
    }

    /// Returns the input witnesses (signatures authorising each input).
    #[wasm_bindgen(getter = "inputWitnesses")]
    pub fn input_witnesses(&self) -> Vec<AddressWitnessWasm> {
        match &self.0 {
            ShieldTransition::V0(v0) => v0
                .input_witnesses
                .iter()
                .cloned()
                .map(AddressWitnessWasm::from)
                .collect(),
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

    #[wasm_bindgen(js_name = toStateTransition)]
    pub fn to_state_transition(&self) -> crate::state_transitions::base::StateTransitionWasm {
        StateTransition::Shield(self.0.clone()).into()
    }
}

impl_wasm_conversions_inner!(
    ShieldTransitionWasm,
    ShieldTransition,
    ShieldTransition,
    ShieldTransitionObjectJs,
    ShieldTransitionJSONJs
);

impl_wasm_type_info!(ShieldTransitionWasm, ShieldTransition);
