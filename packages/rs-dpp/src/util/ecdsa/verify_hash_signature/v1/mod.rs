use anyhow::ensure;
use dashcore::crypto::ecdsa::EcdsaError;
use dashcore::{sign_message, signer};

/// Generation 1, from protocol version 15: `dashcore`'s recovery check, then the signature must
/// be the one form a signer produces. A header naming an uncompressed key (`27 + id` in place
/// of `31 + id`) and an S in the upper half of the curve order are refused with
/// [`EcdsaError::VerifyFailed`], the error of a signature that does not verify.
///
/// The library checks first, so every other input fails exactly as it does in generation 0,
/// with the library's own error.
#[inline(always)]
pub(super) fn verify_hash_signature_v1(
    data_hash: &[u8],
    signature: &[u8],
    public_key_hash: &[u8],
) -> Result<(), anyhow::Error> {
    signer::verify_hash_signature(data_hash, signature, public_key_hash)?;
    // `dashcore` parsed the signature to recover its key, so parsing it again succeeds.
    let signature = sign_message::from_slice(signature)?;
    // The key hash compared above is the compressed key's, whatever the header says.
    ensure!(signature.is_compressed()?, EcdsaError::VerifyFailed);
    ensure!(signature.signature()?.is_low_s(), EcdsaError::VerifyFailed);
    Ok(())
}
