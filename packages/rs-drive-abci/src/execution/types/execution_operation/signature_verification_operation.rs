use crate::error::Error;
use crate::execution::types::execution_operation::OperationLike;
use dpp::fee::Credits;
use dpp::identity::KeyType;
use dpp::version::PlatformVersion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignatureVerificationOperation {
    pub signature_type: KeyType,
}

impl SignatureVerificationOperation {
    pub fn new(signature_type: KeyType) -> Self {
        Self { signature_type }
    }
}

impl OperationLike for SignatureVerificationOperation {
    fn processing_cost(&self, platform_version: &PlatformVersion) -> Result<Credits, Error> {
        Ok(self
            .signature_type
            .signature_verify_cost(platform_version)?)
    }

    fn storage_cost(&self, _platform_version: &PlatformVersion) -> Result<Credits, Error> {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_version::version::PlatformVersion;

    #[test]
    fn new_creates_operation_with_correct_type() {
        let op = SignatureVerificationOperation::new(KeyType::ECDSA_SECP256K1);
        assert_eq!(op.signature_type, KeyType::ECDSA_SECP256K1);
    }

    #[test]
    fn processing_cost_returns_nonzero_for_ecdsa() {
        let op = SignatureVerificationOperation::new(KeyType::ECDSA_SECP256K1);
        let cost = op.processing_cost(PlatformVersion::latest()).unwrap();
        assert!(cost > 0, "ECDSA verification cost should be nonzero");
    }

    #[test]
    fn processing_cost_returns_nonzero_for_bls() {
        let op = SignatureVerificationOperation::new(KeyType::BLS12_381);
        let cost = op.processing_cost(PlatformVersion::latest()).unwrap();
        assert!(cost > 0, "BLS verification cost should be nonzero");
    }

    #[test]
    fn processing_cost_returns_nonzero_for_eddsa() {
        let op = SignatureVerificationOperation::new(KeyType::EDDSA_25519_HASH160);
        let cost = op.processing_cost(PlatformVersion::latest()).unwrap();
        assert!(cost > 0, "EDDSA verification cost should be nonzero");
    }

    #[test]
    fn storage_cost_is_always_zero() {
        let op = SignatureVerificationOperation::new(KeyType::ECDSA_SECP256K1);
        let cost = op.storage_cost(PlatformVersion::latest()).unwrap();
        assert_eq!(cost, 0);
    }

    #[test]
    fn different_key_types_have_different_processing_costs() {
        let pv = PlatformVersion::latest();
        let ecdsa_cost = SignatureVerificationOperation::new(KeyType::ECDSA_SECP256K1)
            .processing_cost(pv)
            .unwrap();
        let bls_cost = SignatureVerificationOperation::new(KeyType::BLS12_381)
            .processing_cost(pv)
            .unwrap();
        // BLS verification is typically more expensive
        assert_ne!(ecdsa_cost, bls_cost);
    }
}
