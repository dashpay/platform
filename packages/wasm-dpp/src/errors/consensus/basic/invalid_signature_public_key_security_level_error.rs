use dpp::identity::SecurityLevel;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidSignaturePublicKeySecurityLevelError)]
pub struct InvalidSignaturePublicKeySecurityLevelErrorWasm {
    public_key_security_level: SecurityLevel,
    allowed_key_security_levels: Vec<SecurityLevel>,
    code: u32,
}

impl InvalidSignaturePublicKeySecurityLevelErrorWasm {
    pub fn new(
        public_key_security_level: SecurityLevel,
        allowed_key_security_levels: Vec<SecurityLevel>,
        code: u32,
    ) -> Self {
        InvalidSignaturePublicKeySecurityLevelErrorWasm {
            public_key_security_level,
            allowed_key_security_levels,
            code,
        }
    }
}

#[wasm_bindgen(js_class=InvalidSignaturePublicKeySecurityLevelError)]
impl InvalidSignaturePublicKeySecurityLevelErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeySecurityLevel)]
    pub fn get_public_key_security_level(&self) -> u8 {
        self.public_key_security_level as u8
    }

    #[wasm_bindgen(js_name=getKeySecurityLevelRequirement)]
    pub fn get_allowed_key_security_levels(&self) -> Vec<u8> {
        self.allowed_key_security_levels
            .iter()
            .map(|security_level| *security_level as u8)
            .collect()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
