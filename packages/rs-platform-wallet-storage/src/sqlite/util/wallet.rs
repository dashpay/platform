//! External-signable wallet reconstruction
//!
//! Load is seedless — each wallet is rebuilt watch-only from its manifest and
//! the manager consumes the carried snapshot directly, so no wrong-seed check
//! runs here; that gate lives in the resolver-backed signing entrypoints.

use key_wallet::account::account_collection::AccountCollection;
use key_wallet::account::{Account, AccountType};
use key_wallet::managed_account::address_pool::{AddressPoolType, PublicKeyType};
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Network;

use platform_wallet::changeset::{
    rebuild_provider_key_account, AccountRegistrationEntry, CoreChangeSet,
    ProviderAccountRebuildError,
};
use platform_wallet::wallet::provider_key_at_index::{
    insert_platform_node_pool_entry, PlatformNodePoolError,
};

use crate::sqlite::load_ctx::{LoadCtx, LoadSite};
use crate::sqlite::schema::accounts::{self, AccountManifest};
use crate::sqlite::schema::core_pool::{self, OwningAccount};
use crate::WalletStorageError;

/// Build a [`Wallet`] that will be provided to the platform-wallet during rehydration.
pub(crate) fn build_wallet(
    network: Network,
    expected_wallet_id: [u8; 32],
    manifest: &AccountManifest,
) -> Result<Wallet, WalletStorageError> {
    if manifest.is_empty() {
        return Err(WalletStorageError::MissingAccount {
            wallet_id: expected_wallet_id,
        });
    }
    let mut accounts = AccountCollection::new();
    for entry in &manifest.ecdsa {
        // `Account::from_xpub` is infallible in the pinned key-wallet rev; this
        // map_err is a defensive guard for when that signature becomes fallible.
        let account = Account::from_xpub(
            Some(expected_wallet_id),
            entry.account_type,
            entry.account_xpub,
            network,
        )
        .map_err(|e| WalletStorageError::AccountRecordInvalid { e })?;
        accounts
            .insert(account)
            .map_err(|_| WalletStorageError::AccountRegistrationEntryMismatch)?;
    }
    // Provider key-material accounts take the shared rebuild path (the FFI
    // backend's restore side calls the same helper).
    for entry in &manifest.provider {
        rebuild_provider_key_account(
            &mut accounts,
            expected_wallet_id,
            network,
            entry.account_type,
            &entry.extended_public_key,
        )
        .map_err(|e| match e {
            ProviderAccountRebuildError::Invalid(e) => {
                WalletStorageError::AccountRecordInvalid { e }
            }
            ProviderAccountRebuildError::Rejected(_) => {
                WalletStorageError::ProviderKeyAccountEntryMismatch
            }
        })?;
    }
    Ok(Wallet::new_external_signable(
        network,
        expected_wallet_id,
        accounts,
    ))
}

/// Restore pre-derived platform-node public keys from typed address-pool rows.
///
/// The account's `AbsentHardened` EdDSA entries cannot be regenerated from a
/// watch-only xpub, so they round-trip verbatim through
/// [`core_pool::load_typed_pool_entries`].
pub(crate) fn restore_provider_platform_node_pool(
    wallet_info: &mut ManagedWalletInfo,
    conn: &rusqlite::Connection,
    wallet_id: &platform_wallet::wallet::platform_wallet::WalletId,
    network: Network,
) -> Result<(), WalletStorageError> {
    if wallet_info.accounts.provider_platform_keys.is_none() {
        return Ok(());
    }

    let entries = core_pool::load_typed_pool_entries(
        conn,
        wallet_id,
        &AccountType::ProviderPlatformKeys,
        AddressPoolType::AbsentHardened,
    )?;
    if entries.is_empty() {
        return Ok(());
    }

    // TODO(#4188): `reserved_at` is persisted but deliberately not consumed here;
    // restoring it requires widening `insert_platform_node_pool_entry` in rs-platform-wallet.
    for (index, script_bytes, public_key, used) in entries {
        let PublicKeyType::EdDSA(public_key) = public_key else {
            return Err(WalletStorageError::blob_decode(
                "provider platform pool row does not carry an EdDSA public key",
            ));
        };
        let public_key = public_key.try_into().map_err(|_| {
            WalletStorageError::blob_decode(
                "provider platform pool row has the wrong EdDSA public-key length",
            )
        })?;
        let script_pubkey = dashcore::ScriptBuf::from_bytes(script_bytes);
        let address = dashcore::Address::from_script(&script_pubkey, network)?;
        insert_platform_node_pool_entry(
            wallet_info,
            network,
            index,
            address,
            script_pubkey,
            public_key,
            used,
        )
        .map_err(|error| match error {
            PlatformNodePoolError::InvalidAccountPath { source } => {
                WalletStorageError::AccountRecordInvalid { e: source }
            }
            PlatformNodePoolError::MissingHardenedPool => WalletStorageError::blob_decode(
                "provider platform account has no AbsentHardened pool",
            ),
            PlatformNodePoolError::InvalidChildIndex { source, .. } => {
                WalletStorageError::AccountRecordInvalid { e: source }
            }
        })?;
    }
    Ok(())
}

/// Apply the keyless persisted core-state projection onto a
/// freshly-minted `ManagedWalletInfo` skeleton.
///
/// # Parameters
///
/// - `wallet_info`: the skeleton to hydrate in place.
/// - `manifest`: keyless account manifest (one entry per registered
///   account). Each entry carries an `account_type` → `account_xpub`
///   mapping used by [`extend_pools_for_restored_addresses`] to derive
///   addresses for restored UTXOs. If an account's `account_type` is
///   absent from the manifest, deep-index derivation is skipped for that
///   account (no xpub → no derivation possible); already-derived in-window
///   addresses are still marked used.
/// - `core`: the persisted core-state changeset to apply.
/// - `utxo_accounts`: per-outpoint owning-account side channel from
///   [`load_state`](crate::sqlite::schema::core_state::load_state) — the
///   `CoreChangeSet` cannot carry it. Each restored unspent UTXO is routed
///   to the funds account whose identity matches its entry; a UTXO absent
///   from the map (its script matched no pool row) falls back to the first
///   funds account and re-warms on the next sync.
/// - `used_pool_addresses`: addresses the persisted pool snapshot marked
///   used, each mapped to its owning account (`None` when the script matched
///   no pool row). Each is routed to its owning funds account — via the same
///   identity match as `utxo_accounts` — and marked used there, in union with
///   the still-unspent UTXO addresses, so a previously-used address whose
///   funds were since spent is never re-handed-out as a fresh receive address
///   from its own account (address-reuse guard). An owner absent from this
///   wallet's funds accounts, or a `None` owner, falls back to the first
///   account. Empty = no pool used-state carried.
///
/// # Reconstructed (safety-critical-correct)
///
/// - **Wallet balance** (`wallet_info.balance`, the no-silent-zero
///   guarantee): every persisted UTXO is restored and the per-account
///   and wallet totals are recomputed via `update_balance()`. A UTXO
///   carrying a block height is marked confirmed so it lands in the
///   `confirmed` bucket; the wallet total is exact regardless.
/// - **UTXO set**: every unspent persisted outpoint is restored into its
///   owning funds-bearing account (matched via `utxo_accounts` across any
///   topology — BIP44, BIP32, CoinJoin, DashPay), so per-account balance,
///   coin selection, and reservations are correct after restart. An
///   outpoint that resolves to no account falls back to the first.
/// - **Address-pool depth**: each pool is forward-derived to cover
///   restored UTXOs at deep derivation indices, then the gap window is
///   refilled beyond the deepest restored index so the per-address view
///   reconciles with the wallet total.
/// - **Address-pool used-state**: every `used_pool_addresses` entry is
///   re-marked used (in union with the unspent-UTXO addresses), so an
///   address whose funds were since spent is not re-handed-out as fresh.
/// - **Sync watermarks**: `synced_height` / `last_processed_height`.
///
/// # Reconstructed when the persister supplies it
///
/// - **`last_applied_chain_lock`**: restored from `core` when the
///   supplied [`CoreChangeSet`](platform_wallet::changeset::CoreChangeSet) carries
///   it (the FFI/iOS persister round-trips the value Swift held), so the
///   asset-lock-resume CL-from-metadata fallback (`proof.rs`) fires at
///   launch instead of waiting for SPV. The SQLite storage path has no
///   V001 column for it yet (dashpay/platform#3968), so there it is
///   absent from `core` and stays `None` until SPV re-applies a fresh
///   chainlock on the first post-restart sync.
///
/// # Deferred to the first post-load `sync` (safe re-warm)
///
/// - **Deep-index address visibility**: each chain's pool scan stops
///   after [`MAX_REHYDRATION_DERIVATION_INDEX`] or after `gap_limit`
///   consecutive non-matching indices past the deepest resolved index.
///   The horizon only advances when an unspent UTXO anchors a match, so a
///   UTXO address can be left unresolved in two distinct cases: (1) it is
///   genuinely foreign (a different account's key routed here, or corrupt),
///   and (2) it is a *legitimately-owned but deep-and-sparse* address —
///   owned by this account, yet sitting past the first `gap_limit` window
///   with no nearer unspent UTXO to walk the horizon out to it. Both cases
///   are counted and logged via `tracing::warn!` and re-warm on the next
///   full sync. The wallet *total* stays exact (every UTXO is summed
///   regardless of pool visibility); only the per-address view is
///   incomplete until that sync. This is the accepted behavior of the
///   horizon-walk algorithm — see [`extend_pools_for_restored_addresses`].
/// - **Per-UTXO `is_coinbase` / `is_instantlocked` / `is_trusted`
///   flags**: not columns in `core_utxos`; conservatively defaulted
///   (non-coinbase, confirmed-by-height) and refreshed on the next
///   scan. Coinbase-maturity nuance re-warms on sync.
/// - **Transaction-record history**: rebuilt by the next scan; not a
///   balance input.
///
/// # Errors
///
/// [`WalletStorageError::MissingAccount`] if there are persisted UTXOs to
/// restore but the reconstructed account collection has **no**
/// funds-bearing account to hold them. Fail-closed rather than
/// reconstructing a silent zero balance (the no-silent-zero mandate). An
/// empty UTXO set is always `Ok`.
///
/// This never touches key material.
pub fn apply_persisted_core_state(
    wallet_info: &mut ManagedWalletInfo,
    manifest: &[AccountRegistrationEntry],
    core: &CoreChangeSet,
    utxo_accounts: &std::collections::HashMap<dashcore::OutPoint, OwningAccount>,
    used_pool_addresses: &std::collections::HashMap<key_wallet::Address, Option<OwningAccount>>,
    ctx: &LoadCtx,
) -> Result<(), WalletStorageError> {
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

    // Captured before the mutable account borrow below so it can flow into
    // pool-extension diagnostics without re-borrowing `wallet_info`.
    let wallet_id = wallet_info.wallet_id;

    // Sync watermarks first so `update_balance`'s maturity check sees
    // the restored tip.
    if let Some(h) = core.last_processed_height {
        wallet_info.metadata.last_processed_height =
            wallet_info.metadata.last_processed_height.max(h);
    }
    if let Some(h) = core.synced_height {
        wallet_info.metadata.synced_height = wallet_info.metadata.synced_height.max(h);
    }

    // Restore the highest applied chainlock when the persister carries it
    // (FFI path) so the asset-lock proof CL-from-metadata fallback fires at launch.
    if let Some(cl) = &core.last_applied_chain_lock {
        wallet_info.metadata.last_applied_chain_lock = Some(cl.clone());
    }

    // INTENTIONAL(rehydration gaps): `core` also carries instant-send locks and
    // transaction records, but neither can be replayed here —
    // `ManagedWalletInfo.instant_send_locks` is `pub(crate)` with no public
    // setter, and there is no public API to inject tx records. Both re-warm on
    // the next sync (no regression vs. the prior loader); populating them at
    // load needs an upstream key_wallet change.

    // Restore the UTXO set, routing each unspent outpoint to its true owning
    // funds account via `utxo_accounts` (matched on the same account identity
    // the writer keyed the pool row on). Two miss cases share the first-account
    // best-effort fallback but differ in signal:
    //   (a) no side-channel entry — the UTXO's script matched no pool row; an
    //       expected fallback (deep/sparse or non-funds address) that re-warms
    //       on the next sync, no log.
    //   (b) a side-channel entry whose owner is not among this wallet's funds
    //       accounts — a pool row references an unregistered account (store
    //       drift). Counted and surfaced via `tracing::warn!` after the loop so
    //       the misattribution is never silent.
    // A wallet with unspent UTXOs but no funds account at all fails closed
    // rather than silently reconstructing a zero balance.
    let spent_outpoints: std::collections::HashSet<dashcore::OutPoint> =
        core.spent_utxos.iter().map(|u| u.outpoint).collect();
    let unspent: Vec<&key_wallet::Utxo> = core
        .new_utxos
        .iter()
        .filter(|u| !spent_outpoints.contains(&u.outpoint))
        .collect();

    let mut funding = wallet_info.accounts.all_funding_accounts_mut();
    if !unspent.is_empty() && funding.is_empty() {
        return Err(WalletStorageError::MissingAccount { wallet_id });
    }
    if !funding.is_empty() {
        let account_keys: Vec<OwningAccount> =
            funding.iter().map(|a| owning_account_of(a)).collect();

        // Per-account addresses to derive-and-mark-used: each account gets only
        // its own restored UTXO addresses, so `extend_pools_for_restored_addresses`
        // never scans another account's keys as "unresolved".
        let mut per_account_addrs: Vec<Vec<key_wallet::Address>> = vec![Vec::new(); funding.len()];

        // Owners that resolve to no funds account (case (b) above), plus used
        // addresses with an owner absent from this wallet — collected across
        // both routing loops and warned once after them, never per iteration.
        let mut orphaned_owners: Vec<String> = Vec::new();
        for utxo in &unspent {
            let target = route_to_funds_account(
                &account_keys,
                utxo_accounts.get(&utxo.outpoint),
                &mut orphaned_owners,
            );
            funding[target].utxos.insert(utxo.outpoint, (*utxo).clone());
            per_account_addrs[target].push(utxo.address.clone());
        }

        // The persisted pool used-state restores addresses whose funds were
        // since spent — without it a previously-used address comes back marked
        // unused and could be handed out again as a fresh receive address
        // (address-reuse privacy leak). Each is routed to its owning funds
        // account (same identity match as the UTXOs), so a used address on a
        // non-first account is marked used on ITS OWN pool. A `None` owner, or
        // one absent from this wallet, falls back to the first account. An
        // empty map marks only the unspent-UTXO addresses.
        for (addr, owner) in used_pool_addresses {
            let target =
                route_to_funds_account(&account_keys, owner.as_ref(), &mut orphaned_owners);
            per_account_addrs[target].push(addr.clone());
        }

        if !orphaned_owners.is_empty() {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                orphaned_owners = ?orphaned_owners,
                count = orphaned_owners.len(),
                "rehydration: restored UTXO(s) or used address(es) reference a funds \
                 account not present in this wallet — routed to the first account as \
                 best effort; the per-account view re-warms on the next sync"
            );
        }

        // Eager derivation covers only `0..gap_limit`; extend each chain to
        // cover restored / used addresses at deeper indices.
        for i in 0..funding.len() {
            if !per_account_addrs[i].is_empty() {
                extend_pools_for_restored_addresses(
                    funding[i],
                    manifest,
                    &per_account_addrs[i],
                    wallet_id,
                    ctx,
                )?;
            }
        }
    }

    // Recompute per-account + wallet balance from the restored set.
    // After this, a non-zero persisted balance is non-zero here — a
    // silent zero would be a hard FAIL of the rehydration contract.
    wallet_info.update_balance();
    Ok(())
}

/// Resolve an owning account to its position among `account_keys`, or fall
/// back to the first funds account. A `None` owner (no attribution available)
/// falls back silently; an owner not present in `account_keys` (store drift)
/// falls back too but is recorded in `orphaned_owners` for a single post-loop
/// `tracing::warn!`. Shared by the UTXO and used-address routing loops so both
/// bucket funds and used-state by the exact same identity match.
fn route_to_funds_account(
    account_keys: &[OwningAccount],
    owner: Option<&OwningAccount>,
    orphaned_owners: &mut Vec<String>,
) -> usize {
    match owner {
        None => 0,
        Some(owner) => account_keys
            .iter()
            .position(|k| k == owner)
            .unwrap_or_else(|| {
                orphaned_owners.push(format!("{}[{}]", owner.account_type, owner.account_index));
                0
            }),
    }
}

/// Owning-account identity of a funds account, keyed on the same
/// discriminators the UTXO side channel resolves from `core_address_pool`:
/// the `account_type` label, numeric index, and DashPay identity pair. Enough
/// to pick one account among funding accounts that share a numeric index
/// (Standard BIP44/BIP32 and CoinJoin can all sit at index 0; DashPay accounts
/// all carry index 0 and differ only by the identity pair).
fn owning_account_of(
    account: &key_wallet::managed_account::ManagedCoreFundsAccount,
) -> OwningAccount {
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    let at = account.managed_account_type().to_account_type();
    let (user_identity_id, friend_identity_id) = accounts::account_dashpay_ids(&at);
    OwningAccount {
        account_type: accounts::account_type_db_label(&at).to_string(),
        account_index: accounts::account_index(&at),
        user_identity_id,
        friend_identity_id,
    }
}

/// Upper bound on forward derivation while resolving a restored UTXO
/// address to its derivation index. Addresses that don't resolve within
/// this many indices (e.g. they belong to a different funds account whose
/// UTXOs were routed here, or are corrupt) are left for the next full
/// rescan to re-warm — generous enough to cover any realistic per-account
/// derivation depth. The common (single funds account) path terminates at
/// the true high-water mark well before this and never reaches the cap.
const MAX_REHYDRATION_DERIVATION_INDEX: u32 = 10_000;

/// Soft threshold past which a single chain's discovery scan is treated as
/// abnormally deep and worth a `tracing::warn!`. Real funds chains anchor
/// well below this; reaching it means either a corrupt / foreign-heavy UTXO
/// set walking the horizon out, or an approach toward the hard
/// [`MAX_REHYDRATION_DERIVATION_INDEX`] ceiling — both worth surfacing.
const REHYDRATION_DEEP_SCAN_WARN_INDEX: u32 = 1_000;

/// Extend `account`'s address pools so every resolved address (a
/// still-unspent UTXO address or a persisted pool used-address) is derived
/// at its exact `(chain, index)` slot and marked used, then refill the gap
/// window beyond — following the sync path's `mark_used` →
/// `maintain_gap_limit` sequence. Each chain is scanned independently,
/// stopping once no unresolved address matches within a `gap_limit`-sized
/// window past the deepest resolved index; [`MAX_REHYDRATION_DERIVATION_INDEX`]
/// is the hard ceiling. Addresses that don't resolve from this account's
/// xpub — foreign keys, multi-account mismatch, or legitimately-owned but
/// deep-and-sparse slots with no nearer resolved address to anchor the horizon —
/// are counted and logged via `tracing::warn!`; they re-warm on the next
/// full sync. Every resolved address the pools *do* hold (in-window or
/// deep-resolved) is marked used so a funded or previously-used address is
/// never handed out as a fresh receive address.
///
/// Tested with Standard BIP44 topology (External + Internal pools) and
/// CoinJoin topology (single External pool). The per-chain probe loop has no
/// topology-specific branches, so the non-hardened single-pool type
/// (`Absent`) follows the same code path with a different relative derivation
/// path. `AbsentHardened` pools cannot be derived from a public xpub at all —
/// hardened child derivation needs the private key — so under watch-only
/// rehydration their addresses never resolve and always defer to the next
/// sync (shared code path, but the outcome is "unresolved").
///
/// # Errors
///
/// [`WalletStorageError::RehydrationPoolMismatch`] if the discovery probes
/// don't mirror the real pools 1:1 (a structural invariant break, not
/// user-reachable). Fail-closed rather than apply a probe depth to the wrong
/// pool by position.
///
/// Never touches key material — the xpub is the keyless account public key.
fn extend_pools_for_restored_addresses(
    account: &mut key_wallet::managed_account::ManagedCoreFundsAccount,
    manifest: &[AccountRegistrationEntry],
    restored_addresses: &[key_wallet::Address],
    wallet_id: [u8; 32],
    ctx: &LoadCtx,
) -> Result<(), WalletStorageError> {
    use key_wallet::managed_account::address_pool::{AddressPool, KeySource};
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use std::collections::HashSet;

    let account_type = account.managed_account_type().to_account_type();

    // The funds account carries no key material; recover its watch-only xpub
    // from the keyless manifest by account type. Without it we cannot derive
    // deeper, but can still mark already-derived (in-window) addresses used.
    let key_source = manifest
        .iter()
        .find(|e| e.account_type == account_type)
        .map(|e| KeySource::Public(e.account_xpub));

    // Probe pools mirror each real pool's chain 1:1 so the index search
    // derives into throwaway state (real pools keep their own exact depth)
    // and the resolved depth can be applied back by position. Re-deriving
    // each probe from index 0 is an accepted, bounded one-time-load cost
    // (per chain capped at MAX_REHYDRATION_DERIVATION_INDEX); rehydration
    // runs once per wallet at startup, never on a hot path.
    let mut probes: Vec<(AddressPool, Option<u32>)> = account
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
                None,
            )
        })
        .collect();

    // Deep-index discovery (requires the xpub): resolve restored addresses the
    // eager derivation didn't already cover, recording the matching index per
    // chain. Each chain advances independently and stops once no unresolved
    // address resolves within gap_limit indices past its deepest match
    // (preventing a full scan when the UTXO set carries foreign addresses);
    // MAX_REHYDRATION_DERIVATION_INDEX is the hard ceiling regardless.
    if let Some(key_source) = key_source.as_ref() {
        let mut unresolved: HashSet<key_wallet::Address> = {
            let pools = account.managed_account_type().address_pools();
            restored_addresses
                .iter()
                .filter(|addr| !pools.iter().any(|p| p.contains_address(addr)))
                .cloned()
                .collect()
        };

        for (probe, deepest_resolved) in probes.iter_mut() {
            if unresolved.is_empty() {
                break;
            }
            let chain_gap = probe.gap_limit;
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

                if let Some(addr) = ensure_derived(probe, key_source, index) {
                    // Indices are visited in ascending order, so the last match
                    // is the deepest — record it directly (no per-chain set).
                    if unresolved.remove(&addr) {
                        *deepest_resolved = Some(index);
                    }
                }

                if unresolved.is_empty() {
                    break;
                }

                index = index.saturating_add(1);
            }

            // Surface an abnormally deep scan once per chain (outside the loop
            // — never log inside the per-index walk).
            if index > REHYDRATION_DEEP_SCAN_WARN_INDEX {
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    account_type = ?account_type,
                    pool_type = ?probe.pool_type,
                    deepest_resolved = ?deepest_resolved,
                    scanned_to = index.saturating_sub(1),
                    "rehydration: chain discovery scanned abnormally deep — \
                     likely a foreign-heavy or sparse UTXO set"
                );
            }
        }

        // Still-unresolved addresses are either foreign (a different account's
        // key routed here, or corrupt) or legitimately-owned but deep-and-sparse
        // (past the first gap window with no nearer unspent UTXO to anchor the
        // horizon). Either way they re-warm on the next full sync; the wallet
        // total is exact regardless.
        if !unresolved.is_empty() {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                account_type = ?account_type,
                unresolved_count = unresolved.len(),
                "rehydration: UTXO address(es) unresolved for this account xpub \
                 — will re-warm on next sync; balance total is exact"
            );
        }
    }

    // No explicit aggregate-derivation cap is needed: a funds account exposes
    // a fixed, small number of chains (Standard = 2, others = 1), each already
    // capped at MAX_REHYDRATION_DERIVATION_INDEX, so total derivation is bounded
    // by chains × MAX with no unbounded growth — an aggregate cap would either
    // equal that natural bound (no-op) or clip a legitimate deep multi-chain
    // wallet. The per-chain ceiling plus the deep-scan warn above are the
    // proportionate guard against a corrupt/foreign-heavy UTXO set.

    // Apply discovered depths and mark restored addresses used. `probes` is
    // built directly from `address_pools()`, so it mirrors `address_pools_mut()`
    // 1:1 and in chain order; verify that invariant before zipping by position.
    let mut pools = account.managed_account_type_mut().address_pools_mut();
    if pools.len() != probes.len() {
        return Err(WalletStorageError::RehydrationPoolMismatch {
            expected: probes.len(),
            found: pools.len(),
        });
    }
    for (position, (pool, (probe, deepest_resolved))) in
        pools.iter_mut().zip(probes.iter()).enumerate()
    {
        // `iter_mut()` over `Vec<&mut AddressPool>` yields `&mut &mut _`;
        // reborrow once so the pool flows into `ensure_derived` cleanly.
        let pool: &mut AddressPool = pool;

        // Runtime fail-closed guard (a release build compiles out a
        // `debug_assert!`): applying a probe's depth to a pool of a different
        // chain would misattribute derivation to the wrong pool by position.
        if pool.pool_type != probe.pool_type {
            return Err(WalletStorageError::RehydrationPoolTypeMismatch {
                position,
                expected: probe.pool_type,
                found: pool.pool_type,
            });
        }

        // Derive up to the deepest discovered index so its address exists in
        // the real pool before we mark it used.
        // TODO(recovery-mode): defensive site with no reachable seed from
        // the storage layer — the probe resolved this index from the same
        // xpub, so the real pool derives it too. Kept fail-closed because a
        // deferred address is an address that can be re-issued as fresh.
        if let Some(deepest) = *deepest_resolved {
            if let Some(key_source) = key_source.as_ref() {
                if ensure_derived(pool, key_source, deepest).is_none() {
                    ctx.tolerate(
                        LoadSite::RehydrationEnsureDerived,
                        WalletStorageError::RehydrationDerivationFailed {
                            stage: "ensure_derived",
                            index: deepest,
                            cause: "the pool produced no address at this index".to_owned(),
                        },
                    )?;
                    warn_deferred_pool(wallet_id, &account_type, pool.pool_type);
                }
            }
        }

        // Mark every restored address this pool now holds as used — covers both
        // deep-resolved addresses (just derived) and in-window addresses the
        // discovery scan never visits. Without this an already-derived but
        // funded address keeps `used = false` and could be handed out as a fresh
        // receive address. `mark_used` is a no-op for addresses not in this
        // pool, so an underived (foreign / sparse) index is never marked.
        //
        // Mark ↔ refill runs to a FIXPOINT: marking raises `highest_used`,
        // whose gap refill can derive a deeper previously-used address that
        // the discovery walk missed (e.g. used idx 45 with in-window used
        // idx 20 and gap 30 — the walk's horizon stops at 30, but the refill
        // reaches 50 and derives idx 45). A single mark-then-refill pass
        // would leave that address in the pool with `used = false`, handing
        // a previously-used address back out as fresh. Terminates: each
        // round marks at least one new address from the finite restored set
        // (`mark_used` returns `true` only on an unused→used flip).
        loop {
            let mut marked_any = false;
            for addr in restored_addresses {
                if pool.mark_used(addr) {
                    marked_any = true;
                }
            }
            if !marked_any {
                break;
            }
            // Refill the gap window past the deepest used index (needs the
            // xpub); without one no deeper address can be derived, so a
            // single mark pass is all that's possible.
            let Some(key_source) = key_source.as_ref() else {
                break;
            };
            if let Err(e) = pool.maintain_gap_limit(key_source) {
                ctx.tolerate(
                    LoadSite::RehydrationGapLimit,
                    WalletStorageError::RehydrationDerivationFailed {
                        stage: "maintain_gap_limit",
                        index: pool.highest_used.unwrap_or(0),
                        cause: e.to_string(),
                    },
                )?;
                warn_deferred_pool(wallet_id, &account_type, pool.pool_type);
                break;
            }
        }
    }
    Ok(())
}

/// Locate a pool whose window a tolerated derivation failure left short.
/// The failure itself is logged by [`LoadCtx::tolerate`]; this adds only
/// the coordinates, and so fires only when the policy tolerated it.
fn warn_deferred_pool(
    wallet_id: [u8; 32],
    account_type: &AccountType,
    pool_type: key_wallet::managed_account::address_pool::AddressPoolType,
) {
    tracing::warn!(
        wallet_id = %hex::encode(wallet_id),
        account_type = ?account_type,
        pool_type = ?pool_type,
        "rehydration: this address pool's window is short until the next sync"
    );
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
        let ecdsa = manifest_for(&w);
        let manifest = AccountManifest {
            ecdsa: ecdsa.clone(),
            provider: Vec::new(),
        };

        let restored = build_wallet(Network::Testnet, id, &manifest).unwrap();
        assert_eq!(restored.wallet_id, id);
        assert_eq!(restored.compute_wallet_id(), id);
        let restored_types: Vec<_> = restored
            .accounts
            .all_accounts()
            .into_iter()
            .map(|a| a.account_type)
            .collect();
        let manifest_types: Vec<_> = ecdsa.iter().map(|e| e.account_type).collect();
        assert_eq!(restored_types.len(), manifest_types.len());
        for t in &manifest_types {
            assert!(restored_types.contains(t));
        }
    }

    #[test]
    fn empty_manifest_is_missing_manifest() {
        let err = build_wallet(Network::Testnet, [0u8; 32], &AccountManifest::default())
            .expect_err("empty manifest must be MissingManifest");
        assert!(matches!(err, WalletStorageError::MissingAccount { .. }));
    }

    /// Regression: after restart-in-place the watch-only pools eagerly
    /// cover only `0..gap_limit`, but persisted UTXOs can sit at deeper
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
    /// Standard BIP44 topology (External + Internal pools) is exercised.
    /// Asserts that maintain_gap_limit fills beyond the deepest resolved.
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
        let core = platform_wallet::changeset::CoreChangeSet {
            new_utxos,
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };

        apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .unwrap();

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

        // maintain_gap_limit must refill BEYOND the deepest restored
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

    /// Regression (dashpay/platform#3968): restored unspent UTXOs must land in
    /// their TRUE owning funds account, not collapse onto the first. A `Default`
    /// wallet carries Standard BIP44, BIP32, and CoinJoin accounts all at numeric
    /// index 0, so routing by bare `account_index` is ambiguous — the
    /// owning-account side channel disambiguates by account type. Asserts each
    /// account holds only its own UTXO and its per-account balance is exact.
    #[test]
    fn rehydration_routes_utxos_to_their_owning_account() {
        use dashcore::blockdata::transaction::txout::TxOut;
        use dashcore::{OutPoint, Txid};
        use key_wallet::bip32::DerivationPath;
        use key_wallet::gap_limit::DEFAULT_EXTERNAL_GAP_LIMIT;
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::{Address, Utxo};
        use std::collections::HashMap;

        let seed = [11u8; 64];
        let wallet = Wallet::from_seed_bytes(
            seed,
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&wallet);
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 1);

        // The two funds accounts that share numeric index 0 but differ by type.
        let bip44_type = wallet_info
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .unwrap()
            .managed_account_type()
            .to_account_type();
        let coinjoin_type = wallet_info
            .accounts
            .coinjoin_accounts
            .get(&0)
            .unwrap()
            .managed_account_type()
            .to_account_type();

        // Derive external index-0 address from a given account xpub; `base_path`
        // is record-keeping only and does not affect the derived address.
        let derive = |at: key_wallet::account::AccountType| -> Address {
            let xpub = manifest
                .iter()
                .find(|e| e.account_type == at)
                .map(|e| e.account_xpub)
                .expect("account xpub in manifest");
            let mut p = AddressPool::new_without_generation(
                DerivationPath::master(),
                AddressPoolType::External,
                DEFAULT_EXTERNAL_GAP_LIMIT,
                Network::Testnet,
            );
            p.generate_addresses(1, &KeySource::Public(xpub), true)
                .unwrap();
            p.address_at_index(0).unwrap()
        };

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
        let bip44_utxo = utxo(derive(bip44_type), 5_000, 1);
        let coinjoin_utxo = utxo(derive(coinjoin_type), 7_000, 2);
        let bip44_op = bip44_utxo.outpoint;
        let coinjoin_op = coinjoin_utxo.outpoint;

        // Side channel attributing each outpoint to its true owning account —
        // keyed exactly as production resolves it from `core_address_pool`.
        let mut utxo_accounts: HashMap<OutPoint, OwningAccount> = HashMap::new();
        utxo_accounts.insert(
            bip44_op,
            owning_account_of(
                wallet_info
                    .accounts
                    .standard_bip44_accounts
                    .get(&0)
                    .unwrap(),
            ),
        );
        utxo_accounts.insert(
            coinjoin_op,
            owning_account_of(wallet_info.accounts.coinjoin_accounts.get(&0).unwrap()),
        );

        let core = platform_wallet::changeset::CoreChangeSet {
            new_utxos: vec![bip44_utxo, coinjoin_utxo],
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };

        apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &utxo_accounts,
            &Default::default(),
            &LoadCtx::strict(),
        )
        .unwrap();

        let bip44 = wallet_info
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .unwrap();
        let coinjoin = wallet_info.accounts.coinjoin_accounts.get(&0).unwrap();

        assert!(
            bip44.utxos.contains_key(&bip44_op),
            "BIP44 UTXO must route to the BIP44 account"
        );
        assert!(
            !bip44.utxos.contains_key(&coinjoin_op),
            "CoinJoin UTXO must NOT collapse onto the first (BIP44) account"
        );
        assert!(
            coinjoin.utxos.contains_key(&coinjoin_op),
            "CoinJoin UTXO must route to the CoinJoin account"
        );
        assert!(!coinjoin.utxos.contains_key(&bip44_op));

        assert_eq!(
            bip44.balance.total(),
            5_000,
            "per-account BIP44 balance must be exact"
        );
        assert_eq!(
            coinjoin.balance.total(),
            7_000,
            "per-account CoinJoin balance must be exact, not zero"
        );
        assert_eq!(
            wallet_info.balance.total(),
            12_000,
            "wallet total is the sum across accounts"
        );
    }

    /// Regression (dashpay/platform#3968): a restored *used* address (its funds
    /// since spent, so no unspent UTXO anchors it) owned by a non-first funds
    /// account must be marked used on ITS OWN account's pool — not collapsed
    /// onto the first account. On a `Default` wallet CoinJoin[0] is not first
    /// (Standard BIP44[0] is), so a used CoinJoin address routed by owner must
    /// land `used` in the CoinJoin pool and be absent from the BIP44 pool —
    /// otherwise it stays "unused" on CoinJoin and could be re-issued as a
    /// fresh receive address (the address-reuse privacy leak).
    #[test]
    fn rehydration_routes_used_address_to_owning_account() {
        use key_wallet::bip32::DerivationPath;
        use key_wallet::gap_limit::DEFAULT_EXTERNAL_GAP_LIMIT;
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::Address;
        use std::collections::HashMap;

        let wallet = Wallet::from_seed_bytes(
            [12u8; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&wallet);
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 1);

        let coinjoin_type = wallet_info
            .accounts
            .coinjoin_accounts
            .get(&0)
            .unwrap()
            .managed_account_type()
            .to_account_type();

        // CoinJoin external index-0 address (in the eager window, already
        // derived in the CoinJoin pool) — a previously-used receive address.
        let coinjoin_used: Address = {
            let xpub = manifest
                .iter()
                .find(|e| e.account_type == coinjoin_type)
                .map(|e| e.account_xpub)
                .expect("coinjoin xpub in manifest");
            let mut p = AddressPool::new_without_generation(
                DerivationPath::master(),
                AddressPoolType::External,
                DEFAULT_EXTERNAL_GAP_LIMIT,
                Network::Testnet,
            );
            p.generate_addresses(1, &KeySource::Public(xpub), true)
                .unwrap();
            p.address_at_index(0).unwrap()
        };

        // Known owner: CoinJoin[0], exactly as the pool resolver attributes it.
        let mut used: HashMap<Address, Option<OwningAccount>> = HashMap::new();
        used.insert(
            coinjoin_used.clone(),
            Some(owning_account_of(
                wallet_info.accounts.coinjoin_accounts.get(&0).unwrap(),
            )),
        );

        // No UTXOs — only the persisted pool used-state.
        let core = platform_wallet::changeset::CoreChangeSet {
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };
        apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &used,
            &LoadCtx::strict(),
        )
        .unwrap();

        let coinjoin = wallet_info.accounts.coinjoin_accounts.get(&0).unwrap();
        let cj_external = coinjoin
            .managed_account_type()
            .address_pools()
            .into_iter()
            .find(|p| p.pool_type == AddressPoolType::External)
            .expect("CoinJoin External pool");
        assert!(
            cj_external
                .address_info(&coinjoin_used)
                .expect("used address must be present in the CoinJoin pool")
                .is_used(),
            "used CoinJoin address must be marked used on the CoinJoin pool, not account 0"
        );

        // It must NOT have been (mis)routed onto the first (BIP44) account.
        let bip44 = wallet_info
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .unwrap();
        for pool in bip44.managed_account_type().address_pools() {
            assert!(
                pool.address_info(&coinjoin_used).is_none(),
                "the CoinJoin used address must not appear in any BIP44 pool"
            );
        }
    }

    /// A UTXO whose address is not derivable from this account's
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

        let core = platform_wallet::changeset::CoreChangeSet {
            new_utxos: vec![
                utxo(normal_addr, normal_val, 1),
                utxo(foreign_addr, foreign_val, 2),
            ],
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };

        // Must not panic. tracing::warn! fires for the unresolved count.
        apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .unwrap();

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

    /// CoinJoin topology (External pool, deep index).
    /// Verifies that `extend_pools_for_restored_addresses` handles the
    /// CoinJoin External pool at a deep derivation index (idx 30, just past
    /// the eager window). CoinJoin accounts carry both an External and an
    /// Internal pool (mirroring `Standard`); this test exercises the
    /// External side only.
    #[test]
    fn rehydration_coinjoin_single_pool_deep_index() {
        use dashcore::blockdata::transaction::txout::TxOut;
        use dashcore::{OutPoint, Txid};
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
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
            // CoinJoin carries both an External and an Internal pool; this
            // test targets the External side specifically.
            let p = pools
                .iter()
                .find(|p| p.pool_type == AddressPoolType::External)
                .expect("CoinJoin topology: must have an External pool");
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

        let core = platform_wallet::changeset::CoreChangeSet {
            new_utxos: vec![utxo],
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };

        apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .unwrap();

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
        let cj_pool = funds_post
            .managed_account_type()
            .address_pools()
            .into_iter()
            .find(|p| p.pool_type == AddressPoolType::External)
            .expect("CoinJoin topology: must have an External pool");
        assert_eq!(
            cj_pool.address_at_index(30).as_ref(),
            Some(&deep_cj_addr),
            "CoinJoin pool must be extended to cover deep-index address at idx 30"
        );
    }

    /// In-window restored UTXO: an address already covered by the eager
    /// derivation (idx 3, inside `0..=gap_limit-1`) must still be marked
    /// `used` during rehydration. The discovery scan never visits in-window
    /// addresses, so without an explicit mark pass a funded address would keep
    /// `used = false` and could later be handed out as a fresh receive address.
    #[test]
    fn rehydration_marks_in_window_restored_address_used() {
        use dashcore::blockdata::transaction::txout::TxOut;
        use dashcore::{OutPoint, Txid};
        use key_wallet::bip32::DerivationPath;
        use key_wallet::gap_limit::DEFAULT_EXTERNAL_GAP_LIMIT;
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::{Address, Utxo};

        let wallet = Wallet::from_seed_bytes(
            [5u8; 64],
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

        // External idx 3 — inside the eager window, so NOT in the discovery set.
        let in_window: Address = {
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

        let core = platform_wallet::changeset::CoreChangeSet {
            new_utxos: vec![Utxo {
                outpoint: OutPoint {
                    txid: Txid::from([1u8; 32]),
                    vout: 0,
                },
                txout: TxOut {
                    value: 12_345,
                    script_pubkey: in_window.script_pubkey(),
                },
                address: in_window.clone(),
                height: 1,
                is_coinbase: false,
                is_confirmed: true,
                is_instantlocked: false,
                is_locked: false,
                is_trusted: false,
            }],
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };

        apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .unwrap();

        let funds = wallet_info
            .accounts
            .all_funding_accounts()
            .into_iter()
            .next()
            .unwrap();
        let pools = funds.managed_account_type().address_pools();
        let external = pools.iter().find(|p| p.is_external()).unwrap();
        let info = external
            .address_info(&in_window)
            .expect("in-window address must be present in the pool");
        assert!(
            info.is_used(),
            "in-window restored UTXO address must be marked used"
        );
        assert!(
            external.used_indices.contains(&3),
            "used_indices must record the in-window slot"
        );
        assert_eq!(
            external.highest_used,
            Some(3),
            "highest_used must reflect the in-window slot"
        );
    }

    /// Privacy / address-reuse: a previously-used address whose UTXO was
    /// SINCE SPENT must still come back marked `used` when the caller passes
    /// it via `used_pool_addresses`.
    /// Without it the address resets to `used = false` and could be handed
    /// out again as a fresh receive address. The used flag must survive even
    /// though the UTXO is gone (`spent_utxos` cancels `new_utxos` → zero
    /// balance), proving it is NOT just a side effect of a live UTXO. Covers
    /// an in-window slot (idx 5) and a deeper slot the horizon walk resolves
    /// (idx 30), and asserts the empty-snapshot baseline does NOT mark them.
    #[test]
    fn rehydration_used_state_survives_spent_utxo() {
        use dashcore::blockdata::transaction::txout::TxOut;
        use dashcore::{OutPoint, Txid};
        use key_wallet::bip32::DerivationPath;
        use key_wallet::gap_limit::DEFAULT_EXTERNAL_GAP_LIMIT;
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::{Address, Utxo};
        use platform_wallet::changeset::CoreChangeSet;

        let wallet = Wallet::from_seed_bytes(
            [42u8; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&wallet);

        let funds_type = ManagedWalletInfo::from_wallet(&wallet, 1)
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

        let derive = |index: u32| -> Address {
            let mut p = AddressPool::new_without_generation(
                DerivationPath::master(),
                AddressPoolType::External,
                DEFAULT_EXTERNAL_GAP_LIMIT,
                Network::Testnet,
            );
            p.generate_addresses(index + 1, &KeySource::Public(xpub), true)
                .unwrap();
            p.address_at_index(index).unwrap()
        };
        let in_window_used = derive(5);
        let deep_used = derive(30);

        // The in-window address received funds (new_utxos) that were later
        // spent (spent_utxos) — so it carries NO unspent UTXO. Exactly the
        // reuse hazard: zero balance, yet the address must stay `used`.
        let spent = Utxo {
            outpoint: OutPoint {
                txid: Txid::from([1u8; 32]),
                vout: 0,
            },
            txout: TxOut {
                value: 50_000,
                script_pubkey: in_window_used.script_pubkey(),
            },
            address: in_window_used.clone(),
            height: 1,
            is_coinbase: false,
            is_confirmed: true,
            is_instantlocked: false,
            is_locked: false,
            is_trusted: false,
        };
        let core = CoreChangeSet {
            new_utxos: vec![spent.clone()],
            spent_utxos: vec![spent],
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };

        // Pool used-state carried for both addresses (the reuse guard the
        // SQLite persister feeds via `core_state::load_used_addresses`). Single
        // funds account, so a `None` owner routes to it.
        let used_core_addresses: std::collections::HashMap<Address, Option<OwningAccount>> =
            [in_window_used.clone(), deep_used.clone()]
                .into_iter()
                .map(|a| (a, None))
                .collect();

        // Baseline: drop the pool used-state (empty) — the spent-out address
        // resets to unused (the pre-fix behaviour, and the reuse hazard).
        {
            let mut baseline = ManagedWalletInfo::from_wallet(&wallet, 1);
            apply_persisted_core_state(
                &mut baseline,
                &manifest,
                &core,
                &Default::default(),
                &Default::default(),
                &LoadCtx::strict(),
            )
            .unwrap();
            let funds = baseline
                .accounts
                .all_funding_accounts()
                .into_iter()
                .next()
                .unwrap();
            let pools = funds.managed_account_type().address_pools();
            let external = pools.iter().find(|p| p.is_external()).unwrap();
            assert!(
                !external
                    .address_info(&in_window_used)
                    .map(|i| i.is_used())
                    .unwrap_or(false),
                "without pool used-state a spent-out address resets to unused"
            );
        }

        // With the persisted used-state passed as `used_pool_addresses`, both
        // come back used.
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 1);
        apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &used_core_addresses,
            &LoadCtx::strict(),
        )
        .unwrap();

        // The spent UTXO contributes no balance — the used flag is NOT a
        // side effect of a live UTXO.
        assert_eq!(
            wallet_info.balance.total(),
            0,
            "the spent UTXO must not contribute balance"
        );

        let funds = wallet_info
            .accounts
            .all_funding_accounts()
            .into_iter()
            .next()
            .unwrap();
        let pools = funds.managed_account_type().address_pools();
        let external = pools.iter().find(|p| p.is_external()).unwrap();
        assert!(
            external
                .address_info(&in_window_used)
                .expect("in-window used address present")
                .is_used(),
            "in-window spent-out address must be restored as used"
        );
        assert!(external.used_indices.contains(&5), "idx 5 recorded used");
        assert!(
            external
                .address_info(&deep_used)
                .expect("deep used address derived into pool")
                .is_used(),
            "deep spent-out address must be derived + restored as used"
        );
        assert!(external.used_indices.contains(&30), "idx 30 recorded used");
        assert_eq!(
            external.highest_used,
            Some(30),
            "highest_used must reflect the deepest restored used slot"
        );
    }

    /// Regression (mark↔refill fixpoint): a previously-used address in the
    /// "wedge zone" — past the discovery horizon but within reach of the
    /// gap refill — must come back `used`. With used addresses at idx 20
    /// (in the eager window) and idx 45 (gap 30): the discovery walk
    /// excludes in-window addresses from `unresolved`, so nothing anchors
    /// the horizon past 30 and idx 45 is never scanned; marking idx 20 then
    /// makes `maintain_gap_limit` derive out to 20+30=50, which brings the
    /// idx-45 address into the pool. A single mark-then-refill pass left it
    /// there with `used = false` — pool-visible as a FRESH address, handed
    /// out again, and its stale `used = false` persisted back over the
    /// store's `is_used = true` on the next pool snapshot. The fixpoint
    /// re-marks after every refill until nothing new resolves.
    #[test]
    fn rehydration_wedge_zone_used_address_marked_after_refill() {
        use key_wallet::bip32::DerivationPath;
        use key_wallet::gap_limit::DEFAULT_EXTERNAL_GAP_LIMIT;
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::Address;

        let wallet = Wallet::from_seed_bytes(
            [61u8; 64],
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

        let derive = |index: u32| -> Address {
            let mut p = AddressPool::new_without_generation(
                DerivationPath::master(),
                AddressPoolType::External,
                DEFAULT_EXTERNAL_GAP_LIMIT,
                Network::Testnet,
            );
            p.generate_addresses(index + 1, &KeySource::Public(xpub), true)
                .unwrap();
            p.address_at_index(index).unwrap()
        };
        // Reachable multi-device state: this device saw idx 20 used;
        // another device (same mnemonic) handed out and used idx 45.
        let in_window_used = derive(20);
        let wedge_used = derive(45);

        // No UTXOs at all — only the persisted pool used-state. Single funds
        // account, so a `None` owner routes to it.
        let core = platform_wallet::changeset::CoreChangeSet {
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };
        let used: std::collections::HashMap<Address, Option<OwningAccount>> =
            [in_window_used.clone(), wedge_used.clone()]
                .into_iter()
                .map(|a| (a, None))
                .collect();
        apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &used,
            &LoadCtx::strict(),
        )
        .unwrap();

        let funds = wallet_info
            .accounts
            .all_funding_accounts()
            .into_iter()
            .next()
            .unwrap();
        let pools = funds.managed_account_type().address_pools();
        let external = pools.iter().find(|p| p.is_external()).unwrap();
        assert!(
            external
                .address_info(&in_window_used)
                .expect("in-window used address present")
                .is_used(),
            "in-window used address must be restored as used"
        );
        let wedge_info = external
            .address_info(&wedge_used)
            .expect("wedge-zone address must be derived into the pool by the refill");
        assert!(
            wedge_info.is_used(),
            "wedge-zone previously-used address must be re-marked used, \
             not left pool-visible as fresh"
        );
        assert!(external.used_indices.contains(&45), "idx 45 recorded used");
        assert_eq!(
            external.highest_used,
            Some(45),
            "highest_used must reflect the wedge-zone slot"
        );
        // And the window is refilled past the re-marked slot.
        assert!(
            external.highest_generated >= Some(45 + DEFAULT_EXTERNAL_GAP_LIMIT),
            "gap window must extend past the re-marked wedge slot (got {:?})",
            external.highest_generated,
        );
    }

    /// Documented limitation (solution b): a legitimately-owned but
    /// deep-and-sparse UTXO — external idx 45 with nothing unspent at idx
    /// <= 30 — is left unresolved because the discovery horizon (gap_limit
    /// past the deepest match) never advances far enough to reach it. The
    /// wallet total stays exact; only the per-address view is incomplete
    /// until the next sync (a `tracing::warn!` records the deferral).
    #[test]
    fn rehydration_deep_sparse_utxo_left_unresolved_total_exact() {
        use dashcore::blockdata::transaction::txout::TxOut;
        use dashcore::{OutPoint, Txid};
        use key_wallet::bip32::DerivationPath;
        use key_wallet::gap_limit::DEFAULT_EXTERNAL_GAP_LIMIT;
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::{Address, Utxo};
        use std::collections::HashSet;

        let wallet = Wallet::from_seed_bytes(
            [21u8; 64],
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

        // External idx 45 — past the eager window AND past the initial scan
        // window (horizon = gap_limit = 30 with no nearer match to extend it).
        let sparse_deep: Address = {
            let mut p = AddressPool::new_without_generation(
                DerivationPath::master(),
                AddressPoolType::External,
                DEFAULT_EXTERNAL_GAP_LIMIT,
                Network::Testnet,
            );
            p.generate_addresses(46, &KeySource::Public(xpub), true)
                .unwrap();
            p.address_at_index(45).unwrap()
        };

        let value = 500_000u64;
        let core = platform_wallet::changeset::CoreChangeSet {
            new_utxos: vec![Utxo {
                outpoint: OutPoint {
                    txid: Txid::from([4u8; 32]),
                    vout: 0,
                },
                txout: TxOut {
                    value,
                    script_pubkey: sparse_deep.script_pubkey(),
                },
                address: sparse_deep.clone(),
                height: 1,
                is_coinbase: false,
                is_confirmed: true,
                is_instantlocked: false,
                is_locked: false,
                is_trusted: false,
            }],
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };

        apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .unwrap();

        // The wallet total is exact regardless (a sum over the UTXO set).
        assert_eq!(wallet_info.balance.total(), value);

        let funds = wallet_info
            .accounts
            .all_funding_accounts()
            .into_iter()
            .next()
            .unwrap();
        let pools = funds.managed_account_type().address_pools();
        let external = pools.iter().find(|p| p.is_external()).unwrap();
        assert!(
            !external.contains_address(&sparse_deep),
            "deep-sparse idx 45 must be left unresolved (absent from the pool)"
        );

        // Per-address view: the deep-sparse UTXO is not pool-visible yet.
        let pool_addresses: HashSet<Address> = pools
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
            visible, 0,
            "the deep-sparse UTXO is deferred — not pool-visible until next sync"
        );
        assert!(visible < value, "per-address visible < exact total");
    }

    /// Topology guard: a wallet with persisted UTXOs but NO funds-bearing
    /// account cannot hold them — fail closed with
    /// `RehydrationTopologyUnsupported` (reporting the persisted count) rather
    /// than reconstruct a silent zero balance.
    #[test]
    fn rehydration_utxos_without_funds_account_errors() {
        use dashcore::address::Payload;
        use dashcore::blockdata::transaction::txout::TxOut;
        use dashcore::hashes::Hash;
        use dashcore::{OutPoint, PubkeyHash, Txid};
        use key_wallet::account::AccountType;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::{Address, Utxo};
        use std::collections::BTreeSet;

        // Keys-only wallet: a single IdentityRegistration account, no funds.
        let opts = WalletAccountCreationOptions::SpecificAccounts(
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            Some(vec![AccountType::IdentityRegistration]),
        );
        let wallet = Wallet::from_seed_bytes([23u8; 64], Network::Testnet, opts).unwrap();
        let manifest = manifest_for(&wallet);
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 1);
        assert!(
            wallet_info.accounts.all_funding_accounts().is_empty(),
            "fixture must have NO funds-bearing account"
        );

        let addr = Address::new(
            Network::Testnet,
            Payload::PubkeyHash(PubkeyHash::from_byte_array([9u8; 20])),
        );
        let core = platform_wallet::changeset::CoreChangeSet {
            new_utxos: vec![Utxo {
                outpoint: OutPoint {
                    txid: Txid::from([2u8; 32]),
                    vout: 0,
                },
                txout: TxOut {
                    value: 800_000,
                    script_pubkey: addr.script_pubkey(),
                },
                address: addr,
                height: 1,
                is_coinbase: false,
                is_confirmed: true,
                is_instantlocked: false,
                is_locked: false,
                is_trusted: false,
            }],
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };

        let err = apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .expect_err("must fail closed when no funds account can hold the UTXOs");
        match err {
            WalletStorageError::MissingAccount { wallet_id: id } => {
                assert_eq!(
                    id, wallet_info.wallet_id,
                    "wallet_id must match the rehydrated wallet"
                );
            }
            other => panic!("expected MissingAccount, got {other:?}"),
        }
    }

    /// Companion to the topology guard: the same keys-only wallet with an
    /// EMPTY persisted UTXO set is `Ok` — there is nothing to hold, so the
    /// guard does not trip.
    #[test]
    fn rehydration_no_funds_account_empty_utxos_ok() {
        use key_wallet::account::AccountType;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use std::collections::BTreeSet;

        let opts = WalletAccountCreationOptions::SpecificAccounts(
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            Some(vec![AccountType::IdentityRegistration]),
        );
        let wallet = Wallet::from_seed_bytes([24u8; 64], Network::Testnet, opts).unwrap();
        let manifest = manifest_for(&wallet);
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 1);
        assert!(wallet_info.accounts.all_funding_accounts().is_empty());

        let core = platform_wallet::changeset::CoreChangeSet {
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };
        apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .expect("empty UTXO set must be Ok even with no funds account");
    }

    /// Regression: a `last_applied_chain_lock` carried in the persisted
    /// `CoreChangeSet` must be restored onto the rehydrated wallet
    /// metadata. Without it, the asset-lock-resume CL-from-metadata
    /// fallback (`proof.rs`) cannot fire at app launch and a pre-restart
    /// chain-locked asset lock can't produce a proof until SPV re-applies
    /// a fresh chainlock. Fails (`None != Some`) if the apply step drops it.
    #[test]
    fn rehydration_restores_last_applied_chain_lock() {
        use dashcore::ephemerealdata::chain_lock::ChainLock;
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

        let wallet = Wallet::from_seed_bytes(
            [5u8; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&wallet);
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 1);
        assert!(
            wallet_info.metadata.last_applied_chain_lock.is_none(),
            "fresh watch-only skeleton starts with no chain lock"
        );

        let cl = ChainLock {
            block_height: 123_456,
            block_hash: BlockHash::from_byte_array([7u8; 32]),
            signature: [9u8; 96].into(),
        };
        let core = platform_wallet::changeset::CoreChangeSet {
            last_applied_chain_lock: Some(cl.clone()),
            ..Default::default()
        };

        apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .unwrap();

        assert_eq!(
            wallet_info.metadata.last_applied_chain_lock.as_ref(),
            Some(&cl),
            "persisted last_applied_chain_lock must be restored onto wallet metadata"
        );
    }
}
