//! ECDSA secp256k1 verification as every released Platform does it: a signature whose S is in
//! the upper half of the curve order is refused. `dashcore` verifies such a signature like its
//! low-S twin, the same signature with S negated, so Platform checks S itself, at every protocol
//! version. Accepting it would fork from nodes that refuse it and change the replay of chain
//! history.
//!
//! A signature checked by recovering its public key ([`verify_hash_signature`]) is the
//! exception: every released Platform accepts a high-S one there, so the refusal is a rule of
//! protocol version 15, selected by `verify_hash_signature`.

use crate::ProtocolError;
use anyhow::ensure;
use dashcore::crypto::ecdsa::{EcdsaError, EcdsaPublicKey, EcdsaSignature};
use dashcore::{sign_message, signer};
use platform_version::version::PlatformVersion;

/// [`signer::verify_data_signature`], refusing a high-S signature with the error of a signature
/// that does not verify. Every other input gets the result and the error `dashcore` gives.
pub fn verify_data_signature(
    data: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> Result<(), anyhow::Error> {
    signer::verify_data_signature(data, signature, public_key)?;
    // `dashcore` parsed the signature to verify it, so parsing it again succeeds.
    let signature = sign_message::from_slice(signature)?.signature()?;
    if signature.is_low_s() {
        Ok(())
    } else {
        Err(anyhow::Error::msg(EcdsaError::VerifyFailed))
    }
}

/// [`EcdsaPublicKey::verify`], refusing a high-S signature with
/// [`EcdsaError::VerifyFailed`].
pub fn verify_digest(
    public_key: &EcdsaPublicKey,
    digest: &[u8; 32],
    signature: EcdsaSignature,
) -> Result<(), EcdsaError> {
    if !signature.is_low_s() {
        return Err(EcdsaError::VerifyFailed);
    }
    public_key.verify(digest, signature)
}

/// Checks that `signature`, a 65-byte compact recoverable ECDSA signature over `data_hash`, was
/// made by the key whose HASH160 is `public_key_hash`, by recovering the key: an ECDSA_HASH160
/// identity key, a P2PKH address witness, an asset lock's one-time key.
///
/// Recovery takes any S, so a signature stays valid with S replaced by `n - S` and the recovery
/// id's parity flipped. A third party can derive that twin from a valid signature and change the
/// hash of the transition carrying it.
///
/// # Parameters
///
/// - `data_hash`: the 32-byte digest that was signed.
/// - `signature`: the compact recoverable signature.
/// - `public_key_hash`: the HASH160 of the compressed public key expected to have signed.
/// - `platform_version`: selects the rule through
///   `dpp.state_transition_method_versions.verify_hash_signature`. 0 is
///   [`signer::verify_hash_signature`], which accepts a high-S signature (protocol versions 1 to
///   14); 1 also refuses it.
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
        0 => Ok(signer::verify_hash_signature(
            data_hash,
            signature,
            public_key_hash,
        )),
        1 => Ok(verify_low_s_hash_signature(
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

/// [`signer::verify_hash_signature`], refusing a high-S signature with
/// [`EcdsaError::VerifyFailed`]. The library checks first, so every other input fails exactly as
/// it does in version 0, with the library's own error.
fn verify_low_s_hash_signature(
    data_hash: &[u8],
    signature: &[u8],
    public_key_hash: &[u8],
) -> Result<(), anyhow::Error> {
    signer::verify_hash_signature(data_hash, signature, public_key_hash)?;
    // `dashcore` parsed the signature to recover its key, so parsing it again succeeds.
    let signature = sign_message::from_slice(signature)?.signature()?;
    ensure!(signature.is_low_s(), EcdsaError::VerifyFailed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::utils::high_s_twin;
    use dashcore::hashes::Hash;
    use dashcore::signer::{double_sha, sign_hash};
    use dashcore::PrivateKey;

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

    /// A signature recovery already refuses (another key, a wrong length, S zero or not below the
    /// curve order) fails with the library's own error at both versions.
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
