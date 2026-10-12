//! Platform BLS keys and Basic signatures backed by rust-dashcore's dash-pkc primitives.

use dash_pkc::bls::{BlsPublicKey, BlsScChia, BlsScIetf, BlsSecretKey, BlsSignature, Fr};
use rand::{CryptoRng, Rng, RngCore};
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

/// A validated G1 key, including the identity permitted by historical Platform parsing.
#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct PublicKey(PublicKeyPoint);

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum PublicKeyPoint {
    #[default]
    Infinity,
    Validated(BlsPublicKey<BlsScIetf>),
}

impl PublicKey {
    /// Return the canonical compressed IETF encoding.
    pub fn to_bytes(&self) -> [u8; 48] {
        match &self.0 {
            PublicKeyPoint::Infinity => infinity(),
            PublicKeyPoint::Validated(key) => key.to_bytes(),
        }
    }

    /// Re-encode a public key for Core's legacy wire format.
    pub fn to_legacy_bytes(&self) -> Result<[u8; 48], BlsError> {
        let PublicKeyPoint::Validated(key) = &self.0 else {
            return Ok(infinity());
        };
        key.to_scheme::<BlsScChia>()
            .map(|key| key.to_bytes())
            .map_err(|_| BlsError::InvalidInputs("Invalid byte sequence".into()))
    }

    fn validated(&self) -> Result<&BlsPublicKey<BlsScIetf>, BlsError> {
        match &self.0 {
            PublicKeyPoint::Infinity => Err(BlsError::InvalidSignature),
            PublicKeyPoint::Validated(key) => Ok(key),
        }
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
        if bytes == infinity() {
            return Ok(Self(PublicKeyPoint::Infinity));
        }
        BlsPublicKey::from_bytes(&bytes)
            .map(|key| Self(PublicKeyPoint::Validated(key)))
            .map_err(|_| BlsError::InvalidInputs("Invalid byte sequence".into()))
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.to_bytes()))
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
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
        PublicKey(PublicKeyPoint::Validated(self.0.public_key()))
    }

    /// Sign arbitrary message bytes with the Basic IETF domain separation tag.
    pub fn sign(&self, message: &[u8]) -> Result<Signature, BlsError> {
        Ok(Signature::from_validated(self.0.sign(message)))
    }
}

/// A validated G2 Basic signature, with canonical infinity represented separately.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Signature(Option<BlsSignature<BlsScIetf>>);

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Signature").field(&self.to_bytes()).finish()
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.to_bytes()))
    }
}

impl Signature {
    /// Decode a subgroup point, including the identity rejected during verification.
    pub fn from_compressed(bytes: &[u8; 96]) -> Option<Self> {
        if *bytes == infinity() {
            return Some(Self(None));
        }
        BlsSignature::from_bytes(bytes)
            .ok()
            .map(|signature| Self(Some(signature)))
    }

    fn from_validated(signature: BlsSignature<BlsScIetf>) -> Self {
        // Aggregation can produce infinity; keep its historical parsing and verification policy.
        Self((signature.to_bytes() != infinity()).then_some(signature))
    }

    /// Return the compressed IETF encoding without a scheme tag.
    pub fn to_bytes(&self) -> [u8; 96] {
        self.0
            .as_ref()
            .map_or_else(infinity, BlsSignature::to_bytes)
    }

    /// Verify a Basic signature over arbitrary message bytes.
    pub fn verify(
        &self,
        public_key: &PublicKey,
        message: impl AsRef<[u8]>,
    ) -> Result<(), BlsError> {
        let signature = self
            .0
            .as_ref()
            .ok_or_else(|| BlsError::InvalidInputs("signature is the identity point".into()))?;
        let PublicKeyPoint::Validated(key) = &public_key.0 else {
            return Err(BlsError::InvalidInputs(
                "public key is the identity point".into(),
            ));
        };
        key.verify(message.as_ref(), signature)
            .map_err(|_| BlsError::InvalidSignature)
    }

    /// Aggregate signatures with deterministic public-key weights.
    pub fn aggregate_secure(
        signatures: &[Self],
        public_keys: &[PublicKey],
    ) -> Result<Self, BlsError> {
        let signatures: Vec<_> = signatures
            .iter()
            .map(|sig| sig.0.as_ref().ok_or(BlsError::InvalidSignature))
            .collect::<Result<_, _>>()?;
        let keys: Vec<_> = public_keys
            .iter()
            .map(PublicKey::validated)
            .collect::<Result<_, _>>()?;
        BlsSignature::secure_aggregate(&signatures, &keys)
            .map(Self::from_validated)
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
        let signature = self.0.as_ref().ok_or(BlsError::InvalidSignature)?;
        signature
            .secure_verify_aggregates(message.as_ref(), &keys)
            .map_err(|_| BlsError::InvalidSignature)
    }
}
