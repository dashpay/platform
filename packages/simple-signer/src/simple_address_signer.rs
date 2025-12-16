//! Simple signer for platform addresses
//!
//! This module provides a simple implementation of the `Signer<PlatformAddress>` trait
//! for signing with P2PKH addresses. It maps address hashes to their corresponding private keys.

use bincode::Encode;
// TODO: Looks like we have duplicate address signer implementations.
// Discuss and remove one of them.
use dpp::address_funds::AddressWitness;
use dpp::address_funds::PlatformAddress;
use dpp::dashcore::hashes::{hash160, Hash};
use dpp::dashcore::secp256k1::{Secp256k1, SecretKey};
use dpp::dashcore::signer;
use dpp::identity::signer::Signer;
use dpp::platform_serialization::de::Decode;
use dpp::platform_value::BinaryData;
use dpp::ProtocolError;
use std::collections::BTreeMap;

/// A simple signer for platform addresses that maps addresses to their private keys.
///
/// This signer supports P2PKH addresses only. For P2SH multisig support, a more
/// sophisticated signer implementation is required.
#[derive(Debug, Default, Clone, PartialEq, Encode, Decode)]
pub struct SimpleAddressSigner {
    /// Maps address hash (20 bytes) to private key (32 bytes)
    keys: BTreeMap<[u8; 20], [u8; 32]>,
}

impl SimpleAddressSigner {
    /// Create a new empty address signer
    pub fn new() -> Self {
        Self {
            keys: BTreeMap::new(),
        }
    }

    /// Create an address signer from addresses and their corresponding private keys.
    ///
    /// # Arguments
    /// * `addresses` - Slice of platform addresses
    /// * `private_keys` - Slice of private keys (each must be 32 bytes)
    ///
    /// # Errors
    /// Returns an error if:
    /// - The number of addresses doesn't match the number of private keys
    /// - Any private key is not exactly 32 bytes
    /// - Any private key doesn't correspond to its paired address
    /// - Any address is P2SH (not supported)
    pub fn from_addresses_and_keys(
        addresses: &[PlatformAddress],
        private_keys: &[[u8; 32]],
    ) -> Result<Self, ProtocolError> {
        if addresses.len() != private_keys.len() {
            return Err(ProtocolError::Generic(
                "Number of addresses must match number of private keys".to_string(),
            ));
        }

        let secp = Secp256k1::new();
        let mut keys = BTreeMap::new();

        for (address, private_key) in addresses.iter().zip(private_keys.iter()) {
            // Verify the private key corresponds to this address
            let secret_key = SecretKey::from_byte_array(&private_key)
                .map_err(|e| ProtocolError::Generic(format!("Invalid private key: {}", e)))?;
            let public_key =
                dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
            let pubkey_hash: [u8; 20] =
                hash160::Hash::hash(&public_key.serialize()).to_byte_array();

            let address_hash = match address {
                PlatformAddress::P2pkh(hash) => *hash,
                PlatformAddress::P2sh(_) => {
                    return Err(ProtocolError::Generic(
                        "P2SH addresses not yet supported in SimpleAddressSigner".to_string(),
                    ));
                }
            };

            if pubkey_hash != address_hash {
                return Err(ProtocolError::Generic(
                    "Private key does not match address".to_string(),
                ));
            }

            keys.insert(address_hash, *private_key);
        }

        Ok(Self { keys })
    }

    /// Add a P2PKH address and its private key to the signer.
    ///
    /// # Arguments
    /// * `address` - The platform address (must be P2PKH)
    /// * `private_key` - The 32-byte private key
    ///
    /// # Errors
    /// Returns an error if:
    /// - The private key is not exactly 32 bytes
    /// - The private key doesn't correspond to the address
    /// - The address is P2SH (not supported)
    pub fn add_key(
        &mut self,
        address: &PlatformAddress,
        private_key: &[u8],
    ) -> Result<(), ProtocolError> {
        if private_key.len() != 32 {
            return Err(ProtocolError::Generic(
                "Private key must be 32 bytes".to_string(),
            ));
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(private_key);

        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_byte_array(&key_bytes)
            .map_err(|e| ProtocolError::Generic(format!("Invalid private key: {}", e)))?;
        let public_key = dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let pubkey_hash: [u8; 20] = hash160::Hash::hash(&public_key.serialize()).to_byte_array();

        let address_hash = match address {
            PlatformAddress::P2pkh(hash) => *hash,
            PlatformAddress::P2sh(_) => {
                return Err(ProtocolError::Generic(
                    "P2SH addresses not yet supported in SimpleAddressSigner".to_string(),
                ));
            }
        };

        if pubkey_hash != address_hash {
            return Err(ProtocolError::Generic(
                "Private key does not match address".to_string(),
            ));
        }

        self.keys.insert(address_hash, key_bytes);
        Ok(())
    }
}

impl Signer<PlatformAddress> for SimpleAddressSigner {
    fn sign(&self, key: &PlatformAddress, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        let hash = match key {
            PlatformAddress::P2pkh(hash) => hash,
            PlatformAddress::P2sh(_) => {
                return Err(ProtocolError::Generic(
                    "P2SH addresses not supported".to_string(),
                ));
            }
        };

        let private_key = self.keys.get(hash).ok_or_else(|| {
            ProtocolError::Generic(format!("No private key found for address {:?}", key))
        })?;

        let signature = signer::sign(data, private_key)?;
        Ok(signature.to_vec().into())
    }

    fn sign_create_witness(
        &self,
        key: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let signature = self.sign(key, data)?;
        match key {
            PlatformAddress::P2pkh(_) => Ok(AddressWitness::P2pkh { signature }),
            PlatformAddress::P2sh(_) => Err(ProtocolError::Generic(
                "P2SH addresses not supported".to_string(),
            )),
        }
    }

    fn can_sign_with(&self, key: &PlatformAddress) -> bool {
        match key {
            PlatformAddress::P2pkh(hash) => self.keys.contains_key(hash),
            PlatformAddress::P2sh(_) => false,
        }
    }
}
