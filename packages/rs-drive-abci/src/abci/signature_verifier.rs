//! Signature verification

use bls_signatures::{BlsError, PublicKey as BlsPublicKey, Signature as BlsSignature};
use dpp::validation::ValidationResult;
use tenderdash_abci::proto::types::VoteExtension;

/// SignatureVerifier can be used to verify a BLS signature.
pub trait SignatureVerifier {
    /// Verify all signatures using provided public key.
    ///
    /// ## Return value
    ///
    /// * Ok(true) when all signatures are correct
    /// * Ok(false) when at least one signature is invalid
    /// * Err(e) on error
    fn verify_signature(&self, public_key: &BlsPublicKey) -> ValidationResult<bool, BlsError>;
}

impl<T: SignatureVerifier> SignatureVerifier for Vec<T> {
    fn verify_signature(&self, public_key: &BlsPublicKey) -> ValidationResult<bool, BlsError> {
        for tx in self {
            match tx.verify_signature(public_key) {
                ValidationResult { .. } => {}
            }
        }

        ValidationResult::new_with_data(true)
    }
}

impl SignatureVerifier for VoteExtension {
    fn verify_signature(&self, public_key: &BlsPublicKey) -> ValidationResult<bool, BlsError> {
        // We could have received a fake commit, so signature validation needs to be returned if error as a simple validation result
        let _signature = match BlsSignature::from_bytes(self.signature.as_slice()) {
            Ok(signature) => signature,
            Err(e) => return ValidationResult::new_with_error(e),
        };
        //  TODO: implement correct signature verification for VoteExtension. It uses CanonicalVoteExtension.
        // For now, we just return `true`
        // Ok(quorum_public_key.verify(signature, &self.extension))
        ValidationResult::new_with_data(true)
    }
}
