//! Platform BLS keys and Basic signatures backed by rust-dashcore's dash-pkc primitives.

use dashcore::ProTxHash;
use dashcore_crypto::dash_pkc::bls::{
    BlsPkShare, BlsPublicKey, BlsScChia, BlsScIetf, BlsSecretKey, BlsShareId, BlsSignature, Fr,
};
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

    /// Whether this is the identity point, which parses but never verifies a signature.
    pub fn is_identity(&self) -> bool {
        matches!(self.0, PublicKeyPoint::Infinity)
    }

    fn validated(&self) -> Result<&BlsPublicKey<BlsScIetf>, BlsError> {
        match &self.0 {
            PublicKeyPoint::Infinity => Err(BlsError::InvalidSignature),
            PublicKeyPoint::Validated(key) => Ok(key),
        }
    }
}

impl PublicKey {
    /// Recover a threshold public key from its members' public key shares by Lagrange
    /// interpolation, as Core and Tenderdash recover a quorum's key.
    ///
    /// A member's id is its `proTxHash` as Core prints it. Shares of at least the threshold
    /// number of members recover the quorum's key; fewer shares, or shares that do not lie on one
    /// polynomial, recover some other key.
    ///
    /// Fails with fewer than two shares, a share that is the identity, or ids that reduce to
    /// zero or to the same scalar.
    pub fn recover_threshold(shares: &[(ProTxHash, PublicKey)]) -> Result<Self, BlsError> {
        let shares = shares
            .iter()
            .map(|(pro_tx_hash, key)| {
                // Platform holds a proTxHash in the order Core prints it.
                let id = BlsShareId::from_hex(&hex::encode(pro_tx_hash.to_byte_array()))
                    .map_err(|e| BlsError::InvalidInputs(e.to_string()))?;
                let key = key
                    .validated()
                    .map_err(|_| BlsError::InvalidInputs("a key share is the identity".into()))?;
                Ok(BlsPkShare::new(id, *key))
            })
            .collect::<Result<Vec<_>, BlsError>>()?;
        let shares: Vec<&BlsPkShare<BlsScIetf>> = shares.iter().collect();
        BlsPublicKey::recover_shares(&shares)
            .map(|key| Self(PublicKeyPoint::Validated(key)))
            .map_err(|e| BlsError::InvalidInputs(e.to_string()))
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
        Ok(Signature::from_validated(self.0.sign_message(message)))
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
        key.verify_message(message.as_ref(), signature)
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
            .secure_verify_aggregates_message(message.as_ref(), &keys)
            .map_err(|_| BlsError::InvalidSignature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn base64(text: &str) -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(text)
            .expect("expected base64")
    }

    /// Tenderdash's `TestRecoverThresholdPublicKeyFromPublicKeys` vectors: the members'
    /// proTxHashes as Core prints them, their public key shares and the quorum's public key.
    /// Tenderdash holds a proTxHash in the reverse of that order, as Platform does, and recovers
    /// with it; the same key must come out here.
    #[test]
    fn should_recover_the_quorum_key_tenderdash_recovers_from_the_same_shares() {
        let cases: [(&[&str], &[&str]); 3] = [
            (
                &[
                    "FDC09407DA9473CDC5E5AFCBB55712C95765343B2AF900B28BE4004E69CEDBB3",
                    "02AE8AAAF330949260BA537B05E408CEFA162FFA7CBB04C6C95BE2F922650A9D",
                    "036BCAEE159C09B75FD6404FAD6E76620AF595EF74734572E9D3DB4C226466FD",
                    "04FFEFD49498E2FC51A6FEB7FC695ED614EEA24B1C42F62AECDCA36B0FDCC026",
                ],
                &[
                    "ql8sHSxaXsWu5Zt+0HULXCFEfzG/sStFuYa2NWykFzeE6IVG4kyUt+uO66JNHUkQ",
                    "hn14ZB7dB/xAf45abtVjQDsch8ogt42LAHj+8v5/r6MWTdseESPCnAhT+huBAQ7i",
                    "hVxdmB/tL//Xp3aNCQdqkc84W0/RmoAeONTQd2eUYhgGFe1ymX85XTTS8E7UrY4Y",
                    "tGc2Wt/Q2P9SB5F4+TedXuu68ixcuphB372iyoST5QGFUpBBfYlz6/CzxiesaOWI",
                ],
            ),
            (
                &[
                    "FDC09407DA9473CDC5E5AFCBB55712C95765343B2AF900B28BE4004E69CEDBB3",
                    "02AE8AAAF330949260BA537B05E408CEFA162FFA7CBB04C6C95BE2F922650A9D",
                    "036BCAEE159C09B75FD6404FAD6E76620AF595EF74734572E9D3DB4C226466FD",
                    "04FFEFD49498E2FC51A6FEB7FC695ED614EEA24B1C42F62AECDCA36B0FDCC026",
                    "0552e56ba3564c124ac8a6a0fa481219d100478566e81c7943507296b14484ae",
                ],
                &[
                    "r/w8vr15blUV17GPU4ss5CaMkjwyvy0cMqhTWDdSahOhVuU8LFYXB0SnlI1fX25L",
                    "tLpx1kFIAVEkSWAhhqI+pOEvtL4u/KYEqDJDOO0bzunlF6kyy3hZ8I6bAzqN6dOZ",
                    "mIPmlDjKop9VGZl2o36ryBOxJbsWwfczsv5JVu34tijDp9YkZh1Sdsy3pS8NTpo5",
                    "gM0fFG0Pi/ri9u9uJ33f/ie67RbYli2r5zyJMlz4DPvr892udooIVxbLpEcQzyPv",
                    "i9KQRLVpp9u6wrI4rEXlzTwByh86QNhm5rBz4oWmxen0CPIxoZa+JjIkWYB5ut41",
                ],
            ),
            (
                &[
                    "0740ce79c4e016fe5bc9b5438d1cab95bb3222352322cfb5fcd8048b9c6568cb",
                    "007456fb4969a55aef48c6638ccb423dbd4dc5a871f373607f1f34d148dee0b0",
                    "f726a232e89395adf70cce664326eb9e7bfcc1de77e0e7ea3b65e6765cdd685c",
                ],
                &[
                    "pUkeF0hf5IBVvVfg5cOCG9jNPQAx8GXWFflZKrSeIaypdhOEn6rbHkcahHCyybTe",
                    "rJi5LeaMUbdWDec2z9zQWN/dDvg9NeWmFGREs1W+DEYWeruPISR7nO2BKhpwXyl3",
                    "j4udh9zVOyKmNAWmdM2nD6ARTLRV8qd+EhPBFDM/o3Z6vnNsigA4AbAeCKdJh8on",
                ],
            ),
        ];
        let quorum_key = base64("hiQykLvL/ZrnW97OeYGWU1AgjrXpmwTVzSTpVa2pYfjAoWLe50C+e9xsPAYTui6x");

        for (pro_tx_hashes, key_shares) in cases {
            let shares: Vec<(ProTxHash, PublicKey)> = pro_tx_hashes
                .iter()
                .zip(key_shares)
                .map(|(pro_tx_hash, key_share)| {
                    let mut bytes: [u8; 32] = hex::decode(pro_tx_hash)
                        .expect("expected hex")
                        .try_into()
                        .expect("expected 32 bytes");
                    bytes.reverse();
                    let key = PublicKey::try_from(base64(key_share).as_slice())
                        .expect("expected a public key share");
                    (ProTxHash::from_byte_array(bytes), key)
                })
                .collect();

            let recovered = PublicKey::recover_threshold(&shares).expect("expected a key");

            assert_eq!(recovered.to_bytes().to_vec(), quorum_key);
        }
    }

    /// One share is not enough to interpolate, and the identity is not a key share.
    #[test]
    fn should_refuse_too_few_shares_or_the_identity() {
        let key = SecretKey::from_be_bytes(&[1; 32])
            .expect("expected a key")
            .public_key();
        let member = ProTxHash::from_byte_array([1; 32]);
        let other = ProTxHash::from_byte_array([2; 32]);

        assert!(PublicKey::recover_threshold(&[(member, key)]).is_err());
        assert!(
            PublicKey::recover_threshold(&[(member, key), (other, PublicKey::default())]).is_err()
        );
    }
}
