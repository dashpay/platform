//! Identity signer for WASM bindings.
//!
//! This module provides a signer for identity-based state transitions that implements
//! `Signer<IdentityPublicKey>`.

use crate::core::private_key::PrivateKeyWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_options;
use crate::utils::IntoWasm;
use dpp::ProtocolError;
use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::signer;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType};
use dpp::platform_value::BinaryData;
use std::collections::BTreeMap;
use std::fmt;
use wasm_bindgen::prelude::*;

/// A signer for identity-based state transitions.
///
/// Private keys are stored by their public key hash (20 bytes).
/// Both ECDSA_HASH160 and ECDSA_SECP256K1 keys are looked up by hash160.
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

/// Compute hash160 (RIPEMD160(SHA256(data))) of a compressed public key.
fn hash160(pubkey: &[u8]) -> [u8; 20] {
    dpp::dashcore::hashes::hash160::Hash::hash(pubkey).to_byte_array()
}

#[wasm_bindgen(js_class = IdentitySigner)]
impl IdentitySignerWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "IdentitySigner".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "IdentitySigner".to_string()
    }

    /// Creates a new empty IdentitySigner.
    #[wasm_bindgen(constructor)]
    pub fn new() -> IdentitySignerWasm {
        IdentitySignerWasm {
            private_keys: BTreeMap::new(),
        }
    }

    /// Adds a private key to the signer.
    ///
    /// The key is stored by public key hash (20 bytes): Hash160(compressed_public_key)
    ///
    /// @param privateKey - The PrivateKey object
    #[wasm_bindgen(js_name = "addKey")]
    pub fn add_key(&mut self, private_key: &PrivateKeyWasm) -> WasmDppResult<()> {
        let private_key_bytes: [u8; 32] = private_key
            .to_bytes()
            .try_into()
            .map_err(|_| WasmDppError::invalid_argument("Private key must be 32 bytes"))?;

        // Derive public key hash from private key
        let platform_address = PlatformAddress::from(private_key.inner());
        let public_key_hash = *platform_address.hash();

        self.private_keys.insert(public_key_hash, private_key_bytes);

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

    /// Returns the number of keys in this signer.
    #[wasm_bindgen(getter = keyCount)]
    pub fn key_count(&self) -> usize {
        self.private_keys.len()
    }
}

impl IdentitySignerWasm {
    /// Get the public key hash from an identity public key.
    /// For ECDSA_HASH160: the key data is already the hash.
    /// For ECDSA_SECP256K1: compute hash160 of the compressed public key.
    #[allow(clippy::result_large_err)]
    fn get_key_hash(identity_public_key: &IdentityPublicKey) -> Result<[u8; 20], ProtocolError> {
        let key_data = identity_public_key.data().as_slice();

        match identity_public_key.key_type() {
            KeyType::ECDSA_HASH160 => {
                if key_data.len() != 20 {
                    return Err(ProtocolError::Generic(format!(
                        "Expected 20-byte public key hash for ECDSA_HASH160, got {} bytes",
                        key_data.len()
                    )));
                }
                let mut key_hash = [0u8; 20];
                key_hash.copy_from_slice(key_data);
                Ok(key_hash)
            }
            KeyType::ECDSA_SECP256K1 => {
                if key_data.len() != 33 {
                    return Err(ProtocolError::Generic(format!(
                        "Expected 33-byte compressed public key for ECDSA_SECP256K1, got {} bytes",
                        key_data.len()
                    )));
                }
                Ok(hash160(key_data))
            }
            _ => Err(ProtocolError::Generic(format!(
                "IdentitySigner only supports ECDSA_HASH160 and ECDSA_SECP256K1 keys, got {:?}",
                identity_public_key.key_type()
            ))),
        }
    }
}

impl Signer<IdentityPublicKey> for IdentitySignerWasm {
    fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let key_hash = Self::get_key_hash(identity_public_key)?;

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
        let signature = self.sign(key, data)?;

        match key.key_type() {
            KeyType::ECDSA_HASH160 | KeyType::ECDSA_SECP256K1 => {
                Ok(AddressWitness::P2pkh { signature })
            }
            _ => Err(ProtocolError::Generic(format!(
                "IdentitySigner only supports ECDSA_HASH160 and ECDSA_SECP256K1 keys, got {:?}",
                key.key_type()
            ))),
        }
    }

    fn can_sign_with(&self, identity_public_key: &IdentityPublicKey) -> bool {
        Self::get_key_hash(identity_public_key)
            .map(|hash| self.private_keys.contains_key(&hash))
            .unwrap_or(false)
    }
}

impl IdentitySignerWasm {
    /// Returns a reference to the inner signer for use in Rust code.
    pub fn inner(&self) -> &Self {
        self
    }
}

impl_try_from_options!(IdentitySignerWasm, "IdentitySigner", "signer");

impl TryFrom<&JsValue> for IdentitySignerWasm {
    type Error = WasmDppError;

    fn try_from(value: &JsValue) -> Result<Self, Self::Error> {
        value
            .to_wasm::<IdentitySignerWasm>("IdentitySigner")
            .map(|boxed| (*boxed).clone())
            .map_err(|_| WasmDppError::invalid_argument("Expected an IdentitySigner object"))
    }
}
