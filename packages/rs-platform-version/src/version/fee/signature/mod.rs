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
    use super::{FeeSignatureVersion, FeeSignatureVersionFieldsBeforeVersion4};

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

    /// A pre-1.4 platform state stored exactly five signature fees, in this order. Their
    /// encoding is built here from a plain tuple, not from the frozen struct, so a field added
    /// to or reordered in the struct fails to decode it or decodes it wrongly.
    #[test]
    fn should_decode_the_five_field_encoding_of_pre_1_4_platform_states() {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let stored: (u64, u64, u64, u64, u64) = (11, 22, 33, 44, 55);
        let bytes = bincode::encode_to_vec(stored, config).expect("encodes");

        let (decoded, read): (FeeSignatureVersionFieldsBeforeVersion4, usize) =
            bincode::decode_from_slice(&bytes, config).expect("decodes");
        assert_eq!(read, bytes.len(), "no trailing bytes are expected");
        assert_eq!(
            decoded,
            FeeSignatureVersionFieldsBeforeVersion4 {
                verify_signature_ecdsa_secp256k1: 11,
                verify_signature_bls12_381: 22,
                verify_signature_ecdsa_hash160: 33,
                verify_signature_bip13_script_hash: 44,
                verify_signature_eddsa25519_hash160: 55,
            }
        );
        assert_eq!(
            bincode::encode_to_vec(&decoded, config).expect("encodes"),
            bytes,
            "the frozen struct encodes as the five fees and nothing else"
        );
        assert_eq!(
            FeeSignatureVersion::from(decoded),
            FeeSignatureVersion {
                verify_signature_ecdsa_secp256k1: 11,
                verify_signature_bls12_381: 22,
                verify_signature_ecdsa_hash160: 33,
                verify_signature_bip13_script_hash: 44,
                verify_signature_eddsa25519_hash160: 55,
                verify_signature_redpallas: 0,
            }
        );
    }
}
