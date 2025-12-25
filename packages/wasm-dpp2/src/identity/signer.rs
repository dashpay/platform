//! Identity signer for WASM bindings.
//!
//! This module provides a signer for identity-based state transitions that implements
//! `Signer<IdentityPublicKey>`.

use crate::error::{WasmDppError, WasmDppResult};
use crate::private_key::PrivateKeyWasm;
use crate::utils::IntoWasm;
use dpp::dashcore::hashes::{hash160, Hash};
use dpp::dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};
use dpp::dashcore::signer;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType};
use dpp::platform_value::BinaryData;
use dpp::address_funds::AddressWitness;
use dpp::ProtocolError;
use std::collections::BTreeMap;
use std::fmt;
use wasm_bindgen::prelude::*;

/// A signer for identity-based state transitions.
///
/// This signer holds private keys mapped by their public key hash (for ECDSA_HASH160 keys)
/// and can sign state transitions that require identity keys.
#[wasm_bindgen(js_name = "IdentitySigner")]
#[derive(Clone, Default)]
pub struct IdentitySignerWasm {
    /// Maps public key hash (20 bytes) to private key bytes (32 bytes)
    private_keys: BTreeMap<[u8; 20], [u8; 32]>,
}

impl fmt::Debug for IdentitySignerWasm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IdentitySigner")
            .field("key_count", &self.private_keys.len())
            .finish()
    }
}

#[wasm_bindgen(js_class = IdentitySigner)]
impl IdentitySignerWasm {
    /// Creates a new empty IdentitySigner.
    #[wasm_bindgen(constructor)]
    pub fn new() -> IdentitySignerWasm {
        IdentitySignerWasm {
            private_keys: BTreeMap::new(),
        }
    }

    /// Adds a private key to the signer.
    ///
    /// The public key hash is derived automatically from the private key.
    ///
    /// @param privateKey - The PrivateKey object
    #[wasm_bindgen(js_name = "addKey")]
    pub fn add_key(&mut self, private_key: &PrivateKeyWasm) -> WasmDppResult<()> {
        let key_bytes = private_key.to_bytes();
        if key_bytes.len() != 32 {
            return Err(WasmDppError::invalid_argument(format!(
                "Private key must be 32 bytes, got {}",
                key_bytes.len()
            )));
        }

        // Derive public key hash from private key
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&key_bytes).map_err(|e| {
            WasmDppError::invalid_argument(format!("Invalid secret key: {}", e))
        })?;
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize();
        let public_key_hash = hash160::Hash::hash(&public_key_bytes[..])
            .to_byte_array();

        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&key_bytes);

        self.private_keys.insert(public_key_hash, key_array);
        Ok(())
    }

    /// Adds a private key from WIF format.
    ///
    /// @param wif - The private key in WIF format
    #[wasm_bindgen(js_name = "addKeyFromWif")]
    pub fn add_key_from_wif(&mut self, wif: &str) -> WasmDppResult<()> {
        let private_key = PrivateKeyWasm::from_wif(wif)?;
        self.add_key(&private_key)
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "IdentitySigner".to_string()
    }

    /// Returns the number of keys in this signer.
    #[wasm_bindgen(getter = keyCount)]
    pub fn key_count(&self) -> usize {
        self.private_keys.len()
    }
}

impl Signer<IdentityPublicKey> for IdentitySignerWasm {
    fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        // Only support ECDSA_HASH160 keys for now
        if identity_public_key.key_type() != KeyType::ECDSA_HASH160 {
            return Err(ProtocolError::Generic(format!(
                "IdentitySigner only supports ECDSA_HASH160 keys, got {:?}",
                identity_public_key.key_type()
            )));
        }

        // The key data is the public key hash (20 bytes)
        let key_data = identity_public_key.data().as_slice();
        if key_data.len() != 20 {
            return Err(ProtocolError::Generic(format!(
                "Expected 20-byte public key hash, got {} bytes",
                key_data.len()
            )));
        }

        let mut key_hash = [0u8; 20];
        key_hash.copy_from_slice(key_data);

        let private_key = self.private_keys.get(&key_hash).ok_or_else(|| {
            ProtocolError::Generic(format!(
                "No private key found for public key hash {}",
                hex::encode(key_hash)
            ))
        })?;

        let signature = signer::sign(data, private_key)?;
        Ok(signature.to_vec().into())
    }

    fn sign_create_witness(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        // First, sign the data to get the signature
        let signature = self.sign(key, data)?;

        // Create the appropriate AddressWitness based on the key type
        // IdentitySigner only supports ECDSA_HASH160 keys
        match key.key_type() {
            KeyType::ECDSA_HASH160 => {
                // P2PKH witness only needs the signature - the public key is recovered
                // during verification
                Ok(AddressWitness::P2pkh { signature })
            }
            _ => Err(ProtocolError::Generic(format!(
                "IdentitySigner only supports ECDSA_HASH160 keys, got {:?}",
                key.key_type()
            ))),
        }
    }

    fn can_sign_with(&self, identity_public_key: &IdentityPublicKey) -> bool {
        if identity_public_key.key_type() != KeyType::ECDSA_HASH160 {
            return false;
        }

        let key_data = identity_public_key.data().as_slice();
        if key_data.len() != 20 {
            return false;
        }

        let mut key_hash = [0u8; 20];
        key_hash.copy_from_slice(key_data);

        self.private_keys.contains_key(&key_hash)
    }
}

impl IdentitySignerWasm {
    /// Returns a reference to the inner signer for use in Rust code.
    pub fn inner(&self) -> &Self {
        self
    }

    /// Extracts an IdentitySigner from a JS options object.
    ///
    /// This helper reads the "signer" field from an options object and converts it
    /// to an IdentitySignerWasm. Useful for state transition functions that
    /// need a signer from their options.
    pub fn try_from_options(options: &JsValue) -> WasmDppResult<Self> {
        Self::try_from_options_with_field(options, "signer")
    }

    /// Extracts an IdentitySigner from a JS options object with a custom field name.
    ///
    /// This helper reads the specified field from an options object and converts it
    /// to an IdentitySignerWasm.
    pub fn try_from_options_with_field(options: &JsValue, field_name: &str) -> WasmDppResult<Self> {
        let signer_js = js_sys::Reflect::get(options, &JsValue::from_str(field_name))
            .map_err(|_| WasmDppError::invalid_argument(format!("Missing '{}' field", field_name)))?;

        if signer_js.is_undefined() || signer_js.is_null() {
            return Err(WasmDppError::invalid_argument(format!(
                "'{}' is required",
                field_name
            )));
        }

        Self::try_from(&signer_js)
    }
}

impl TryFrom<&JsValue> for IdentitySignerWasm {
    type Error = WasmDppError;

    fn try_from(value: &JsValue) -> Result<Self, Self::Error> {
        value
            .to_wasm::<IdentitySignerWasm>("IdentitySigner")
            .map(|boxed| (*boxed).clone())
            .map_err(|_| WasmDppError::invalid_argument("Expected an IdentitySigner object"))
    }
}
