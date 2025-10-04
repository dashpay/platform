use dpp::identity::Purpose;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "Purpose")]
pub enum PurposeWasm {
    AUTHENTICATION = 0,
    ENCRYPTION = 1,
    DECRYPTION = 2,
    TRANSFER = 3,
    SYSTEM = 4,
    VOTING = 5,
    OWNER = 6,
}

impl TryFrom<JsValue> for PurposeWasm {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "authentication" => Ok(PurposeWasm::AUTHENTICATION),
                    "encryption" => Ok(PurposeWasm::ENCRYPTION),
                    "decryption" => Ok(PurposeWasm::DECRYPTION),
                    "transfer" => Ok(PurposeWasm::TRANSFER),
                    "system" => Ok(PurposeWasm::SYSTEM),
                    "voting" => Ok(PurposeWasm::VOTING),
                    "owner" => Ok(PurposeWasm::OWNER),
                    _ => Err(JsValue::from(format!(
                        "unsupported purpose value ({})",
                        enum_val
                    ))),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(PurposeWasm::AUTHENTICATION),
                    1 => Ok(PurposeWasm::ENCRYPTION),
                    2 => Ok(PurposeWasm::DECRYPTION),
                    3 => Ok(PurposeWasm::TRANSFER),
                    4 => Ok(PurposeWasm::SYSTEM),
                    5 => Ok(PurposeWasm::VOTING),
                    6 => Ok(PurposeWasm::OWNER),
                    _ => Err(JsValue::from(format!(
                        "unsupported purpose value ({})",
                        enum_val
                    ))),
                },
            },
        }
    }
}

impl From<PurposeWasm> for String {
    fn from(value: PurposeWasm) -> Self {
        match value {
            PurposeWasm::AUTHENTICATION => String::from("AUTHENTICATION"),
            PurposeWasm::ENCRYPTION => String::from("ENCRYPTION"),
            PurposeWasm::DECRYPTION => String::from("DECRYPTION"),
            PurposeWasm::TRANSFER => String::from("TRANSFER"),
            PurposeWasm::SYSTEM => String::from("SYSTEM"),
            PurposeWasm::VOTING => String::from("VOTING"),
            PurposeWasm::OWNER => String::from("OWNER"),
        }
    }
}

impl From<PurposeWasm> for Purpose {
    fn from(purpose: PurposeWasm) -> Self {
        match purpose {
            PurposeWasm::AUTHENTICATION => Purpose::AUTHENTICATION,
            PurposeWasm::ENCRYPTION => Purpose::ENCRYPTION,
            PurposeWasm::DECRYPTION => Purpose::DECRYPTION,
            PurposeWasm::TRANSFER => Purpose::TRANSFER,
            PurposeWasm::SYSTEM => Purpose::SYSTEM,
            PurposeWasm::VOTING => Purpose::VOTING,
            PurposeWasm::OWNER => Purpose::OWNER,
        }
    }
}

impl From<Purpose> for PurposeWasm {
    fn from(purpose: Purpose) -> Self {
        match purpose {
            Purpose::AUTHENTICATION => PurposeWasm::AUTHENTICATION,
            Purpose::ENCRYPTION => PurposeWasm::ENCRYPTION,
            Purpose::DECRYPTION => PurposeWasm::DECRYPTION,
            Purpose::TRANSFER => PurposeWasm::TRANSFER,
            Purpose::SYSTEM => PurposeWasm::SYSTEM,
            Purpose::VOTING => PurposeWasm::VOTING,
            Purpose::OWNER => PurposeWasm::OWNER,
        }
    }
}
