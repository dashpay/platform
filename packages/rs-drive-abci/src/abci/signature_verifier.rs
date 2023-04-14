//! Signature verification

use dpp::bls_signatures::Serialize;
use tenderdash_abci::proto::signatures::SignBytes;

/// Errors occured during signature verification
#[derive(Debug, thiserror::Error)]
pub enum SignatureError {
    /// Error occurred during bls signature verification
    #[error("bls error set: {0}")]
    BLSError(#[from] dpp::bls_signatures::Error),

    /// Error creating canonical form of signed data
    #[error("error creating canonical form of signed data: {0}")]
    CanonicalError(tenderdash_abci::proto::Error),
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
        signature: &Vec<u8>,
        chain_id: &str,
        height: i64,
        round: i32,
        quorum_public_key: &dpp::bls_signatures::PublicKey,
    ) -> Result<bool, SignatureError>;
}

impl<T: SignBytes> SignatureVerifier for T {
    fn verify_signature(
        &self,
        signature: &Vec<u8>,
        chain_id: &str,
        height: i64,
        round: i32,
        quorum_public_key: &dpp::bls_signatures::PublicKey,
    ) -> Result<bool, SignatureError> {
        // We could have received a fake commit, so signature validation needs to be returned if error as a simple validation result
        let signature = match dpp::bls_signatures::Signature::from_bytes(signature.as_slice()) {
            Ok(signature) => signature,
            Err(e) => return Err(SignatureError::from(e)),
        };

        let hash = self
            .sha256(chain_id, height, round)
            .map_err(SignatureError::CanonicalError)?;
        Ok(quorum_public_key.verify(signature, hash))
    }
}
