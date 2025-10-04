use dpp::tokens::emergency_action::TokenEmergencyAction;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "TokenEmergencyAction")]
#[allow(non_camel_case_types)]
#[derive(Default)]
pub enum TokenEmergencyActionWasm {
    #[default]
    Pause = 0,
    Resume = 1,
}

impl From<TokenEmergencyActionWasm> for TokenEmergencyAction {
    fn from(distribution_type: TokenEmergencyActionWasm) -> Self {
        match distribution_type {
            TokenEmergencyActionWasm::Pause => TokenEmergencyAction::Pause,
            TokenEmergencyActionWasm::Resume => TokenEmergencyAction::Resume,
        }
    }
}

impl From<TokenEmergencyAction> for TokenEmergencyActionWasm {
    fn from(distribution_type: TokenEmergencyAction) -> Self {
        match distribution_type {
            TokenEmergencyAction::Pause => TokenEmergencyActionWasm::Pause,
            TokenEmergencyAction::Resume => TokenEmergencyActionWasm::Resume,
        }
    }
}

impl TryFrom<JsValue> for TokenEmergencyActionWasm {
    type Error = JsValue;

    fn try_from(value: JsValue) -> Result<TokenEmergencyActionWasm, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "pause" => Ok(TokenEmergencyActionWasm::Pause),
                    "resume" => Ok(TokenEmergencyActionWasm::Resume),
                    _ => Err(JsValue::from("unknown distribution type")),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(TokenEmergencyActionWasm::Pause),
                    1 => Ok(TokenEmergencyActionWasm::Resume),
                    _ => Err(JsValue::from("unknown distribution type")),
                },
            },
        }
    }
}

impl From<TokenEmergencyActionWasm> for String {
    fn from(distribution_type: TokenEmergencyActionWasm) -> Self {
        match distribution_type {
            TokenEmergencyActionWasm::Pause => String::from("Pause"),
            TokenEmergencyActionWasm::Resume => String::from("Resume"),
        }
    }
}
