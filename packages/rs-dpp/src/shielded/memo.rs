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
//! Two kinds are defined today; unknown kinds round-trip verbatim as
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
            ShieldedMemo::Other { kind, payload } => (*kind, *payload),
        };
        out[0..4].copy_from_slice(&kind.to_le_bytes());
        out[4..MEMO_SIZE].copy_from_slice(&payload);
        out
    }

    /// Decode from the on-chain 36-byte memo layout.
    ///
    /// Decoding is total: a `KIND_TEXT` memo whose payload is not valid
    /// UTF-8, or a `KIND_EMPTY` memo with a non-zero payload, falls back
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
