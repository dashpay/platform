use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "TokenDistributionType")]
#[allow(non_camel_case_types)]
#[derive(Default)]
pub enum TokenDistributionTypeWasm {
    #[default]
    PreProgrammed = 0,
    Perpetual = 1,
}

impl From<TokenDistributionTypeWasm> for TokenDistributionType {
    fn from(distribution_type: TokenDistributionTypeWasm) -> Self {
        match distribution_type {
            TokenDistributionTypeWasm::PreProgrammed => TokenDistributionType::PreProgrammed,
            TokenDistributionTypeWasm::Perpetual => TokenDistributionType::Perpetual,
        }
    }
}

impl From<TokenDistributionType> for TokenDistributionTypeWasm {
    fn from(distribution_type: TokenDistributionType) -> Self {
        match distribution_type {
            TokenDistributionType::Perpetual => TokenDistributionTypeWasm::Perpetual,
            TokenDistributionType::PreProgrammed => TokenDistributionTypeWasm::PreProgrammed,
        }
    }
}

impl TryFrom<JsValue> for TokenDistributionTypeWasm {
    type Error = JsValue;

    fn try_from(value: JsValue) -> Result<TokenDistributionTypeWasm, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "preprogrammed" => Ok(TokenDistributionTypeWasm::PreProgrammed),
                    "perpetual" => Ok(TokenDistributionTypeWasm::Perpetual),
                    _ => Err(JsValue::from("unknown distribution type")),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(TokenDistributionTypeWasm::PreProgrammed),
                    1 => Ok(TokenDistributionTypeWasm::Perpetual),
                    _ => Err(JsValue::from("unknown distribution type")),
                },
            },
        }
    }
}

impl From<TokenDistributionTypeWasm> for String {
    fn from(distribution_type: TokenDistributionTypeWasm) -> Self {
        match distribution_type {
            TokenDistributionTypeWasm::PreProgrammed => String::from("PreProgrammed"),
            TokenDistributionTypeWasm::Perpetual => String::from("Perpetual"),
        }
    }
}
