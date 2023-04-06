use wasm_bindgen::prelude::wasm_bindgen;
use crate::buffer::Buffer;
use dpp::dashcore::secp256k1::rand::thread_rng;
use dpp::dashcore::secp256k1::Secp256k1;
use dpp::dashcore::{
    secp256k1::SecretKey, InstantLock, Network, OutPoint, PrivateKey, Script, Transaction, TxIn,
    TxOut, Txid,
};

#[wasm_bindgen(js_name = generateTemporaryEcdsaPrivateKey)]
pub fn generate_temporary_ecdsa_private_key() -> Buffer {
    let mut rng = thread_rng();

    let secret_key = SecretKey::new(&mut rng);
    let one_time_private_key = PrivateKey::new(secret_key, Network::Testnet);

    Buffer::from_bytes_owned(one_time_private_key.to_bytes())
}