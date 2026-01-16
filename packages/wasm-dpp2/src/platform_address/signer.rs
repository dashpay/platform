use super::PlatformAddressWasm;
use crate::core::private_key::PrivateKeyWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_options;
use crate::impl_wasm_type_info;
use crate::utils::IntoWasm;
use dpp::ProtocolError;
use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::dashcore::signer;
use dpp::identity::signer::Signer;
use dpp::platform_value::BinaryData;
use std::collections::BTreeMap;
use std::fmt;
use wasm_bindgen::prelude::*;

/// A signer for Platform address-based state transitions.
///
/// This signer holds private keys for Platform addresses and can sign
/// state transitions that spend from those addresses.
#[wasm_bindgen(js_name = "PlatformAddressSigner")]
#[derive(Clone, Default)]
pub struct PlatformAddressSignerWasm {
    /// Maps platform address to private key
    private_keys: BTreeMap<PlatformAddressWasm, PrivateKeyWasm>,
}

impl fmt::Debug for PlatformAddressSignerWasm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlatformAddressSigner")
            .field("key_count", &self.private_keys.len())
            .finish()
    }
}

#[wasm_bindgen(js_class = PlatformAddressSigner)]
impl PlatformAddressSignerWasm {
    /// Creates a new empty PlatformAddressSigner.
    #[wasm_bindgen(constructor)]
    pub fn new() -> PlatformAddressSignerWasm {
        PlatformAddressSignerWasm {
            private_keys: BTreeMap::new(),
        }
    }

    /// Adds a private key and derives the Platform address from it.
    ///
    /// The address is derived as: P2PKH(Hash160(compressed_public_key))
    ///
    /// @param privateKey - The PrivateKey object
    /// @returns The derived Platform address
    #[wasm_bindgen(js_name = "addKey")]
    pub fn add_key(&mut self, private_key: &PrivateKeyWasm) -> WasmDppResult<PlatformAddressWasm> {
        let platform_address =
            PlatformAddressWasm::from(PlatformAddress::from(private_key.inner()));
        self.private_keys
            .insert(platform_address, private_key.clone());
        Ok(platform_address)
    }

    /// Returns the number of keys in this signer.
    #[wasm_bindgen(getter = keyCount)]
    pub fn key_count(&self) -> usize {
        self.private_keys.len()
    }

    /// Returns true if this signer has a key for the given address.
    #[wasm_bindgen(js_name = "hasKey")]
    pub fn has_key(
        &self,
        #[wasm_bindgen(unchecked_param_type = "PlatformAddressLike")] address: &JsValue,
    ) -> WasmDppResult<bool> {
        let platform_address = PlatformAddressWasm::try_from(address)?;
        Ok(self.private_keys.contains_key(&platform_address))
    }

    /// Returns all private keys as an array of {addressHash: Uint8Array, privateKey: Uint8Array}.
    /// This is used internally for cross-package access.
    #[wasm_bindgen(js_name = "getPrivateKeysBytes")]
    pub fn get_private_keys_bytes(&self) -> WasmDppResult<js_sys::Array> {
        let result = js_sys::Array::new();
        for (addr, key) in &self.private_keys {
            let entry = js_sys::Object::new();
            let addr_inner: PlatformAddress = (*addr).into();
            let hash_array = js_sys::Uint8Array::from(addr_inner.hash().as_slice());
            let key_bytes = key.to_bytes();
            let key_array = js_sys::Uint8Array::from(key_bytes.as_slice());
            js_sys::Reflect::set(&entry, &JsValue::from_str("addressHash"), &hash_array)
                .map_err(|_| WasmDppError::generic("Failed to set addressHash property"))?;
            js_sys::Reflect::set(&entry, &JsValue::from_str("privateKey"), &key_array)
                .map_err(|_| WasmDppError::generic("Failed to set privateKey property"))?;
            result.push(&entry);
        }
        Ok(result)
    }
}

impl Signer<PlatformAddress> for PlatformAddressSignerWasm {
    fn sign(&self, address: &PlatformAddress, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        let wasm_address = PlatformAddressWasm::from(*address);

        let private_key = self.private_keys.get(&wasm_address).ok_or_else(|| {
            ProtocolError::Generic(format!(
                "No private key found for address hash {}",
                hex::encode(address.hash())
            ))
        })?;

        let key_bytes = private_key.to_bytes();
        let signature = signer::sign(data, &key_bytes)?;
        Ok(signature.to_vec().into())
    }

    fn sign_create_witness(
        &self,
        address: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let signature = self.sign(address, data)?;

        match address {
            PlatformAddress::P2pkh(_) => Ok(AddressWitness::P2pkh { signature }),
            PlatformAddress::P2sh(_) => Err(ProtocolError::Generic(
                "P2SH addresses not supported for signing".to_string(),
            )),
        }
    }

    fn can_sign_with(&self, address: &PlatformAddress) -> bool {
        let wasm_address = PlatformAddressWasm::from(*address);
        self.private_keys.contains_key(&wasm_address)
    }
}

impl PlatformAddressSignerWasm {
    /// Returns a reference to the inner signer for use in Rust code.
    pub fn inner(&self) -> &Self {
        self
    }

    /// Returns a reference to the private keys map.
    pub fn private_keys(&self) -> &BTreeMap<PlatformAddressWasm, PrivateKeyWasm> {
        &self.private_keys
    }
}

impl_try_from_options!(PlatformAddressSignerWasm, "PlatformAddressSigner", "signer");

impl TryFrom<&JsValue> for PlatformAddressSignerWasm {
    type Error = WasmDppError;

    fn try_from(value: &JsValue) -> Result<Self, Self::Error> {
        value
            .to_wasm::<PlatformAddressSignerWasm>("PlatformAddressSigner")
            .map(|boxed| (*boxed).clone())
            .map_err(|_| WasmDppError::invalid_argument("Expected a PlatformAddressSigner object"))
    }
}

impl_wasm_type_info!(PlatformAddressSignerWasm, PlatformAddressSigner);
