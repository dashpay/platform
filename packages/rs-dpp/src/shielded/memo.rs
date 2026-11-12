//! Structured 36-byte shielded note memo (`DashMemo`).
//!
//! Every shielded output carries a fixed 36-byte memo
//! ([`MEMO_SIZE`]). The dashpay Orchard fork pins the memo to 36
//! bytes (Zcash's is 512) by making the memo size a type parameter;
//! the on-chain note ciphertext reserves exactly those bytes. This
//! type imposes a small self-describing structure on those bytes so
//! the wallet can attach an optional text note to a transfer and
//! recover it on the receive side:
//!
//!   * bytes `[0..4]` — `kind` as a little-endian `u32`.
//!   * bytes `[4..36]` — a 32-byte payload whose meaning depends on
//!     `kind`.
//!
//! Three kinds are defined today; unknown kinds round-trip verbatim as
//! [`ShieldedMemo::Other`] so a future writer's memo is never
//! corrupted by an older reader.

use crate::ProtocolError;

/// Total memo length in bytes (`DashMemo = [u8; 36]`).
pub const MEMO_SIZE: usize = 36;

/// Length of the payload that follows the 4-byte kind tag.
pub const MEMO_PAYLOAD_SIZE: usize = MEMO_SIZE - 4;

/// `kind` tag for an empty memo (the all-zero 36 bytes today's senders write).
const KIND_EMPTY: u32 = 0;
/// `kind` tag for a UTF-8 text memo (zero-padded payload).
const KIND_TEXT: u32 = 1;
/// `kind` tag for a payment reference (zero-padded payload).
const KIND_PAYMENT_REF: u32 = 2;

/// A decoded shielded note memo.
///
/// `to_bytes`/`from_bytes` are exact inverses for every variant: a
/// memo written by [`to_bytes`](Self::to_bytes) decodes back to an
/// equal value, and any 36 bytes decode to *some* variant (unknown
/// kinds and malformed text fall back to [`Self::Other`]), so decoding
/// is total and never loses bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShieldedMemo {
    /// No memo: the all-zero 36 bytes. Today's `[0u8; 36]` memos decode to this.
    Empty,
    /// A UTF-8 text memo whose byte length is at most [`MEMO_PAYLOAD_SIZE`].
    Text(String),
    /// A payment reference: what the payment is for, in a value the payee chose and
    /// gave to the payer, such as an invoice or order number, padded on the right with
    /// zero bytes. The payee matches all 32 bytes against its own records.
    ///
    /// It never says who paid. Anyone who has seen a reference can attach it to a
    /// payment, so the payee checks the amount it received against the request.
    PaymentRef([u8; MEMO_PAYLOAD_SIZE]),
    /// Any memo whose `kind` this version does not interpret (or a
    /// structurally-invalid known kind). Retained verbatim so it
    /// round-trips unchanged.
    Other {
        /// The raw 4-byte `kind` tag.
        kind: u32,
        /// The raw 32-byte payload.
        payload: [u8; MEMO_PAYLOAD_SIZE],
    },
}

impl ShieldedMemo {
    /// Build a text memo, validating that `s` is at most
    /// [`MEMO_PAYLOAD_SIZE`] bytes when UTF-8 encoded.
    pub fn text(s: impl Into<String>) -> Result<Self, ProtocolError> {
        let s = s.into();
        if s.len() > MEMO_PAYLOAD_SIZE {
            return Err(ProtocolError::Generic(format!(
                "shielded text memo is {} bytes, exceeds the {MEMO_PAYLOAD_SIZE}-byte limit",
                s.len()
            )));
        }
        Ok(ShieldedMemo::Text(s))
    }

    /// Build a payment reference memo from the reference the payee supplied, of 1 to
    /// [`MEMO_PAYLOAD_SIZE`] bytes, padded on the right with zero bytes. A payee whose
    /// own identifier is longer issues a 32-byte hash of it.
    ///
    /// An empty or all-zero reference is refused: 32 zero bytes are no reference.
    pub fn payment_ref(reference: &[u8]) -> Result<Self, ProtocolError> {
        if reference.len() > MEMO_PAYLOAD_SIZE {
            return Err(ProtocolError::Generic(format!(
                "shielded payment reference is {} bytes, exceeds the {MEMO_PAYLOAD_SIZE}-byte limit",
                reference.len()
            )));
        }
        if reference.iter().all(|&byte| byte == 0) {
            return Err(ProtocolError::Generic(
                "shielded payment reference is empty or all zero bytes".to_string(),
            ));
        }
        let mut payload = [0u8; MEMO_PAYLOAD_SIZE];
        payload[..reference.len()].copy_from_slice(reference);
        Ok(ShieldedMemo::PaymentRef(payload))
    }

    /// A payment reference as text, when it is valid UTF-8 once its zero padding is
    /// removed. A reference that is not, a hash for example, is shown in hexadecimal.
    pub fn payment_ref_text(reference: &[u8; MEMO_PAYLOAD_SIZE]) -> Option<&str> {
        let end = reference.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
        std::str::from_utf8(&reference[..end])
            .ok()
            .filter(|text| !text.is_empty())
    }

    /// Encode to the on-chain 36-byte memo layout.
    pub fn to_bytes(&self) -> [u8; MEMO_SIZE] {
        let mut out = [0u8; MEMO_SIZE];
        let (kind, payload): (u32, [u8; MEMO_PAYLOAD_SIZE]) = match self {
            ShieldedMemo::Empty => (KIND_EMPTY, [0u8; MEMO_PAYLOAD_SIZE]),
            ShieldedMemo::Text(s) => {
                // `text()` is the only constructor and enforces the length bound; a
                // directly-built `Text` longer than the payload is truncated rather than
                // panicking (the surplus simply cannot fit the fixed-size memo).
                let mut payload = [0u8; MEMO_PAYLOAD_SIZE];
                let bytes = s.as_bytes();
                let n = bytes.len().min(MEMO_PAYLOAD_SIZE);
                payload[..n].copy_from_slice(&bytes[..n]);
                (KIND_TEXT, payload)
            }
            ShieldedMemo::PaymentRef(reference) => (KIND_PAYMENT_REF, *reference),
            ShieldedMemo::Other { kind, payload } => (*kind, *payload),
        };
        out[0..4].copy_from_slice(&kind.to_le_bytes());
        out[4..MEMO_SIZE].copy_from_slice(&payload);
        out
    }

    /// Decode from the on-chain 36-byte memo layout.
    ///
    /// Decoding is total: a `KIND_TEXT` memo whose payload is not valid
    /// UTF-8, a `KIND_EMPTY` memo with a non-zero payload, or a
    /// `KIND_PAYMENT_REF` memo with an all-zero payload falls back
    /// to [`Self::Other`] so the raw bytes are preserved rather than lost.
    pub fn from_bytes(bytes: &[u8; MEMO_SIZE]) -> Self {
        let kind = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let mut payload = [0u8; MEMO_PAYLOAD_SIZE];
        payload.copy_from_slice(&bytes[4..MEMO_SIZE]);

        match kind {
            KIND_EMPTY => {
                if payload == [0u8; MEMO_PAYLOAD_SIZE] {
                    ShieldedMemo::Empty
                } else {
                    // kind 0 is defined as the all-zero memo; a non-zero payload under
                    // kind 0 is not something we wrote, so keep it verbatim.
                    ShieldedMemo::Other { kind, payload }
                }
            }
            KIND_TEXT => {
                // Trailing zero padding is not part of the text; trim it before decoding.
                let end = payload.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
                match std::str::from_utf8(&payload[..end]) {
                    Ok(s) => ShieldedMemo::Text(s.to_string()),
                    Err(_) => ShieldedMemo::Other { kind, payload },
                }
            }
            // 32 zero bytes are no reference, so that memo is not read as one.
            KIND_PAYMENT_REF if payload != [0u8; MEMO_PAYLOAD_SIZE] => {
                ShieldedMemo::PaymentRef(payload)
            }
            _ => ShieldedMemo::Other { kind, payload },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_round_trips() {
        let memo = ShieldedMemo::Empty;
        let bytes = memo.to_bytes();
        assert_eq!(bytes, [0u8; MEMO_SIZE], "empty memo must be all-zero bytes");
        assert_eq!(ShieldedMemo::from_bytes(&bytes), ShieldedMemo::Empty);
    }

    #[test]
    fn legacy_all_zero_memo_decodes_as_empty() {
        // Every memo written before this type existed was `[0u8; 36]`.
        assert_eq!(
            ShieldedMemo::from_bytes(&[0u8; MEMO_SIZE]),
            ShieldedMemo::Empty
        );
    }

    #[test]
    fn text_round_trips() {
        let memo = ShieldedMemo::text("thanks for lunch").expect("within limit");
        let bytes = memo.to_bytes();
        assert_eq!(
            ShieldedMemo::from_bytes(&bytes),
            ShieldedMemo::Text("thanks for lunch".to_string())
        );
    }

    #[test]
    fn empty_string_text_round_trips() {
        let memo = ShieldedMemo::text("").expect("empty string is within limit");
        let bytes = memo.to_bytes();
        // An empty text payload is all-zero, but the kind tag is 1, so it stays Text("").
        assert_eq!(
            ShieldedMemo::from_bytes(&bytes),
            ShieldedMemo::Text(String::new())
        );
    }

    #[test]
    fn multibyte_utf8_exactly_at_limit_round_trips() {
        // 8 × 🍕 (U+1F355) = 8 × 4 = 32 bytes, exactly MEMO_PAYLOAD_SIZE.
        let s = "🍕".repeat(8);
        assert_eq!(
            s.len(),
            MEMO_PAYLOAD_SIZE,
            "fixture must be exactly 32 bytes"
        );
        let memo = ShieldedMemo::text(s.clone()).expect("32 bytes is within limit");
        let bytes = memo.to_bytes();
        assert_eq!(ShieldedMemo::from_bytes(&bytes), ShieldedMemo::Text(s));
    }

    #[test]
    fn over_limit_text_is_rejected() {
        // 33 ASCII bytes — one over the 32-byte payload.
        let s = "a".repeat(MEMO_PAYLOAD_SIZE + 1);
        assert!(
            ShieldedMemo::text(s).is_err(),
            "a 33-byte text memo must be rejected"
        );
    }

    #[test]
    fn multibyte_over_limit_is_rejected() {
        // 9 × 🍕 = 36 bytes > 32.
        let s = "🍕".repeat(9);
        assert!(s.len() > MEMO_PAYLOAD_SIZE);
        assert!(
            ShieldedMemo::text(s).is_err(),
            "a 36-byte multibyte memo must be rejected"
        );
    }

    #[test]
    fn should_encode_a_payment_reference_padded_to_32_bytes() {
        // The vector of the DashPay shielded payment addresses DIP.
        let memo = ShieldedMemo::payment_ref(b"INV-2026-0042").expect("a 13-byte reference");
        let bytes = memo.to_bytes();
        let mut expected = [0u8; MEMO_SIZE];
        expected[0] = 2;
        expected[4..17].copy_from_slice(b"INV-2026-0042");
        assert_eq!(bytes, expected);
        assert_eq!(ShieldedMemo::from_bytes(&bytes), memo);

        let ShieldedMemo::PaymentRef(reference) = memo else {
            panic!("expected a payment reference");
        };
        assert_eq!(
            ShieldedMemo::payment_ref_text(&reference),
            Some("INV-2026-0042")
        );
    }

    #[test]
    fn should_round_trip_a_32_byte_payment_reference_that_is_not_text() {
        // A hash: every byte used, not UTF-8, ending in a zero byte that is part of it.
        let mut reference = [0xF5u8; MEMO_PAYLOAD_SIZE];
        reference[31] = 0;
        let memo = ShieldedMemo::payment_ref(&reference).expect("a 32-byte reference");
        assert_eq!(memo, ShieldedMemo::PaymentRef(reference));
        assert_eq!(ShieldedMemo::from_bytes(&memo.to_bytes()), memo);
        assert_eq!(ShieldedMemo::payment_ref_text(&reference), None);
    }

    #[test]
    fn should_read_references_that_differ_only_in_trailing_zero_bytes_as_one() {
        assert_eq!(
            ShieldedMemo::payment_ref(&[1]).expect("a reference"),
            ShieldedMemo::payment_ref(&[1, 0]).expect("a reference")
        );
    }

    #[test]
    fn should_refuse_a_payment_reference_that_is_empty_all_zero_or_too_long() {
        assert!(ShieldedMemo::payment_ref(&[]).is_err());
        assert!(ShieldedMemo::payment_ref(&[0; 5]).is_err());
        assert!(ShieldedMemo::payment_ref(&[0; MEMO_PAYLOAD_SIZE]).is_err());
        assert!(ShieldedMemo::payment_ref(&[1; MEMO_PAYLOAD_SIZE + 1]).is_err());
    }

    #[test]
    fn should_not_read_an_all_zero_payload_as_a_payment_reference() {
        let mut bytes = [0u8; MEMO_SIZE];
        bytes[0..4].copy_from_slice(&KIND_PAYMENT_REF.to_le_bytes());
        assert_eq!(
            ShieldedMemo::from_bytes(&bytes),
            ShieldedMemo::Other {
                kind: KIND_PAYMENT_REF,
                payload: [0u8; MEMO_PAYLOAD_SIZE],
            }
        );
        // The bytes survive the round trip.
        assert_eq!(ShieldedMemo::from_bytes(&bytes).to_bytes(), bytes);
    }

    #[test]
    fn unknown_kind_round_trips() {
        let payload = [7u8; MEMO_PAYLOAD_SIZE];
        let memo = ShieldedMemo::Other { kind: 42, payload };
        let bytes = memo.to_bytes();
        assert_eq!(
            u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            42
        );
        assert_eq!(ShieldedMemo::from_bytes(&bytes), memo);
    }

    #[test]
    fn kind_text_with_invalid_utf8_falls_back_to_other() {
        let mut bytes = [0u8; MEMO_SIZE];
        bytes[0..4].copy_from_slice(&KIND_TEXT.to_le_bytes());
        // 0xFF is never a valid UTF-8 byte.
        bytes[4] = 0xFF;
        match ShieldedMemo::from_bytes(&bytes) {
            ShieldedMemo::Other { kind, payload } => {
                assert_eq!(kind, KIND_TEXT);
                assert_eq!(payload[0], 0xFF);
            }
            other => panic!("expected Other for invalid UTF-8, got {other:?}"),
        }
    }

    #[test]
    fn kind_empty_with_nonzero_payload_falls_back_to_other() {
        let mut bytes = [0u8; MEMO_SIZE];
        // kind 0 but a non-zero payload — not something we wrote.
        bytes[4] = 0x01;
        match ShieldedMemo::from_bytes(&bytes) {
            ShieldedMemo::Other { kind, payload } => {
                assert_eq!(kind, KIND_EMPTY);
                assert_eq!(payload[0], 0x01);
            }
            other => panic!("expected Other for non-zero kind-0 payload, got {other:?}"),
        }
    }

    #[test]
    fn text_with_embedded_then_trailing_zeros_trims_only_trailing() {
        // Build a memo whose payload has an interior zero followed by data,
        // to confirm we trim trailing zeros only (rposition of last non-zero).
        let memo = ShieldedMemo::text("ab").expect("within limit");
        let bytes = memo.to_bytes();
        // payload = [b'a', b'b', 0, 0, ...]; decoding trims to "ab".
        assert_eq!(
            ShieldedMemo::from_bytes(&bytes),
            ShieldedMemo::Text("ab".to_string())
        );
    }
}
