use dpp::identity::KeyType;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[allow(non_camel_case_types)]
#[wasm_bindgen(js_name = "KeyType")]
pub enum KeyTypeWasm {
    ECDSA_SECP256K1 = 0,
    BLS12_381 = 1,
    ECDSA_HASH160 = 2,
    BIP13_SCRIPT_HASH = 3,
    EDDSA_25519_HASH160 = 4,
}

impl TryFrom<JsValue> for KeyTypeWasm {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "ecdsa_secp256k1" => Ok(KeyTypeWasm::ECDSA_SECP256K1),
                    "bls12_381" => Ok(KeyTypeWasm::BLS12_381),
                    "ecdsa_hash160" => Ok(KeyTypeWasm::ECDSA_HASH160),
                    "bip13_script_hash" => Ok(KeyTypeWasm::BIP13_SCRIPT_HASH),
                    "eddsa_25519_hash160" => Ok(KeyTypeWasm::EDDSA_25519_HASH160),
                    _ => Err(JsValue::from(format!("unsupported key type {}", enum_val))),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(KeyTypeWasm::ECDSA_SECP256K1),
                    1 => Ok(KeyTypeWasm::BLS12_381),
                    2 => Ok(KeyTypeWasm::ECDSA_HASH160),
                    3 => Ok(KeyTypeWasm::BIP13_SCRIPT_HASH),
                    4 => Ok(KeyTypeWasm::EDDSA_25519_HASH160),
                    _ => Err(JsValue::from(format!(
                        "unsupported key type ({})",
                        enum_val
                    ))),
                },
            },
        }
    }
}

impl From<KeyTypeWasm> for String {
    fn from(value: KeyTypeWasm) -> Self {
        match value {
            KeyTypeWasm::ECDSA_SECP256K1 => String::from("ECDSA_SECP256K1"),
            KeyTypeWasm::BLS12_381 => String::from("BLS12_381"),
            KeyTypeWasm::ECDSA_HASH160 => String::from("ECDSA_HASH160"),
            KeyTypeWasm::BIP13_SCRIPT_HASH => String::from("BIP13_SCRIPT_HASH"),
            KeyTypeWasm::EDDSA_25519_HASH160 => String::from("EDDSA_25519_HASH160"),
        }
    }
}

impl From<KeyTypeWasm> for KeyType {
    fn from(key_type: KeyTypeWasm) -> Self {
        match key_type {
            KeyTypeWasm::ECDSA_SECP256K1 => KeyType::ECDSA_SECP256K1,
            KeyTypeWasm::BLS12_381 => KeyType::BLS12_381,
            KeyTypeWasm::ECDSA_HASH160 => KeyType::ECDSA_HASH160,
            KeyTypeWasm::BIP13_SCRIPT_HASH => KeyType::BIP13_SCRIPT_HASH,
            KeyTypeWasm::EDDSA_25519_HASH160 => KeyType::EDDSA_25519_HASH160,
        }
    }
}

impl From<KeyType> for KeyTypeWasm {
    fn from(key_type: KeyType) -> Self {
        match key_type {
            KeyType::ECDSA_SECP256K1 => KeyTypeWasm::ECDSA_SECP256K1,
            KeyType::BLS12_381 => KeyTypeWasm::BLS12_381,
            KeyType::ECDSA_HASH160 => KeyTypeWasm::ECDSA_HASH160,
            KeyType::BIP13_SCRIPT_HASH => KeyTypeWasm::BIP13_SCRIPT_HASH,
            KeyType::EDDSA_25519_HASH160 => KeyTypeWasm::EDDSA_25519_HASH160,
        }
    }
}

impl From<KeyTypeWasm> for u8 {
    fn from(key_type: KeyTypeWasm) -> Self {
        match key_type {
            KeyTypeWasm::ECDSA_SECP256K1 => 0,
            KeyTypeWasm::BLS12_381 => 1,
            KeyTypeWasm::ECDSA_HASH160 => 2,
            KeyTypeWasm::BIP13_SCRIPT_HASH => 3,
            KeyTypeWasm::EDDSA_25519_HASH160 => 4,
        }
    }
}
