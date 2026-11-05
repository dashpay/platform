//! Answers about a recovery phrase that are needed before any wallet exists,
//! such as when importing, migrating or repairing a phrase.
//!
//! Every phrase is read in any supported BIP-39 language and without a
//! passphrase, like the wallets the manager creates.

use std::fmt::Display;

use dashcore::{Address, Network};
use key_wallet::account::derivation::AccountDerivation;
use key_wallet::account::StandardAccountType;
use key_wallet::bip32::{ExtendedPrivKey, ExtendedPubKey};
use key_wallet::managed_account::address_pool::AddressPoolType;
use key_wallet::wallet::root_extended_keys::RootExtendedPrivKey;
use key_wallet::{Account, AccountType, Mnemonic, Wallet};
use zeroize::{Zeroize, Zeroizing};

use super::platform_wallet::WalletId;
use crate::error::PlatformWalletError;

/// Returns the id the manager gives the wallet of `mnemonic` on `network`,
/// without creating a wallet. One phrase has a distinct id per network.
pub fn wallet_id_for_mnemonic(
    mnemonic: &str,
    network: Network,
) -> Result<WalletId, PlatformWalletError> {
    let seed = seed_for_mnemonic(mnemonic)?;
    let root = RootExtendedPrivKey::new_master(&*seed).map_err(key_derivation)?;
    Ok(Wallet::compute_wallet_id_from_root_extended_pub_key(
        &root.to_root_extended_pub_key(),
        Some(network),
    ))
}

/// Returns the first receive address of `mnemonic` on `network`: index 0 of
/// the external chain of BIP-44 account 0.
pub fn first_receive_address_for_mnemonic(
    mnemonic: &str,
    network: Network,
) -> Result<Address, PlatformWalletError> {
    let account_type = AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    };
    let path = account_type
        .derivation_path(network)
        .map_err(key_derivation)?;
    let mut master = master_key_for_mnemonic(mnemonic, network)?;
    let account_xpriv = master.derive_priv(&path);
    master.private_key.zeroize();
    let mut account_xpriv = account_xpriv.map_err(key_derivation)?;
    let account_xpub = ExtendedPubKey::from_priv(&account_xpriv);
    account_xpriv.private_key.zeroize();
    Account::from_xpub(None, account_type, account_xpub, network)
        .and_then(|account| account.derive_address_at(AddressPoolType::External, 0, None))
        .map_err(key_derivation)
}

pub(super) fn key_derivation(e: impl Display) -> PlatformWalletError {
    PlatformWalletError::KeyDerivation(e.to_string())
}

/// Parses a BIP-39 mnemonic in any supported language.
///
/// Since rust-dashcore#981 `Mnemonic::from_phrase` is the auto-detecting
/// parse, with English diagnostics kept when nothing matches. This reduces its
/// error to the one message every caller reports.
pub fn parse_mnemonic_any_language(phrase: &str) -> Result<Mnemonic, &'static str> {
    Mnemonic::from_phrase(phrase).map_err(|_| "phrase does not match any supported BIP-39 wordlist")
}

/// Returns the 64-byte BIP-39 seed of `mnemonic`, wiped when dropped.
pub fn seed_for_mnemonic(mnemonic: &str) -> Result<Zeroizing<[u8; 64]>, PlatformWalletError> {
    let mnemonic = parse_mnemonic_any_language(mnemonic)
        .map_err(|e| PlatformWalletError::InvalidParameter(e.to_string()))?;
    Ok(Zeroizing::new(mnemonic.to_seed("")))
}

/// Returns the BIP-32 master key of `mnemonic` on `network`, wiped when
/// dropped.
pub fn master_key_for_mnemonic(
    mnemonic: &str,
    network: Network,
) -> Result<ExtendedPrivKey, PlatformWalletError> {
    let seed = seed_for_mnemonic(mnemonic)?;
    ExtendedPrivKey::new_master(network, &*seed).map_err(key_derivation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        TEST_FRENCH_MNEMONIC as FRENCH, TEST_INVALID_MNEMONIC as GIBBERISH,
        TEST_PLATFORM_WALLET_MNEMONIC as ENGLISH,
    };
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;

    #[test]
    fn should_match_the_id_of_a_created_wallet() {
        for phrase in [ENGLISH, FRENCH] {
            assert_eq!(
                seed_for_mnemonic(phrase).unwrap()[..],
                Mnemonic::from_phrase(phrase).unwrap().to_seed("")[..]
            );
            for network in [Network::Mainnet, Network::Testnet, Network::Devnet] {
                let wallet = Wallet::from_mnemonic(
                    Mnemonic::from_phrase(phrase).unwrap(),
                    network,
                    WalletAccountCreationOptions::Default,
                )
                .unwrap();
                assert_eq!(
                    wallet_id_for_mnemonic(phrase, network).unwrap(),
                    wallet.wallet_id,
                    "{network:?}"
                );
            }
        }
        assert_ne!(
            wallet_id_for_mnemonic(ENGLISH, Network::Mainnet).unwrap(),
            wallet_id_for_mnemonic(ENGLISH, Network::Testnet).unwrap()
        );
        assert!(matches!(
            wallet_id_for_mnemonic(GIBBERISH, Network::Mainnet),
            Err(PlatformWalletError::InvalidParameter(_))
        ));
    }

    #[test]
    fn should_derive_the_pinned_first_receive_addresses() {
        for (phrase, network, expected) in [
            (
                ENGLISH,
                Network::Mainnet,
                "XoJA8qE3N2Y3jMLEtZ3vcN42qseZ8LvFf5",
            ),
            (
                FRENCH,
                Network::Mainnet,
                "XvdJKc1Qf9QWHfHTp8rpiNZ3ZWLGcFnoqV",
            ),
            (
                FRENCH,
                Network::Testnet,
                "yiz7bBUGQUJ4cTfUWqwwPq7g8XTc7ynEEw",
            ),
        ] {
            assert_eq!(
                first_receive_address_for_mnemonic(phrase, network)
                    .unwrap()
                    .to_string(),
                expected
            );
        }
        assert!(matches!(
            first_receive_address_for_mnemonic(GIBBERISH, Network::Mainnet),
            Err(PlatformWalletError::InvalidParameter(_))
        ));
    }
}
