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
use zeroize::Zeroizing;

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
///
/// The key holds that private key and nothing else, in a [`Zeroizing`] buffer wiped when the
/// key is dropped. Each method derives `ask` or `nk` from it again and drops them before it
/// returns, and the byte buffers it derives them through (the 64-byte expansions, the canonical
/// bytes of `ask` and the signing seed) are [`Zeroizing`] too.
///
/// Some copies are short-lived but not wiped. reddsa's `SigningKey` (which is `Copy` and has no
/// clearing `Drop`), pasta's field elements, rand's `StdRng` and `blake2b_simd`'s hashing state,
/// which buffers the private key, have no zeroize support, so what they hold stays on the stack,
/// uncleared, once the call that made them returns.
pub struct RedPallasHecateSecretKey {
    secret: Zeroizing<[u8; 32]>,
}

impl RedPallasHecateSecretKey {
    /// Keep a 32-byte private key. Fails only for the negligible secrets whose `ask` is zero.
    pub fn from_secret(secret: &[u8; 32]) -> Result<Self, ProtocolError> {
        let key = Self {
            secret: Zeroizing::new(*secret),
        };
        // Derive `ask` once now, so that a secret which cannot sign is refused here and the
        // methods below derive it again only from one that can.
        key.signing_key()?;
        Ok(key)
    }

    /// The key's registered data: `ak || Poseidon(nk)`.
    ///
    /// Fails only where [`Self::from_secret`] would have, since it derives the same `ask`.
    pub fn public_key_data(&self) -> Result<[u8; REDPALLAS_HECATE_KEY_DATA_SIZE], ProtocolError> {
        let ak: [u8; 32] = VerificationKey::from(&self.signing_key()?).into();
        let mut data = [0u8; REDPALLAS_HECATE_KEY_DATA_SIZE];
        data[..32].copy_from_slice(&ak);
        data[32..].copy_from_slice(&nk_commitment(&self.tag_key()).to_repr());
        Ok(data)
    }

    /// The tag key `nk`, as the circuit takes it, in a buffer wiped when it is dropped.
    pub fn nk(&self) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(self.tag_key().to_repr())
    }

    /// A RedPallas signature by `ak` over `message`.
    ///
    /// The signature's randomness is seeded from the private key and the message, so the same
    /// key signs the same bytes the same way, as the other key types do. RedPallas hashes the
    /// randomness with the key and the message again, so the seed never needs to be secret
    /// beyond the private key it comes from. Fails only where [`Self::from_secret`] would have.
    pub fn sign(&self, message: &[u8]) -> Result<[u8; REDPALLAS_SIGNATURE_SIZE], ProtocolError> {
        let signing_key = self.signing_key()?;
        let seed = self.signing_seed(message);
        Ok(signing_key.sign(StdRng::from_seed(*seed), message).into())
    }

    /// The RedPallas signing key `ask`. As Orchard's, it is negated when `ak` has its sign bit
    /// set, so every registered `ak` has it clear.
    fn signing_key(&self) -> Result<SigningKey<SpendAuth>, ProtocolError> {
        let ask = pallas::Scalar::from_uniform_bytes(&expand(&self.secret, EXPAND_ASK));
        if bool::from(ask.is_zero()) {
            return Err(ProtocolError::Generic(
                "this secret derives a zero RedPallas signing key".to_string(),
            ));
        }
        let candidate = signing_key_from_scalar(&ask)?;
        let ak: [u8; 32] = VerificationKey::from(&candidate).into();
        if ak[31] >> 7 == 1 {
            signing_key_from_scalar(&-ask)
        } else {
            Ok(candidate)
        }
    }

    /// The tag key `nk`.
    fn tag_key(&self) -> pallas::Base {
        pallas::Base::from_uniform_bytes(&expand(&self.secret, EXPAND_NK))
    }

    /// `BLAKE2b-256(personal = "DashHecateSigRng", secret || message)`, the seed of a
    /// signature's randomness.
    fn signing_seed(&self, message: &[u8]) -> Zeroizing<[u8; 32]> {
        let hash = Params::new()
            .hash_length(32)
            .personal(SIGNING_SEED_PERSONALIZATION)
            .to_state()
            .update(self.secret.as_slice())
            .update(message)
            .finalize();
        let mut seed = Zeroizing::new([0u8; 32]);
        seed.copy_from_slice(hash.as_bytes());
        seed
    }
}

/// A RedPallas signing key from its scalar, through the scalar's canonical bytes.
fn signing_key_from_scalar(ask: &pallas::Scalar) -> Result<SigningKey<SpendAuth>, ProtocolError> {
    let repr = Zeroizing::new(ask.to_repr());
    SigningKey::<SpendAuth>::try_from(*repr)
        .map_err(|e| ProtocolError::Generic(format!("RedPallas signing key: {e}")))
}

/// `expand(secret, t) = BLAKE2b-512(personal = "DashHecateExpand", secret || [t])`.
fn expand(secret: &[u8; 32], domain: u8) -> Zeroizing<[u8; 64]> {
    let hash = Params::new()
        .hash_length(64)
        .personal(EXPAND_PERSONALIZATION)
        .to_state()
        .update(secret)
        .update(&[domain])
        .finalize();
    let mut bytes = Zeroizing::new([0u8; 64]);
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

    fn key_data(byte: u8) -> [u8; REDPALLAS_HECATE_KEY_DATA_SIZE] {
        key(byte).public_key_data().expect("derivable data")
    }

    /// What a fixed secret derives, as hex.
    struct KnownAnswers {
        secret: [u8; 32],
        /// `expand(secret, 0)`.
        expand_ask: &'static str,
        /// `expand(secret, 0)` reduced to a scalar, before the sign-bit negation.
        ask_raw: &'static str,
        /// Whether `[ask_raw]G` has its sign bit set, so that `ask = q - ask_raw`.
        negated: bool,
        /// The signing scalar `ask`.
        ask: &'static str,
        /// `expand(secret, 1)`.
        expand_nk: &'static str,
        /// `expand(secret, 1)` reduced to a base element.
        nk: &'static str,
        /// `ak || Poseidon(nk)`.
        public_key_data: &'static str,
    }

    /// `[0x00; 32]`: `[ask_raw]G` has its sign bit clear, so `ask = ask_raw`.
    const KEEPS_ASK: KnownAnswers = KnownAnswers {
        secret: [0x00; 32],
        expand_ask: "35e02eea697b23a89d0d40e4738d84a74cc19787a9c2bd21cf123e9a4d7912b2\
                     11ca0758f317c69881e90ee24c3a15825fd027726dbb6c5b77d6598e3bcf638d",
        ask_raw: "35e488ccb3724bc6dee0a8b891bed5e174ca0f0b341cffff900906d5e965e112",
        negated: false,
        ask: "35e488ccb3724bc6dee0a8b891bed5e174ca0f0b341cffff900906d5e965e112",
        expand_nk: "0e90697dd40135cbe7e60ae0e33c50d26f148233d9d9c271166114c3149c1e0c\
                    6b932e166a5cf089c0c544ff811308313dc464d2c35cc1683b892b8bb4e74426",
        nk: "e332d74b4f7e44e234a2e86ee2c7f609cb2eddc6a9f5d21505f299102e5b3f21",
        public_key_data: "79f071ee8b8d39979379487dcd893851b7626f09ec4d058795a8f004714d3812\
                          ca86611872c2d6b0c8339b944747d426d4633c2ee1412f87b934f46c5f4a9728",
    };

    /// `[0x02; 32]`: `[ask_raw]G` has its sign bit set, so `ask = q - ask_raw`.
    const NEGATES_ASK: KnownAnswers = KnownAnswers {
        secret: [0x02; 32],
        expand_ask: "23ce6ba84583c3090e18666520356aae7a1d2dd7fad2bdb2595940cf0843dc99\
                     152573e3810113c83be8099e0770901de2f59b0a2ee1e1dd89f726fafd47ce1a",
        ask_raw: "0e61b1933a19c3bdd45a7b13de03a8acc1f5fd37d98ce2d433ed7a73be940101",
        negated: true,
        ask: "f39e4e6ce6d183ce084e19f61d959e753e0a02c826731d2bcc12858c416bfe3e",
        expand_nk: "4a969bd22ffd3af8a2dc180651c1aded4cfa7fc9a0f9e9223077ecf8758abced\
                    a11f1a02b984fddcf73ccfe1ea646f1e89c1c7f149cb5b98060587a8f10c561e",
        nk: "08e83dfc4b4def51fdfbbf471703856ed73821068d320a6120bf9ecda3ddac2c",
        public_key_data: "fb8ad4bfb8208ac7f7145b7cacdd9cd2adaa59df2d97659346922d8f0904e103\
                          079011341871911d7ad29c85fd298cadf56cb04fa3fac9bd28572513653e2e20",
    };

    /// The message of the signature vector.
    const SIGNATURE_VECTOR_MESSAGE: &[u8] = b"Hecate signer key vector";

    /// [`NEGATES_ASK`]'s signature over [`SIGNATURE_VECTOR_MESSAGE`].
    const SIGNATURE_VECTOR: &str = "4bc5740819065bd82a8031be98834e2d4cdf4f7ebeb6f4ca37c64c62ef278b18\
                                    11b539c7612de2e894b23685c56c32f2d564e9af21ccff957f687ffe7a0bf720";

    fn ask_bytes(key: &RedPallasHecateSecretKey) -> [u8; 32] {
        <[u8; 32]>::from(key.signing_key().expect("a nonzero ask"))
    }

    fn assert_known_answers(vector: &KnownAnswers) {
        let key = RedPallasHecateSecretKey::from_secret(&vector.secret).expect("a nonzero ask");
        assert_eq!(
            hex::encode(expand(&vector.secret, EXPAND_ASK)),
            vector.expand_ask
        );
        assert_eq!(
            hex::encode(expand(&vector.secret, EXPAND_NK)),
            vector.expand_nk
        );

        let ask_raw = pallas::Scalar::from_uniform_bytes(&expand(&vector.secret, EXPAND_ASK));
        assert_eq!(hex::encode(ask_raw.to_repr()), vector.ask_raw);
        // The branch the vector takes, and its cause: the sign bit of `[ask_raw]G`.
        let raw_ak: [u8; 32] = VerificationKey::from(
            &SigningKey::<SpendAuth>::try_from(ask_raw.to_repr()).expect("a canonical scalar"),
        )
        .into();
        assert_eq!(raw_ak[31] >> 7 == 1, vector.negated);
        let ask = ask_bytes(&key);
        let branch = if vector.negated { -ask_raw } else { ask_raw };
        assert_eq!(ask, branch.to_repr());
        assert_eq!(hex::encode(ask), vector.ask);

        assert_eq!(hex::encode(*key.nk()), vector.nk);
        assert_eq!(
            hex::encode(key.public_key_data().expect("derivable data")),
            vector.public_key_data
        );
    }

    /// The known answers of [`KEEPS_ASK`].
    ///
    /// The hashes and field reductions of both vectors are checked independently of this crate
    /// by this Python, run with `python3 -I`. `pasta_curves`' `from_uniform_bytes` on 64 bytes
    /// is exactly this reduction of them as a little-endian integer.
    ///
    /// ```text
    /// import hashlib
    /// p = 0x40000000000000000000000000000000224698fc094cf91b992d30ed00000001  # pallas::Base
    /// q = 0x40000000000000000000000000000000224698fc0994a8dd8c46eb2100000001  # pallas::Scalar
    /// le = lambda b: int.from_bytes(b, "little")
    /// hex32 = lambda n: n.to_bytes(32, "little").hex()
    /// for secret in (bytes([0x00]) * 32, bytes([0x02]) * 32):
    ///     e0, e1 = (hashlib.blake2b(secret + bytes([t]), digest_size=64,
    ///                               person=b"DashHecateExpand").digest() for t in (0, 1))
    ///     ask_raw = le(e0) % q
    ///     print(e0.hex(), e1.hex())                  # expand_ask, expand_nk
    ///     print(hex32(ask_raw), hex32(q - ask_raw))  # ask_raw, and q - ask_raw
    ///     print(hex32(le(e1) % p))                   # nk
    /// ```
    ///
    /// `ask` is `ask_raw` for `[0x00; 32]` and `q - ask_raw` for `[0x02; 32]`. Which one
    /// follows from `ak`'s sign bit, which takes Pallas point arithmetic, so the script leaves
    /// it out; `ak`, both branches and the signature vector were checked against a Python
    /// Pallas implementation when these vectors were made. `Poseidon(nk)` is pinned as
    /// halo2's Poseidon computes it.
    #[test]
    fn should_derive_the_known_answers_of_a_secret_that_keeps_ask() {
        assert_known_answers(&KEEPS_ASK);
    }

    /// The known answers of [`NEGATES_ASK`]; see the recipe on
    /// [`should_derive_the_known_answers_of_a_secret_that_keeps_ask`].
    #[test]
    fn should_derive_the_known_answers_of_a_secret_that_negates_ask() {
        assert_known_answers(&NEGATES_ASK);
    }

    #[test]
    fn should_sign_the_known_signature_vector() {
        let signer =
            RedPallasHecateSecretKey::from_secret(&NEGATES_ASK.secret).expect("a nonzero ask");
        assert_eq!(
            hex::encode(signer.sign(SIGNATURE_VECTOR_MESSAGE).expect("a signature")),
            SIGNATURE_VECTOR
        );
    }

    #[test]
    fn should_verify_the_known_signature_vector_and_refuse_any_one_bit_change() {
        let data = hex::decode(NEGATES_ASK.public_key_data).expect("hex");
        let signature = hex::decode(SIGNATURE_VECTOR).expect("hex");
        verify_signature(&data, SIGNATURE_VECTOR_MESSAGE, &signature).expect("a valid signature");
        for byte in 0..REDPALLAS_SIGNATURE_SIZE {
            let mut tampered = signature.clone();
            tampered[byte] ^= 1;
            assert!(
                verify_signature(&data, SIGNATURE_VECTOR_MESSAGE, &tampered).is_err(),
                "byte {byte} flipped"
            );
        }
    }

    /// The key keeps its private key alone, so no derived secret outlives the call that
    /// derives it.
    #[test]
    fn should_hold_only_the_32_byte_private_key() {
        assert_eq!(size_of::<RedPallasHecateSecretKey>(), 32);
    }

    #[test]
    fn should_derive_data_that_validates_with_the_sign_bit_clear() {
        for byte in 0..32u8 {
            let data = key_data(byte);
            assert_eq!(data[31] >> 7, 0, "secret {byte}");
            validate_public_key_data(&data).expect("derived data is valid");
        }
    }

    #[test]
    fn should_derive_the_same_key_from_the_same_secret() {
        assert_eq!(key_data(7), key_data(7));
        assert_ne!(key_data(7), key_data(8));
        assert_eq!(*key(7).nk(), *key(7).nk());
    }

    #[test]
    fn should_commit_to_nk_with_poseidon() {
        let key = key(9);
        let nk = pallas::Base::from_repr(*key.nk()).unwrap();
        assert_eq!(
            &key.public_key_data().expect("derivable data")[32..],
            &PoseidonHash::<_, P128Pow5T3, ConstantLength<1>, 3, 2>::init()
                .hash([nk])
                .to_repr()[..]
        );
    }

    #[test]
    fn should_verify_its_own_signature_and_refuse_others() {
        let signer = key(1);
        let data = key_data(1);
        let signature = signer.sign(b"signable bytes").expect("a signature");
        verify_signature(&data, b"signable bytes", &signature).expect("a valid signature");
        // Deterministic for the same key and message.
        assert_eq!(
            signature,
            signer.sign(b"signable bytes").expect("a signature")
        );

        assert!(verify_signature(&data, b"other bytes", &signature).is_err());
        assert!(verify_signature(&key_data(2), b"signable bytes", &signature).is_err());
        let mut tampered = signature;
        tampered[0] ^= 1;
        assert!(verify_signature(&data, b"signable bytes", &tampered).is_err());
        assert!(verify_signature(&data, b"signable bytes", &signature[..63]).is_err());
    }

    #[test]
    fn should_refuse_malformed_data() {
        let data = key_data(3);
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
