use crate::buffer::Buffer;
use dpp::dashcore::secp256k1::rand::thread_rng;
use dpp::dashcore::secp256k1::Secp256k1;
use dpp::dashcore::{
    secp256k1::SecretKey, InstantLock, Network, OutPoint, PrivateKey, Script, Transaction, TxIn,
    TxOut, Txid,
};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = generateTemporaryEcdsaPrivateKey)]
pub fn generate_temporary_ecdsa_private_key() -> JsValue {
    let mut rng = thread_rng();

    let secret_key = SecretKey::new(&mut rng);
    let one_time_private_key = PrivateKey::new(secret_key, Network::Testnet).to_string();

    JsValue::from_str(&one_time_private_key)
}
