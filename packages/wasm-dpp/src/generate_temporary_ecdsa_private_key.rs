use dpp::dashcore::ecdsa::EcdsaSecretKey;
use dpp::dashcore::{Network, PrivateKey};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = generateTemporaryEcdsaPrivateKey)]
pub fn generate_temporary_ecdsa_private_key() -> JsValue {
    let secret_key = EcdsaSecretKey::generate();
    let one_time_private_key = PrivateKey::new(secret_key, Network::Testnet).to_string();

    JsValue::from_str(&one_time_private_key)
}
