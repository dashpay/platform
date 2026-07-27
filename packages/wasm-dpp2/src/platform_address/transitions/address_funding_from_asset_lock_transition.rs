use crate::asset_lock_proof::AssetLockProofWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use crate::platform_address::{
    PlatformAddressInputWasm, PlatformAddressOutputWasm, fee_strategy_from_js_options,
    fee_strategy_from_steps_or_default, inputs_from_js_options, outputs_from_js_options,
};
use crate::state_transitions::StateTransitionWasm;
use crate::utils::{try_from_options, try_from_options_optional_with, try_to_u16};
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::StateTransition;
use dpp::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use dpp::state_transition::address_funding_from_asset_lock_transition::accessors::AddressFundingFromAssetLockTransitionAccessorsV0;
use dpp::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
export interface AddressFundingFromAssetLockTransitionOptions {
    assetLockProof: AssetLockProof;
    inputs: PlatformAddressInput[];
    outputs: PlatformAddressOutput[];
    feeStrategy?: FeeStrategyStep[];
    userFeeIncrease?: number;
}

export interface AddressFundingFromAssetLockTransitionObject {
    assetLockProof: AssetLockProofObject;
    inputs: PlatformAddressInputObject[];
    outputs: PlatformAddressOutputObject[];
    feeStrategy: FeeStrategyStepObject[];
    userFeeIncrease: number;
}

export interface AddressFundingFromAssetLockTransitionJSON {
    assetLockProof: AssetLockProofJSON;
    inputs: PlatformAddressInputJSON[];
    outputs: PlatformAddressOutputJSON[];
    feeStrategy: FeeStrategyStepJSON[];
    userFeeIncrease: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "AddressFundingFromAssetLockTransitionOptions")]
    pub type AddressFundingFromAssetLockTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "AddressFundingFromAssetLockTransitionObject")]
    pub type AddressFundingFromAssetLockTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "AddressFundingFromAssetLockTransitionJSON")]
    pub type AddressFundingFromAssetLockTransitionJSONJs;
}

#[wasm_bindgen(js_name = "AddressFundingFromAssetLockTransition")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct AddressFundingFromAssetLockTransitionWasm(AddressFundingFromAssetLockTransition);

#[wasm_bindgen(js_class = AddressFundingFromAssetLockTransition)]
impl AddressFundingFromAssetLockTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: AddressFundingFromAssetLockTransitionOptionsJs,
    ) -> WasmDppResult<AddressFundingFromAssetLockTransitionWasm> {
        let js_opts: &JsValue = options.as_ref();

        // Extract complex wasm-bindgen types manually
        let asset_lock: AssetLockProofWasm = try_from_options(&options, "assetLockProof")?;
        let inputs = inputs_from_js_options(js_opts, "inputs")?;
        let outputs = outputs_from_js_options(js_opts, "outputs")?;

        // Extract simple fields
        let fee_strategy = fee_strategy_from_js_options(js_opts, "feeStrategy")?;
        let user_fee_increase: UserFeeIncrease =
            try_from_options_optional_with(js_opts, "userFeeIncrease", |v| {
                try_to_u16(v, "userFeeIncrease")
            })?
            .unwrap_or(0);

        let inputs_map = crate::platform_address::inputs_to_btree_map(inputs)?;
        let outputs = crate::platform_address::outputs_to_optional_btree_map(outputs)?;
        let fee_strategy = fee_strategy_from_steps_or_default(fee_strategy);

        Ok(AddressFundingFromAssetLockTransitionWasm(
            AddressFundingFromAssetLockTransition::V0(AddressFundingFromAssetLockTransitionV0 {
                asset_lock_proof: asset_lock.into(),
                inputs: inputs_map,
                outputs,
                fee_strategy,
                user_fee_increase,
                signature: Default::default(),
                input_witnesses: Default::default(),
            }),
        ))
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(self.0.serialize_to_bytes()?)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Hex))
    }

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Base64))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<AddressFundingFromAssetLockTransitionWasm> {
        let rs_transition =
            AddressFundingFromAssetLockTransition::deserialize_from_bytes(bytes.as_slice())?;
        Ok(AddressFundingFromAssetLockTransitionWasm(rs_transition))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<AddressFundingFromAssetLockTransitionWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Self::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<AddressFundingFromAssetLockTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Self::from_bytes(bytes)
    }

    #[wasm_bindgen(getter = "assetLockProof")]
    pub fn asset_lock_proof(&self) -> AssetLockProofWasm {
        AssetLockProofWasm::from(self.0.asset_lock_proof().clone())
    }

    #[wasm_bindgen(setter = "assetLockProof")]
    pub fn set_asset_lock_proof(&mut self, proof: AssetLockProofWasm) {
        self.0.set_asset_lock_proof(proof.into());
    }

    #[wasm_bindgen(getter = "inputs")]
    pub fn inputs(&self) -> Vec<PlatformAddressInputWasm> {
        let inputs_map = match &self.0 {
            AddressFundingFromAssetLockTransition::V0(v0) => &v0.inputs,
        };
        inputs_map
            .iter()
            .map(|(address, (nonce, amount))| {
                PlatformAddressInputWasm::new(*address, *nonce, *amount)
            })
            .collect()
    }

    #[wasm_bindgen(setter = "inputs")]
    pub fn set_inputs(&mut self, inputs: Vec<PlatformAddressInputWasm>) -> WasmDppResult<()> {
        let inputs_map = crate::platform_address::inputs_to_btree_map(inputs)?;
        match &mut self.0 {
            AddressFundingFromAssetLockTransition::V0(v0) => {
                v0.inputs = inputs_map;
            }
        }
        Ok(())
    }

    #[wasm_bindgen(getter = "outputs")]
    pub fn outputs(&self) -> Vec<PlatformAddressOutputWasm> {
        self.0
            .outputs()
            .iter()
            .map(|(address, amount)| PlatformAddressOutputWasm::new_optional(*address, *amount))
            .collect()
    }

    #[wasm_bindgen(setter = "outputs")]
    pub fn set_outputs(&mut self, outputs: Vec<PlatformAddressOutputWasm>) -> WasmDppResult<()> {
        let outputs_map = crate::platform_address::outputs_to_optional_btree_map(outputs)?;
        self.0.set_outputs(outputs_map);
        Ok(())
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> u16 {
        match &self.0 {
            AddressFundingFromAssetLockTransition::V0(v0) => v0.user_fee_increase,
        }
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(
        &mut self,
        #[wasm_bindgen(js_name = "userFeeIncrease")] amount: &js_sys::Number,
    ) -> WasmDppResult<()> {
        match &mut self.0 {
            AddressFundingFromAssetLockTransition::V0(v0) => {
                v0.user_fee_increase = try_to_u16(amount, "userFeeIncrease")?;
            }
        }
        Ok(())
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::AddressFundingFromAssetLock(self.0.clone()))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<AddressFundingFromAssetLockTransitionWasm> {
        let rs_st: StateTransition = st.clone().into();
        match rs_st {
            StateTransition::AddressFundingFromAssetLock(st) => {
                Ok(AddressFundingFromAssetLockTransitionWasm(st))
            }
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type",
            )),
        }
    }
}

impl_wasm_conversions_inner!(
    AddressFundingFromAssetLockTransitionWasm,
    AddressFundingFromAssetLockTransition,
    AddressFundingFromAssetLockTransition,
    AddressFundingFromAssetLockTransitionObjectJs,
    AddressFundingFromAssetLockTransitionJSONJs
);

impl_wasm_type_info!(
    AddressFundingFromAssetLockTransitionWasm,
    AddressFundingFromAssetLockTransition
);
