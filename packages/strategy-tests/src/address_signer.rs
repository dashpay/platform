use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::dashcore::hashes::{hash160, Hash};
use dpp::dashcore::secp256k1::{self, PublicKey as SecpPublicKey, Secp256k1, SecretKey};
use dpp::identity::signer::Signer;
use dpp::platform_value::BinaryData;
use dpp::ProtocolError;
use rand::rngs::StdRng;
use rand::RngCore;
use std::collections::BTreeMap;

/// Simple signer that manages randomly generated P2PKH addresses for strategy tests.
#[derive(Debug, Clone)]
pub struct StrategyAddressSigner {
    secp: Secp256k1<secp256k1::All>,
    /// Mapping between P2PKH hashes and their private keys.
    keys: BTreeMap<[u8; 20], SecretKey>,
}

impl Default for StrategyAddressSigner {
    fn default() -> Self {
        Self {
            secp: Secp256k1::new(),
            keys: BTreeMap::new(),
        }
    }
}

impl StrategyAddressSigner {
    /// Creates a new signer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Generates a random P2PKH address backed by a fresh private key.
    pub fn generate_p2pkh(&mut self, rng: &mut StdRng) -> PlatformAddress {
        let mut secret_key_bytes = [0u8; 32];
        loop {
            rng.fill_bytes(&mut secret_key_bytes);
            if let Ok(secret_key) = SecretKey::from_slice(&secret_key_bytes) {
                let public_key = SecpPublicKey::from_secret_key(&self.secp, &secret_key);
                let hash = hash160::Hash::hash(&public_key.serialize());
                let hash_bytes = hash.to_byte_array();
                self.keys.insert(hash_bytes, secret_key);
                return PlatformAddress::P2pkh(hash_bytes);
            }
        }
    }

    fn secret_key_for(&self, key: &PlatformAddress) -> Result<&SecretKey, ProtocolError> {
        if let PlatformAddress::P2pkh(hash) = key {
            self.keys.get(hash).ok_or_else(|| {
                ProtocolError::Generic(format!(
                    "StrategyAddressSigner missing private key for address {:?}",
                    key
                ))
            })
        } else {
            Err(ProtocolError::Generic(
                "StrategyAddressSigner only supports P2PKH addresses".to_string(),
            ))
        }
    }

    fn sign_internal(&self, key: &PlatformAddress, data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let secret_key = self.secret_key_for(key)?;
        let signature = dpp::dashcore::signer::sign(data, secret_key.as_ref())?;
        Ok(signature.to_vec())
    }
}

impl Signer<PlatformAddress> for StrategyAddressSigner {
    fn sign(&self, key: &PlatformAddress, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        Ok(BinaryData::new(self.sign_internal(key, data)?))
    }

    fn sign_create_witness(
        &self,
        key: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let signature = self.sign_internal(key, data)?;
        Ok(AddressWitness::P2pkh {
            signature: BinaryData::new(signature),
        })
    }

    fn can_sign_with(&self, key: &PlatformAddress) -> bool {
        matches!(key, PlatformAddress::P2pkh(hash) if self.keys.contains_key(hash))
    }
}
