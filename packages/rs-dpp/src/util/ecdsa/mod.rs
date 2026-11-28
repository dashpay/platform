//! ECDSA secp256k1 verification as every released Platform does it: a signature whose S is in
//! the upper half of the curve order is refused as a signature that does not verify.
//! `EcdsaPublicKey::verify` verifies such a signature like its low-S twin, the same signature with
//! S negated, and `signer::verify_data_signature` refuses it with an error of its own, so Platform
//! checks S itself, at every protocol version. Accepting it would fork from nodes that refuse it
//! and change the replay of chain history.
//!
//! A signature checked by recovering its public key ([`verify_hash_signature`]) is the
//! exception: every released Platform accepts a high-S one there, and one whose header names an
//! uncompressed key, so refusing them is a rule of protocol version 15, selected by
//! `verify_hash_signature`.

use dashcore::crypto::ecdsa::{EcdsaError, EcdsaPublicKey, EcdsaSignature};
use dashcore::{sign_message, signer, PublicKey};

mod random_secret_key;
mod verify_hash_signature;

pub use random_secret_key::random_secret_key;
pub use verify_hash_signature::verify_hash_signature;

/// [`signer::verify_data_signature`], refusing a high-S signature with the error of a signature
/// that does not verify. Every other input gets the result and the error `dashcore` gives.
pub fn verify_data_signature(
    data: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> Result<(), anyhow::Error> {
    let result = signer::verify_data_signature(data, signature, public_key);
    // `dashcore` reads the key before it checks S, so a key it cannot read keeps its error.
    if result.is_err() && is_high_s(signature) && PublicKey::from_slice(public_key).is_ok() {
        return Err(anyhow::Error::msg(EcdsaError::VerifyFailed));
    }
    result
}

/// Whether `signature` reads as a recoverable signature whose S is high.
fn is_high_s(signature: &[u8]) -> bool {
    sign_message::from_slice(signature)
        .ok()
        .and_then(|signature| signature.signature().ok())
        .is_some_and(|signature| !signature.is_low_s())
}

/// [`EcdsaPublicKey::verify`], refusing a high-S signature with
/// [`EcdsaError::VerifyFailed`].
pub fn verify_digest(
    public_key: &EcdsaPublicKey,
    digest: &[u8; 32],
    signature: EcdsaSignature,
) -> Result<(), EcdsaError> {
    if !signature.is_low_s() {
        return Err(EcdsaError::VerifyFailed);
    }
    public_key.verify(digest, signature)
}
