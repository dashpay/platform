use bincode::{Decode, Encode};

pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeSignatureVersion {
    pub verify_signature_ecdsa_secp256k1: u64,
    pub verify_signature_bls12_381: u64,
    pub verify_signature_ecdsa_hash160: u64,
    pub verify_signature_bip13_script_hash: u64,
    pub verify_signature_eddsa25519_hash160: u64,
    /// Verifying a RedPallas signature by a `REDPALLAS_HECATE` key (read from protocol version
    /// 15, the first that admits the key type). Platform states store only
    /// `fee_version_number`, so this field touches no stored format;
    /// `FeeSignatureVersionFieldsBeforeVersion4` must not gain it.
    pub verify_signature_redpallas: u64,
}

/// The signature fee table exactly as every pre-1.4 platform state serialized it (5 fields).
/// `FeeSignatureVersion` gained `verify_signature_redpallas` in 5.1; embedding the live struct
/// in `FeeVersionFieldsBeforeVersion4` would shift that frozen wire format and break
/// deserialization of old stored states. Any future field added to `FeeSignatureVersion` must
/// NOT be added here.
#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeSignatureVersionFieldsBeforeVersion4 {
    pub verify_signature_ecdsa_secp256k1: u64,
    pub verify_signature_bls12_381: u64,
    pub verify_signature_ecdsa_hash160: u64,
    pub verify_signature_bip13_script_hash: u64,
    pub verify_signature_eddsa25519_hash160: u64,
}

impl From<FeeSignatureVersionFieldsBeforeVersion4> for FeeSignatureVersion {
    fn from(value: FeeSignatureVersionFieldsBeforeVersion4) -> Self {
        FeeSignatureVersion {
            verify_signature_ecdsa_secp256k1: value.verify_signature_ecdsa_secp256k1,
            verify_signature_bls12_381: value.verify_signature_bls12_381,
            verify_signature_ecdsa_hash160: value.verify_signature_ecdsa_hash160,
            verify_signature_bip13_script_hash: value.verify_signature_bip13_script_hash,
            verify_signature_eddsa25519_hash160: value.verify_signature_eddsa25519_hash160,
            // Pre-1.4 tables predate the key type; the field is unread there.
            verify_signature_redpallas: 0,
        }
    }
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
            verify_signature_redpallas: 6,
        };

        let version2 = FeeSignatureVersion {
            verify_signature_ecdsa_secp256k1: 1,
            verify_signature_bls12_381: 2,
            verify_signature_ecdsa_hash160: 3,
            verify_signature_bip13_script_hash: 4,
            verify_signature_eddsa25519_hash160: 5,
            verify_signature_redpallas: 6,
        };

        // This assertion will check if all fields are considered in the equality comparison
        assert_eq!(version1, version2, "FeeSignatureVersion equality test failed. If a field was added or removed, update the Eq implementation.");
    }
}
