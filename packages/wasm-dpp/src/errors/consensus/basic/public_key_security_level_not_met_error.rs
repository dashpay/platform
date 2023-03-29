use crate::Serialize;
use dpp::identity::SecurityLevel;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
#[wasm_bindgen(js_name=PublicKeySecurityLevelNotMetError)]
pub struct PublicKeySecurityLevelNotMetErrorWasm {
    public_key_security_level: SecurityLevel,
    required_security_level: SecurityLevel,
    code: u32,
}

impl PublicKeySecurityLevelNotMetErrorWasm {
    pub fn new(
        public_key_security_level: SecurityLevel,
        required_security_level: SecurityLevel,
        code: u32,
    ) -> Self {
        PublicKeySecurityLevelNotMetErrorWasm {
            public_key_security_level,
            required_security_level,
            code,
        }
    }
}

#[wasm_bindgen(js_class=PublicKeySecurityLevelNotMetError)]
impl PublicKeySecurityLevelNotMetErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeySecurityLevel)]
    pub fn get_public_key_security_level(&self) -> u8 {
        self.public_key_security_level as u8
    }

    #[wasm_bindgen(js_name=getKeySecurityLevelRequirement)]
    pub fn get_key_security_level_requirement(&self) -> u8 {
        self.required_security_level as u8
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }

    #[wasm_bindgen(js_name=toString)]
    pub fn to_string(&self) -> Result<JsValue, JsValue> {
        let json_string = serde_json::ser::to_string(self).map_err(|e| JsValue::from(e.to_string()))?;
        Ok(JsValue::from(
            format!("PublicKeySecurityLevelNotMetError: {}", json_string),
        ))
    }
}
