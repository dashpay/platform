use dpp::identity::Purpose;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "Purpose")]
pub enum PurposeWASM {
    AUTHENTICATION = 0,
    ENCRYPTION = 1,
    DECRYPTION = 2,
    TRANSFER = 3,
    SYSTEM = 4,
    VOTING = 5,
    OWNER = 6,
}

impl TryFrom<JsValue> for PurposeWASM {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "authentication" => Ok(PurposeWASM::AUTHENTICATION),
                    "encryption" => Ok(PurposeWASM::ENCRYPTION),
                    "decryption" => Ok(PurposeWASM::DECRYPTION),
                    "transfer" => Ok(PurposeWASM::TRANSFER),
                    "system" => Ok(PurposeWASM::SYSTEM),
                    "voting" => Ok(PurposeWASM::VOTING),
                    "owner" => Ok(PurposeWASM::OWNER),
                    _ => Err(JsValue::from(format!(
                        "unsupported purpose value ({})",
                        enum_val
                    ))),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(PurposeWASM::AUTHENTICATION),
                    1 => Ok(PurposeWASM::ENCRYPTION),
                    2 => Ok(PurposeWASM::DECRYPTION),
                    3 => Ok(PurposeWASM::TRANSFER),
                    4 => Ok(PurposeWASM::SYSTEM),
                    5 => Ok(PurposeWASM::VOTING),
                    6 => Ok(PurposeWASM::OWNER),
                    _ => Err(JsValue::from(format!(
                        "unsupported purpose value ({})",
                        enum_val
                    ))),
                },
            },
        }
    }
}

impl From<PurposeWASM> for String {
    fn from(value: PurposeWASM) -> Self {
        match value {
            PurposeWASM::AUTHENTICATION => String::from("AUTHENTICATION"),
            PurposeWASM::ENCRYPTION => String::from("ENCRYPTION"),
            PurposeWASM::DECRYPTION => String::from("DECRYPTION"),
            PurposeWASM::TRANSFER => String::from("TRANSFER"),
            PurposeWASM::SYSTEM => String::from("SYSTEM"),
            PurposeWASM::VOTING => String::from("VOTING"),
            PurposeWASM::OWNER => String::from("OWNER"),
        }
    }
}

impl From<PurposeWASM> for Purpose {
    fn from(purpose: PurposeWASM) -> Self {
        match purpose {
            PurposeWASM::AUTHENTICATION => Purpose::AUTHENTICATION,
            PurposeWASM::ENCRYPTION => Purpose::ENCRYPTION,
            PurposeWASM::DECRYPTION => Purpose::DECRYPTION,
            PurposeWASM::TRANSFER => Purpose::TRANSFER,
            PurposeWASM::SYSTEM => Purpose::SYSTEM,
            PurposeWASM::VOTING => Purpose::VOTING,
            PurposeWASM::OWNER => Purpose::OWNER,
        }
    }
}

impl From<Purpose> for PurposeWASM {
    fn from(purpose: Purpose) -> Self {
        match purpose {
            Purpose::AUTHENTICATION => PurposeWASM::AUTHENTICATION,
            Purpose::ENCRYPTION => PurposeWASM::ENCRYPTION,
            Purpose::DECRYPTION => PurposeWASM::DECRYPTION,
            Purpose::TRANSFER => PurposeWASM::TRANSFER,
            Purpose::SYSTEM => PurposeWASM::SYSTEM,
            Purpose::VOTING => PurposeWASM::VOTING,
            Purpose::OWNER => PurposeWASM::OWNER,
        }
    }
}
