//! Reading a secp256k1 private key typed by a person, as hex or WIF.

use dpp::dashcore::PrivateKey;
use dpp::secp256k1::SecretKey;
use std::fmt;
use zeroize::Zeroizing;

/// The length of a private key written as hex: 32 bytes, two characters each.
const PRIVATE_KEY_HEX_LEN: usize = 64;

/// The longest text read as WIF. A compressed WIF is 52 base58 characters;
/// the margin only keeps the refusal message about the length rather than the
/// encoding for text that is clearly not a key.
const MAX_WIF_LEN: usize = 60;

/// Why text could not be read as a private key.
///
/// The messages never include the text, which may hold a secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivateKeyTextError {
    /// The text is neither 64 hex characters nor a WIF private key.
    NotHexOrWif,
    /// The text is a WIF that marks its key uncompressed.
    UncompressedWif,
    /// The 32 bytes are not a secp256k1 secret key: zero, or not below the
    /// curve order.
    NotASecretKey,
    /// The text is longer than any WIF private key.
    TooLong,
}

impl fmt::Display for PrivateKeyTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NotHexOrWif => "the private key is neither 64 hex characters nor WIF",
            Self::UncompressedWif => {
                "uncompressed WIF keys are not supported; Platform keys use the compressed \
                 public key"
            }
            Self::NotASecretKey => {
                "the private key is not a secp256k1 secret key (zero, or not below the curve \
                 order)"
            }
            Self::TooLong => "the private key text is too long to be hex or WIF",
        };
        f.write_str(message)
    }
}

impl std::error::Error for PrivateKeyTextError {}

/// Reads a secp256k1 private key typed by a person and returns its 32 secret
/// bytes.
///
/// Leading and trailing whitespace is ignored. Exactly 64 characters are read
/// as hex; anything else is read as WIF, whatever network the WIF names (the
/// secret is the same on every network). A WIF must mark the key compressed:
/// Platform keys are formed from the compressed public key, so an uncompressed
/// WIF names a different key and is refused.
pub fn private_key_from_text(text: &str) -> Result<Zeroizing<[u8; 32]>, PrivateKeyTextError> {
    let text = text.trim();
    if text.len() == PRIVATE_KEY_HEX_LEN {
        let mut secret = Zeroizing::new([0u8; 32]);
        // Decoded straight into the zeroizing array, so a bad digit late in
        // the text leaves no partial secret in another buffer.
        hex::decode_to_slice(text, secret.as_mut_slice())
            .map_err(|_| PrivateKeyTextError::NotHexOrWif)?;
        SecretKey::from_secret_bytes(*secret).map_err(|_| PrivateKeyTextError::NotASecretKey)?;
        return Ok(secret);
    }
    if text.len() > MAX_WIF_LEN {
        return Err(PrivateKeyTextError::TooLong);
    }
    let private_key = PrivateKey::from_wif(text).map_err(|_| PrivateKeyTextError::NotHexOrWif)?;
    if !private_key.compressed {
        return Err(PrivateKeyTextError::UncompressedWif);
    }
    Ok(private_key.inner.to_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::dashcore::Network;

    fn wif(secret: &[u8; 32], compressed: bool) -> String {
        let mut private_key =
            PrivateKey::from_byte_array(secret, Network::Testnet).expect("a secret key");
        private_key.compressed = compressed;
        private_key.to_wif()
    }

    #[test]
    fn should_read_a_hex_key_and_the_same_key_in_wif() {
        let secret = [2u8; 32];
        assert_eq!(
            private_key_from_text(&hex::encode(secret)).as_deref(),
            Ok(&secret)
        );
        assert_eq!(
            private_key_from_text(&format!("  {}\n", hex::encode(secret))).as_deref(),
            Ok(&secret)
        );
        assert_eq!(
            private_key_from_text(&wif(&secret, true)).as_deref(),
            Ok(&secret)
        );
    }

    #[test]
    fn should_refuse_an_uncompressed_wif() {
        assert_eq!(
            private_key_from_text(&wif(&[2u8; 32], false)).as_deref(),
            Err(&PrivateKeyTextError::UncompressedWif)
        );
    }

    #[test]
    fn should_refuse_a_zero_key() {
        assert_eq!(
            private_key_from_text(&"0".repeat(64)).as_deref(),
            Err(&PrivateKeyTextError::NotASecretKey)
        );
    }

    #[test]
    fn should_refuse_text_that_is_not_hex_or_wif() {
        for text in [
            "",
            &"zz".repeat(32),
            &format!("{}zz", "ab".repeat(31)),
            // Short hex, read as WIF.
            &"ab".repeat(26),
            "cNotAWifKey",
        ] {
            assert_eq!(
                private_key_from_text(text).as_deref(),
                Err(&PrivateKeyTextError::NotHexOrWif),
                "{text}"
            );
        }
    }

    #[test]
    fn should_refuse_text_longer_than_a_wif() {
        assert_eq!(
            private_key_from_text(&"c".repeat(MAX_WIF_LEN + 1)).as_deref(),
            Err(&PrivateKeyTextError::TooLong)
        );
        assert_eq!(
            private_key_from_text(&"ab".repeat(33)).as_deref(),
            Err(&PrivateKeyTextError::TooLong)
        );
        assert_eq!(
            private_key_from_text(&"ab".repeat(31)).as_deref(),
            Err(&PrivateKeyTextError::TooLong)
        );
    }
}
