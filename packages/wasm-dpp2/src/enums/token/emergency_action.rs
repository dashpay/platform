use dpp::tokens::emergency_action::TokenEmergencyAction;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "TokenEmergencyAction")]
#[allow(non_camel_case_types)]
#[derive(Default)]
pub enum TokenEmergencyActionWASM {
    #[default]
    Pause = 0,
    Resume = 1,
}

impl From<TokenEmergencyActionWASM> for TokenEmergencyAction {
    fn from(distribution_type: TokenEmergencyActionWASM) -> Self {
        match distribution_type {
            TokenEmergencyActionWASM::Pause => TokenEmergencyAction::Pause,
            TokenEmergencyActionWASM::Resume => TokenEmergencyAction::Resume,
        }
    }
}

impl From<TokenEmergencyAction> for TokenEmergencyActionWASM {
    fn from(distribution_type: TokenEmergencyAction) -> Self {
        match distribution_type {
            TokenEmergencyAction::Pause => TokenEmergencyActionWASM::Pause,
            TokenEmergencyAction::Resume => TokenEmergencyActionWASM::Resume,
        }
    }
}

impl TryFrom<JsValue> for TokenEmergencyActionWASM {
    type Error = JsValue;

    fn try_from(value: JsValue) -> Result<TokenEmergencyActionWASM, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "pause" => Ok(TokenEmergencyActionWASM::Pause),
                    "resume" => Ok(TokenEmergencyActionWASM::Resume),
                    _ => Err(JsValue::from("unknown distribution type")),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(TokenEmergencyActionWASM::Pause),
                    1 => Ok(TokenEmergencyActionWASM::Resume),
                    _ => Err(JsValue::from("unknown distribution type")),
                },
            },
        }
    }
}

impl From<TokenEmergencyActionWASM> for String {
    fn from(distribution_type: TokenEmergencyActionWASM) -> Self {
        match distribution_type {
            TokenEmergencyActionWASM::Pause => String::from("Pause"),
            TokenEmergencyActionWASM::Resume => String::from("Resume"),
        }
    }
}
