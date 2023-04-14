use crate::util::hash::ripemd160_sha256;
use anyhow::bail;
use bincode::{Decode, Encode};
use bls_signatures::Serialize;
use ciborium::value::Value as CborValue;
use dashcore::secp256k1::rand::rngs::StdRng as EcdsaRng;
use dashcore::secp256k1::rand::SeedableRng;
use dashcore::secp256k1::Secp256k1;
use dashcore::Network;
use itertools::Itertools;
use lazy_static::lazy_static;
use rand::rngs::StdRng;
use rand::Rng;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;
use std::convert::TryFrom;

#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Serialize_repr,
    Deserialize_repr,
    Hash,
    Ord,
    PartialOrd,
    Encode,
    Decode,
)]
pub enum KeyType {
    ECDSA_SECP256K1 = 0,
    BLS12_381 = 1,
    ECDSA_HASH160 = 2,
    BIP13_SCRIPT_HASH = 3,
    // TODO: re-enable this keys
    // EDDSA_25519 = 4,
    // EDDSA_25519_HASH160 = 5,
}

lazy_static! {
    static ref KEY_TYPE_SIZES: HashMap<KeyType, usize> = vec![
        (KeyType::ECDSA_SECP256K1, 33),
        (KeyType::BLS12_381, 48),
        (KeyType::ECDSA_HASH160, 20),
        (KeyType::BIP13_SCRIPT_HASH, 20)
    ]
    .iter()
    .copied()
    .collect();
    pub static ref KEY_TYPE_MAX_SIZE_TYPE: KeyType = KEY_TYPE_SIZES
        .iter()
        .sorted_by(|a, b| Ord::cmp(&b.1, &a.1))
        .last()
        .map(|(key_type, _)| *key_type)
        .unwrap();
}

impl KeyType {
    /// Gets the default size of the public key
    pub fn default_size(&self) -> usize {
        KEY_TYPE_SIZES[self]
    }

    //todo: put this in a specific feature
    /// Gets the default size of the public key
    pub fn random_public_key_data(&self, rng: &mut StdRng) -> Vec<u8> {
        match self {
            KeyType::ECDSA_SECP256K1 => {
                let secp = Secp256k1::new();
                let mut rng = EcdsaRng::from_rng(rng).unwrap();
                let secret_key = dashcore::secp256k1::SecretKey::new(&mut rng);
                let private_key = dashcore::PrivateKey::new(secret_key, Network::Dash);
                private_key.public_key(&secp).to_bytes()
            }
            KeyType::BLS12_381 => {
                let private_key = bls_signatures::PrivateKey::generate(rng);
                private_key.public_key().as_bytes()
            }
            KeyType::ECDSA_HASH160 | KeyType::BIP13_SCRIPT_HASH => {
                (0..self.default_size()).map(|_| rng.gen::<u8>()).collect()
            }
        }
    }

    //todo: put this in a specific feature
    /// Gets the default size of the public key
    pub fn random_public_and_private_key_data(&self, rng: &mut StdRng) -> (Vec<u8>, Vec<u8>) {
        match self {
            KeyType::ECDSA_SECP256K1 => {
                let secp = Secp256k1::new();
                let mut rng = EcdsaRng::from_rng(rng).unwrap();
                let secret_key = dashcore::secp256k1::SecretKey::new(&mut rng);
                let private_key = dashcore::PrivateKey::new(secret_key, Network::Dash);
                (
                    private_key.public_key(&secp).to_bytes(),
                    private_key.to_bytes(),
                )
            }
            KeyType::BLS12_381 => {
                let private_key = bls_signatures::PrivateKey::generate(rng);
                (private_key.public_key().as_bytes(), private_key.as_bytes())
            }
            KeyType::ECDSA_HASH160 => {
                let secp = Secp256k1::new();
                let mut rng = EcdsaRng::from_rng(rng).unwrap();
                let secret_key = dashcore::secp256k1::SecretKey::new(&mut rng);
                let private_key = dashcore::PrivateKey::new(secret_key, Network::Dash);
                (
                    ripemd160_sha256(private_key.public_key(&secp).to_bytes().as_slice()),
                    private_key.to_bytes(),
                )
            }
            KeyType::BIP13_SCRIPT_HASH => {
                //todo (using ECDSA_HASH160 for now)
                let secp = Secp256k1::new();
                let mut rng = EcdsaRng::from_rng(rng).unwrap();
                let secret_key = dashcore::secp256k1::SecretKey::new(&mut rng);
                let private_key = dashcore::PrivateKey::new(secret_key, Network::Dash);
                (
                    ripemd160_sha256(private_key.public_key(&secp).to_bytes().as_slice()),
                    private_key.to_bytes(),
                )
            }
        }
    }
}

impl std::fmt::Display for KeyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TryFrom<u8> for KeyType {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::ECDSA_SECP256K1),
            1 => Ok(Self::BLS12_381),
            2 => Ok(Self::ECDSA_HASH160),
            3 => Ok(Self::BIP13_SCRIPT_HASH),
            value => bail!("unrecognized key type: {}", value),
        }
    }
}

impl Into<CborValue> for KeyType {
    fn into(self) -> CborValue {
        CborValue::from(self as u128)
    }
}
