//! DIP-15 `accountReference` (masked account index).

/// `ASK28 = (HMAC-SHA256(sender_secret_key, compact_xpub))[28..32] big-endian >> 4`.
///
/// HMAC input is the 69-byte DIP-15 compact form (the `encryptedPublicKey`
/// plaintext). The ASK28 byte order matches iOS dash-shared-core
/// (`be(ASK[28..32]) >> 4`); see [`extract_ask28`] for the full four-convention
/// split (Android, dash-evo-tool, and the DIP literal all differ). Since
/// `accountReference` is a one-time-pad obfuscation that recipients ignore (only
/// the original sender un-masks it on re-send), every convention round-trips for
/// its own sender; we match iOS so our sent requests are bit-identical to the
/// incumbent wallet's.
fn account_secret_key_28(sender_secret_key: &[u8; 32], compact_xpub: &[u8]) -> u32 {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(sender_secret_key)
        .expect("HMAC accepts a key of any length");
    mac.update(compact_xpub);
    let mut ask = [0u8; 32];
    ask.copy_from_slice(&mac.finalize().into_bytes());
    extract_ask28(&ask)
}

/// Extract `ASK28` from the 32-byte HMAC digest using the iOS dash-shared-core
/// convention: `be(ASK[28..32]) >> 4`. DIP-15 leaves the extraction ambiguous
/// ("28 most significant bits of ASK"); four readings exist in the wild and
/// give different values, but since the field is a sender-private one-time pad
/// there is no on-chain interop failure — we lock to iOS (the most-deployed
/// DashPay wallet) for bit-identical sent requests.
fn extract_ask28(ask_bytes: &[u8; 32]) -> u32 {
    u32::from_be_bytes([ask_bytes[28], ask_bytes[29], ask_bytes[30], ask_bytes[31]]) >> 4
}

/// Calculate the masked DIP-15 `accountReference`:
/// `result = (version << 28) | (ASK28 ^ (account_index & 0x0FFF_FFFF))`.
///
/// Top 4 bits carry the rotation `version` (bumped on each friendship re-key);
/// the low 28 bits are the account index masked by a PRF of the contact xpub so
/// observers can't correlate accounts across requests. Keyed by the sender's
/// 32-byte ECDH private key (the same key that encrypts the xpub).
pub fn calculate_account_reference(
    sender_secret_key: &[u8; 32],
    compact_xpub: &[u8],
    account_index: u32,
    version: u32,
) -> u32 {
    let ask28 = account_secret_key_28(sender_secret_key, compact_xpub);
    let shortened_account_bits = account_index & 0x0FFF_FFFF;
    let version_bits = version << 28;
    version_bits | (ask28 ^ shortened_account_bits)
}

/// Recover `(version, account_index)` from a masked `accountReference`. Inverse
/// of [`calculate_account_reference`] for the same `(sender_secret_key,
/// compact_xpub)` — only the original sender can un-mask (the PRF key is their
/// ECDH private key). Used on re-send to read the previous rotation version.
pub fn unmask_account_reference(
    account_reference: u32,
    sender_secret_key: &[u8; 32],
    compact_xpub: &[u8],
) -> (u32, u32) {
    let ask28 = account_secret_key_28(sender_secret_key, compact_xpub);
    let version = account_reference >> 28;
    let account_index = (account_reference & 0x0FFF_FFFF) ^ ask28;
    (version, account_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic 69-byte compact xpub fixture for the account-reference
    /// tests (the helper only HMACs the bytes, so a synthetic buffer of the
    /// right length keeps the vectors stable).
    fn test_compact_xpub() -> [u8; 69] {
        std::array::from_fn(|i| i as u8)
    }

    #[test]
    fn account_reference_version_bits() {
        let secret_key = [1u8; 32];
        let compact = test_compact_xpub();
        assert_eq!(
            calculate_account_reference(&secret_key, &compact, 0, 0) >> 28,
            0
        );
        assert_eq!(
            calculate_account_reference(&secret_key, &compact, 0, 1) >> 28,
            1
        );
        assert_eq!(
            calculate_account_reference(&secret_key, &compact, 0, 15) >> 28,
            15
        );
    }

    #[test]
    fn account_reference_deterministic() {
        let secret_key = [0xABu8; 32];
        let compact = test_compact_xpub();
        assert_eq!(
            calculate_account_reference(&secret_key, &compact, 0, 0),
            calculate_account_reference(&secret_key, &compact, 0, 0),
            "same inputs → same account reference"
        );
    }

    /// ASK28 must come from HMAC digest bytes `[28..32]` big-endian `>> 4` (iOS
    /// dash-shared-core) — not the head-of-digest reading (the old bug).
    #[test]
    fn account_reference_ask28_uses_digest_tail_big_endian() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let secret_key = [0x42u8; 32];
        let compact = test_compact_xpub();

        let mut mac = Hmac::<Sha256>::new_from_slice(&secret_key).expect("hmac key");
        mac.update(&compact);
        let mut digest = [0u8; 32];
        digest.copy_from_slice(&mac.finalize().into_bytes());
        let expected_ask28 =
            u32::from_be_bytes([digest[28], digest[29], digest[30], digest[31]]) >> 4;

        let reference = calculate_account_reference(&secret_key, &compact, 0, 0);
        assert_eq!(
            reference & 0x0FFF_FFFF,
            expected_ask28,
            "ASK28 must be digest bytes [28..32] big-endian >> 4 (iOS dash-shared-core)"
        );
        let old_ask28 = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) >> 4;
        assert_ne!(
            reference & 0x0FFF_FFFF,
            old_ask28,
            "head-of-digest extraction is the old bug"
        );
    }

    /// Mask → unmask round-trips `(version, account_index)` for the sender.
    #[test]
    fn account_reference_round_trips_version_and_account() {
        let secret_key = [0x07u8; 32];
        let compact = test_compact_xpub();
        for version in [0u32, 1, 7, 15] {
            for account in [0u32, 1, 5, 0x0FFF_FFFF] {
                let reference =
                    calculate_account_reference(&secret_key, &compact, account, version);
                let (got_version, got_account) =
                    unmask_account_reference(reference, &secret_key, &compact);
                assert_eq!(got_version, version, "version round-trip");
                assert_eq!(got_account, account, "account round-trip");
            }
        }
        let reference = calculate_account_reference(&secret_key, &compact, 5, 0);
        let (_, wrong) = unmask_account_reference(reference, &[0x08u8; 32], &compact);
        assert_ne!(wrong, 5, "a different PRF key must not unmask the account");
    }

    /// Known-answer pin for the ASK28 extraction conventions (iOS vs the others).
    #[test]
    fn ask28_extraction_matches_ios_and_diverges_from_others() {
        let ask: [u8; 32] = std::array::from_fn(|i| i as u8);
        assert_eq!(
            extract_ask28(&ask),
            0x01c1_d1e1,
            "iOS dash-shared-core: be(ASK[28..32])>>4"
        );
        let android = u32::from_le_bytes([ask[0], ask[1], ask[2], ask[3]]) >> 4;
        let dip_literal = u32::from_be_bytes([ask[0], ask[1], ask[2], ask[3]]) >> 4;
        assert_eq!(android, 0x0030_2010, "kotlin-platform: le(ASK[0..4])>>4");
        assert_eq!(
            dip_literal, 0x0000_1020,
            "dash-evo-tool / DIP literal: be(ASK[0..4])>>4"
        );
        assert_ne!(extract_ask28(&ask), android);
        assert_ne!(extract_ask28(&ask), dip_literal);
    }
}
