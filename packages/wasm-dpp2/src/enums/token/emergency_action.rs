use crate::error::WasmDppError;
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
    type Error = WasmDppError;

    fn try_from(value: JsValue) -> Result<TokenEmergencyActionWasm, Self::Error> {
        if let Some(enum_val) = value.as_string() {
            return match enum_val.to_lowercase().as_str() {
                "pause" => Ok(TokenEmergencyActionWasm::Pause),
                "resume" => Ok(TokenEmergencyActionWasm::Resume),
                _ => Err(WasmDppError::invalid_argument(
                    "unknown emergency action type",
                )),
            };
        }

        if let Some(enum_val) = value.as_f64() {
            return match enum_val as u8 {
                0 => Ok(TokenEmergencyActionWasm::Pause),
                1 => Ok(TokenEmergencyActionWasm::Resume),
                _ => Err(WasmDppError::invalid_argument(
                    "unknown emergency action type",
                )),
            };
        }

        Err(WasmDppError::invalid_argument(
            "cannot read value from emergency action enum",
        ))
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
