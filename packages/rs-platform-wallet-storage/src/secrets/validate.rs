//! Input validation for the `secrets` key space.
//!
//! `wallet_id` is fixed-width 32 bytes — enforced by the [`WalletId`]
//! type, not at runtime. `label` is reject-not-sanitize against a
//! strict allowlist before any backend maps it to a filename or a
//! keyring attribute (CWE-22 path traversal, CWE-20 improper input).

/// A 32-byte wallet identifier — the per-vault namespace key.
///
/// Public correlation material, **not** a secret: it is derived from
/// public wallet state, never from the seed's private bytes. Fixed width
/// is a type invariant, so no runtime length check is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WalletId(pub [u8; 32]);

impl WalletId {
    /// The raw 32 id bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Lowercase hex form, for filesystem / keyring namespacing.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl From<[u8; 32]> for WalletId {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// Maximum `label` length, matching the allowlist's `{1,64}` bound.
const MAX_LABEL_LEN: usize = 64;

/// Marker returned by [`validated_label`] on rejection. Backend
/// adapters lift this into their own typed error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InvalidLabel;

/// Validate a `label` against `^[A-Za-z0-9._-]{1,64}$` and return it
/// unchanged on success. Rejects (never sanitizes) so a traversal /
/// attribute-injection attempt is a hard error, not silently rewritten.
pub(crate) fn validated_label(label: &str) -> Result<&str, InvalidLabel> {
    let ok = (1..=MAX_LABEL_LEN).contains(&label.len())
        && label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    if ok {
        Ok(label)
    } else {
        Err(InvalidLabel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_allowlisted_labels() {
        for ok in [
            "bip39_mnemonic",
            "bip32-seed",
            "x.priv.0",
            "A",
            &"a".repeat(64),
        ] {
            assert!(validated_label(ok).is_ok(), "should accept {ok:?}");
        }
    }

    #[test]
    fn rejects_traversal_and_injection() {
        for bad in [
            "",
            &"a".repeat(65),
            "../etc/passwd",
            "a/b",
            "a\\b",
            "a b",
            "lab\0el",
            "lab\nel",
            "café",
            "a:b",
            "a;DROP TABLE",
        ] {
            assert!(validated_label(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn wallet_id_hex_is_fixed_width() {
        let id = WalletId::from([0xAB; 32]);
        assert_eq!(id.to_hex().len(), 64);
        assert_eq!(id.as_bytes().len(), 32);
    }
}
