//! Platform BLS keys and Basic signatures backed by rust-dashcore's dash-pkc primitives.

use dash_pkc::bls::{BlsPublicKey, BlsScChia, BlsScIetf, BlsSecretKey, BlsSignature, Fr};
use rand::{CryptoRng, Rng, RngCore};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use zeroize::Zeroizing;

const fn infinity<const N: usize>() -> [u8; N] {
    let mut bytes = [0; N];
    bytes[0] = 0xc0;
    bytes
}

/// Errors exposed by Platform's BLS operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum BlsError {
    /// An input has an invalid length or encoding.
    #[error("invalid inputs: {0}")]
    InvalidInputs(String),
    /// The signature does not verify.
    #[error("invalid signature")]
    InvalidSignature,
    /// A key or signature could not be decoded.
    #[error("deserialization error: {0}")]
    DeserializationError(String),
}

/// A compressed G1 key, including the identity permitted by historical Platform parsing.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PublicKey([u8; 48]);

impl PublicKey {
    /// Return the canonical compressed IETF encoding.
    pub fn to_bytes(&self) -> [u8; 48] {
        self.0
    }

    /// Re-encode a public key for Core's legacy wire format.
    pub fn to_legacy_bytes(&self) -> Result<[u8; 48], BlsError> {
        if self.0 == infinity() {
            return Ok(infinity());
        }
        self.validated()?
            .to_scheme::<BlsScChia>()
            .map(|key| key.to_bytes())
            .map_err(|_| BlsError::InvalidInputs("Invalid byte sequence".into()))
    }

    fn validated(&self) -> Result<BlsPublicKey<BlsScIetf>, BlsError> {
        BlsPublicKey::from_bytes(&self.0).map_err(|_| BlsError::InvalidSignature)
    }
}

impl Default for PublicKey {
    fn default() -> Self {
        Self(infinity())
    }
}

impl From<&SecretKey> for PublicKey {
    fn from(key: &SecretKey) -> Self {
        key.public_key()
    }
}

impl TryFrom<&[u8]> for PublicKey {
    type Error = BlsError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let bytes = value.try_into().map_err(|_| {
            BlsError::InvalidInputs(format!("Invalid length, expected 48, got {}", value.len()))
        })?;
        // Persisted state and shipped validation accept canonical infinity, but verification rejects it.
        if bytes != infinity() {
            BlsPublicKey::<BlsScIetf>::from_bytes(&bytes)
                .map_err(|_| BlsError::InvalidInputs("Invalid byte sequence".into()))?;
        }
        Ok(Self(bytes))
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Serialize for PublicKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(self.0))
        } else {
            let mut tuple = serializer.serialize_tuple(48)?;
            for byte in self.0 {
                tuple.serialize_element(&byte)?;
            }
            tuple.end()
        }
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            crate::serialization::dashcore::bls_pubkey::deserialize(deserializer)
        } else {
            struct KeyVisitor;
            impl<'de> serde::de::Visitor<'de> for KeyVisitor {
                type Value = PublicKey;
                fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.write_str("48 compressed public key bytes")
                }
                fn visit_seq<A: serde::de::SeqAccess<'de>>(
                    self,
                    mut seq: A,
                ) -> Result<Self::Value, A::Error> {
                    let mut bytes = [0; 48];
                    for (i, byte) in bytes.iter_mut().enumerate() {
                        *byte = seq
                            .next_element()?
                            .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
                    }
                    PublicKey::try_from(bytes.as_slice()).map_err(serde::de::Error::custom)
                }
            }
            deserializer.deserialize_tuple(48, KeyVisitor)
        }
    }
}

/// A secret scalar; the backend clears its storage on drop.
#[derive(Clone, Debug)]
pub struct SecretKey(BlsSecretKey<BlsScIetf>);

impl SecretKey {
    /// Parse a big-endian scalar using Platform's historical nonzero reduction rule.
    pub fn from_be_bytes(bytes: &[u8; 32]) -> Option<Self> {
        let scalar = Zeroizing::new(Fr::from_bendian_reduce(bytes).ok()?);
        BlsSecretKey::try_from(*scalar).ok().map(Self)
    }

    /// Derive a key from at least 32 bytes of keying material using Core's keygen-v3.
    pub fn from_ikm(ikm: &[u8]) -> Result<Self, BlsError> {
        BlsSecretKey::from_ikm(ikm)
            .map(Self)
            .map_err(|e| BlsError::InvalidInputs(e.to_string()))
    }

    /// Draw keying material with Platform's rand interface and derive a secret key.
    pub fn random(mut rng: impl RngCore + CryptoRng) -> Self {
        loop {
            let material = Zeroizing::new(rng.gen::<[u8; 32]>());
            if let Ok(key) = Self::from_ikm(material.as_ref()) {
                return key;
            }
        }
    }

    /// Return the canonical big-endian secret scalar.
    pub fn to_be_bytes(&self) -> [u8; 32] {
        *self.0.to_bytes()
    }

    /// Derive the corresponding public key.
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.public_key().to_bytes())
    }

    /// Sign arbitrary message bytes with the Basic IETF domain separation tag.
    pub fn sign(&self, message: &[u8]) -> Result<Signature, BlsError> {
        Ok(Signature(self.0.sign(message).to_bytes()))
    }
}

/// A compressed G2 Basic signature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Signature([u8; 96]);

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

impl Signature {
    /// Decode a subgroup point, including the identity rejected during verification.
    pub fn from_compressed(bytes: &[u8; 96]) -> Option<Self> {
        if *bytes != infinity() {
            BlsSignature::<BlsScIetf>::from_bytes(bytes).ok()?;
        }
        Some(Self(*bytes))
    }

    /// Return the compressed IETF encoding without a scheme tag.
    pub fn to_bytes(&self) -> [u8; 96] {
        self.0
    }

    /// Verify a Basic signature over arbitrary message bytes.
    pub fn verify(
        &self,
        public_key: &PublicKey,
        message: impl AsRef<[u8]>,
    ) -> Result<(), BlsError> {
        if self.0 == infinity() {
            return Err(BlsError::InvalidInputs(
                "signature is the identity point".into(),
            ));
        }
        if public_key.0 == infinity() {
            return Err(BlsError::InvalidInputs(
                "public key is the identity point".into(),
            ));
        }
        let key = public_key.validated()?;
        let signature =
            BlsSignature::from_bytes(&self.0).map_err(|_| BlsError::InvalidSignature)?;
        key.verify(message.as_ref(), &signature)
            .map_err(|_| BlsError::InvalidSignature)
    }

    /// Aggregate signatures with deterministic public-key weights.
    pub fn aggregate_secure(
        signatures: &[Self],
        public_keys: &[PublicKey],
    ) -> Result<Self, BlsError> {
        let signatures: Result<Vec<_>, _> = signatures
            .iter()
            .map(|sig| BlsSignature::<BlsScIetf>::from_bytes(&sig.0))
            .collect();
        let signatures = signatures.map_err(|_| BlsError::InvalidSignature)?;
        let keys: Vec<_> = public_keys
            .iter()
            .map(PublicKey::validated)
            .collect::<Result<_, _>>()?;
        BlsSignature::secure_aggregate(
            &signatures.iter().collect::<Vec<_>>(),
            &keys.iter().collect::<Vec<_>>(),
        )
        .map(|signature| Self(signature.to_bytes()))
        .map_err(|_| BlsError::InvalidSignature)
    }

    /// Verify a secure aggregate over a common message.
    pub fn verify_secure(
        &self,
        public_keys: &[PublicKey],
        message: impl AsRef<[u8]>,
    ) -> Result<(), BlsError> {
        let keys: Vec<_> = public_keys
            .iter()
            .map(PublicKey::validated)
            .collect::<Result<_, _>>()?;
        let signature =
            BlsSignature::from_bytes(&self.0).map_err(|_| BlsError::InvalidSignature)?;
        signature
            .secure_verify_aggregates(message.as_ref(), &keys.iter().collect::<Vec<_>>())
            .map_err(|_| BlsError::InvalidSignature)
    }
}
