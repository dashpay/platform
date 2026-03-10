use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use crate::platform_address::{
    PlatformAddressInputWasm, PlatformAddressOutputWasm, fee_strategy_from_js_options,
    fee_strategy_from_steps_or_default, inputs_from_js_options, optional_output_from_js_options,
};
use crate::state_transitions::StateTransitionWasm;
use crate::utils::{try_from_options_optional_with, try_to_u16};
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::StateTransition;
use dpp::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use dpp::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
export interface IdentityTopUpFromAddressesTransitionOptions {
    identityId: IdentifierLike;
    inputs: PlatformAddressInput[];
    output?: PlatformAddressOutput;
    feeStrategy?: FeeStrategyStep[];
    userFeeIncrease?: number;
}

export interface IdentityTopUpFromAddressesTransitionObject {
    identityId: Uint8Array;
    inputs: PlatformAddressInputObject[];
    output?: PlatformAddressOutputObject;
    feeStrategy: FeeStrategyStepObject[];
    userFeeIncrease: number;
}

export interface IdentityTopUpFromAddressesTransitionJSON {
    identityId: string;
    inputs: object[];
    output?: object;
    feeStrategy: object[];
    userFeeIncrease: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityTopUpFromAddressesTransitionOptions")]
    pub type IdentityTopUpFromAddressesTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "IdentityTopUpFromAddressesTransitionObject")]
    pub type IdentityTopUpFromAddressesTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "IdentityTopUpFromAddressesTransitionJSON")]
    pub type IdentityTopUpFromAddressesTransitionJSONJs;
}

#[wasm_bindgen(js_name = "IdentityTopUpFromAddressesTransition")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct IdentityTopUpFromAddressesTransitionWasm(IdentityTopUpFromAddressesTransition);

#[wasm_bindgen(js_class = IdentityTopUpFromAddressesTransition)]
impl IdentityTopUpFromAddressesTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: IdentityTopUpFromAddressesTransitionOptionsJs,
    ) -> WasmDppResult<IdentityTopUpFromAddressesTransitionWasm> {
        let js_opts: &JsValue = options.as_ref();

        // Extract complex wasm-bindgen types manually
        let identity_id: IdentifierWasm = crate::utils::try_from_options(&options, "identityId")?;
        let inputs = inputs_from_js_options(js_opts, "inputs")?;
        let output = optional_output_from_js_options(js_opts, "output")?;

        // Extract simple fields
        let fee_strategy = fee_strategy_from_js_options(js_opts, "feeStrategy")?;
        let user_fee_increase: UserFeeIncrease =
            try_from_options_optional_with(js_opts, "userFeeIncrease", |v| {
                try_to_u16(v, "userFeeIncrease")
            })?
            .unwrap_or(0);

        let inputs_map = inputs.into_iter().map(|i| i.into_inner()).collect();
        let output = output.map(|o| o.try_into_inner()).transpose()?;
        let fee_strategy = fee_strategy_from_steps_or_default(fee_strategy);

        Ok(IdentityTopUpFromAddressesTransitionWasm(
            IdentityTopUpFromAddressesTransition::V0(IdentityTopUpFromAddressesTransitionV0 {
                identity_id: identity_id.into(),
                inputs: inputs_map,
                output,
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
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<IdentityTopUpFromAddressesTransitionWasm> {
        let rs_transition =
            IdentityTopUpFromAddressesTransition::deserialize_from_bytes(bytes.as_slice())?;
        Ok(IdentityTopUpFromAddressesTransitionWasm(rs_transition))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<IdentityTopUpFromAddressesTransitionWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Self::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<IdentityTopUpFromAddressesTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Self::from_bytes(bytes)
    }

    #[wasm_bindgen(getter = "identityId")]
    pub fn identity_id(&self) -> IdentifierWasm {
        match &self.0 {
            IdentityTopUpFromAddressesTransition::V0(v0) => v0.identity_id.into(),
        }
    }

    #[wasm_bindgen(setter = "identityId")]
    pub fn set_identity_id(
        &mut self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        let id: IdentifierWasm = identity_id.try_into()?;
        match &mut self.0 {
            IdentityTopUpFromAddressesTransition::V0(v0) => {
                v0.identity_id = id.into();
            }
        }
        Ok(())
    }

    #[wasm_bindgen(getter = "inputs")]
    pub fn inputs(&self) -> Vec<PlatformAddressInputWasm> {
        let inputs_map = match &self.0 {
            IdentityTopUpFromAddressesTransition::V0(v0) => &v0.inputs,
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
            IdentityTopUpFromAddressesTransition::V0(v0) => {
                v0.inputs = inputs_map;
            }
        }
    }

    #[wasm_bindgen(getter = "output")]
    pub fn output(&self) -> Option<PlatformAddressOutputWasm> {
        let output = match &self.0 {
            IdentityTopUpFromAddressesTransition::V0(v0) => &v0.output,
        };
        output.map(|(address, credits)| PlatformAddressOutputWasm::new(address, credits))
    }

    #[wasm_bindgen(setter = "output")]
    pub fn set_output(&mut self, output: Option<PlatformAddressOutputWasm>) -> WasmDppResult<()> {
        let new_output = output.map(|o| o.try_into_inner()).transpose()?;
        match &mut self.0 {
            IdentityTopUpFromAddressesTransition::V0(v0) => {
                v0.output = new_output;
            }
        }
        Ok(())
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> u16 {
        match &self.0 {
            IdentityTopUpFromAddressesTransition::V0(v0) => v0.user_fee_increase,
        }
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(
        &mut self,
        #[wasm_bindgen(js_name = "userFeeIncrease")] amount: &js_sys::Number,
    ) -> WasmDppResult<()> {
        match &mut self.0 {
            IdentityTopUpFromAddressesTransition::V0(v0) => {
                v0.user_fee_increase = try_to_u16(amount, "userFeeIncrease")?;
            }
        }
        Ok(())
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::IdentityTopUpFromAddresses(self.0.clone()))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<IdentityTopUpFromAddressesTransitionWasm> {
        let rs_st: StateTransition = st.clone().into();
        match rs_st {
            StateTransition::IdentityTopUpFromAddresses(st) => {
                Ok(IdentityTopUpFromAddressesTransitionWasm(st))
            }
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type",
            )),
        }
    }
}

impl_wasm_conversions!(
    IdentityTopUpFromAddressesTransitionWasm,
    IdentityTopUpFromAddressesTransition,
    IdentityTopUpFromAddressesTransitionObjectJs,
    IdentityTopUpFromAddressesTransitionJSONJs
);

impl_wasm_type_info!(
    IdentityTopUpFromAddressesTransitionWasm,
    IdentityTopUpFromAddressesTransition
);
