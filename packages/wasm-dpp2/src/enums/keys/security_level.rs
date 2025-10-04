use dpp::identity::SecurityLevel;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = SecurityLevel)]
pub enum SecurityLevelWASM {
    MASTER = 0,
    CRITICAL = 1,
    HIGH = 2,
    MEDIUM = 3,
}

impl TryFrom<JsValue> for SecurityLevelWASM {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "master" => Ok(SecurityLevelWASM::MASTER),
                    "critical" => Ok(SecurityLevelWASM::CRITICAL),
                    "high" => Ok(SecurityLevelWASM::HIGH),
                    "medium" => Ok(SecurityLevelWASM::MEDIUM),
                    _ => Err(JsValue::from(format!(
                        "unsupported security level value ({})",
                        enum_val
                    ))),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(SecurityLevelWASM::MASTER),
                    1 => Ok(SecurityLevelWASM::CRITICAL),
                    2 => Ok(SecurityLevelWASM::HIGH),
                    3 => Ok(SecurityLevelWASM::MEDIUM),
                    _ => Err(JsValue::from(format!(
                        "unsupported security level value ({})",
                        enum_val
                    ))),
                },
            },
        }
    }
}

impl From<SecurityLevelWASM> for String {
    fn from(level: SecurityLevelWASM) -> String {
        match level {
            SecurityLevelWASM::MASTER => String::from("MASTER"),
            SecurityLevelWASM::CRITICAL => String::from("CRITICAL"),
            SecurityLevelWASM::HIGH => String::from("HIGH"),
            SecurityLevelWASM::MEDIUM => String::from("MEDIUM"),
        }
    }
}

impl From<SecurityLevelWASM> for SecurityLevel {
    fn from(security_level: SecurityLevelWASM) -> Self {
        match security_level {
            SecurityLevelWASM::MASTER => SecurityLevel::MASTER,
            SecurityLevelWASM::CRITICAL => SecurityLevel::CRITICAL,
            SecurityLevelWASM::HIGH => SecurityLevel::HIGH,
            SecurityLevelWASM::MEDIUM => SecurityLevel::MEDIUM,
        }
    }
}

impl From<SecurityLevel> for SecurityLevelWASM {
    fn from(security_level: SecurityLevel) -> Self {
        match security_level {
            SecurityLevel::MASTER => SecurityLevelWASM::MASTER,
            SecurityLevel::CRITICAL => SecurityLevelWASM::CRITICAL,
            SecurityLevel::HIGH => SecurityLevelWASM::HIGH,
            SecurityLevel::MEDIUM => SecurityLevelWASM::MEDIUM,
        }
    }
}
