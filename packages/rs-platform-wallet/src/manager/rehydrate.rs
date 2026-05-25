//! Watch-only wallet reconstruction + persisted core-state application.
//!
//! Load is **seedless** (see [`load_from_persistor`]). For each
//! persisted wallet we build a watch-only [`Wallet`] from its keyless
//! `AccountRegistrationEntry` manifest, then apply the keyless
//! core-state projection on top. No seed, no signing-key derivation.
//!
//! The wrong-seed gate has moved to the **first sign** path
//! (`rs-platform-wallet-ffi::sign_with_mnemonic_resolver` and its
//! resolver-fed siblings): each sign entrypoint constant-time-compares
//! the recomputed `wallet_id` against the loaded `wallet_id` and fails
//! closed on mismatch.
//!
//! [`load_from_persistor`]: super::PlatformWalletManager::load_from_persistor

use key_wallet::account::account_collection::AccountCollection;
use key_wallet::account::Account;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Network;

use crate::changeset::AccountRegistrationEntry;
use crate::error::PlatformWalletError;
use crate::manager::load_outcome::CorruptKind;

/// Per-row failure surfacing during watch-only rehydration of a single
/// persisted wallet. Maps 1:1 to [`CorruptKind`] for the
/// [`SkipReason`](super::load_outcome::SkipReason) the load loop
/// records.
#[derive(Debug)]
pub(super) enum RehydrateRowError {
    /// Manifest was empty — no account to rebuild the wallet around.
    MissingManifest,
    /// Building a watch-only [`Account`] from a manifest entry failed
    /// (xpub structurally malformed for its [`AccountType`]).
    ///
    /// [`AccountType`]: key_wallet::account::AccountType
    MalformedXpub,
    /// `AccountCollection::insert` rejected an account (typically a
    /// duplicate `account_type` within the manifest).
    DecodeError(String),
}

impl From<RehydrateRowError> for CorruptKind {
    fn from(e: RehydrateRowError) -> Self {
        match e {
            RehydrateRowError::MissingManifest => CorruptKind::MissingManifest,
            RehydrateRowError::MalformedXpub => CorruptKind::MalformedXpub,
            RehydrateRowError::DecodeError(s) => CorruptKind::DecodeError(s),
        }
    }
}

/// Build a watch-only [`Wallet`] from the keyless account manifest.
///
/// Each `AccountRegistrationEntry` becomes an [`Account::from_xpub`]
/// (watch-only) keyed to `expected_wallet_id`; the assembled
/// [`AccountCollection`] is handed to [`Wallet::new_watch_only`] under
/// the same id. No key material crosses this function.
///
/// Returns [`RehydrateRowError`] when the row is structurally unusable
/// (caller maps it onto a per-row [`SkipReason`]).
pub(super) fn build_watch_only_wallet(
    network: Network,
    expected_wallet_id: [u8; 32],
    manifest: &[AccountRegistrationEntry],
) -> Result<Wallet, RehydrateRowError> {
    if manifest.is_empty() {
        return Err(RehydrateRowError::MissingManifest);
    }
    let mut accounts = AccountCollection::new();
    for entry in manifest {
        let account = Account::from_xpub(
            Some(expected_wallet_id),
            entry.account_type,
            entry.account_xpub,
            network,
        )
        .map_err(|_| RehydrateRowError::MalformedXpub)?;
        accounts
            .insert(account)
            .map_err(|e| RehydrateRowError::DecodeError(e.to_string()))?;
    }
    Ok(Wallet::new_watch_only(
        network,
        expected_wallet_id,
        accounts,
    ))
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
/// - **UTXO set**: every unspent persisted outpoint is restored into a
///   funds-bearing account of the wallet (whatever topology it has —
///   BIP44, BIP32, CoinJoin, DashPay).
/// - **Sync watermarks**: `synced_height` / `last_processed_height`.
///
/// # Deferred to the first post-load `sync` (safe re-warm)
///
/// - **Per-account UTXO attribution**: `core_utxos.account_index` is
///   written as `0` at persist time, so per-account bucketing is not
///   recoverable from disk; UTXOs are restored against the wallet's
///   first funds-bearing account and re-attributed on the next scan.
///   The *wallet total* is unaffected (it is a sum across all funds
///   accounts).
/// - **`last_applied_chain_lock`**: not a persisted column (V001) and
///   never written by the core-state writer; always `None` from disk.
///   SPV re-applies a fresh chainlock on the first post-restart sync.
/// - **Per-UTXO `is_coinbase` / `is_instantlocked` / `is_trusted`
///   flags**: not columns in `core_utxos`; conservatively defaulted
///   (non-coinbase, confirmed-by-height) and refreshed on the next
///   scan. Coinbase-maturity nuance re-warms on sync.
/// - **Transaction-record history**: rebuilt by the next scan; not a
///   balance input.
///
/// # Errors
///
/// [`PlatformWalletError::RehydrationTopologyUnsupported`] if there are
/// persisted UTXOs to restore but the reconstructed account collection
/// has **no** funds-bearing account to hold them. Fail-closed rather
/// than reconstructing a silent zero balance (the no-silent-zero
/// mandate). An empty UTXO set is always `Ok`.
///
/// This never logs and never touches key material.
pub fn apply_persisted_core_state(
    wallet_info: &mut ManagedWalletInfo,
    core: &crate::changeset::CoreChangeSet,
) -> Result<(), PlatformWalletError> {
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

    // Restore the UTXO set. Persisted attribution is lost at write time
    // (account_index is always 0), so route every restored UTXO to the
    // wallet's first funds-bearing account *of any topology* (BIP44,
    // BIP32, CoinJoin, DashPay) — the wallet total is a sum across all
    // funds accounts and stays exact. A wallet with persisted UTXOs but
    // no funds account at all cannot be represented: fail closed rather
    // than silently reconstruct a zero balance.
    let unspent: Vec<&key_wallet::Utxo> = core
        .new_utxos
        .iter()
        .filter(|u| !core.spent_utxos.iter().any(|s| s.outpoint == u.outpoint))
        .collect();
    if !unspent.is_empty() {
        match wallet_info
            .accounts
            .all_funding_accounts_mut()
            .into_iter()
            .next()
        {
            Some(account) => {
                for utxo in unspent {
                    account.utxos.insert(utxo.outpoint, utxo.clone());
                }
            }
            None => {
                return Err(PlatformWalletError::RehydrationTopologyUnsupported {
                    wallet_id: wallet_info.wallet_id,
                    utxo_count: core.new_utxos.len(),
                });
            }
        }
    }

    // Recompute per-account + wallet balance from the restored set.
    // After this, a non-zero persisted balance is non-zero here — a
    // silent zero would be a hard FAIL of the rehydration contract.
    wallet_info.update_balance();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;

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
    fn watch_only_rebuild_round_trips_manifest_and_id() {
        let seed = [3u8; 64];
        let w = Wallet::from_seed_bytes(
            seed,
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let id = w.compute_wallet_id();
        let manifest = manifest_for(&w);

        let restored = build_watch_only_wallet(Network::Testnet, id, &manifest).unwrap();
        assert_eq!(restored.wallet_id, id);
        assert_eq!(restored.compute_wallet_id(), id);
        // Every manifest account survives the round trip (count, types).
        let restored_types: Vec<_> = restored
            .accounts
            .all_accounts()
            .into_iter()
            .map(|a| a.account_type)
            .collect();
        let manifest_types: Vec<_> = manifest.iter().map(|e| e.account_type).collect();
        assert_eq!(restored_types.len(), manifest_types.len());
        for t in &manifest_types {
            assert!(restored_types.contains(t));
        }
    }

    #[test]
    fn empty_manifest_is_missing_manifest() {
        let err = build_watch_only_wallet(Network::Testnet, [0u8; 32], &[])
            .expect_err("empty manifest must be MissingManifest");
        assert!(matches!(err, RehydrateRowError::MissingManifest));
    }
}
