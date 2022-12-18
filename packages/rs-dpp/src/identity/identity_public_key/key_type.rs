use anyhow::bail;
use bls_signatures::Serialize;
use ciborium::value::Value as CborValue;
use dashcore::secp256k1::rand::rngs::StdRng as EcdsaRng;
use dashcore::secp256k1::rand::SeedableRng;
use dashcore::secp256k1::Secp256k1;
use dashcore::Network;
use rand::rngs::StdRng;
use rand::Rng;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::convert::TryFrom;

#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize_repr, Deserialize_repr, Hash)]
pub enum KeyType {
    ECDSA_SECP256K1 = 0,
    BLS12_381 = 1,
    ECDSA_HASH160 = 2,
    BIP13_SCRIPT_HASH = 3,
}

impl KeyType {
    /// Gets the default size of the public key
    pub fn default_size(&self) -> usize {
        match self {
            KeyType::ECDSA_SECP256K1 => 33,
            KeyType::BLS12_381 => 48,
            KeyType::ECDSA_HASH160 => 32,
            KeyType::BIP13_SCRIPT_HASH => 32,
        }
    }

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
}

impl std::fmt::Display for KeyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
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
