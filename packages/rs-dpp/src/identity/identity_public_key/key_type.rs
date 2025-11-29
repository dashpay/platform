use crate::util::hash::ripemd160_sha256;
use anyhow::bail;
use bincode::{Decode, Encode};
#[cfg(feature = "cbor")]
use ciborium::value::Value as CborValue;
#[cfg(feature = "random-public-keys")]
use dashcore::secp256k1::rand::rngs::StdRng as EcdsaRng;
#[cfg(feature = "random-public-keys")]
use dashcore::secp256k1::rand::SeedableRng;
use dashcore::secp256k1::Secp256k1;
use dashcore::Network;
use itertools::Itertools;
use lazy_static::lazy_static;

#[cfg(feature = "bls-signatures")]
use crate::bls_signatures::{self as bls_signatures, Bls12381G2Impl, BlsError};
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

    /// Can this key type be understood as an address on the Core chain?
    pub fn is_core_address_key_type(&self) -> bool {
        match self {
            KeyType::ECDSA_SECP256K1 => false,
            KeyType::BLS12_381 => false,
            KeyType::ECDSA_HASH160 => true,
            KeyType::BIP13_SCRIPT_HASH => true,
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
                let private_key = bls_signatures::SecretKey::<Bls12381G2Impl>::random(rng);
                private_key.public_key().0.to_compressed().to_vec()
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

    /// Gets the public key data for a private key depending on the key type
    pub fn public_key_data_from_private_key_data(
        &self,
        private_key_bytes: &[u8; 32],
        network: Network,
    ) -> Result<Vec<u8>, ProtocolError> {
        match self {
            KeyType::ECDSA_SECP256K1 => {
                let secp = Secp256k1::new();
                let secret_key = dashcore::secp256k1::SecretKey::from_byte_array(private_key_bytes)
                    .map_err(|e| ProtocolError::Generic(e.to_string()))?;
                let private_key = dashcore::PrivateKey::new(secret_key, network);

                Ok(private_key.public_key(&secp).to_bytes())
            }
            KeyType::BLS12_381 => {
                #[cfg(feature = "bls-signatures")]
                {
                    let private_key: Option<bls_signatures::SecretKey<Bls12381G2Impl>> =
                        bls_signatures::SecretKey::<Bls12381G2Impl>::from_be_bytes(
                            private_key_bytes,
                        )
                        .into();
                    if private_key.is_none() {
                        return Err(ProtocolError::BlsError(BlsError::DeserializationError(
                            "private key bytes not a valid secret key".to_string(),
                        )));
                    }
                    let private_key = private_key.expect("expected private key");
                    let public_key_bytes = private_key.public_key().0.to_compressed().to_vec();
                    Ok(public_key_bytes)
                }
                #[cfg(not(feature = "bls-signatures"))]
                return Err(ProtocolError::NotSupported(
                    "Converting a private key to a bls public key is not supported without the bls-signatures feature".to_string(),
                ));
            }
            KeyType::ECDSA_HASH160 => {
                let secp = Secp256k1::new();
                let secret_key = dashcore::secp256k1::SecretKey::from_byte_array(private_key_bytes)
                    .map_err(|e| ProtocolError::Generic(e.to_string()))?;
                let private_key = dashcore::PrivateKey::new(secret_key, network);

                Ok(ripemd160_sha256(private_key.public_key(&secp).to_bytes().as_slice()).to_vec())
            }
            KeyType::EDDSA_25519_HASH160 => {
                #[cfg(feature = "ed25519-dalek")]
                {
                    let key_pair =
                        dashcore::ed25519_dalek::SigningKey::from_bytes(private_key_bytes);
                    Ok(ripemd160_sha256(key_pair.verifying_key().to_bytes().as_slice()).to_vec())
                }
                #[cfg(not(feature = "ed25519-dalek"))]
                return Err(ProtocolError::NotSupported(
                    "Converting a private key to a eddsa hash 160 is not supported without the ed25519-dalek feature".to_string(),
                ));
            }
            KeyType::BIP13_SCRIPT_HASH => Err(ProtocolError::NotSupported(
                "Converting a private key to a script hash is not supported".to_string(),
            )),
        }
    }

    #[cfg(feature = "random-public-keys")]
    /// Gets the default size of the public key
    pub fn random_public_and_private_key_data_v0(&self, rng: &mut StdRng) -> (Vec<u8>, [u8; 32]) {
        match self {
            KeyType::ECDSA_SECP256K1 => {
                let secp = Secp256k1::new();
                let mut rng = EcdsaRng::from_rng(rng).unwrap();
                let secret_key = dashcore::secp256k1::SecretKey::new(&mut rng);
                let private_key = dashcore::PrivateKey::new(secret_key, Network::Dash);
                (
                    private_key.public_key(&secp).to_bytes(),
                    private_key.inner.secret_bytes(),
                )
            }
            KeyType::BLS12_381 => {
                let private_key = dashcore::blsful::SecretKey::<Bls12381G2Impl>::random(rng);
                let public_key_bytes = private_key.public_key().0.to_compressed().to_vec();
                (public_key_bytes, private_key.0.to_be_bytes())
            }
            KeyType::ECDSA_HASH160 => {
                let secp = Secp256k1::new();
                let mut rng = EcdsaRng::from_rng(rng).unwrap();
                let secret_key = dashcore::secp256k1::SecretKey::new(&mut rng);
                let private_key = dashcore::PrivateKey::new(secret_key, Network::Dash);
                (
                    ripemd160_sha256(private_key.public_key(&secp).to_bytes().as_slice()).to_vec(),
                    private_key.inner.secret_bytes(),
                )
            }
            KeyType::EDDSA_25519_HASH160 => {
                let key_pair = dashcore::ed25519_dalek::SigningKey::generate(rng);
                (
                    ripemd160_sha256(key_pair.verifying_key().to_bytes().as_slice()).to_vec(),
                    key_pair.to_bytes(),
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
                    private_key.inner.secret_bytes(),
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
    ) -> Result<(Vec<u8>, [u8; 32]), ProtocolError> {
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
