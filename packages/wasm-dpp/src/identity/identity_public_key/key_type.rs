use dpp::identity::KeyType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=KeyType)]
#[allow(non_camel_case_types)]
pub enum KeyTypeWasm {
    ECDSA_SECP256K1 = 0,
    BLS12_381 = 1,
    ECDSA_HASH160 = 2,
    BIP13_SCRIPT_HASH = 3,
    EDDSA_25519_HASH160 = 4,
}

impl From<KeyType> for KeyTypeWasm {
    fn from(key_type: KeyType) -> Self {
        match key_type {
            KeyType::ECDSA_SECP256K1 => Self::ECDSA_SECP256K1,
            KeyType::BLS12_381 => Self::BLS12_381,
            KeyType::ECDSA_HASH160 => Self::ECDSA_HASH160,
            KeyType::BIP13_SCRIPT_HASH => Self::BIP13_SCRIPT_HASH,
            KeyType::EDDSA_25519_HASH160 => Self::EDDSA_25519_HASH160,
        }
    }
}
