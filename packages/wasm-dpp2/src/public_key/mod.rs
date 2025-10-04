use dpp::dashcore::key::constants;
use dpp::dashcore::{PublicKey, secp256k1};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "PublicKey")]
pub struct PublicKeyWASM(PublicKey);

impl From<PublicKey> for PublicKeyWASM {
    fn from(pk: PublicKey) -> Self {
        Self(pk)
    }
}

impl From<PublicKeyWASM> for PublicKey {
    fn from(pk: PublicKeyWASM) -> Self {
        pk.0
    }
}

#[wasm_bindgen(js_class = PublicKey)]
impl PublicKeyWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "PublicKey".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "PublicKey".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(compressed: bool, public_key_bytes: Vec<u8>) -> Result<PublicKeyWASM, JsValue> {
        let inner = match compressed {
            true => {
                if public_key_bytes.len() != constants::PUBLIC_KEY_SIZE {
                    return Err(JsValue::from(String::from(format!(
                        "compressed public key size must be equal to {}",
                        constants::PUBLIC_KEY_SIZE
                    ))));
                }

                secp256k1::PublicKey::from_byte_array_compressed(&public_key_bytes.try_into()?)
            }
            false => {
                if public_key_bytes.len() != constants::UNCOMPRESSED_PUBLIC_KEY_SIZE {
                    return Err(JsValue::from(String::from(format!(
                        "uncompressed public key size must be equal to {}",
                        constants::UNCOMPRESSED_PUBLIC_KEY_SIZE
                    ))));
                }

                secp256k1::PublicKey::from_byte_array_uncompressed(&public_key_bytes.try_into()?)
            }
        }
        .map_err(|err| JsValue::from(err.to_string()))?;

        Ok(PublicKeyWASM(PublicKey { compressed, inner }))
    }

    #[wasm_bindgen(getter = "compressed")]
    pub fn compressed(&self) -> bool {
        self.0.compressed
    }

    #[wasm_bindgen(getter = "inner")]
    pub fn inner(&self) -> Vec<u8> {
        match self.0.compressed {
            true => self.0.inner.serialize().into(),
            false => self.0.inner.serialize_uncompressed().into(),
        }
    }

    #[wasm_bindgen(setter = "compressed")]
    pub fn set_compressed(&mut self, compressed: bool) {
        self.0.compressed = compressed;
    }

    #[wasm_bindgen(setter = "inner")]
    pub fn set_inner(&mut self, inner: Vec<u8>) -> Result<(), JsValue> {
        match inner.len() == constants::PUBLIC_KEY_SIZE {
            true => {
                self.0.compressed = true;
                self.0.inner = secp256k1::PublicKey::from_byte_array_compressed(&inner.try_into()?)
                    .map_err(|err| JsValue::from(err.to_string()))?
            }
            false => {
                if inner.len() != constants::UNCOMPRESSED_PUBLIC_KEY_SIZE {
                    return Err(JsValue::from(String::from(format!(
                        "uncompressed public key size must be equal to {}",
                        constants::UNCOMPRESSED_PUBLIC_KEY_SIZE
                    ))));
                }

                self.0.compressed = false;
                self.0.inner =
                    secp256k1::PublicKey::from_byte_array_uncompressed(&inner.try_into()?)
                        .map_err(|err| JsValue::from(err.to_string()))?
            }
        };

        Ok(())
    }

    #[wasm_bindgen(js_name = getPublicKeyHash)]
    pub fn get_public_key_hash(&self) -> String {
        self.0.pubkey_hash().to_hex()
    }

    #[wasm_bindgen(js_name = "bytes")]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> Result<PublicKeyWASM, JsValue> {
        Ok(PublicKeyWASM(
            PublicKey::from_slice(bytes.as_slice())
                .map_err(|err| JsValue::from(err.to_string()))?,
        ))
    }
}
