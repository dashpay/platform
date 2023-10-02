use anyhow::anyhow;
use dpp::{BlsModule, PublicKeyValidationError};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    #[derive(Clone)]
    pub type JsBlsAdapter;

    #[wasm_bindgen(method)]
    pub fn validatePublicKey(this: &JsBlsAdapter, pk: &[u8]) -> bool;

    #[wasm_bindgen(method, catch)]
    pub fn verifySignature(
        this: &JsBlsAdapter,
        signature: &[u8],
        data: &[u8],
        public_key: &[u8],
    ) -> Result<bool, JsValue>;

    #[wasm_bindgen(method, catch)]
    pub fn privateKeyToPublicKey(
        this: &JsBlsAdapter,
        private_key: &[u8],
    ) -> Result<js_sys::Uint8Array, JsValue>;

    #[wasm_bindgen(method, catch)]
    pub fn sign(
        this: &JsBlsAdapter,
        data: &[u8],
        private_key: &[u8],
    ) -> Result<js_sys::Uint8Array, JsValue>;
}

#[derive(Clone)]
pub struct BlsAdapter(pub JsBlsAdapter);

impl BlsModule for BlsAdapter {
    fn validate_public_key(&self, pk: &[u8]) -> Result<(), PublicKeyValidationError> {
        let is_valid = self.0.validatePublicKey(pk);

        if !is_valid {
            return Err(PublicKeyValidationError::new("Invalid public key"));
        }

        Ok(())
    }

    fn verify_signature(
        &self,
        signature: &[u8],
        data: &[u8],
        public_key: &[u8],
    ) -> Result<bool, dpp::ProtocolError> {
        self.0
            .verifySignature(signature, data, public_key)
            .map_err(|_v| anyhow!("Can't verify signature").into())
    }

    fn private_key_to_public_key(&self, private_key: &[u8]) -> Result<Vec<u8>, dpp::ProtocolError> {
        self.0
            .privateKeyToPublicKey(private_key)
            .map(|arr| arr.to_vec())
            .map_err(|_v| anyhow!("Can't convert private key to public key").into())
    }

    fn sign(&self, data: &[u8], private_key: &[u8]) -> Result<Vec<u8>, dpp::ProtocolError> {
        self.0
            .sign(data, private_key)
            .map(|arr| arr.to_vec())
            .map_err(|e: JsValue| {
                let error = e.dyn_into::<js_sys::Error>();

                match error {
                    Ok(e) => {
                        let message: String = e
                            .message()
                            .as_string()
                            .unwrap_or_else(|| String::from("Unknown error, can't sign"));
                        anyhow!(message).into()
                    }
                    Err(_) => anyhow!("Unknown error, can't sign").into(),
                }
            })
    }
}
