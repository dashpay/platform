use crate::version::fee::signature::v1::FEE_SIGNATURE_VERSION1;
use bincode::{Decode, Encode};

pub mod v1;

#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeSignatureVersion {
    pub verify_signature_ecdsa_secp256k1: u64,
    pub verify_signature_bls12_381: u64,
    pub verify_signature_ecdsa_hash160: u64,
    pub verify_signature_bip13_script_hash: u64,
    pub verify_signature_eddsa25519_hash160: u64,
    /// Reading an ML_DSA_65 public key and verifying one signature with it. Read by
    /// `KeyType::signature_verify_cost` 0 from protocol version 16: no key of the type exists
    /// before it, so no earlier version reads the field. Platform states store only
    /// `fee_version_number`, so the field touches no stored format;
    /// `FeeSignatureVersionFieldsBeforeVersion4` must not gain it.
    pub verify_signature_ml_dsa_65: u64,
}

/// The signature fee table exactly as the platform states saved before version 1.4 serialized
/// it (5 fields). `FeeSignatureVersion` gained `verify_signature_ml_dsa_65` with protocol
/// version 16; embedding the live struct in `FeeVersionFieldsBeforeVersion4` would shift that
/// frozen wire format and break deserialization of old stored states. Any future field added to
/// `FeeSignatureVersion` must NOT be added here.
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
            // Tables saved before version 1.4 predate the key type; the field is unread there.
            verify_signature_ml_dsa_65: FEE_SIGNATURE_VERSION1.verify_signature_ml_dsa_65,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FeeSignatureVersion, FeeSignatureVersionFieldsBeforeVersion4};
    use crate::version::fee::signature::v1::FEE_SIGNATURE_VERSION1;

    #[test]
    // If this test failed, then a new field was added in FeeSignatureVersion. And the corresponding eq needs to be updated as well
    fn test_fee_signature_version_equality() {
        let version1 = FeeSignatureVersion {
            verify_signature_ecdsa_secp256k1: 1,
            verify_signature_bls12_381: 2,
            verify_signature_ecdsa_hash160: 3,
            verify_signature_bip13_script_hash: 4,
            verify_signature_eddsa25519_hash160: 5,
            verify_signature_ml_dsa_65: 6,
        };

        let version2 = FeeSignatureVersion {
            verify_signature_ecdsa_secp256k1: 1,
            verify_signature_bls12_381: 2,
            verify_signature_ecdsa_hash160: 3,
            verify_signature_bip13_script_hash: 4,
            verify_signature_eddsa25519_hash160: 5,
            verify_signature_ml_dsa_65: 6,
        };

        // This assertion will check if all fields are considered in the equality comparison
        assert_eq!(version1, version2, "FeeSignatureVersion equality test failed. If a field was added or removed, update the Eq implementation.");
    }

    /// A platform state saved before version 1.4 holds the signature fees as five numbers. The
    /// frozen struct must keep reading exactly those bytes, whatever the live table gains.
    #[test]
    fn should_read_the_signature_fees_of_a_state_saved_before_version_1_4() {
        let config = bincode::config::standard();
        let saved = bincode::encode_to_vec(
            (15_000u64, 300_000u64, 15_500u64, 300_000u64, 3_500u64),
            config,
        )
        .expect("expected to encode the five saved fees");

        let (frozen, read): (FeeSignatureVersionFieldsBeforeVersion4, usize) =
            bincode::decode_from_slice(&saved, config).expect("expected to decode the saved fees");
        assert_eq!(read, saved.len());

        assert_eq!(FeeSignatureVersion::from(frozen), FEE_SIGNATURE_VERSION1);
    }
}
