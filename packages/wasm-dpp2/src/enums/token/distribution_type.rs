use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "TokenDistributionType")]
#[allow(non_camel_case_types)]
#[derive(Default)]
pub enum TokenDistributionTypeWASM {
    #[default]
    PreProgrammed = 0,
    Perpetual = 1,
}

impl From<TokenDistributionTypeWASM> for TokenDistributionType {
    fn from(distribution_type: TokenDistributionTypeWASM) -> Self {
        match distribution_type {
            TokenDistributionTypeWASM::PreProgrammed => TokenDistributionType::PreProgrammed,
            TokenDistributionTypeWASM::Perpetual => TokenDistributionType::Perpetual,
        }
    }
}

impl From<TokenDistributionType> for TokenDistributionTypeWASM {
    fn from(distribution_type: TokenDistributionType) -> Self {
        match distribution_type {
            TokenDistributionType::Perpetual => TokenDistributionTypeWASM::Perpetual,
            TokenDistributionType::PreProgrammed => TokenDistributionTypeWASM::PreProgrammed,
        }
    }
}

impl TryFrom<JsValue> for TokenDistributionTypeWASM {
    type Error = JsValue;

    fn try_from(value: JsValue) -> Result<TokenDistributionTypeWASM, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "preprogrammed" => Ok(TokenDistributionTypeWASM::PreProgrammed),
                    "perpetual" => Ok(TokenDistributionTypeWASM::Perpetual),
                    _ => Err(JsValue::from("unknown distribution type")),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(TokenDistributionTypeWASM::PreProgrammed),
                    1 => Ok(TokenDistributionTypeWASM::Perpetual),
                    _ => Err(JsValue::from("unknown distribution type")),
                },
            },
        }
    }
}

impl From<TokenDistributionTypeWASM> for String {
    fn from(distribution_type: TokenDistributionTypeWASM) -> Self {
        match distribution_type {
            TokenDistributionTypeWASM::PreProgrammed => String::from("PreProgrammed"),
            TokenDistributionTypeWASM::Perpetual => String::from("Perpetual"),
        }
    }
}
