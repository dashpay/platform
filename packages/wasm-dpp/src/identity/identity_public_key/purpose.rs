use dpp::identity::Purpose;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = KeyPurpose)]
pub enum PurposeWasm {
    /// at least one authentication key must be registered for all security levels
    AUTHENTICATION = 0,
    /// this key cannot be used for signing documents
    ENCRYPTION = 1,
    /// this key cannot be used for signing documents
    DECRYPTION = 2,
    /// this key cannot be used for signing documents
    WITHDRAW = 3,
    /// this key cannot be used for signing documents
    SYSTEM = 4,
    /// this key cannot be used for signing documents
    VOTING = 5,
}

impl From<Purpose> for PurposeWasm {
    fn from(p: Purpose) -> Self {
        match p {
            Purpose::AUTHENTICATION => PurposeWasm::AUTHENTICATION,
            Purpose::ENCRYPTION => PurposeWasm::ENCRYPTION,
            Purpose::DECRYPTION => PurposeWasm::DECRYPTION,
            Purpose::WITHDRAW => PurposeWasm::WITHDRAW,
            Purpose::SYSTEM => PurposeWasm::SYSTEM,
            Purpose::VOTING => PurposeWasm::VOTING,
        }
    }
}
