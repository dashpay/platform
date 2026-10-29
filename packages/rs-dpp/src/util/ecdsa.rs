//! ECDSA secp256k1 verification as every released Platform does it: a signature whose S is in
//! the upper half of the curve order is refused. `dashcore` verifies such a signature like its
//! low-S twin, the same signature with S negated, so Platform checks S itself, at every protocol
//! version. Accepting it would fork from nodes that refuse it and change the replay of chain
//! history.

use dashcore::crypto::ecdsa::{EcdsaError, EcdsaPublicKey, EcdsaSignature};
use dashcore::{sign_message, signer};

/// [`signer::verify_data_signature`], refusing a high-S signature with the error of a signature
/// that does not verify. Every other input gets the result and the error `dashcore` gives.
pub fn verify_data_signature(
    data: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> Result<(), anyhow::Error> {
    signer::verify_data_signature(data, signature, public_key)?;
    // `dashcore` parsed the signature to verify it, so parsing it again succeeds.
    let signature = sign_message::from_slice(signature)?.signature()?;
    if signature.is_low_s() {
        Ok(())
    } else {
        Err(anyhow::Error::msg(EcdsaError::VerifyFailed))
    }
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
