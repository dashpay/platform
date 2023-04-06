//! Signature verification

use dpp::bls_signatures::Serialize;
use tenderdash_abci::proto::types::VoteExtension;

/// Errors occured during signature verification
#[derive(Debug, thiserror::Error)]
pub enum SignatureError {
    /// Error occurred during bls signature verification
    #[error("bls error set: {0}")]
    BLSError(#[from] dpp::bls_signatures::Error),
}
/// SignatureVerifier can be used to verify a BLS signature.
pub trait SignatureVerifier {
    /// Verify all signatures using provided public key.
    ///
    /// ## Return value
    ///
    /// * Ok(true) when all signatures are correct
    /// * Ok(false) when at least one signature is invalid
    /// * Err(e) on error
    fn verify_signature(
        &self,
        quorum_public_key: dpp::bls_signatures::PublicKey,
    ) -> Result<bool, SignatureError>;
}

impl<T: SignatureVerifier> SignatureVerifier for Vec<T> {
    fn verify_signature(
        &self,
        quorum_public_key: dpp::bls_signatures::PublicKey,
    ) -> Result<bool, SignatureError> {
        for tx in self {
            if !tx.verify_signature(quorum_public_key)? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

impl SignatureVerifier for VoteExtension {
    fn verify_signature(
        &self,
        _quorum_public_key: dpp::bls_signatures::PublicKey,
    ) -> Result<bool, SignatureError> {
        let signature = &self.signature;

        // We could have received a fake commit, so signature validation needs to be returned if error as a simple validation result
        let _signature = match dpp::bls_signatures::Signature::from_bytes(signature.as_slice()) {
            Ok(signature) => signature,
            Err(e) => return Err(SignatureError::from(e)),
        };
        //  TODO: implement correct signature verification for VoteExtension. It uses CanonicalVoteExtension.
        // For now, we just return `true`
        // Ok(quorum_public_key.verify(signature, &self.extension))
        Ok(true)
    }
}
