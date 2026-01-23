use std::convert::TryInto;

use super::network::{NetworkLikeJs, NetworkWasm};
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::public_key::PublicKeyWasm;
use dpp::dashcore::PrivateKey;
use dpp::dashcore::hashes::hex::FromHex;
use dpp::dashcore::key::Secp256k1;
use dpp::dashcore::secp256k1::hashes::hex::{Case, DisplayHex};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "PrivateKey")]
#[derive(Clone)]
pub struct PrivateKeyWasm(PrivateKey);

impl From<PrivateKeyWasm> for PrivateKey {
    fn from(key: PrivateKeyWasm) -> Self {
        key.0
    }
}

impl From<PrivateKey> for PrivateKeyWasm {
    fn from(key: PrivateKey) -> Self {
        PrivateKeyWasm(key)
    }
}

impl PrivateKeyWasm {
    /// Returns a reference to the inner PrivateKey
    pub fn inner(&self) -> &PrivateKey {
        &self.0
    }
}

#[wasm_bindgen(js_class = PrivateKey)]
impl PrivateKeyWasm {
    #[wasm_bindgen(js_name = "fromWIF")]
    pub fn from_wif(wif: &str) -> WasmDppResult<Self> {
        let pk = PrivateKey::from_wif(wif)
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        Ok(PrivateKeyWasm(pk))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>, network: NetworkLikeJs) -> WasmDppResult<Self> {
        let network_wasm: NetworkWasm = network.try_into()?;

        let key_bytes: [u8; 32] = bytes.try_into().map_err(|_| {
            WasmDppError::invalid_argument("Private key bytes must be exactly 32 bytes".to_string())
        })?;

        let pk = PrivateKey::from_byte_array(&key_bytes, network_wasm.into())
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        Ok(PrivateKeyWasm(pk))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(
        #[wasm_bindgen(js_name = "hexKey")] hex_key: &str,
        network: NetworkLikeJs,
    ) -> WasmDppResult<Self> {
        let network_wasm: NetworkWasm = network.try_into()?;

        let bytes = Vec::from_hex(hex_key)
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        let key_bytes: [u8; 32] = bytes.try_into().map_err(|_| {
            WasmDppError::invalid_argument("Private key hex must decode to 32 bytes".to_string())
        })?;

        let pk = PrivateKey::from_byte_array(&key_bytes, network_wasm.into())
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        Ok(PrivateKeyWasm(pk))
    }

    #[wasm_bindgen(js_name = "getPublicKey")]
    pub fn get_public_key(&self) -> PublicKeyWasm {
        let secp = Secp256k1::new();

        let public_key = self.0.public_key(&secp);

        public_key.into()
    }
}

#[wasm_bindgen(js_class = PrivateKey)]
impl PrivateKeyWasm {
    #[wasm_bindgen(js_name = "toWIF")]
    pub fn to_wif(&self) -> String {
        self.0.to_wif()
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> String {
        self.0.to_bytes().to_hex_string(Case::Upper)
    }

    #[wasm_bindgen(js_name = "getPublicKeyHash")]
    pub fn get_public_key_hash(&self) -> String {
        let secp = Secp256k1::new();

        self.0.public_key(&secp).pubkey_hash().to_hex()
    }
}

impl_try_from_js_value!(PrivateKeyWasm, "PrivateKey");
impl_wasm_type_info!(PrivateKeyWasm, PrivateKey);
