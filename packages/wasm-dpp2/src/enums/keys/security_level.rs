use dpp::identity::SecurityLevel;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = SecurityLevel)]
pub enum SecurityLevelWasm {
    MASTER = 0,
    CRITICAL = 1,
    HIGH = 2,
    MEDIUM = 3,
}

impl TryFrom<JsValue> for SecurityLevelWasm {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "master" => Ok(SecurityLevelWasm::MASTER),
                    "critical" => Ok(SecurityLevelWasm::CRITICAL),
                    "high" => Ok(SecurityLevelWasm::HIGH),
                    "medium" => Ok(SecurityLevelWasm::MEDIUM),
                    _ => Err(JsValue::from(format!(
                        "unsupported security level value ({})",
                        enum_val
                    ))),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(SecurityLevelWasm::MASTER),
                    1 => Ok(SecurityLevelWasm::CRITICAL),
                    2 => Ok(SecurityLevelWasm::HIGH),
                    3 => Ok(SecurityLevelWasm::MEDIUM),
                    _ => Err(JsValue::from(format!(
                        "unsupported security level value ({})",
                        enum_val
                    ))),
                },
            },
        }
    }
}

impl From<SecurityLevelWasm> for String {
    fn from(level: SecurityLevelWasm) -> String {
        match level {
            SecurityLevelWasm::MASTER => String::from("MASTER"),
            SecurityLevelWasm::CRITICAL => String::from("CRITICAL"),
            SecurityLevelWasm::HIGH => String::from("HIGH"),
            SecurityLevelWasm::MEDIUM => String::from("MEDIUM"),
        }
    }
}

impl From<SecurityLevelWasm> for SecurityLevel {
    fn from(security_level: SecurityLevelWasm) -> Self {
        match security_level {
            SecurityLevelWasm::MASTER => SecurityLevel::MASTER,
            SecurityLevelWasm::CRITICAL => SecurityLevel::CRITICAL,
            SecurityLevelWasm::HIGH => SecurityLevel::HIGH,
            SecurityLevelWasm::MEDIUM => SecurityLevel::MEDIUM,
        }
    }
}

impl From<SecurityLevel> for SecurityLevelWasm {
    fn from(security_level: SecurityLevel) -> Self {
        match security_level {
            SecurityLevel::MASTER => SecurityLevelWasm::MASTER,
            SecurityLevel::CRITICAL => SecurityLevelWasm::CRITICAL,
            SecurityLevel::HIGH => SecurityLevelWasm::HIGH,
            SecurityLevel::MEDIUM => SecurityLevelWasm::MEDIUM,
        }
    }
}
