//! Watch-only wallet reconstruction + persisted core-state application.
//!
//! Load is **seedless** (see [`load_from_persistor`]). For each
//! persisted wallet we build a watch-only [`Wallet`] from its keyless
//! `AccountRegistrationEntry` manifest, then apply the keyless
//! core-state projection on top. No seed, no signing-key derivation.
//!
//! Because load never touches the seed, it performs no wrong-seed check.
//! A sign-time wrong-seed gate is deferred to separate FFI work and is
//! not part of this path.
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
    manifest: &[AccountRegistrationEntry],
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
                for utxo in &unspent {
                    account.utxos.insert(utxo.outpoint, (*utxo).clone());
                }
                // Eager derivation covers only `0..=gap_limit`; extend each
                // chain to cover restored UTXOs at deeper indices.
                extend_pools_for_restored_utxos(account, manifest, &unspent);
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

/// Upper bound on forward derivation while resolving a restored UTXO
/// address to its derivation index. Addresses that don't resolve within
/// this many indices (e.g. they belong to a different funds account whose
/// UTXOs were routed here, or are corrupt) are left for the next full
/// rescan to re-warm — generous enough to cover any realistic per-account
/// derivation depth. The common (single funds account) path terminates at
/// the true high-water mark well before this and never reaches the cap.
const MAX_REHYDRATION_DERIVATION_INDEX: u32 = 10_000;

/// Extend `account`'s address pools so every resolved UTXO address is
/// derived at its exact `(chain, index)` slot, then refill the gap window
/// beyond — following the sync path's `mark_used` → `maintain_gap_limit`
/// sequence. Each chain is scanned independently, stopping once no
/// unresolved address matches within a `gap_limit`-sized window past the
/// deepest resolved index; [`MAX_REHYDRATION_DERIVATION_INDEX`] is the
/// hard ceiling. Addresses not derivable from this account's xpub (foreign
/// keys, multi-account mismatch) are counted and logged via
/// `tracing::warn!`; they re-warm on the next full sync.
///
/// Tested with Standard BIP44 topology (External + Internal pools) and
/// CoinJoin topology (single External pool). The per-chain probe loop has
/// no topology-specific branches, so Absent and AbsentHardened pool types
/// follow the same code path with a different relative derivation path.
///
/// Never touches key material — the xpub is the keyless account public key.
fn extend_pools_for_restored_utxos(
    account: &mut key_wallet::managed_account::ManagedCoreFundsAccount,
    manifest: &[AccountRegistrationEntry],
    restored: &[&key_wallet::Utxo],
) {
    use key_wallet::managed_account::address_pool::{AddressPool, KeySource};
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use std::collections::{BTreeSet, HashSet};

    // The funds account carries no key material; recover its watch-only
    // xpub from the keyless manifest by account type.
    let account_type = account.managed_account_type().to_account_type();
    let Some(account_xpub) = manifest
        .iter()
        .find(|e| e.account_type == account_type)
        .map(|e| e.account_xpub)
    else {
        return;
    };
    let key_source = KeySource::Public(account_xpub);

    // Restored addresses not already covered by the eager derivation.
    let mut unresolved: HashSet<key_wallet::Address> = {
        let pools = account.managed_account_type().address_pools();
        restored
            .iter()
            .map(|u| u.address.clone())
            .filter(|addr| !pools.iter().any(|p| p.contains_address(addr)))
            .collect()
    };
    if unresolved.is_empty() {
        return;
    }

    // Probe pools mirror each real pool's chain so the index search derives
    // into throwaway state (real pools keep their own exact depth).
    let mut probes: Vec<(AddressPool, BTreeSet<u32>)> = account
        .managed_account_type()
        .address_pools()
        .iter()
        .map(|p| {
            (
                AddressPool::new_without_generation(
                    p.base_path.clone(),
                    p.pool_type,
                    p.gap_limit,
                    p.network,
                ),
                BTreeSet::new(),
            )
        })
        .collect();

    // Per-chain scan: each chain advances independently. We stop a chain
    // once no unresolved address resolves within gap_limit indices past the
    // deepest match on that chain (preventing a full 10k scan when the UTXO
    // set contains foreign addresses). MAX_REHYDRATION_DERIVATION_INDEX is
    // the hard ceiling regardless.
    for (probe, matched) in probes.iter_mut() {
        if unresolved.is_empty() {
            break;
        }
        let chain_gap = probe.gap_limit;
        let mut deepest_resolved: Option<u32> = None;
        let mut index: u32 = 0;

        loop {
            // Horizon: gap_limit past the deepest match, or the initial
            // gap_limit window when nothing has resolved yet.
            let horizon = deepest_resolved
                .map(|d| d.saturating_add(chain_gap))
                .unwrap_or(chain_gap);

            if index > horizon || index > MAX_REHYDRATION_DERIVATION_INDEX {
                break;
            }

            if let Some(addr) = ensure_derived(probe, &key_source, index) {
                if unresolved.remove(&addr) {
                    matched.insert(index);
                    deepest_resolved = Some(index);
                }
            }

            if unresolved.is_empty() {
                break;
            }

            index = index.saturating_add(1);
        }
    }

    // Addresses still unresolved are not derivable from this account's xpub
    // (foreign key, routed from a different funds account, or corrupt).
    // They re-warm on the next full sync; total balance is exact regardless.
    if !unresolved.is_empty() {
        tracing::warn!(
            unresolved_count = unresolved.len(),
            "rehydration: {} UTXO address(es) unresolved for this account \
             xpub — will re-warm on next sync; balance total is exact",
            unresolved.len(),
        );
    }

    // Apply each chain's resolved depth to its real pool: derive up to the
    // deepest resolved index, mark the resolved slots used, then maintain
    // the gap window beyond the highest used index.
    // Chains with no resolved UTXOs are skipped — their eager gap window
    // (from initialization) already covers the correct depth, and calling
    // maintain_gap_limit without any used indices would be a no-op anyway.
    let mut pools = account.managed_account_type_mut().address_pools_mut();
    for (i, (_, matched)) in probes.iter().enumerate() {
        let Some(&deepest) = matched.iter().next_back() else {
            continue;
        };
        let pool = &mut *pools[i];
        ensure_derived(pool, &key_source, deepest);
        for &idx in matched {
            pool.mark_index_used(idx);
        }
        let _ = pool.maintain_gap_limit(&key_source);
    }
}

/// Ensure `pool` has derived through `index` (generating only the missing
/// tail), and return that index's address. `None` only on a derivation
/// error.
fn ensure_derived(
    pool: &mut key_wallet::managed_account::address_pool::AddressPool,
    key_source: &key_wallet::managed_account::address_pool::KeySource,
    index: u32,
) -> Option<key_wallet::Address> {
    let needs_more = match pool.highest_generated {
        Some(highest) => highest < index,
        None => true,
    };
    if needs_more {
        let start = pool.highest_generated.map(|h| h + 1).unwrap_or(0);
        pool.generate_addresses(index - start + 1, key_source, true)
            .ok()?;
    }
    pool.address_at_index(index)
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

    /// Regression: after restart-in-place the watch-only pools eagerly
    /// cover only `0..=gap_limit`, but persisted UTXOs can sit at deeper
    /// derivation indices. Rehydration must extend each chain's pool to its
    /// deepest restored index so the per-address view reconciles with the
    /// wallet total instead of undercounting.
    ///
    /// Index layout (gap_limit = 30):
    /// - external idx 3:  within eager window (not in `unresolved`), balance included
    /// - external idx 30: first index past eager window; anchors the initial scan
    ///   window and extends it to idx 60
    /// - external idx 50: within extended window (50 < 60), resolved
    /// - internal idx 30: within initial scan window, resolved
    ///
    /// QA-003: Standard BIP44 topology (External + Internal pools) is exercised.
    /// QA-005: asserts that maintain_gap_limit fills beyond the deepest resolved.
    #[test]
    fn rehydration_extends_pools_to_cover_deep_index_utxos() {
        use dashcore::blockdata::transaction::txout::TxOut;
        use dashcore::{OutPoint, Txid};
        use key_wallet::bip32::DerivationPath;
        use key_wallet::gap_limit::DEFAULT_EXTERNAL_GAP_LIMIT;
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::{Address, Utxo};
        use std::collections::HashSet;

        let seed = [7u8; 64];
        let wallet = Wallet::from_seed_bytes(
            seed,
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&wallet);

        // Mint the watch-only skeleton (pools cover only the eager gap
        // window) and resolve the first funds account's keyless xpub.
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 1);
        let funds_type = wallet_info
            .accounts
            .all_funding_accounts()
            .first()
            .unwrap()
            .managed_account_type()
            .to_account_type();
        let xpub = manifest
            .iter()
            .find(|e| e.account_type == funds_type)
            .map(|e| e.account_xpub)
            .expect("funds account xpub");

        // Derive addresses on each chain from the same account xpub the
        // pools use; `base_path` is record-keeping only and does not affect
        // the derived address, so DerivationPath::master() is fine here.
        let derive = |pool_type, index: u32| -> Address {
            let mut p = AddressPool::new_without_generation(
                DerivationPath::master(),
                pool_type,
                DEFAULT_EXTERNAL_GAP_LIMIT,
                Network::Testnet,
            );
            p.generate_addresses(index + 1, &KeySource::Public(xpub), true)
                .unwrap();
            p.address_at_index(index).unwrap()
        };

        // idx 3: within eager window (0..=29) — covered by init, NOT in
        // unresolved. Contributes to balance but needs no pool extension.
        let shallow_recv = derive(AddressPoolType::External, 3);
        // idx 30: first past eager window; falls in initial scan window
        // (horizon = gap_limit = 30 on a chain with no prior matches).
        // Anchors the external probe and extends horizon to 60.
        let mid_recv = derive(AddressPoolType::External, 30);
        // idx 50: within the extended window (50 < 30+30=60), resolved.
        let deep_recv = derive(AddressPoolType::External, 50);
        // idx 30: within the internal chain's initial scan window (<=30).
        let deep_change = derive(AddressPoolType::Internal, 30);

        let utxo = |addr: Address, value: u64, n: u8| Utxo {
            outpoint: OutPoint {
                txid: Txid::from([n; 32]),
                vout: 0,
            },
            txout: TxOut {
                value,
                script_pubkey: addr.script_pubkey(),
            },
            address: addr,
            height: 1,
            is_coinbase: false,
            is_confirmed: true,
            is_instantlocked: false,
            is_locked: false,
            is_trusted: false,
        };
        let new_utxos = vec![
            utxo(shallow_recv, 1_000, 1),
            utxo(mid_recv.clone(), 10_000, 2),
            utxo(deep_recv.clone(), 20_000, 3),
            utxo(deep_change.clone(), 300_000, 4),
        ];
        let expected_total: u64 = new_utxos.iter().map(|u| u.value()).sum();
        let core = crate::changeset::CoreChangeSet {
            new_utxos,
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };

        apply_persisted_core_state(&mut wallet_info, &manifest, &core).unwrap();

        // The wallet total is exact regardless (a sum over the UTXO set).
        assert_eq!(wallet_info.balance.total(), expected_total);

        // The per-address view joins pool addresses to UTXOs; every
        // resolved UTXO address must now be derived into a pool.
        let funds = wallet_info
            .accounts
            .all_funding_accounts()
            .into_iter()
            .next()
            .unwrap();
        let pool_addresses: HashSet<Address> = funds
            .managed_account_type()
            .address_pools()
            .iter()
            .flat_map(|p| p.addresses.values().map(|i| i.address.clone()))
            .collect();
        let visible: u64 = funds
            .utxos
            .values()
            .filter(|u| pool_addresses.contains(&u.address))
            .map(|u| u.value())
            .sum();
        assert_eq!(
            visible, expected_total,
            "all UTXO addresses (including deep-index) must be derived into their pools"
        );

        // Each deep address resolves to its exact derivation slot.
        let pools = funds.managed_account_type().address_pools();
        let external = pools.iter().find(|p| p.is_external()).unwrap();
        let internal = pools.iter().find(|p| p.is_internal()).unwrap();
        assert_eq!(external.address_at_index(30).as_ref(), Some(&mid_recv));
        assert_eq!(external.address_at_index(50).as_ref(), Some(&deep_recv));
        assert_eq!(internal.address_at_index(30).as_ref(), Some(&deep_change));

        // QA-005: maintain_gap_limit must refill BEYOND the deepest restored
        // index so the gap window is actually exercised, not just the restore.
        // Deepest external resolved = idx 50; gap window must reach >= 50+30=80.
        let expected_min_gen = 50 + DEFAULT_EXTERNAL_GAP_LIMIT;
        assert!(
            external.highest_generated >= Some(expected_min_gen),
            "maintain_gap_limit must extend external pool to >= {} (got {:?})",
            expected_min_gen,
            external.highest_generated,
        );
    }

    /// QA-004: a UTXO whose address is not derivable from this account's
    /// xpub (foreign key, multi-account mismatch) must not cause a panic or
    /// hang. The total balance is exact (the UTXO is in the set regardless),
    /// but the foreign address is absent from the pool so per-address
    /// visibility is reduced. `tracing::warn!` fires for the unresolved count.
    #[test]
    fn rehydration_unresolvable_address_is_deferred_not_panics() {
        use dashcore::blockdata::transaction::txout::TxOut;
        use dashcore::{OutPoint, Txid};
        use key_wallet::bip32::DerivationPath;
        use key_wallet::gap_limit::DEFAULT_EXTERNAL_GAP_LIMIT;
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::{Address, Utxo};
        use std::collections::HashSet;

        let seed = [13u8; 64];
        let wallet = Wallet::from_seed_bytes(
            seed,
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&wallet);
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 1);

        let funds_type = wallet_info
            .accounts
            .all_funding_accounts()
            .first()
            .unwrap()
            .managed_account_type()
            .to_account_type();
        let xpub = manifest
            .iter()
            .find(|e| e.account_type == funds_type)
            .map(|e| e.account_xpub)
            .expect("funds account xpub");

        // Normal UTXO at external index 3 (within eager window, pool-visible).
        let normal_addr = {
            let mut p = AddressPool::new_without_generation(
                DerivationPath::master(),
                AddressPoolType::External,
                DEFAULT_EXTERNAL_GAP_LIMIT,
                Network::Testnet,
            );
            p.generate_addresses(4, &KeySource::Public(xpub), true)
                .unwrap();
            p.address_at_index(3).unwrap()
        };

        // Foreign address: derive from a completely different wallet seed so
        // it cannot be resolved from this wallet's xpub.
        let foreign_addr = {
            let fw = Wallet::from_seed_bytes(
                [99u8; 64],
                Network::Testnet,
                WalletAccountCreationOptions::Default,
            )
            .unwrap();
            let fw_info = ManagedWalletInfo::from_wallet(&fw, 1);
            fw_info
                .accounts
                .all_funding_accounts()
                .into_iter()
                .next()
                .unwrap()
                .managed_account_type()
                .address_pools()
                .first()
                .unwrap()
                .address_at_index(0)
                .unwrap()
        };
        assert_ne!(
            normal_addr, foreign_addr,
            "test fixture: foreign address must differ from normal"
        );

        let utxo = |addr: Address, value: u64, n: u8| Utxo {
            outpoint: OutPoint {
                txid: Txid::from([n; 32]),
                vout: 0,
            },
            txout: TxOut {
                value,
                script_pubkey: addr.script_pubkey(),
            },
            address: addr,
            height: 1,
            is_coinbase: false,
            is_confirmed: true,
            is_instantlocked: false,
            is_locked: false,
            is_trusted: false,
        };

        let normal_val = 100_000u64;
        let foreign_val = 200_000u64;
        let expected_total = normal_val + foreign_val;

        let core = crate::changeset::CoreChangeSet {
            new_utxos: vec![
                utxo(normal_addr, normal_val, 1),
                utxo(foreign_addr, foreign_val, 2),
            ],
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };

        // Must not panic. tracing::warn! fires for the unresolved count.
        apply_persisted_core_state(&mut wallet_info, &manifest, &core).unwrap();

        // Total balance is exact — foreign UTXO is in the set regardless.
        assert_eq!(
            wallet_info.balance.total(),
            expected_total,
            "total must include foreign UTXO even though it is unresolved"
        );

        // Per-address visible: only the normal UTXO is in the pool.
        let funds = wallet_info
            .accounts
            .all_funding_accounts()
            .into_iter()
            .next()
            .unwrap();
        let pool_addresses: HashSet<Address> = funds
            .managed_account_type()
            .address_pools()
            .iter()
            .flat_map(|p| p.addresses.values().map(|i| i.address.clone()))
            .collect();
        let visible: u64 = funds
            .utxos
            .values()
            .filter(|u| pool_addresses.contains(&u.address))
            .map(|u| u.value())
            .sum();
        assert_eq!(
            visible, normal_val,
            "only the non-foreign UTXO is pool-visible; foreign deferred to re-warm"
        );
        assert!(
            visible < expected_total,
            "foreign UTXO is deferred — per-address visible < total"
        );
    }

    /// QA-003: CoinJoin topology (single External pool, no Internal chain).
    /// Verifies that `extend_pools_for_restored_utxos` handles a single-pool
    /// account at a deep derivation index (idx 30, just past the eager window).
    #[test]
    fn rehydration_coinjoin_single_pool_deep_index() {
        use dashcore::blockdata::transaction::txout::TxOut;
        use dashcore::{OutPoint, Txid};
        use key_wallet::managed_account::address_pool::{AddressPool, KeySource};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::Utxo;
        use std::collections::BTreeSet;

        // CoinJoin-only wallet: no BIP44, one CoinJoin account at index 0.
        let mut cj_set = BTreeSet::new();
        cj_set.insert(0u32);
        let opts = WalletAccountCreationOptions::SpecificAccounts(
            BTreeSet::new(),
            BTreeSet::new(),
            cj_set,
            BTreeSet::new(),
            BTreeSet::new(),
            None,
        );
        let seed = [11u8; 64];
        let wallet = Wallet::from_seed_bytes(seed, Network::Testnet, opts).unwrap();
        assert!(
            !wallet.accounts.coinjoin_accounts.is_empty(),
            "fixture must have a CoinJoin account"
        );

        let manifest = manifest_for(&wallet);
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 1);

        // Extract pool metadata before the mutable borrow of wallet_info.
        let (funds_type, pool_base_path, pool_type_val, pool_gap_limit) = {
            let funds = wallet_info
                .accounts
                .all_funding_accounts()
                .into_iter()
                .next()
                .expect("CoinJoin account must be the only funds account");
            let ft = funds.managed_account_type().to_account_type();
            let pools = funds.managed_account_type().address_pools();
            // CoinJoin has a single pool (External).
            assert_eq!(
                pools.len(),
                1,
                "CoinJoin topology: must have exactly one pool"
            );
            let p = &pools[0];
            (ft, p.base_path.clone(), p.pool_type, p.gap_limit)
        };

        let xpub = manifest
            .iter()
            .find(|e| e.account_type == funds_type)
            .map(|e| e.account_xpub)
            .expect("CoinJoin xpub must be in manifest");

        // Derive the CoinJoin address at index 30 (first past the eager
        // window 0..=29) using the real pool's base_path and pool_type.
        let mut probe = AddressPool::new_without_generation(
            pool_base_path,
            pool_type_val,
            pool_gap_limit,
            Network::Testnet,
        );
        probe
            .generate_addresses(31, &KeySource::Public(xpub), true)
            .unwrap();
        let deep_cj_addr = probe.address_at_index(30).unwrap();

        let utxo_val = 7_777u64;
        let utxo = Utxo {
            outpoint: OutPoint {
                txid: Txid::from([7u8; 32]),
                vout: 0,
            },
            txout: TxOut {
                value: utxo_val,
                script_pubkey: deep_cj_addr.script_pubkey(),
            },
            address: deep_cj_addr.clone(),
            height: 1,
            is_coinbase: false,
            is_confirmed: true,
            is_instantlocked: false,
            is_locked: false,
            is_trusted: false,
        };

        let core = crate::changeset::CoreChangeSet {
            new_utxos: vec![utxo],
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };

        apply_persisted_core_state(&mut wallet_info, &manifest, &core).unwrap();

        // Balance is exact.
        assert_eq!(
            wallet_info.balance.total(),
            utxo_val,
            "CoinJoin deep-index balance must be exact"
        );

        // The CoinJoin pool was extended to include the deep-index address.
        let funds_post = wallet_info
            .accounts
            .all_funding_accounts()
            .into_iter()
            .next()
            .unwrap();
        let cj_pool = &funds_post.managed_account_type().address_pools()[0];
        assert_eq!(
            cj_pool.address_at_index(30).as_ref(),
            Some(&deep_cj_addr),
            "CoinJoin pool must be extended to cover deep-index address at idx 30"
        );
    }
}
