use crate::error::{WasmDppError, WasmDppResult};
use crate::identity::transitions::public_key_in_creation::IdentityPublicKeyInCreationWasm;
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use crate::platform_address::{
    PlatformAddressInputWasm, PlatformAddressOutputWasm, fee_strategy_from_js_options,
    fee_strategy_from_steps_or_default, inputs_from_js_options,
};
use crate::state_transitions::StateTransitionWasm;
use crate::utils::{
    try_from_options_optional, try_from_options_optional_with, try_from_options_with, try_to_array,
    try_to_u16,
};
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::StateTransition;
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
export interface IdentityCreateFromAddressesTransitionOptions {
    publicKeys: IdentityPublicKeyInCreation[];
    inputs: PlatformAddressInput[];
    output?: PlatformAddressOutput;
    feeStrategy?: FeeStrategyStep[];
    userFeeIncrease?: number;
}

export interface IdentityCreateFromAddressesTransitionObject {
    publicKeys: IdentityPublicKeyInCreationObject[];
    inputs: PlatformAddressInputObject[];
    output?: PlatformAddressOutputObject;
    feeStrategy: FeeStrategyStepObject[];
    userFeeIncrease: number;
}

export interface IdentityCreateFromAddressesTransitionJSON {
    publicKeys: object[];
    inputs: object[];
    output?: object;
    feeStrategy: object[];
    userFeeIncrease: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityCreateFromAddressesTransitionOptions")]
    pub type IdentityCreateFromAddressesTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "IdentityCreateFromAddressesTransitionObject")]
    pub type IdentityCreateFromAddressesTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "IdentityCreateFromAddressesTransitionJSON")]
    pub type IdentityCreateFromAddressesTransitionJSONJs;
}

#[wasm_bindgen(js_name = "IdentityCreateFromAddressesTransition")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct IdentityCreateFromAddressesTransitionWasm(IdentityCreateFromAddressesTransition);

#[wasm_bindgen(js_class = IdentityCreateFromAddressesTransition)]
impl IdentityCreateFromAddressesTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: IdentityCreateFromAddressesTransitionOptionsJs,
    ) -> WasmDppResult<IdentityCreateFromAddressesTransitionWasm> {
        let js_opts: &JsValue = options.as_ref();

        // Extract complex wasm-bindgen types manually
        let js_public_keys_array =
            try_from_options_with(&options, "publicKeys", |v| try_to_array(v, "publicKeys"))?;
        let public_keys: Vec<IdentityPublicKeyInCreationWasm> =
            IdentityPublicKeyInCreationWasm::vec_from_array(&js_public_keys_array)?;
        let inputs = inputs_from_js_options(js_opts, "inputs")?;
        let output: Option<PlatformAddressOutputWasm> =
            try_from_options_optional(js_opts, "output")?;

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

        Ok(IdentityCreateFromAddressesTransitionWasm(
            IdentityCreateFromAddressesTransition::V0(IdentityCreateFromAddressesTransitionV0 {
                public_keys: public_keys.iter().map(|k| k.clone().into()).collect(),
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
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<IdentityCreateFromAddressesTransitionWasm> {
        let rs_transition =
            IdentityCreateFromAddressesTransition::deserialize_from_bytes(bytes.as_slice())?;
        Ok(IdentityCreateFromAddressesTransitionWasm(rs_transition))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<IdentityCreateFromAddressesTransitionWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Self::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<IdentityCreateFromAddressesTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Self::from_bytes(bytes)
    }

    #[wasm_bindgen(getter = "publicKeys")]
    pub fn public_keys(&self) -> Vec<IdentityPublicKeyInCreationWasm> {
        match &self.0 {
            IdentityCreateFromAddressesTransition::V0(v0) => v0
                .public_keys
                .iter()
                .map(|key| IdentityPublicKeyInCreationWasm::from(key.clone()))
                .collect(),
        }
    }

    #[wasm_bindgen(setter = "publicKeys")]
    pub fn set_public_keys(
        &mut self,
        #[wasm_bindgen(js_name = "publicKeys")] public_keys: &js_sys::Array,
    ) -> WasmDppResult<()> {
        let keys: Vec<IdentityPublicKeyInCreationWasm> =
            IdentityPublicKeyInCreationWasm::vec_from_array(public_keys)?;
        match &mut self.0 {
            IdentityCreateFromAddressesTransition::V0(v0) => {
                v0.public_keys = keys
                    .iter()
                    .map(|k| IdentityPublicKeyInCreation::from(k.clone()))
                    .collect();
            }
        }
        Ok(())
    }

    #[wasm_bindgen(getter = "inputs")]
    pub fn inputs(&self) -> Vec<PlatformAddressInputWasm> {
        let inputs_map = match &self.0 {
            IdentityCreateFromAddressesTransition::V0(v0) => &v0.inputs,
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
            IdentityCreateFromAddressesTransition::V0(v0) => {
                v0.inputs = inputs_map;
            }
        }
    }

    #[wasm_bindgen(getter = "output")]
    pub fn output(&self) -> Option<PlatformAddressOutputWasm> {
        let output = match &self.0 {
            IdentityCreateFromAddressesTransition::V0(v0) => &v0.output,
        };
        output.map(|(address, credits)| PlatformAddressOutputWasm::new(address, credits))
    }

    #[wasm_bindgen(setter = "output")]
    pub fn set_output(&mut self, output: Option<PlatformAddressOutputWasm>) -> WasmDppResult<()> {
        let new_output = output.map(|o| o.try_into_inner()).transpose()?;
        match &mut self.0 {
            IdentityCreateFromAddressesTransition::V0(v0) => {
                v0.output = new_output;
            }
        }
        Ok(())
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> u16 {
        match &self.0 {
            IdentityCreateFromAddressesTransition::V0(v0) => v0.user_fee_increase,
        }
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(
        &mut self,
        #[wasm_bindgen(js_name = "userFeeIncrease")] amount: &js_sys::Number,
    ) -> WasmDppResult<()> {
        match &mut self.0 {
            IdentityCreateFromAddressesTransition::V0(v0) => {
                v0.user_fee_increase = try_to_u16(amount, "userFeeIncrease")?;
            }
        }
        Ok(())
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::IdentityCreateFromAddresses(self.0.clone()))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<IdentityCreateFromAddressesTransitionWasm> {
        let rs_st: StateTransition = st.clone().into();
        match rs_st {
            StateTransition::IdentityCreateFromAddresses(st) => {
                Ok(IdentityCreateFromAddressesTransitionWasm(st))
            }
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type",
            )),
        }
    }
}

impl_wasm_conversions!(
    IdentityCreateFromAddressesTransitionWasm,
    IdentityCreateFromAddressesTransition,
    IdentityCreateFromAddressesTransitionObjectJs,
    IdentityCreateFromAddressesTransitionJSONJs
);

impl_wasm_type_info!(
    IdentityCreateFromAddressesTransitionWasm,
    IdentityCreateFromAddressesTransition
);
