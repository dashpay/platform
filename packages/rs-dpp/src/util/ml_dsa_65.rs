//! ML-DSA-65 (FIPS 204), the signature scheme of ML_DSA_65 identity keys.
//!
//! A signature is over the message itself with the empty context string, as `ML-DSA.Sign` and
//! `ML-DSA.Verify` define it: the scheme hashes the message together with a hash of the public
//! key, so nothing is hashed beforehand. Signing is the deterministic variant, so a private key
//! and a message always give the same signature and signing needs no random source.
//!
//! The private key is the 32 byte seed that `ML-DSA.KeyGen_internal` expands into the key pair,
//! the form FIPS 204 recommends storing. It is the same size as the private key of every other
//! key type.

use ml_dsa::{
    EncodedSignature, EncodedVerifyingKey, Keypair, MlDsa65, Seed, Signature, Signer, SigningKey,
    Verifier, VerifyingKey,
};
use thiserror::Error;

/// The size of an encoded ML-DSA-65 public key.
pub const PUBLIC_KEY_SIZE: usize = 1952;

/// The size of an encoded ML-DSA-65 signature.
pub const SIGNATURE_SIZE: usize = 3309;

/// Why an ML-DSA-65 signature was not made or did not verify.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlDsa65Error {
    /// The public key does not have the size of an encoded key.
    #[error("the public key is {0} bytes, an ML-DSA-65 public key is {PUBLIC_KEY_SIZE} bytes")]
    InvalidPublicKeySize(usize),
    /// The signature does not have the size of an encoded signature.
    #[error("the signature is {0} bytes, an ML-DSA-65 signature is {SIGNATURE_SIZE} bytes")]
    InvalidSignatureSize(usize),
    /// The signature has the right size and is not an encoding `sigDecode` accepts.
    #[error("the signature is not a well formed ML-DSA-65 signature")]
    MalformedSignature,
    /// The signature is well formed and is not one the key made over the message.
    #[error("the ML-DSA-65 signature does not verify")]
    VerifyFailed,
    /// Signing found no signature. The scheme retries until it finds one, so this is not
    /// expected for any message.
    #[error("ML-DSA-65 signing failed")]
    SigningFailed,
}

/// The encoded public key of the key pair `seed` expands into.
pub fn public_key_from_seed(seed: &[u8; 32]) -> Vec<u8> {
    SigningKey::<MlDsa65>::from_seed(&Seed::from(*seed))
        .verifying_key()
        .encode()
        .to_vec()
}

/// Signs `data` with the key pair `seed` expands into.
pub fn sign(data: &[u8], seed: &[u8; 32]) -> Result<Vec<u8>, MlDsa65Error> {
    SigningKey::<MlDsa65>::from_seed(&Seed::from(*seed))
        .try_sign(data)
        .map(|signature| signature.encode().to_vec())
        .map_err(|_| MlDsa65Error::SigningFailed)
}

/// Verifies that `signature` is a signature over `data` made by the owner of `public_key`.
pub fn verify_data_signature(
    data: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> Result<(), MlDsa65Error> {
    let public_key = EncodedVerifyingKey::<MlDsa65>::try_from(public_key)
        .map_err(|_| MlDsa65Error::InvalidPublicKeySize(public_key.len()))?;
    let signature = EncodedSignature::<MlDsa65>::try_from(signature)
        .map_err(|_| MlDsa65Error::InvalidSignatureSize(signature.len()))?;
    let signature =
        Signature::<MlDsa65>::decode(&signature).ok_or(MlDsa65Error::MalformedSignature)?;
    VerifyingKey::<MlDsa65>::decode(&public_key)
        .verify(data, &signature)
        .map_err(|_| MlDsa65Error::VerifyFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: [u8; 32] = [7; 32];

    #[test]
    fn should_derive_a_public_key_of_the_encoded_size() {
        assert_eq!(public_key_from_seed(&SEED).len(), PUBLIC_KEY_SIZE);
    }

    #[test]
    fn should_verify_a_signature_made_with_the_seed_of_the_key() {
        let public_key = public_key_from_seed(&SEED);
        let signature = sign(b"message", &SEED).expect("expected to sign");
        assert_eq!(signature.len(), SIGNATURE_SIZE);
        assert_eq!(
            verify_data_signature(b"message", &signature, &public_key),
            Ok(())
        );
    }

    #[test]
    fn should_sign_deterministically() {
        assert_eq!(sign(b"message", &SEED), sign(b"message", &SEED));
    }

    #[test]
    fn should_refuse_a_signature_over_another_message() {
        let public_key = public_key_from_seed(&SEED);
        let signature = sign(b"message", &SEED).expect("expected to sign");
        assert_eq!(
            verify_data_signature(b"another message", &signature, &public_key),
            Err(MlDsa65Error::VerifyFailed)
        );
    }

    #[test]
    fn should_refuse_a_signature_made_with_another_key() {
        let public_key = public_key_from_seed(&[8; 32]);
        let signature = sign(b"message", &SEED).expect("expected to sign");
        assert_eq!(
            verify_data_signature(b"message", &signature, &public_key),
            Err(MlDsa65Error::VerifyFailed)
        );
    }

    #[test]
    fn should_refuse_a_public_key_or_a_signature_of_the_wrong_size() {
        let public_key = public_key_from_seed(&SEED);
        let signature = sign(b"message", &SEED).expect("expected to sign");
        assert_eq!(
            verify_data_signature(b"message", &signature, &public_key[1..]),
            Err(MlDsa65Error::InvalidPublicKeySize(PUBLIC_KEY_SIZE - 1))
        );
        assert_eq!(
            verify_data_signature(b"message", &signature[1..], &public_key),
            Err(MlDsa65Error::InvalidSignatureSize(SIGNATURE_SIZE - 1))
        );
        assert_eq!(
            verify_data_signature(b"message", &[], &public_key),
            Err(MlDsa65Error::InvalidSignatureSize(0))
        );
    }

    /// The last bytes of a signature encode which coefficients carry a hint, and only one
    /// encoding of each set of hints is accepted, so a signature cannot be rewritten into a
    /// second form that verifies.
    #[test]
    fn should_refuse_a_signature_whose_hint_is_not_canonically_encoded() {
        let public_key = public_key_from_seed(&SEED);
        let mut signature = sign(b"message", &SEED).expect("expected to sign");
        let last = signature.len() - 1;
        signature[last] = 0xff;
        assert_eq!(
            verify_data_signature(b"message", &signature, &public_key),
            Err(MlDsa65Error::MalformedSignature)
        );
    }

    /// Pins the key pair and the signature of a seed to the values OpenSSL 4.1.0 gives for it
    /// (`openssl genpkey -algorithm ML-DSA-65 -pkeyopt hexseed:0707…07`, then
    /// `openssl pkeyutl -sign -rawin -pkeyopt deterministic:1` over `message`), each as its
    /// double SHA-256. A release of `ml-dsa` that derived or signed differently would make
    /// wallets disagree with each other and with other implementations of FIPS 204, and one
    /// that verified differently would fork the chain.
    #[test]
    fn should_keep_the_public_key_and_the_signature_openssl_gives_for_a_seed() {
        use crate::util::hash::hash_double;

        let public_key = public_key_from_seed(&SEED);
        let signature = sign(b"message", &SEED).expect("expected to sign");
        assert_eq!(
            hex::encode(hash_double(&public_key)),
            "8e1d00af5f3afd10b9c2d2c2574b9a997f1b8284fc5645a1962a80ce7b36cdb8"
        );
        assert_eq!(
            hex::encode(hash_double(&signature)),
            "0c5d1550c4d72e91f7d23c7b6b2770a8c5796725b83b157b2fd79e58c54d9870"
        );
    }
}
