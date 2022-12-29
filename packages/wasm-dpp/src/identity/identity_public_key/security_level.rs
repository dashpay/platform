use dpp::identity::SecurityLevel;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "KeySecurityLevel")]
pub enum SecurityLevelWasm {
    MASTER = 0,
    CRITICAL = 1,
    HIGH = 2,
    MEDIUM = 3,
}

impl From<SecurityLevel> for SecurityLevelWasm {
    fn from(level: SecurityLevel) -> Self {
        match level {
            SecurityLevel::CRITICAL => Self::CRITICAL,
            SecurityLevel::HIGH => Self::HIGH,
            SecurityLevel::MASTER => Self::MASTER,
            SecurityLevel::MEDIUM => Self::MEDIUM,
        }
    }
}
