#[cfg(feature = "random-public-keys")]
use crate::util::hash::ripemd160_sha256;
use anyhow::bail;
use bincode::{Decode, Encode};
#[cfg(feature = "cbor")]
use ciborium::value::Value as CborValue;
#[cfg(feature = "random-public-keys")]
use dashcore::secp256k1::rand::rngs::StdRng as EcdsaRng;
#[cfg(feature = "random-public-keys")]
use dashcore::secp256k1::rand::SeedableRng;
#[cfg(feature = "random-public-keys")]
use dashcore::secp256k1::Secp256k1;
#[cfg(feature = "random-public-keys")]
use dashcore::Network;
use itertools::Itertools;
use lazy_static::lazy_static;

use crate::fee::Credits;
use crate::version::PlatformVersion;
use crate::ProtocolError;
#[cfg(feature = "random-public-keys")]
use rand::rngs::StdRng;
#[cfg(feature = "random-public-keys")]
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
    Default,
    strum::EnumIter,
)]
pub enum KeyType {
    #[default]
    ECDSA_SECP256K1 = 0,
    BLS12_381 = 1,
    ECDSA_HASH160 = 2,
    BIP13_SCRIPT_HASH = 3,
    EDDSA_25519_HASH160 = 4,
}

lazy_static! {
    static ref KEY_TYPE_SIZES: HashMap<KeyType, usize> = [
        (KeyType::ECDSA_SECP256K1, 33),
        (KeyType::BLS12_381, 48),
        (KeyType::ECDSA_HASH160, 20),
        (KeyType::BIP13_SCRIPT_HASH, 20),
        (KeyType::EDDSA_25519_HASH160, 20)
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

    /// All key types
    pub fn all_key_types() -> [KeyType; 5] {
        [
            Self::ECDSA_SECP256K1,
            Self::BLS12_381,
            Self::ECDSA_HASH160,
            Self::BIP13_SCRIPT_HASH,
            Self::EDDSA_25519_HASH160,
        ]
    }

    /// Are keys of this type unique?
    pub fn is_unique_key_type(&self) -> bool {
        match self {
            KeyType::ECDSA_SECP256K1 => true,
            KeyType::BLS12_381 => true,
            KeyType::ECDSA_HASH160 => false,
            KeyType::BIP13_SCRIPT_HASH => false,
            KeyType::EDDSA_25519_HASH160 => false,
        }
    }

    pub fn signature_verify_cost(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        match platform_version.dpp.costs.signature_verify {
            0 => Ok(match self {
                KeyType::ECDSA_SECP256K1 => {
                    platform_version
                        .fee_version
                        .signature
                        .verify_signature_ecdsa_secp256k1
                }
                KeyType::BLS12_381 => {
                    platform_version
                        .fee_version
                        .signature
                        .verify_signature_bls12_381
                }
                KeyType::ECDSA_HASH160 => {
                    platform_version
                        .fee_version
                        .signature
                        .verify_signature_ecdsa_hash160
                }
                KeyType::BIP13_SCRIPT_HASH => {
                    platform_version
                        .fee_version
                        .signature
                        .verify_signature_bip13_script_hash
                }
                KeyType::EDDSA_25519_HASH160 => {
                    platform_version
                        .fee_version
                        .signature
                        .verify_signature_eddsa25519_hash160
                }
            }),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "KeyType::signature_verify_cost".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[cfg(feature = "random-public-keys")]
    /// Gets the default size of the public key
    fn random_public_key_data_v0(&self, rng: &mut StdRng) -> Vec<u8> {
        match self {
            KeyType::ECDSA_SECP256K1 => {
                let secp = Secp256k1::new();
                let mut rng = EcdsaRng::from_rng(rng).unwrap();
                let secret_key = dashcore::secp256k1::SecretKey::new(&mut rng);
                let private_key = dashcore::PrivateKey::new(secret_key, Network::Dash);
                private_key.public_key(&secp).to_bytes()
            }
            KeyType::BLS12_381 => {
                let private_key = bls_signatures::PrivateKey::generate_dash(rng)
                    .expect("expected to generate a bls private key"); // we assume this will never error
                private_key
                    .g1_element()
                    .expect("expected to get a public key from a bls private key")
                    .to_bytes()
                    .to_vec()
            }
            KeyType::ECDSA_HASH160 | KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => {
                (0..self.default_size()).map(|_| rng.gen::<u8>()).collect()
            }
        }
    }

    #[cfg(feature = "random-public-keys")]
    /// Gets the default size of the public key
    pub fn random_public_key_data(
        &self,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_type_method_versions
            .random_public_key_data
        {
            0 => Ok(self.random_public_key_data_v0(rng)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "KeyType::random_public_key_data".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[cfg(feature = "random-public-keys")]
    /// Gets the default size of the public key
    pub fn random_public_and_private_key_data_v0(&self, rng: &mut StdRng) -> (Vec<u8>, Vec<u8>) {
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
                let private_key = bls_signatures::PrivateKey::generate_dash(rng)
                    .expect("expected to generate a bls private key"); // we assume this will never error
                let public_key_bytes = private_key
                    .g1_element()
                    .expect("expected to get a public key from a bls private key")
                    .to_bytes()
                    .to_vec();
                (public_key_bytes, private_key.to_bytes().to_vec())
            }
            KeyType::ECDSA_HASH160 => {
                let secp = Secp256k1::new();
                let mut rng = EcdsaRng::from_rng(rng).unwrap();
                let secret_key = dashcore::secp256k1::SecretKey::new(&mut rng);
                let private_key = dashcore::PrivateKey::new(secret_key, Network::Dash);
                (
                    ripemd160_sha256(private_key.public_key(&secp).to_bytes().as_slice()).to_vec(),
                    private_key.to_bytes(),
                )
            }
            KeyType::EDDSA_25519_HASH160 => {
                let key_pair = ed25519_dalek::SigningKey::generate(rng);
                (
                    key_pair.verifying_key().to_bytes().to_vec(),
                    key_pair.to_bytes().to_vec(),
                )
            }
            KeyType::BIP13_SCRIPT_HASH => {
                //todo (using ECDSA_HASH160 for now)
                let secp = Secp256k1::new();
                let mut rng = EcdsaRng::from_rng(rng).unwrap();
                let secret_key = dashcore::secp256k1::SecretKey::new(&mut rng);
                let private_key = dashcore::PrivateKey::new(secret_key, Network::Dash);
                (
                    ripemd160_sha256(private_key.public_key(&secp).to_bytes().as_slice()).to_vec(),
                    private_key.to_bytes(),
                )
            }
        }
    }

    #[cfg(feature = "random-public-keys")]
    /// Gets the default size of the public key
    pub fn random_public_and_private_key_data(
        &self,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<u8>, Vec<u8>), ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_type_method_versions
            .random_public_and_private_key_data
        {
            0 => Ok(self.random_public_and_private_key_data_v0(rng)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "KeyType::random_public_and_private_key_data".to_string(),
                known_versions: vec![0],
                received: version,
            }),
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
            4 => Ok(Self::EDDSA_25519_HASH160),
            value => bail!("unrecognized key type: {}", value),
        }
    }
}

#[cfg(feature = "cbor")]
impl Into<CborValue> for KeyType {
    fn into(self) -> CborValue {
        CborValue::from(self as u128)
    }
}
