//! Answers about a recovery phrase that are needed before any wallet exists,
//! such as when importing, migrating or repairing a phrase.
//!
//! Every phrase is read in any supported BIP-39 language and without a
//! passphrase, like the wallets the manager creates.

use key_wallet::Mnemonic;

/// Parses a BIP-39 mnemonic in any supported language.
///
/// Since rust-dashcore#981 `Mnemonic::from_phrase` is the auto-detecting
/// parse, with English diagnostics kept when nothing matches. This reduces its
/// error to the one message every caller reports.
pub fn parse_mnemonic_any_language(phrase: &str) -> Result<Mnemonic, &'static str> {
    Mnemonic::from_phrase(phrase).map_err(|_| "phrase does not match any supported BIP-39 wordlist")
}
