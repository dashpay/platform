use bincode::{Decode, Encode};

pub mod v1;

#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeSignatureVersion {
    pub verify_signature_ecdsa_secp256k1: u64,
    pub verify_signature_bls12_381: u64,
    pub verify_signature_ecdsa_hash160: u64,
    pub verify_signature_bip13_script_hash: u64,
    pub verify_signature_eddsa25519_hash160: u64,
}

#[cfg(test)]
mod tests {
    use super::FeeSignatureVersion;

    #[test]
    // If this test failed, then a new field was added in FeeSignatureVersion. And the corresponding eq needs to be updated as well
    fn test_fee_signature_version_equality() {
        let version1 = FeeSignatureVersion {
            verify_signature_ecdsa_secp256k1: 1,
            verify_signature_bls12_381: 2,
            verify_signature_ecdsa_hash160: 3,
            verify_signature_bip13_script_hash: 4,
            verify_signature_eddsa25519_hash160: 5,
        };

        let version2 = FeeSignatureVersion {
            verify_signature_ecdsa_secp256k1: 1,
            verify_signature_bls12_381: 2,
            verify_signature_ecdsa_hash160: 3,
            verify_signature_bip13_script_hash: 4,
            verify_signature_eddsa25519_hash160: 5,
        };

        // This assertion will check if all fields are considered in the equality comparison
        assert_eq!(version1, version2, "FeeSignatureVersion equality test failed. If a field was added or removed, update the Eq implementation.");
    }
}
