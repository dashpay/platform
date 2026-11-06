use crate::util::hash::ripemd160_sha256;
use anyhow::bail;
use bincode::{Decode, DecodeUntrusted, Encode};
#[cfg(feature = "cbor")]
use ciborium::value::Value as CborValue;
use dashcore::Network;
use itertools::Itertools;
use lazy_static::lazy_static;
#[cfg(feature = "random-public-keys")]
use secp256k1::rand::rngs::StdRng as EcdsaRng;
#[cfg(feature = "random-public-keys")]
use secp256k1::rand::SeedableRng;

#[cfg(feature = "bls-signatures")]
use crate::bls::{self, BlsError};
use crate::fee::Credits;
#[cfg(feature = "redpallas")]
use crate::identity::identity_public_key::redpallas_hecate::RedPallasHecateSecretKey;
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
    DecodeUntrusted,
)]
pub enum KeyType {
    #[default]
    ECDSA_SECP256K1 = 0,
    BLS12_381 = 1,
    ECDSA_HASH160 = 2,
    BIP13_SCRIPT_HASH = 3,
    EDDSA_25519_HASH160 = 4,
    /// A Hecate signer key: a RedPallas spend-authorization key `ak` and a commitment to a
    /// separate tag key, 64 bytes (see
    /// [`redpallas_hecate`](crate::identity::identity_public_key::redpallas_hecate)). Only for
    /// [`Purpose::HECATE_SIGNER`](crate::identity::Purpose::HECATE_SIGNER), from protocol
    /// version 15.
    REDPALLAS_HECATE = 5,
}

lazy_static! {
    static ref KEY_TYPE_SIZES: HashMap<KeyType, usize> = [
        (KeyType::ECDSA_SECP256K1, 33),
        (KeyType::BLS12_381, 48),
        (KeyType::ECDSA_HASH160, 20),
        (KeyType::BIP13_SCRIPT_HASH, 20),
        (KeyType::EDDSA_25519_HASH160, 20),
        (KeyType::REDPALLAS_HECATE, 64)
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

    /// The size in bytes of a private key of this type: the bytes that
    /// [`public_key_data_from_private_key_data`](Self::public_key_data_from_private_key_data),
    /// `IdentityPublicKey::validate_private_key_bytes` and `sign_by_private_key` take. SDKs and
    /// FFI read the size from here instead of keeping their own.
    pub const fn private_key_size(&self) -> usize {
        match self {
            // The secp256k1 secret scalar (`secp256k1::SecretKey::from_secret_bytes`).
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => 32,
            // The BLS12-381 secret scalar, big-endian (`bls::SecretKey::from_be_bytes`).
            KeyType::BLS12_381 => 32,
            // A script hash has no private key and cannot sign; its slot takes the secp256k1
            // secret scalar, the key `random_public_and_private_key_data` draws for it.
            KeyType::BIP13_SCRIPT_HASH => 32,
            // The Ed25519 secret key (`ed25519_dalek::SigningKey::from_bytes`).
            KeyType::EDDSA_25519_HASH160 => 32,
            // The Hecate secret (`RedPallasHecateSecretKey::from_secret`), from which the
            // RedPallas signing key and the tag key derive.
            KeyType::REDPALLAS_HECATE => 32,
        }
    }

    /// All key types
    pub fn all_key_types() -> [KeyType; 6] {
        [
            Self::ECDSA_SECP256K1,
            Self::BLS12_381,
            Self::ECDSA_HASH160,
            Self::BIP13_SCRIPT_HASH,
            Self::EDDSA_25519_HASH160,
            Self::REDPALLAS_HECATE,
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
            // Its data is a public key, so one key belongs to one identity, and a signer leaf
            // to one identity.
            KeyType::REDPALLAS_HECATE => true,
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
            KeyType::REDPALLAS_HECATE => false,
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
                KeyType::REDPALLAS_HECATE => {
                    platform_version
                        .fee_version
                        .signature
                        .verify_signature_redpallas
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
                let mut seed = [0u8; 32];
                rng.fill(&mut seed);
                let mut rng = EcdsaRng::from_seed(seed);
                let secret_key = secp256k1::SecretKey::new(&mut rng);
                let public_key = secp256k1::PublicKey::from_secret_key(&secret_key);
                public_key.serialize().to_vec()
            }
            KeyType::BLS12_381 => {
                let private_key = bls::SecretKey::random(rng);
                private_key.public_key().to_bytes().to_vec()
            }
            KeyType::ECDSA_HASH160 | KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => {
                (0..self.default_size()).map(|_| rng.gen::<u8>()).collect()
            }
            KeyType::REDPALLAS_HECATE => random_redpallas_hecate_key(rng).0,
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
        _network: Network,
    ) -> Result<Vec<u8>, ProtocolError> {
        match self {
            KeyType::ECDSA_SECP256K1 => {
                let secret_key = secp256k1::SecretKey::from_secret_bytes(*private_key_bytes)
                    .map_err(|e| ProtocolError::Generic(e.to_string()))?;
                let public_key = secp256k1::PublicKey::from_secret_key(&secret_key);

                Ok(public_key.serialize().to_vec())
            }
            KeyType::BLS12_381 => {
                #[cfg(feature = "bls-signatures")]
                {
                    let private_key: Option<bls::SecretKey> =
                        bls::SecretKey::from_be_bytes(private_key_bytes);
                    if private_key.is_none() {
                        return Err(ProtocolError::BlsError(BlsError::DeserializationError(
                            "private key bytes not a valid secret key".to_string(),
                        )));
                    }
                    let private_key = private_key.expect("expected private key");
                    let public_key_bytes = private_key.public_key().to_bytes().to_vec();
                    Ok(public_key_bytes)
                }
                #[cfg(not(feature = "bls-signatures"))]
                return Err(ProtocolError::NotSupported(
                    "Converting a private key to a bls public key is not supported without the bls-signatures feature".to_string(),
                ));
            }
            KeyType::ECDSA_HASH160 => {
                let secret_key = secp256k1::SecretKey::from_secret_bytes(*private_key_bytes)
                    .map_err(|e| ProtocolError::Generic(e.to_string()))?;
                let public_key = secp256k1::PublicKey::from_secret_key(&secret_key);

                Ok(ripemd160_sha256(public_key.serialize().as_slice()).to_vec())
            }
            KeyType::EDDSA_25519_HASH160 => {
                #[cfg(feature = "ed25519-dalek")]
                {
                    let key_pair = crate::ed25519_dalek::SigningKey::from_bytes(private_key_bytes);
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
            KeyType::REDPALLAS_HECATE => {
                #[cfg(feature = "redpallas")]
                {
                    Ok(RedPallasHecateSecretKey::from_secret(private_key_bytes)?
                        .public_key_data()?
                        .to_vec())
                }
                #[cfg(not(feature = "redpallas"))]
                return Err(ProtocolError::NotSupported(
                    "Converting a private key to a REDPALLAS_HECATE key is not supported without the redpallas feature".to_string(),
                ));
            }
        }
    }

    #[cfg(feature = "random-public-keys")]
    /// Gets the default size of the public key
    pub fn random_public_and_private_key_data_v0(&self, rng: &mut StdRng) -> (Vec<u8>, [u8; 32]) {
        match self {
            KeyType::ECDSA_SECP256K1 => {
                let mut seed = [0u8; 32];
                rng.fill(&mut seed);
                let mut rng = EcdsaRng::from_seed(seed);
                let secret_key = secp256k1::SecretKey::new(&mut rng);
                let public_key = secp256k1::PublicKey::from_secret_key(&secret_key);
                (
                    public_key.serialize().to_vec(),
                    secret_key.to_secret_bytes(),
                )
            }
            KeyType::BLS12_381 => {
                let private_key = crate::bls::SecretKey::random(rng);
                let public_key_bytes = private_key.public_key().to_bytes().to_vec();
                (public_key_bytes, private_key.to_be_bytes())
            }
            KeyType::ECDSA_HASH160 => {
                let mut seed = [0u8; 32];
                rng.fill(&mut seed);
                let mut rng = EcdsaRng::from_seed(seed);
                let secret_key = secp256k1::SecretKey::new(&mut rng);
                let public_key = secp256k1::PublicKey::from_secret_key(&secret_key);
                (
                    ripemd160_sha256(public_key.serialize().as_slice()).to_vec(),
                    secret_key.to_secret_bytes(),
                )
            }
            KeyType::EDDSA_25519_HASH160 => {
                let key_pair = crate::ed25519_dalek::SigningKey::generate(rng);
                (
                    ripemd160_sha256(key_pair.verifying_key().to_bytes().as_slice()).to_vec(),
                    key_pair.to_bytes(),
                )
            }
            KeyType::BIP13_SCRIPT_HASH => {
                //todo (using ECDSA_HASH160 for now)
                let mut seed = [0u8; 32];
                rng.fill(&mut seed);
                let mut rng = EcdsaRng::from_seed(seed);
                let secret_key = secp256k1::SecretKey::new(&mut rng);
                let public_key = secp256k1::PublicKey::from_secret_key(&secret_key);
                (
                    ripemd160_sha256(public_key.serialize().as_slice()).to_vec(),
                    secret_key.to_secret_bytes(),
                )
            }
            KeyType::REDPALLAS_HECATE => random_redpallas_hecate_key(rng),
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
            5 => Ok(Self::REDPALLAS_HECATE),
            value => bail!("unrecognized key type: {}", value),
        }
    }
}

/// A random Hecate signer key: its data and its 32-byte private key. Draws secrets until one
/// derives a nonzero signing key, which the first does but with negligible probability.
#[cfg(feature = "random-public-keys")]
fn random_redpallas_hecate_key(rng: &mut StdRng) -> (Vec<u8>, [u8; 32]) {
    loop {
        let mut secret = [0u8; 32];
        rng.fill(&mut secret);
        if let Ok(data) =
            RedPallasHecateSecretKey::from_secret(&secret).and_then(|key| key.public_key_data())
        {
            return (data.to_vec(), secret);
        }
    }
}

#[cfg(feature = "cbor")]
impl Into<CborValue> for KeyType {
    fn into(self) -> CborValue {
        CborValue::from(self as u128)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- default_size() --

    #[test]
    fn test_default_size_ecdsa_secp256k1() {
        assert_eq!(KeyType::ECDSA_SECP256K1.default_size(), 33);
    }

    #[test]
    fn test_default_size_bls12_381() {
        assert_eq!(KeyType::BLS12_381.default_size(), 48);
    }

    #[test]
    fn test_default_size_ecdsa_hash160() {
        assert_eq!(KeyType::ECDSA_HASH160.default_size(), 20);
    }

    #[test]
    fn test_default_size_bip13_script_hash() {
        assert_eq!(KeyType::BIP13_SCRIPT_HASH.default_size(), 20);
    }

    #[test]
    fn test_default_size_eddsa_25519_hash160() {
        assert_eq!(KeyType::EDDSA_25519_HASH160.default_size(), 20);
    }

    // -- all_key_types() --

    #[test]
    fn test_all_key_types_has_six_elements() {
        let types = KeyType::all_key_types();
        assert_eq!(types.len(), 6);
    }

    #[test]
    fn test_all_key_types_contains_all_variants() {
        let types = KeyType::all_key_types();
        assert_eq!(
            types,
            [
                KeyType::ECDSA_SECP256K1,
                KeyType::BLS12_381,
                KeyType::ECDSA_HASH160,
                KeyType::BIP13_SCRIPT_HASH,
                KeyType::EDDSA_25519_HASH160,
                KeyType::REDPALLAS_HECATE,
            ]
        );
    }

    // -- is_unique_key_type() --

    #[test]
    fn test_ecdsa_secp256k1_is_unique() {
        assert!(KeyType::ECDSA_SECP256K1.is_unique_key_type());
    }

    #[test]
    fn test_bls12_381_is_unique() {
        assert!(KeyType::BLS12_381.is_unique_key_type());
    }

    #[test]
    fn test_ecdsa_hash160_is_not_unique() {
        assert!(!KeyType::ECDSA_HASH160.is_unique_key_type());
    }

    #[test]
    fn test_bip13_script_hash_is_not_unique() {
        assert!(!KeyType::BIP13_SCRIPT_HASH.is_unique_key_type());
    }

    #[test]
    fn test_eddsa_25519_hash160_is_not_unique() {
        assert!(!KeyType::EDDSA_25519_HASH160.is_unique_key_type());
    }

    // -- is_core_address_key_type() --

    #[test]
    fn test_ecdsa_secp256k1_not_core_address() {
        assert!(!KeyType::ECDSA_SECP256K1.is_core_address_key_type());
    }

    #[test]
    fn test_bls12_381_not_core_address() {
        assert!(!KeyType::BLS12_381.is_core_address_key_type());
    }

    #[test]
    fn test_ecdsa_hash160_is_core_address() {
        assert!(KeyType::ECDSA_HASH160.is_core_address_key_type());
    }

    #[test]
    fn test_bip13_script_hash_is_core_address() {
        assert!(KeyType::BIP13_SCRIPT_HASH.is_core_address_key_type());
    }

    #[test]
    fn test_eddsa_25519_hash160_not_core_address() {
        assert!(!KeyType::EDDSA_25519_HASH160.is_core_address_key_type());
    }

    // -- TryFrom<u8> valid --

    #[test]
    fn test_try_from_u8_ecdsa_secp256k1() {
        assert_eq!(KeyType::try_from(0u8).unwrap(), KeyType::ECDSA_SECP256K1);
    }

    #[test]
    fn test_try_from_u8_bls12_381() {
        assert_eq!(KeyType::try_from(1u8).unwrap(), KeyType::BLS12_381);
    }

    #[test]
    fn test_try_from_u8_ecdsa_hash160() {
        assert_eq!(KeyType::try_from(2u8).unwrap(), KeyType::ECDSA_HASH160);
    }

    #[test]
    fn test_try_from_u8_bip13_script_hash() {
        assert_eq!(KeyType::try_from(3u8).unwrap(), KeyType::BIP13_SCRIPT_HASH);
    }

    #[test]
    fn test_try_from_u8_eddsa_25519_hash160() {
        assert_eq!(
            KeyType::try_from(4u8).unwrap(),
            KeyType::EDDSA_25519_HASH160
        );
    }

    // -- TryFrom<u8> invalid --

    #[test]
    fn test_try_from_u8_redpallas_hecate() {
        assert_eq!(KeyType::try_from(5u8).unwrap(), KeyType::REDPALLAS_HECATE);
    }

    #[test]
    fn test_try_from_u8_invalid_6() {
        assert!(KeyType::try_from(6u8).is_err());
    }

    #[test]
    fn should_size_a_redpallas_hecate_key_at_64_bytes_unique_and_not_a_core_address() {
        assert_eq!(KeyType::REDPALLAS_HECATE.default_size(), 64);
        assert!(KeyType::REDPALLAS_HECATE.is_unique_key_type());
        assert!(!KeyType::REDPALLAS_HECATE.is_core_address_key_type());
    }

    #[cfg(feature = "random-public-keys")]
    #[test]
    fn should_derive_a_redpallas_hecate_key_from_its_random_private_key() {
        let mut rng = <StdRng as rand::SeedableRng>::seed_from_u64(5);
        let (data, private_key) = KeyType::REDPALLAS_HECATE
            .random_public_and_private_key_data(&mut rng, PlatformVersion::latest())
            .unwrap();
        assert_eq!(data.len(), 64);
        assert_eq!(
            KeyType::REDPALLAS_HECATE
                .public_key_data_from_private_key_data(&private_key, Network::Testnet)
                .unwrap(),
            data
        );
    }

    #[test]
    fn test_try_from_u8_invalid_255() {
        assert!(KeyType::try_from(255u8).is_err());
    }

    // -- private_key_size() --

    #[cfg(feature = "random-public-keys")]
    #[test]
    fn should_size_each_key_types_private_key_as_the_private_key_apis_take_it() {
        let mut rng = <StdRng as rand::SeedableRng>::seed_from_u64(32);
        for key_type in KeyType::all_key_types() {
            let (data, private_key) = key_type
                .random_public_and_private_key_data(&mut rng, PlatformVersion::latest())
                .unwrap();
            assert_eq!(private_key.len(), key_type.private_key_size(), "{key_type}");
            // A script hash has no key to derive from its private key slot.
            if key_type != KeyType::BIP13_SCRIPT_HASH {
                assert_eq!(
                    key_type
                        .public_key_data_from_private_key_data(&private_key, Network::Testnet)
                        .unwrap(),
                    data,
                    "{key_type}"
                );
            }
        }
    }

    // -- Display --

    #[test]
    fn test_display_ecdsa_secp256k1() {
        assert_eq!(format!("{}", KeyType::ECDSA_SECP256K1), "ECDSA_SECP256K1");
    }

    #[test]
    fn test_display_bls12_381() {
        assert_eq!(format!("{}", KeyType::BLS12_381), "BLS12_381");
    }

    #[test]
    fn test_display_ecdsa_hash160() {
        assert_eq!(format!("{}", KeyType::ECDSA_HASH160), "ECDSA_HASH160");
    }

    #[test]
    fn test_display_bip13_script_hash() {
        assert_eq!(
            format!("{}", KeyType::BIP13_SCRIPT_HASH),
            "BIP13_SCRIPT_HASH"
        );
    }

    #[test]
    fn test_display_eddsa_25519_hash160() {
        assert_eq!(
            format!("{}", KeyType::EDDSA_25519_HASH160),
            "EDDSA_25519_HASH160"
        );
    }

    // -- Default --

    #[test]
    fn test_default_is_ecdsa_secp256k1() {
        assert_eq!(KeyType::default(), KeyType::ECDSA_SECP256K1);
    }

    // -- round-trip: u8 -> KeyType -> u8 --

    #[test]
    fn test_round_trip_all_valid() {
        for val in 0u8..=5 {
            let key_type = KeyType::try_from(val).unwrap();
            assert_eq!(key_type as u8, val);
        }
    }

    // -- unique vs core address are complementary for full-size key types --

    #[test]
    fn test_unique_and_core_address_are_mutually_exclusive() {
        for kt in KeyType::all_key_types() {
            // A key type should not be both unique and a core address key type
            assert!(!(kt.is_unique_key_type() && kt.is_core_address_key_type()));
        }
    }
}
