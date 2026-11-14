use crate::util::ecdsa::verify_hash_signature::v0::verify_hash_signature_v0;
use crate::util::ecdsa::verify_hash_signature::v1::verify_hash_signature_v1;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;
mod v1;

/// Checks that `signature`, a 65-byte compact recoverable ECDSA signature over `data_hash`, was
/// made by the key whose HASH160 is `public_key_hash`, by recovering the key: an ECDSA_HASH160
/// identity key, a P2PKH address witness, an asset lock's one-time key.
///
/// Recovery alone leaves a third party two ways to turn a valid signature into another valid
/// one, and so to change the hash of the transition carrying it:
///
/// - it takes any S, so the signature stays valid with S replaced by `n - S` and the recovery
///   id's parity flipped;
/// - it hashes the recovered key in its compressed form whatever the header says, so the
///   signature stays valid with its header rewritten from `31 + id` (a compressed key) to
///   `27 + id` (an uncompressed one).
///
/// # Parameters
///
/// - `data_hash`: the 32-byte digest that was signed.
/// - `signature`: the compact recoverable signature.
/// - `public_key_hash`: the HASH160 of the compressed public key expected to have signed.
/// - `platform_version`: selects the rule through
///   `dpp.state_transition_method_versions.verify_hash_signature`. 0 is
///   `dashcore::signer::verify_hash_signature`, which accepts both rewrites (protocol versions
///   1 to 14); 1 also refuses a high-S signature and a header that does not name a compressed
///   key.
///
/// # Returns
///
/// `Ok(Ok(()))` when the signature is valid, `Ok(Err(_))` with the reason when it is not, and
/// `Err(ProtocolError::UnknownVersionMismatch)` for a version this build does not know.
pub fn verify_hash_signature(
    data_hash: &[u8],
    signature: &[u8],
    public_key_hash: &[u8],
    platform_version: &PlatformVersion,
) -> Result<Result<(), anyhow::Error>, ProtocolError> {
    match platform_version
        .dpp
        .state_transition_method_versions
        .verify_hash_signature
    {
        0 => Ok(verify_hash_signature_v0(
            data_hash,
            signature,
            public_key_hash,
        )),
        1 => Ok(verify_hash_signature_v1(
            data_hash,
            signature,
            public_key_hash,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "verify_hash_signature".to_string(),
            known_versions: vec![0, 1],
            received: version,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::utils::{high_s_twin, uncompressed_header_twin};
    use dashcore::crypto::ecdsa::{EcdsaError, EcdsaSignature};
    use dashcore::signer::{double_sha, sign_hash};
    use dashcore::{sign_message, signer, PrivateKey};

    /// Half the curve order, `n / 2`, the largest low S.
    const HALF_ORDER: [u8; 32] = [
        0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x5d, 0x57, 0x6e, 0x73, 0x57, 0xa4, 0x50, 0x1d, 0xdf, 0xe9, 0x2f, 0x46, 0x68, 0x1b,
        0x20, 0xa0,
    ];
    /// The curve order `n`.
    const ORDER: [u8; 32] = [
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xfe, 0xba, 0xae, 0xdc, 0xe6, 0xaf, 0x48, 0xa0, 0x3b, 0xbf, 0xd2, 0x5e, 0x8c, 0xd0, 0x36,
        0x41, 0x41,
    ];

    fn signature_s_is_low(signature: &[u8]) -> bool {
        sign_message::from_slice(signature)
            .expect("a compact recoverable signature")
            .signature()
            .expect("a valid signature")
            .is_low_s()
    }

    /// A low-S signature over a digest, the digest, and the signing key's HASH160.
    fn fixture() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let secret = [0x11; 32];
        let key_hash = PrivateKey::from_byte_array(&secret, dashcore::Network::Testnet)
            .expect("a valid key")
            .public_key()
            .pubkey_hash();
        let digest = double_sha(b"a message");
        let signature = sign_hash(&digest, &secret).expect("signs").to_vec();
        (digest, signature, key_hash.to_byte_array().to_vec())
    }

    /// `signature` with its S replaced by `s`.
    fn with_s(signature: &[u8], s: [u8; 32]) -> Vec<u8> {
        let mut changed = signature.to_vec();
        changed[33..].copy_from_slice(&s);
        changed
    }

    /// `signature` with its header byte replaced by `header`.
    fn with_header(signature: &[u8], header: u8) -> Vec<u8> {
        let mut changed = signature.to_vec();
        changed[0] = header;
        changed
    }

    fn protocol_version(version: u32) -> &'static PlatformVersion {
        PlatformVersion::get(version).expect("a known protocol version")
    }

    #[test]
    fn should_hold_s_equal_to_half_the_order_low_and_one_more_high() {
        let mut compact = [1u8; 64];
        compact[32..].copy_from_slice(&HALF_ORDER);
        assert!(EcdsaSignature::from_bytes(&compact)
            .expect("a valid signature")
            .is_low_s());
        compact[63] += 1;
        assert!(!EcdsaSignature::from_bytes(&compact)
            .expect("a valid signature")
            .is_low_s());
    }

    #[test]
    fn should_build_a_high_s_twin_that_recovery_alone_accepts() {
        let (digest, signature, key_hash) = fixture();
        let twin = high_s_twin(&signature);
        assert!(signature_s_is_low(&signature));
        assert!(!signature_s_is_low(&twin));
        signer::verify_hash_signature(&digest, &twin, &key_hash)
            .expect("recovery alone accepts the twin");
    }

    #[test]
    fn should_build_an_uncompressed_header_twin_that_recovery_alone_accepts() {
        let (digest, signature, key_hash) = fixture();
        let twin = uncompressed_header_twin(&signature);
        assert_eq!(twin[0] + 4, signature[0]);
        assert_eq!(twin[1..], signature[1..]);
        assert!(signature_s_is_low(&twin));
        signer::verify_hash_signature(&digest, &twin, &key_hash)
            .expect("recovery alone accepts the twin");
    }

    #[test]
    fn should_accept_the_high_s_twin_up_to_protocol_version_14() {
        let (digest, signature, key_hash) = fixture();
        let twin = high_s_twin(&signature);
        for version in [1, 13, 14] {
            let platform_version = protocol_version(version);
            verify_hash_signature(&digest, &signature, &key_hash, platform_version)
                .expect("a known version")
                .expect("the low-S signature is valid");
            verify_hash_signature(&digest, &twin, &key_hash, platform_version)
                .expect("a known version")
                .expect("protocol version 14 and earlier accept the high-S twin");
        }
    }

    #[test]
    fn should_refuse_the_high_s_twin_at_the_latest_protocol_version() {
        let (digest, signature, key_hash) = fixture();
        let platform_version = PlatformVersion::latest();
        verify_hash_signature(&digest, &signature, &key_hash, platform_version)
            .expect("a known version")
            .expect("the low-S signature is valid");
        let error = verify_hash_signature(
            &digest,
            &high_s_twin(&signature),
            &key_hash,
            platform_version,
        )
        .expect("a known version")
        .expect_err("the high-S twin is refused");
        assert_eq!(error.to_string(), EcdsaError::VerifyFailed.to_string());
    }

    #[test]
    fn should_accept_the_uncompressed_header_twin_up_to_protocol_version_14() {
        let (digest, signature, key_hash) = fixture();
        let twin = uncompressed_header_twin(&signature);
        for version in [1, 13, 14] {
            verify_hash_signature(&digest, &twin, &key_hash, protocol_version(version))
                .expect("a known version")
                .expect("protocol version 14 and earlier accept the uncompressed header");
        }
    }

    #[test]
    fn should_refuse_the_uncompressed_header_twin_at_the_latest_protocol_version() {
        let (digest, signature, key_hash) = fixture();
        let platform_version = PlatformVersion::latest();
        // The twin, and the twin of the high-S twin: neither rewrite hides the other.
        for twin in [
            uncompressed_header_twin(&signature),
            uncompressed_header_twin(&high_s_twin(&signature)),
        ] {
            let error = verify_hash_signature(&digest, &twin, &key_hash, platform_version)
                .expect("a known version")
                .expect_err("the uncompressed header is refused");
            assert_eq!(error.to_string(), EcdsaError::VerifyFailed.to_string());
        }
    }

    /// Of the eight headers recovery parses, 27 to 34, the latest protocol version accepts one:
    /// the compressed-key header carrying the recovery id that recovers the signing key.
    #[test]
    fn should_accept_a_single_header_at_the_latest_protocol_version() {
        let (digest, signature, key_hash) = fixture();
        let accepted: Vec<u8> = (27..=34)
            .filter(|header| {
                let mut candidate = signature.clone();
                candidate[0] = *header;
                verify_hash_signature(&digest, &candidate, &key_hash, PlatformVersion::latest())
                    .expect("a known version")
                    .is_ok()
            })
            .collect();
        assert_eq!(accepted, vec![signature[0]]);
    }

    /// A signature recovery already refuses (another key, a wrong length, S zero or not below the
    /// curve order, a header outside 27 to 34) fails with the library's own error at both
    /// versions.
    #[test]
    fn should_fail_an_invalid_signature_with_the_same_error_at_both_versions() {
        let (digest, signature, key_hash) = fixture();
        let wrong_key_hash = [7u8; 20];
        let cases = [
            (signature.clone(), wrong_key_hash.to_vec(), "another key"),
            (signature[..64].to_vec(), key_hash.clone(), "a wrong length"),
            (with_s(&signature, [0; 32]), key_hash.clone(), "S zero"),
            (with_s(&signature, ORDER), key_hash.clone(), "S equal to n"),
            (
                with_s(&signature, [0xff; 32]),
                key_hash.clone(),
                "S above n",
            ),
            (with_header(&signature, 26), key_hash.clone(), "header 26"),
            (with_header(&signature, 35), key_hash.clone(), "header 35"),
        ];
        for (signature, key_hash, label) in cases {
            let legacy =
                verify_hash_signature(&digest, &signature, &key_hash, protocol_version(14))
                    .expect("a known version")
                    .expect_err(label);
            let latest =
                verify_hash_signature(&digest, &signature, &key_hash, PlatformVersion::latest())
                    .expect("a known version")
                    .expect_err(label);
            assert_eq!(legacy.to_string(), latest.to_string(), "{label}");
        }
    }
}
