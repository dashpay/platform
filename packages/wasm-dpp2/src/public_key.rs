use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_type_info;
use dpp::dashcore::PublicKey;
use dpp::dashcore::ecdsa::EcdsaPublicKey;
use dpp::dashcore::key::constants;
use std::convert::TryInto;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "PublicKey")]
pub struct PublicKeyWasm(PublicKey);

impl From<PublicKey> for PublicKeyWasm {
    fn from(pk: PublicKey) -> Self {
        Self(pk)
    }
}

impl From<PublicKeyWasm> for PublicKey {
    fn from(pk: PublicKeyWasm) -> Self {
        pk.0
    }
}

#[wasm_bindgen(js_class = PublicKey)]
impl PublicKeyWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        compressed: bool,
        #[wasm_bindgen(js_name = "publicKeyBytes")] public_key_bytes: Vec<u8>,
    ) -> WasmDppResult<PublicKeyWasm> {
        let inner = if compressed {
            if public_key_bytes.len() != constants::PUBLIC_KEY_SIZE {
                return Err(WasmDppError::invalid_argument(format!(
                    "compressed public key size must be equal to {}",
                    constants::PUBLIC_KEY_SIZE
                )));
            }

            let bytes: [u8; constants::PUBLIC_KEY_SIZE] =
                public_key_bytes.try_into().map_err(|_| {
                    WasmDppError::invalid_argument("compressed public key must contain 33 bytes")
                })?;
            EcdsaPublicKey::from_bytes(&bytes)
        } else {
            if public_key_bytes.len() != constants::UNCOMPRESSED_PUBLIC_KEY_SIZE {
                return Err(WasmDppError::invalid_argument(format!(
                    "uncompressed public key size must be equal to {}",
                    constants::UNCOMPRESSED_PUBLIC_KEY_SIZE
                )));
            }

            let bytes: [u8; constants::UNCOMPRESSED_PUBLIC_KEY_SIZE] =
                public_key_bytes.try_into().map_err(|_| {
                    WasmDppError::invalid_argument("uncompressed public key must contain 65 bytes")
                })?;

            EcdsaPublicKey::from_bytes(&bytes)
        }
        .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        Ok(PublicKeyWasm(PublicKey { compressed, inner }))
    }

    #[wasm_bindgen(getter = "compressed")]
    pub fn compressed(&self) -> bool {
        self.0.compressed
    }

    #[wasm_bindgen(getter = "inner")]
    pub fn inner(&self) -> Vec<u8> {
        if self.0.compressed {
            self.0.inner.to_compressed().into()
        } else {
            self.0.inner.to_uncompressed().into()
        }
    }

    #[wasm_bindgen(setter = "compressed")]
    pub fn set_compressed(&mut self, compressed: bool) {
        self.0.compressed = compressed;
    }

    #[wasm_bindgen(setter = "inner")]
    pub fn set_inner(&mut self, inner: Vec<u8>) -> WasmDppResult<()> {
        if inner.len() == constants::PUBLIC_KEY_SIZE {
            let bytes: [u8; constants::PUBLIC_KEY_SIZE] = inner.try_into().map_err(|_| {
                WasmDppError::invalid_argument("compressed public key must contain 33 bytes")
            })?;

            self.0.compressed = true;
            self.0.inner = EcdsaPublicKey::from_bytes(&bytes)
                .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;
        } else {
            if inner.len() != constants::UNCOMPRESSED_PUBLIC_KEY_SIZE {
                return Err(WasmDppError::invalid_argument(format!(
                    "uncompressed public key size must be equal to {}",
                    constants::UNCOMPRESSED_PUBLIC_KEY_SIZE
                )));
            }

            let bytes: [u8; constants::UNCOMPRESSED_PUBLIC_KEY_SIZE] =
                inner.try_into().map_err(|_| {
                    WasmDppError::invalid_argument("uncompressed public key must contain 65 bytes")
                })?;

            self.0.compressed = false;
            self.0.inner = EcdsaPublicKey::from_bytes(&bytes)
                .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;
        }

        Ok(())
    }

    #[wasm_bindgen(js_name = "getPublicKeyHash")]
    pub fn get_public_key_hash(&self) -> String {
        self.0.pubkey_hash().to_string()
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<PublicKeyWasm> {
        Ok(PublicKeyWasm(
            PublicKey::from_slice(bytes.as_slice())
                .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?,
        ))
    }
}

impl_wasm_type_info!(PublicKeyWasm, PublicKey);

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::dashcore::ecdsa::EcdsaSecretKey;

    #[test]
    fn should_preserve_public_key_parser_encoding_contracts() {
        let key = EcdsaSecretKey::from_bytes(&[1; 32])
            .expect("valid scalar")
            .public_key();
        let compressed = key.to_compressed();
        let uncompressed = key.to_uncompressed();
        let mut hybrid = uncompressed;
        hybrid[0] = 6 | (hybrid[64] & 1);

        let parsed = PublicKeyWasm::constructor(true, compressed.to_vec()).expect("compressed key");
        assert_eq!(parsed.inner(), compressed);
        assert!(PublicKeyWasm::constructor(true, uncompressed.to_vec()).is_err());
        for bytes in [uncompressed, hybrid] {
            let mut parsed = PublicKeyWasm::constructor(false, bytes.to_vec()).expect("SEC1 key");
            assert_eq!(parsed.inner(), uncompressed);
            parsed.set_inner(bytes.to_vec()).expect("SEC1 setter");
            assert_eq!(parsed.inner(), uncompressed);
        }
        // The Core wire parser is stricter than the raw SEC1 constructor/setter.
        assert!(PublicKeyWasm::from_bytes(hybrid.to_vec()).is_err());
        hybrid[0] ^= 1;
        assert!(PublicKeyWasm::constructor(false, hybrid.to_vec()).is_err());
    }
}
