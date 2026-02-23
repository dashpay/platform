use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use crate::platform_address::{
    PlatformAddressInputWasm, PlatformAddressOutputWasm, fee_strategy_from_js_options,
    fee_strategy_from_steps_or_default, inputs_from_js_options, outputs_from_js_options,
    outputs_to_btree_map,
};
use crate::state_transitions::StateTransitionWasm;
use crate::utils::{try_from_options_optional_with, try_to_u16};
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::state_transition::StateTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
export interface AddressFundsTransferTransitionOptions {
    inputs: PlatformAddressInput[];
    outputs: PlatformAddressOutput[];
    feeStrategy?: FeeStrategyStep[];
    userFeeIncrease?: number;
}

export interface AddressFundsTransferTransitionObject {
    inputs: PlatformAddressInputObject[];
    outputs: PlatformAddressOutputObject[];
    feeStrategy: FeeStrategyStepObject[];
    userFeeIncrease: number;
}

export interface AddressFundsTransferTransitionJSON {
    inputs: object[];
    outputs: object[];
    feeStrategy: object[];
    userFeeIncrease: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "AddressFundsTransferTransitionOptions")]
    pub type AddressFundsTransferTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "AddressFundsTransferTransitionObject")]
    pub type AddressFundsTransferTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "AddressFundsTransferTransitionJSON")]
    pub type AddressFundsTransferTransitionJSONJs;
}

#[wasm_bindgen(js_name = "AddressFundsTransferTransition")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct AddressFundsTransferTransitionWasm(AddressFundsTransferTransition);

#[wasm_bindgen(js_class = AddressFundsTransferTransition)]
impl AddressFundsTransferTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: AddressFundsTransferTransitionOptionsJs,
    ) -> WasmDppResult<AddressFundsTransferTransitionWasm> {
        let js_opts: &JsValue = options.as_ref();

        // Extract wasm-bindgen objects manually (can't go through serde)
        let inputs = inputs_from_js_options(js_opts, "inputs")?;
        let outputs = outputs_from_js_options(js_opts, "outputs")?;

        // Extract simple fields via serde for the remaining options
        let fee_strategy = fee_strategy_from_js_options(js_opts, "feeStrategy")?;
        let user_fee_increase: UserFeeIncrease =
            try_from_options_optional_with(js_opts, "userFeeIncrease", |v| {
                try_to_u16(v, "userFeeIncrease")
            })?
            .unwrap_or(0);

        let inputs_map = inputs.into_iter().map(|i| i.into_inner()).collect();
        let outputs_map = outputs_to_btree_map(outputs);
        let fee_strategy = fee_strategy_from_steps_or_default(fee_strategy);

        Ok(AddressFundsTransferTransitionWasm(
            AddressFundsTransferTransition::V0(AddressFundsTransferTransitionV0 {
                inputs: inputs_map,
                outputs: outputs_map,
                fee_strategy,
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
    pub fn from_bytes(
        bytes: Vec<u8>,
    ) -> WasmDppResult<AddressFundsTransferTransitionWasm> {
        let rs_transition =
            AddressFundsTransferTransition::deserialize_from_bytes(bytes.as_slice())?;
        Ok(AddressFundsTransferTransitionWasm(rs_transition))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<AddressFundsTransferTransitionWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Self::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<AddressFundsTransferTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Self::from_bytes(bytes)
    }

    #[wasm_bindgen(getter = "inputs")]
    pub fn inputs(&self) -> Vec<PlatformAddressInputWasm> {
        let inputs_map = match &self.0 {
            AddressFundsTransferTransition::V0(v0) => &v0.inputs,
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
            AddressFundsTransferTransition::V0(v0) => {
                v0.inputs = inputs_map;
            }
        }
    }

    #[wasm_bindgen(getter = "outputs")]
    pub fn outputs(&self) -> Vec<PlatformAddressOutputWasm> {
        let outputs_map = match &self.0 {
            AddressFundsTransferTransition::V0(v0) => &v0.outputs,
        };
        outputs_map
            .iter()
            .map(|(address, amount)| PlatformAddressOutputWasm::new(*address, *amount))
            .collect()
    }

    #[wasm_bindgen(setter = "outputs")]
    pub fn set_outputs(&mut self, outputs: Vec<PlatformAddressOutputWasm>) {
        let outputs_map = outputs_to_btree_map(outputs);
        match &mut self.0 {
            AddressFundsTransferTransition::V0(v0) => {
                v0.outputs = outputs_map;
            }
        }
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> u16 {
        match &self.0 {
            AddressFundsTransferTransition::V0(v0) => v0.user_fee_increase,
        }
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(
        &mut self,
        #[wasm_bindgen(js_name = "userFeeIncrease")] amount: &js_sys::Number,
    ) -> WasmDppResult<()> {
        match &mut self.0 {
            AddressFundsTransferTransition::V0(v0) => {
                v0.user_fee_increase = try_to_u16(amount, "userFeeIncrease")?;
            }
        }
        Ok(())
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::from(self.0.clone()))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<AddressFundsTransferTransitionWasm> {
        let rs_st: StateTransition = st.clone().into();
        match rs_st {
            StateTransition::AddressFundsTransfer(st) => {
                Ok(AddressFundsTransferTransitionWasm(st))
            }
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type",
            )),
        }
    }
}

impl_wasm_conversions!(
    AddressFundsTransferTransitionWasm,
    AddressFundsTransferTransition,
    AddressFundsTransferTransitionObjectJs,
    AddressFundsTransferTransitionJSONJs
);

impl_wasm_type_info!(
    AddressFundsTransferTransitionWasm,
    AddressFundsTransferTransition
);
