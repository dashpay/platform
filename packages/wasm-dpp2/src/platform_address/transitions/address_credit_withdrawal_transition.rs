use crate::core::core_script::CoreScriptWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identity::transitions::pooling::{PoolingLikeJs, PoolingWasm};
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use crate::platform_address::{
    PlatformAddressInputWasm, PlatformAddressOutputWasm, fee_strategy_from_js_options,
    fee_strategy_from_steps_or_default, inputs_from_js_options, optional_output_from_js_options,
};
use crate::state_transitions::StateTransitionWasm;
use crate::utils::{
    try_from_options, try_from_options_optional_with, try_from_options_with, try_to_u16, try_to_u32,
};
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::StateTransition;
use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use dpp::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
export interface AddressCreditWithdrawalTransitionOptions {
    inputs: PlatformAddressInput[];
    output?: PlatformAddressOutput;
    outputScript: CoreScript;
    pooling: CreditWithdrawalTransitionPoolingLike;
    coreFeePerByte: number;
    feeStrategy?: FeeStrategyStep[];
    userFeeIncrease?: number;
}

export interface AddressCreditWithdrawalTransitionObject {
    inputs: PlatformAddressInputObject[];
    output?: PlatformAddressOutputObject;
    outputScript: Uint8Array;
    pooling: number;
    coreFeePerByte: number;
    feeStrategy: FeeStrategyStepObject[];
    userFeeIncrease: number;
}

export interface AddressCreditWithdrawalTransitionJSON {
    inputs: object[];
    output?: object;
    outputScript: string;
    pooling: string;
    coreFeePerByte: number;
    feeStrategy: object[];
    userFeeIncrease: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "AddressCreditWithdrawalTransitionOptions")]
    pub type AddressCreditWithdrawalTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "AddressCreditWithdrawalTransitionObject")]
    pub type AddressCreditWithdrawalTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "AddressCreditWithdrawalTransitionJSON")]
    pub type AddressCreditWithdrawalTransitionJSONJs;
}

#[wasm_bindgen(js_name = "AddressCreditWithdrawalTransition")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct AddressCreditWithdrawalTransitionWasm(AddressCreditWithdrawalTransition);

#[wasm_bindgen(js_class = AddressCreditWithdrawalTransition)]
impl AddressCreditWithdrawalTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: AddressCreditWithdrawalTransitionOptionsJs,
    ) -> WasmDppResult<AddressCreditWithdrawalTransitionWasm> {
        let js_opts: &JsValue = options.as_ref();

        // Extract complex types manually (wasm-bindgen objects can't go through serde)
        let output_script: CoreScriptWasm = try_from_options(&options, "outputScript")?;
        let pooling: PoolingWasm = PoolingWasm::try_from_options(&options, "pooling")?;
        let inputs = inputs_from_js_options(js_opts, "inputs")?;
        let output = optional_output_from_js_options(js_opts, "output")?;

        // Extract simple fields
        let fee_strategy = fee_strategy_from_js_options(js_opts, "feeStrategy")?;
        let core_fee_per_byte: u32 = try_from_options_with(js_opts, "coreFeePerByte", |v| {
            try_to_u32(v, "coreFeePerByte")
        })?;
        let user_fee_increase: UserFeeIncrease =
            try_from_options_optional_with(js_opts, "userFeeIncrease", |v| {
                try_to_u16(v, "userFeeIncrease")
            })?
            .unwrap_or(0);

        let inputs_map = inputs.into_iter().map(|i| i.into_inner()).collect();
        let output = output.map(|o| o.into_inner());
        let fee_strategy = fee_strategy_from_steps_or_default(fee_strategy);

        Ok(AddressCreditWithdrawalTransitionWasm(
            AddressCreditWithdrawalTransition::V0(AddressCreditWithdrawalTransitionV0 {
                inputs: inputs_map,
                output,
                fee_strategy,
                core_fee_per_byte,
                pooling: pooling.into(),
                output_script: output_script.into(),
                user_fee_increase,
                input_witnesses: Vec::new(),
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
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<AddressCreditWithdrawalTransitionWasm> {
        let rs_transition =
            AddressCreditWithdrawalTransition::deserialize_from_bytes(bytes.as_slice())?;
        Ok(AddressCreditWithdrawalTransitionWasm(rs_transition))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<AddressCreditWithdrawalTransitionWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Self::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<AddressCreditWithdrawalTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Self::from_bytes(bytes)
    }

    #[wasm_bindgen(getter = "inputs")]
    pub fn inputs(&self) -> Vec<PlatformAddressInputWasm> {
        let inputs_map = match &self.0 {
            AddressCreditWithdrawalTransition::V0(v0) => &v0.inputs,
        };
        inputs_map
            .iter()
            .map(|(address, (nonce, amount))| {
                PlatformAddressInputWasm::new(*address, *nonce, *amount)
            })
            .collect()
    }

    #[wasm_bindgen(setter = "inputs")]
    pub fn set_inputs(&mut self, inputs: Vec<PlatformAddressInputWasm>) {
        let inputs_map = inputs.into_iter().map(|i| i.into_inner()).collect();
        match &mut self.0 {
            AddressCreditWithdrawalTransition::V0(v0) => {
                v0.inputs = inputs_map;
            }
        }
    }

    #[wasm_bindgen(getter = "output")]
    pub fn output(&self) -> Option<PlatformAddressOutputWasm> {
        let output = match &self.0 {
            AddressCreditWithdrawalTransition::V0(v0) => &v0.output,
        };
        output.map(|(address, credits)| PlatformAddressOutputWasm::new(address, credits))
    }

    #[wasm_bindgen(setter = "output")]
    pub fn set_output(
        &mut self,
        output: Option<PlatformAddressOutputWasm>,
    ) -> WasmDppResult<()> {
        let new_output = output.map(|o| o.try_into_inner()).transpose()?;
        match &mut self.0 {
            AddressCreditWithdrawalTransition::V0(v0) => {
                v0.output = new_output;
            }
        }
        Ok(())
    }

    #[wasm_bindgen(getter = "outputScript")]
    pub fn output_script(&self) -> CoreScriptWasm {
        match &self.0 {
            AddressCreditWithdrawalTransition::V0(v0) => v0.output_script.clone().into(),
        }
    }

    #[wasm_bindgen(setter = "outputScript")]
    pub fn set_output_script(&mut self, script: &CoreScriptWasm) {
        match &mut self.0 {
            AddressCreditWithdrawalTransition::V0(v0) => {
                v0.output_script = script.clone().into();
            }
        }
    }

    #[wasm_bindgen(getter = "pooling")]
    pub fn pooling(&self) -> String {
        match &self.0 {
            AddressCreditWithdrawalTransition::V0(v0) => PoolingWasm::from(v0.pooling).into(),
        }
    }

    #[wasm_bindgen(setter = "pooling")]
    pub fn set_pooling(&mut self, pooling: PoolingLikeJs) -> WasmDppResult<()> {
        let pooling: dpp::withdrawal::Pooling = pooling.try_into()?;
        match &mut self.0 {
            AddressCreditWithdrawalTransition::V0(v0) => {
                v0.pooling = pooling;
            }
        }
        Ok(())
    }

    #[wasm_bindgen(getter = "coreFeePerByte")]
    pub fn core_fee_per_byte(&self) -> u32 {
        match &self.0 {
            AddressCreditWithdrawalTransition::V0(v0) => v0.core_fee_per_byte,
        }
    }

    #[wasm_bindgen(setter = "coreFeePerByte")]
    pub fn set_core_fee_per_byte(
        &mut self,
        #[wasm_bindgen(js_name = "coreFeePerByte")] core_fee_per_byte: &js_sys::Number,
    ) -> WasmDppResult<()> {
        match &mut self.0 {
            AddressCreditWithdrawalTransition::V0(v0) => {
                v0.core_fee_per_byte = try_to_u32(core_fee_per_byte, "coreFeePerByte")?;
            }
        }
        Ok(())
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> u16 {
        match &self.0 {
            AddressCreditWithdrawalTransition::V0(v0) => v0.user_fee_increase,
        }
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(
        &mut self,
        #[wasm_bindgen(js_name = "userFeeIncrease")] amount: &js_sys::Number,
    ) -> WasmDppResult<()> {
        match &mut self.0 {
            AddressCreditWithdrawalTransition::V0(v0) => {
                v0.user_fee_increase = try_to_u16(amount, "userFeeIncrease")?;
            }
        }
        Ok(())
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::AddressCreditWithdrawal(self.0.clone()))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<AddressCreditWithdrawalTransitionWasm> {
        let rs_st: StateTransition = st.clone().into();
        match rs_st {
            StateTransition::AddressCreditWithdrawal(st) => {
                Ok(AddressCreditWithdrawalTransitionWasm(st))
            }
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type",
            )),
        }
    }
}

impl_wasm_conversions!(
    AddressCreditWithdrawalTransitionWasm,
    AddressCreditWithdrawalTransition,
    AddressCreditWithdrawalTransitionObjectJs,
    AddressCreditWithdrawalTransitionJSONJs
);

impl_wasm_type_info!(
    AddressCreditWithdrawalTransitionWasm,
    AddressCreditWithdrawalTransition
);
