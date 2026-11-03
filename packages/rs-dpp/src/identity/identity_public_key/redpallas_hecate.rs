//! The Hecate signer key, [`KeyType::REDPALLAS_HECATE`](crate::identity::KeyType::REDPALLAS_HECATE).
//!
//! A key a sender registers so that, later, a zero-knowledge proof can show "some identity
//! with a signer key sent this" without saying which. Its 64 bytes of data are two Pallas
//! encodings:
//!
//! ```text
//! data = ak (32) || nk_commitment (32)
//! ```
//!
//! - `ak` is a RedPallas spend-authorization verification key, as Orchard's: a point of the
//!   Pallas curve whose encoding has its sign bit clear, so its x-coordinate alone names it.
//!   The key's proof of possession, and every signature it makes, is a RedPallas signature by
//!   `ak` over the signed bytes.
//! - `nk_commitment` is `Poseidon(nk)` of a second secret, `nk`, a Pallas base element. A
//!   proof keys its per-identity tags with `nk`, which a RedPallas secret (a scalar of the
//!   other field) cannot do in-circuit, so the key carries a commitment to it; `nk` itself is
//!   never registered.
//!
//! Wallets derive both secrets from one 32-byte secret, the key's private key, as Orchard
//! derives `ask` and `nk` from a spending key, under a personalization of Dash's own so that a
//! Hecate secret equal to an Orchard spending key still gives unrelated keys:
//!
//! ```text
//! expand(secret, t) = BLAKE2b-512(personal = "DashHecateExpand", secret || [t])
//! ask = ToScalar(expand(secret, 0x00)), negated when [ask]G has its sign bit set
//! nk  = ToBase(expand(secret, 0x01))
//! ```

use blake2b_simd::Params;
use ff::{Field, FromUniformBytes, PrimeField};
use group::{Group, GroupEncoding};
use halo2_poseidon::{ConstantLength, Hash as PoseidonHash, P128Pow5T3};
use pasta_curves::pallas;
use rand::rngs::StdRng;
use rand::SeedableRng;
use reddsa::orchard::SpendAuth;
use reddsa::{Signature, SigningKey, VerificationKey};

use crate::ProtocolError;

/// The bytes of a [`KeyType::REDPALLAS_HECATE`](crate::identity::KeyType::REDPALLAS_HECATE)
/// key's data: `ak` and `nk_commitment`.
pub const REDPALLAS_HECATE_KEY_DATA_SIZE: usize = 64;

/// The bytes of a RedPallas signature.
pub const REDPALLAS_SIGNATURE_SIZE: usize = 64;

/// The BLAKE2b personalization of the secret expansion.
const EXPAND_PERSONALIZATION: &[u8; 16] = b"DashHecateExpand";

/// The BLAKE2b personalization of the seed of a signature's randomness.
const SIGNING_SEED_PERSONALIZATION: &[u8; 16] = b"DashHecateSigRng";

/// The domain bytes of the secret expansion.
const EXPAND_ASK: u8 = 0x00;
const EXPAND_NK: u8 = 0x01;

/// The secrets of a Hecate signer key, derived from its 32-byte private key.
pub struct RedPallasHecateSecretKey {
    ask: SigningKey<SpendAuth>,
    nk: pallas::Base,
    secret: [u8; 32],
}

impl RedPallasHecateSecretKey {
    /// Derive the key's secrets from its 32-byte private key. Fails only for the negligible
    /// secrets whose `ask` is zero.
    pub fn from_secret(secret: &[u8; 32]) -> Result<Self, ProtocolError> {
        let ask = pallas::Scalar::from_uniform_bytes(&expand(secret, EXPAND_ASK));
        if bool::from(ask.is_zero()) {
            return Err(ProtocolError::Generic(
                "this secret derives a zero RedPallas signing key".to_string(),
            ));
        }
        // As Orchard: negate `ask` when `ak` has its sign bit set, so every registered `ak`
        // has it clear.
        let signing_key = |ask: pallas::Scalar| {
            SigningKey::<SpendAuth>::try_from(ask.to_repr())
                .map_err(|e| ProtocolError::Generic(format!("RedPallas signing key: {e}")))
        };
        let candidate = signing_key(ask)?;
        let ak: [u8; 32] = VerificationKey::from(&candidate).into();
        let ask = if ak[31] >> 7 == 1 {
            signing_key(-ask)?
        } else {
            candidate
        };
        let nk = pallas::Base::from_uniform_bytes(&expand(secret, EXPAND_NK));
        Ok(Self {
            ask,
            nk,
            secret: *secret,
        })
    }

    /// The key's registered data: `ak || Poseidon(nk)`.
    pub fn public_key_data(&self) -> [u8; REDPALLAS_HECATE_KEY_DATA_SIZE] {
        let ak: [u8; 32] = VerificationKey::from(&self.ask).into();
        let mut data = [0u8; REDPALLAS_HECATE_KEY_DATA_SIZE];
        data[..32].copy_from_slice(&ak);
        data[32..].copy_from_slice(&nk_commitment(&self.nk).to_repr());
        data
    }

    /// The tag key `nk`, as the circuit takes it.
    pub fn nk(&self) -> [u8; 32] {
        self.nk.to_repr()
    }

    /// A RedPallas signature by `ak` over `message`.
    ///
    /// The signature's randomness is seeded from the private key and the message, so the same
    /// key signs the same bytes the same way, as the other key types do. RedPallas hashes the
    /// randomness with the key and the message again, so the seed never needs to be secret
    /// beyond the private key it comes from.
    pub fn sign(&self, message: &[u8]) -> [u8; REDPALLAS_SIGNATURE_SIZE] {
        let seed = Params::new()
            .hash_length(32)
            .personal(SIGNING_SEED_PERSONALIZATION)
            .to_state()
            .update(&self.secret)
            .update(message)
            .finalize();
        let mut seed_bytes = [0u8; 32];
        seed_bytes.copy_from_slice(seed.as_bytes());
        self.ask.sign(StdRng::from_seed(seed_bytes), message).into()
    }
}

/// `expand(secret, t) = BLAKE2b-512(personal = "DashHecateExpand", secret || [t])`.
fn expand(secret: &[u8; 32], domain: u8) -> [u8; 64] {
    let hash = Params::new()
        .hash_length(64)
        .personal(EXPAND_PERSONALIZATION)
        .to_state()
        .update(secret)
        .update(&[domain])
        .finalize();
    let mut bytes = [0u8; 64];
    bytes.copy_from_slice(hash.as_bytes());
    bytes
}

/// `Poseidon(nk)`: the commitment a key registers to its tag key.
fn nk_commitment(nk: &pallas::Base) -> pallas::Base {
    PoseidonHash::<_, P128Pow5T3, ConstantLength<1>, 3, 2>::init().hash([*nk])
}

/// The key's `ak` once its data is checked: 64 bytes, an `ak` that encodes a Pallas point other
/// than the identity with its sign bit clear, and an `nk_commitment` that encodes a Pallas base
/// element canonically.
fn checked_ak(data: &[u8]) -> Result<VerificationKey<SpendAuth>, String> {
    if data.len() != REDPALLAS_HECATE_KEY_DATA_SIZE {
        return Err(format!(
            "REDPALLAS_HECATE key data must be {REDPALLAS_HECATE_KEY_DATA_SIZE} bytes, got {}",
            data.len()
        ));
    }
    let mut ak = [0u8; 32];
    ak.copy_from_slice(&data[..32]);
    let mut nk_commitment = [0u8; 32];
    nk_commitment.copy_from_slice(&data[32..]);
    if ak[31] >> 7 == 1 {
        return Err("REDPALLAS_HECATE ak must have its sign bit clear".to_string());
    }
    let point = Option::<pallas::Point>::from(pallas::Point::from_bytes(&ak))
        .ok_or_else(|| "REDPALLAS_HECATE ak is not a Pallas point".to_string())?;
    if bool::from(point.is_identity()) {
        return Err("REDPALLAS_HECATE ak must not be the identity".to_string());
    }
    if Option::<pallas::Base>::from(pallas::Base::from_repr(nk_commitment)).is_none() {
        return Err(
            "REDPALLAS_HECATE nk_commitment is not a canonical Pallas base element".to_string(),
        );
    }
    VerificationKey::<SpendAuth>::try_from(ak)
        .map_err(|e| format!("REDPALLAS_HECATE ak is not a RedPallas key: {e}"))
}

/// Check a key's data: see [`checked_ak`].
pub fn validate_public_key_data(data: &[u8]) -> Result<(), String> {
    checked_ak(data).map(|_| ())
}

/// Verify a RedPallas `signature` by the `ak` of the key `data` over `message`.
pub fn verify_signature(data: &[u8], message: &[u8], signature: &[u8]) -> Result<(), String> {
    let ak = checked_ak(data)?;
    let signature: [u8; REDPALLAS_SIGNATURE_SIZE] = signature.try_into().map_err(|_| {
        format!(
            "a RedPallas signature is {REDPALLAS_SIGNATURE_SIZE} bytes, got {}",
            signature.len()
        )
    })?;
    ak.verify(message, &Signature::<SpendAuth>::from(signature))
        .map_err(|e| format!("RedPallas signature is invalid: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> RedPallasHecateSecretKey {
        RedPallasHecateSecretKey::from_secret(&[byte; 32]).expect("a nonzero ask")
    }

    #[test]
    fn should_derive_data_that_validates_with_the_sign_bit_clear() {
        for byte in 0..32u8 {
            let data = key(byte).public_key_data();
            assert_eq!(data[31] >> 7, 0, "secret {byte}");
            validate_public_key_data(&data).expect("derived data is valid");
        }
    }

    #[test]
    fn should_derive_the_same_key_from_the_same_secret() {
        assert_eq!(key(7).public_key_data(), key(7).public_key_data());
        assert_ne!(key(7).public_key_data(), key(8).public_key_data());
        assert_eq!(key(7).nk(), key(7).nk());
    }

    #[test]
    fn should_commit_to_nk_with_poseidon() {
        let key = key(9);
        let nk = pallas::Base::from_repr(key.nk()).unwrap();
        assert_eq!(
            &key.public_key_data()[32..],
            &PoseidonHash::<_, P128Pow5T3, ConstantLength<1>, 3, 2>::init()
                .hash([nk])
                .to_repr()[..]
        );
    }

    #[test]
    fn should_verify_its_own_signature_and_refuse_others() {
        let signer = key(1);
        let data = signer.public_key_data();
        let signature = signer.sign(b"signable bytes");
        verify_signature(&data, b"signable bytes", &signature).expect("a valid signature");
        // Deterministic for the same key and message.
        assert_eq!(signature, signer.sign(b"signable bytes"));

        assert!(verify_signature(&data, b"other bytes", &signature).is_err());
        assert!(
            verify_signature(&key(2).public_key_data(), b"signable bytes", &signature).is_err()
        );
        let mut tampered = signature;
        tampered[0] ^= 1;
        assert!(verify_signature(&data, b"signable bytes", &tampered).is_err());
        assert!(verify_signature(&data, b"signable bytes", &signature[..63]).is_err());
    }

    #[test]
    fn should_refuse_malformed_data() {
        let data = key(3).public_key_data();
        // Wrong sizes.
        assert!(validate_public_key_data(&data[..63]).is_err());
        assert!(validate_public_key_data(&[data.as_slice(), &[0]].concat()).is_err());
        // The sign bit set.
        let mut flipped = data;
        flipped[31] |= 0x80;
        assert!(validate_public_key_data(&flipped).is_err());
        // The identity.
        let mut identity = data;
        identity[..32].copy_from_slice(&[0u8; 32]);
        assert!(validate_public_key_data(&identity).is_err());
        // A non-canonical nk_commitment.
        let mut non_canonical = data;
        non_canonical[32..].copy_from_slice(&[0xff; 32]);
        non_canonical[63] = 0x7f;
        assert!(validate_public_key_data(&non_canonical).is_err());
        // An x-coordinate off the curve: the first such x from 1.
        let off_curve = (1u8..)
            .map(|x| {
                let mut bytes = [0u8; 32];
                bytes[0] = x;
                bytes
            })
            .find(|bytes| Option::<pallas::Point>::from(pallas::Point::from_bytes(bytes)).is_none())
            .unwrap();
        let mut off = data;
        off[..32].copy_from_slice(&off_curve);
        assert!(validate_public_key_data(&off).is_err());
    }
}
