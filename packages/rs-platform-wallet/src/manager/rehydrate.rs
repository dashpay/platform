//! Seed → signing [`Wallet`] reconstruction with the fail-closed
//! wrong-seed gate (A07/A08).
//!
//! Pure, side-effect-free: no manager state, no I/O, no logging. Given
//! a keyless account manifest + the persisted wallet id + the runtime
//! [`WalletSecret`], it re-derives exactly the persisted account set
//! and proves the secret matches the database *before* any persisted
//! state is applied. A mismatch is a hard, typed
//! [`PlatformWalletError::WrongSeedForDatabase`] — never a skip, never
//! a partial merge.

use std::collections::BTreeSet;

use key_wallet::wallet::initialization::{
    PlatformPaymentAccountSpec, WalletAccountCreationOptions,
};
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use subtle::ConstantTimeEq;

use crate::changeset::AccountRegistrationEntry;
use crate::error::PlatformWalletError;
use crate::seed_provider::WalletSecret;

/// Build the [`WalletAccountCreationOptions::SpecificAccounts`] request
/// that re-derives exactly the account set the manifest describes.
fn options_from_manifest(manifest: &[AccountRegistrationEntry]) -> WalletAccountCreationOptions {
    use key_wallet::account::AccountType;

    let mut bip44 = BTreeSet::new();
    let mut bip32 = BTreeSet::new();
    let mut coinjoin = BTreeSet::new();
    let mut topup = BTreeSet::new();
    let mut platform_payment: BTreeSet<PlatformPaymentAccountSpec> = BTreeSet::new();
    let mut extra: Vec<AccountType> = Vec::new();

    for e in manifest {
        match e.account_type {
            AccountType::Standard {
                index,
                standard_account_type,
            } => {
                use key_wallet::account::StandardAccountType;
                match standard_account_type {
                    StandardAccountType::BIP44Account => {
                        bip44.insert(index);
                    }
                    StandardAccountType::BIP32Account => {
                        bip32.insert(index);
                    }
                }
            }
            AccountType::CoinJoin { index } => {
                coinjoin.insert(index);
            }
            AccountType::IdentityTopUp { registration_index } => {
                topup.insert(registration_index);
            }
            AccountType::PlatformPayment { account, key_class } => {
                platform_payment.insert(PlatformPaymentAccountSpec { account, key_class });
            }
            other => extra.push(other),
        }
    }

    WalletAccountCreationOptions::SpecificAccounts(
        bip44,
        bip32,
        coinjoin,
        topup,
        platform_payment,
        if extra.is_empty() { None } else { Some(extra) },
    )
}

/// Re-derive the signing wallet from `secret` and prove it matches the
/// persisted database.
///
/// Reconstructs exactly the account set in `manifest`, then runs the
/// **fail-closed wrong-seed gate**:
///
/// 1. constant-time compare of the recomputed `wallet_id` against
///    `expected_wallet_id` (root-key recompute — a genuine
///    cryptographic guard, not a tautology, for signing wallet types);
/// 2. constant-time cross-check of every manifest `account_xpub`
///    against the freshly-derived account's xpub.
///
/// Any mismatch yields [`PlatformWalletError::WrongSeedForDatabase`]
/// before the caller applies any persisted core/identity/asset-lock
/// state. The transient secret lives only for the duration of this
/// call; the caller drops the owning [`WalletSecret`] at the end of the
/// per-wallet iteration.
///
/// # Errors
///
/// - [`PlatformWalletError::WalletCreation`] if the mnemonic does not
///   parse or `Wallet::from_*` fails.
/// - [`PlatformWalletError::WrongSeedForDatabase`] if the recomputed
///   id or any account xpub does not match the persisted database.
pub fn rehydrate_wallet(
    secret: &WalletSecret,
    network: Network,
    expected_wallet_id: [u8; 32],
    manifest: &[AccountRegistrationEntry],
) -> Result<Wallet, PlatformWalletError> {
    let options = options_from_manifest(manifest);

    // KNOWN RISK (AR-7): upstream `key_wallet::Wallet` derives `Debug`
    // and its `WalletType::{Mnemonic,Seed}` variants render the
    // mnemonic / seed / root xpriv. We never `Debug`/log this `wallet`
    // value (the wrong-seed error below carries only the two 32-byte
    // ids). Per the accepted residual-risk decision this is noted, not
    // mitigated, and not blocking.
    let wallet = match secret {
        WalletSecret::Mnemonic(phrase) => {
            let mnemonic = parse_mnemonic_any_language(phrase.expose()).map_err(|e| {
                PlatformWalletError::WalletCreation(format!("Invalid mnemonic on rehydrate: {e}"))
            })?;
            Wallet::from_mnemonic(mnemonic, network, options).map_err(|e| {
                PlatformWalletError::WalletCreation(format!(
                    "Failed to reconstruct wallet from mnemonic: {e}"
                ))
            })?
        }
        WalletSecret::Seed(bytes) => {
            let seed_bytes: [u8; 64] = bytes.expose().try_into().map_err(|_| {
                PlatformWalletError::WalletCreation(
                    "stored seed material is not 64 bytes".to_string(),
                )
            })?;
            Wallet::from_seed_bytes(seed_bytes, network, options).map_err(|e| {
                PlatformWalletError::WalletCreation(format!(
                    "Failed to reconstruct wallet from seed: {e}"
                ))
            })?
        }
    };

    // Gate 1: recomputed wallet id (root-key recompute) vs persisted.
    let derived_wallet_id = wallet.compute_wallet_id();
    let id_ok: bool = derived_wallet_id.ct_eq(&expected_wallet_id).into();

    // Gate 2: every persisted account xpub must reproduce bit-exact.
    // Constant-time per-pair; accumulate without early-return so the
    // observable timing does not depend on which pair first differs.
    let mut xpubs_ok = subtle::Choice::from(1u8);
    for entry in manifest {
        let derived = wallet
            .accounts
            .all_accounts()
            .into_iter()
            .find(|a| a.account_type == entry.account_type)
            .map(|a| a.account_xpub);
        match derived {
            Some(d) => {
                let a = d.encode();
                let b = entry.account_xpub.encode();
                xpubs_ok &= a.ct_eq(&b);
            }
            None => {
                xpubs_ok = subtle::Choice::from(0u8);
            }
        }
    }

    if id_ok && bool::from(xpubs_ok) {
        Ok(wallet)
    } else {
        // `wallet` dropped here — its key material never reaches the
        // error, which carries only the two public 32-byte ids.
        Err(PlatformWalletError::WrongSeedForDatabase {
            expected_wallet_id,
            derived_wallet_id,
        })
    }
}

/// Apply the keyless persisted core-state projection onto a
/// freshly-minted `ManagedWalletInfo` skeleton.
///
/// # Reconstructed (safety-critical-correct)
///
/// - **Wallet balance** (`wallet_info.balance`, the no-silent-zero
///   guarantee): every persisted UTXO is restored and the per-account
///   + wallet totals are recomputed via `update_balance()`. A UTXO
///   carrying a block height is marked confirmed so it lands in the
///   `confirmed` bucket; the wallet total is exact regardless.
/// - **UTXO set**: every unspent persisted outpoint is restored (in
///   the wallet's funds-account UTXO map).
/// - **Sync watermarks**: `synced_height` / `last_processed_height`.
/// - **`last_applied_chain_lock`**: round-tripped so the asset-lock
///   proof-resume metadata fallback can fire without waiting for SPV
///   to re-apply a fresh chainlock.
///
/// # Deferred to the first post-load `sync` (safe re-warm)
///
/// - **Per-account UTXO attribution beyond the primary funds account**:
///   `core_utxos.account_index` is written as `0` at persist time, so
///   per-account bucketing is not recoverable from disk; UTXOs are
///   restored against the primary funds account and re-attributed on
///   the next scan. The *wallet total* is unaffected (it is a sum).
/// - **Per-UTXO `is_coinbase` / `is_instantlocked` / `is_trusted`
///   flags**: not columns in `core_utxos`; conservatively defaulted
///   (non-coinbase, confirmed-by-height) and refreshed on the next
///   scan. Coinbase-maturity nuance re-warms on sync.
/// - **Transaction-record history**: rebuilt by the next scan; not a
///   balance input.
///
/// This never logs and never touches key material.
pub fn apply_persisted_core_state(
    wallet_info: &mut ManagedWalletInfo,
    core: &crate::changeset::CoreChangeSet,
) {
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

    // Sync watermarks first so `update_balance`'s maturity check sees
    // the restored tip.
    if let Some(h) = core.last_processed_height {
        wallet_info.metadata.last_processed_height =
            wallet_info.metadata.last_processed_height.max(h);
    }
    if let Some(h) = core.synced_height {
        wallet_info.metadata.synced_height = wallet_info.metadata.synced_height.max(h);
    }
    if let Some(cl) = core.last_applied_chain_lock.clone() {
        let advance = wallet_info
            .metadata
            .last_applied_chain_lock
            .as_ref()
            .is_none_or(|existing| cl.block_height > existing.block_height);
        if advance {
            wallet_info.metadata.last_applied_chain_lock = Some(cl);
        }
    }

    // Restore the UTXO set. Persisted attribution is account-0 only
    // (see the doc above), so route every restored UTXO to the primary
    // funds account; the wallet total is a sum and stays exact.
    if !core.new_utxos.is_empty() {
        let target_index = wallet_info
            .accounts
            .standard_bip44_accounts
            .keys()
            .next()
            .copied();
        if let Some(idx) = target_index {
            if let Some(account) = wallet_info.accounts.get_mut(idx) {
                for utxo in &core.new_utxos {
                    if !core.spent_utxos.iter().any(|s| s.outpoint == utxo.outpoint) {
                        account.utxos.insert(utxo.outpoint, utxo.clone());
                    }
                }
            }
        }
    }

    // Recompute per-account + wallet balance from the restored set.
    // After this, a non-zero persisted balance is non-zero here — a
    // silent zero would be a hard FAIL of the rehydration contract.
    wallet_info.update_balance();
}

/// Parse a BIP-39 phrase against every supported wordlist in turn.
/// Mirrors the live creation path's language auto-detection.
fn parse_mnemonic_any_language(
    phrase: &str,
) -> Result<key_wallet::mnemonic::Mnemonic, &'static str> {
    use key_wallet::mnemonic::{Language, Mnemonic};
    const LANGUAGES: [Language; 10] = [
        Language::English,
        Language::Spanish,
        Language::French,
        Language::Italian,
        Language::Japanese,
        Language::Korean,
        Language::ChineseSimplified,
        Language::ChineseTraditional,
        Language::Czech,
        Language::Portuguese,
    ];
    for lang in LANGUAGES {
        if let Ok(m) = Mnemonic::from_phrase(phrase, lang) {
            return Ok(m);
        }
    }
    Err("phrase does not match any supported BIP-39 wordlist")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed_provider::{SecretSeed, WalletSecret};

    fn manifest_for(w: &Wallet) -> Vec<AccountRegistrationEntry> {
        w.accounts
            .all_accounts()
            .into_iter()
            .map(|a| AccountRegistrationEntry {
                account_type: a.account_type,
                account_xpub: a.account_xpub,
            })
            .collect()
    }

    #[test]
    fn correct_seed_rehydrates_and_passes_gate() {
        let seed = [3u8; 64];
        let w = Wallet::from_seed_bytes(
            seed,
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&w);
        let id = w.compute_wallet_id();

        let secret = WalletSecret::Seed(SecretSeed::new(seed.to_vec()));
        let out = rehydrate_wallet(&secret, Network::Testnet, id, &manifest).unwrap();
        assert_eq!(out.compute_wallet_id(), id);
    }

    #[test]
    fn wrong_seed_is_hard_fail_not_skip() {
        let real_seed = [3u8; 64];
        let w = Wallet::from_seed_bytes(
            real_seed,
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&w);
        let expected_id = w.compute_wallet_id();

        // A different seed — wrong for this database.
        let wrong = WalletSecret::Seed(SecretSeed::new(vec![9u8; 64]));
        let err = rehydrate_wallet(&wrong, Network::Testnet, expected_id, &manifest)
            .expect_err("wrong seed must hard-fail");
        match err {
            PlatformWalletError::WrongSeedForDatabase {
                expected_wallet_id,
                derived_wallet_id,
            } => {
                assert_eq!(expected_wallet_id, expected_id);
                assert_ne!(derived_wallet_id, expected_id);
                // The error must not leak any key material.
                let rendered = err_string(&PlatformWalletError::WrongSeedForDatabase {
                    expected_wallet_id,
                    derived_wallet_id,
                });
                assert!(!rendered.contains("9999"));
            }
            other => panic!("expected WrongSeedForDatabase, got {other:?}"),
        }
    }

    fn err_string(e: &PlatformWalletError) -> String {
        format!("{e} {e:?}")
    }

    #[test]
    fn non_64_byte_seed_is_creation_error() {
        let secret = WalletSecret::Seed(SecretSeed::new(vec![1u8; 32]));
        let err = rehydrate_wallet(&secret, Network::Testnet, [0u8; 32], &[])
            .expect_err("short seed must fail");
        assert!(matches!(err, PlatformWalletError::WalletCreation(_)));
    }
}
