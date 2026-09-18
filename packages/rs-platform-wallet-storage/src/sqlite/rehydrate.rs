//! External-signable wallet reconstruction
//!
//! Load is seedless — each wallet is rebuilt watch-only from its manifest and
//! the manager consumes the carried snapshot directly, so no wrong-seed check
//! runs here; that gate lives in the resolver-backed signing entrypoints.

use key_wallet::account::account_collection::AccountCollection;
use key_wallet::account::{Account, AccountType};
use key_wallet::managed_account::address_pool::{AddressPoolType, PublicKeyType};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Network;

use platform_wallet::changeset::provider_key_account::{
    rebuild_provider_key_account, ProviderAccountRebuildError,
};
use platform_wallet::changeset::{AccountRegistrationEntry, ProviderKeyExtendedPubKey};

use crate::sqlite::provider_accounts::{insert_platform_node_pool_entry, PlatformNodePoolError};

use crate::sqlite::load_ctx::{LoadCtx, LoadSite, SiteCoords};
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
    // Provider accounts use separate curve-specific slots in the collection.
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
    ctx: &LoadCtx,
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

    for (index, script_bytes, public_key, used) in entries {
        let PublicKeyType::EdDSA(public_key) = public_key else {
            return Err(WalletStorageError::blob_decode(
                "provider platform pool row does not carry an EdDSA public key",
            ));
        };
        let public_key: [u8; 32] = public_key.try_into().map_err(|_| {
            WalletStorageError::blob_decode(
                "provider platform pool row has the wrong EdDSA public-key length",
            )
        })?;
        let script_pubkey = dashcore::ScriptBuf::from_bytes(script_bytes);
        // Same condition `core_pool`/`core_state` tolerate as
        // `LoadSite::UndecodableAddressScript`, so it must be tolerable here
        // too — otherwise the crate tolerates an undecodable script in two
        // readers and aborts on it in a third. Doubly worth it here: this
        // error `?`-propagates out of `load()`'s per-wallet loop, so one
        // damaged wallet would abort the load of EVERY wallet in the file, and
        // the data at stake is a re-derivable cache of pre-derived
        // platform-node public keys — no funds, no identity, no address-reuse
        // guard. If anything in this crate should degrade rather than fail,
        // it is this.
        let address = match dashcore::Address::from_script(&script_pubkey, network) {
            Ok(address) => address,
            Err(e) => {
                ctx.tolerate_at(
                    LoadSite::UndecodableAddressScript,
                    SiteCoords {
                        wallet_id: Some(*wallet_id),
                        account_type: &AccountType::ProviderPlatformKeys,
                        affected: 1,
                        detail: Some(&AddressPoolType::AbsentHardened),
                    },
                    WalletStorageError::from(e),
                )?;
                continue;
            }
        };
        if let Some(existing) = wallet_info
            .accounts
            .provider_platform_keys
            .as_ref()
            .and_then(|account| account.get_address_info(&address))
        {
            if existing.index != index
                || existing.script_pubkey != script_pubkey
                || existing.public_key != Some(PublicKeyType::EdDSA(public_key.to_vec()))
            {
                return Err(WalletStorageError::blob_decode(
                    "persisted platform-node key conflicts with wallet snapshot",
                ));
            }
            if used {
                if let Some(account) = wallet_info.accounts.provider_platform_keys.as_mut() {
                    account.mark_address_used(&address);
                }
            }
            continue;
        }
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
            PlatformNodePoolError::NoManagedAccount => {
                WalletStorageError::blob_decode("provider platform account is not managed")
            }
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

/// Restore all derivable Core address pools using their full account identity.
fn restore_indexed_pools(
    wallet_info: &mut ManagedWalletInfo,
    conn: &rusqlite::Connection,
    wallet_id: &[u8; 32],
    manifest: &AccountManifest,
) -> Result<(), WalletStorageError> {
    use key_wallet::managed_account::address_pool::KeySource;

    for mut account in wallet_info.accounts.all_accounts_mut() {
        let account_type = account.managed_account_type().to_account_type();
        if account_type == AccountType::ProviderPlatformKeys {
            continue;
        }
        let source = manifest
            .ecdsa
            .iter()
            .find(|entry| entry.account_type == account_type)
            .map(|entry| KeySource::Public(entry.account_xpub))
            .or_else(|| {
                manifest.provider.iter().find_map(|entry| {
                    if entry.account_type != account_type {
                        return None;
                    }
                    match &entry.extended_public_key {
                        ProviderKeyExtendedPubKey::Bls(key) => {
                            Some(KeySource::BLSPublic(key.clone()))
                        }
                        ProviderKeyExtendedPubKey::EdDSA(_) => None,
                    }
                })
            });
        let Some(source) = source else { continue };
        for pool in account.managed_account_type_mut().address_pools_mut() {
            let mut entries: Vec<_> =
                core_pool::load_typed_pool_entries(conn, wallet_id, &account_type, pool.pool_type)?
                    .into_iter()
                    .map(|(index, script, key, used)| (index, script, Some(key), used))
                    .collect();
            entries.extend(
                core_pool::load_untyped_pool_entries(
                    conn,
                    wallet_id,
                    &account_type,
                    pool.pool_type,
                )?
                .into_iter()
                .map(|(index, script, used)| (index, script, None, used)),
            );
            for (index, script, public_key, used) in entries {
                let address = restore_indexed_address(pool, &source, index).ok_or_else(|| {
                    WalletStorageError::blob_decode(
                        "persisted Core account address cannot be derived",
                    )
                })?;
                let info = pool.address_info(&address).ok_or_else(|| {
                    WalletStorageError::blob_decode(
                        "derived key-account address is missing from its pool",
                    )
                })?;
                if info.script_pubkey.as_bytes() != script
                    || public_key
                        .as_ref()
                        .is_some_and(|key| info.public_key.as_ref() != Some(key))
                {
                    return Err(WalletStorageError::blob_decode(
                        "persisted Core account address disagrees with its account key",
                    ));
                }
                if used {
                    pool.mark_used(&address);
                }
            }
        }
    }
    Ok(())
}

/// Restore persisted address pools without changing Core financial or sync state.
///
/// Works on a fresh wallet awaiting a rescan or a full wallet snapshot. Indexed
/// rows preserve unused addresses; used-address hints also cover historical coins
/// whose pool rows were never persisted. Failure leaves the supplied wallet intact.
pub fn restore_core_address_pools(
    wallet_info: &mut ManagedWalletInfo,
    conn: &rusqlite::Connection,
    wallet_id: &[u8; 32],
    manifest: &AccountManifest,
    used_addresses: &std::collections::HashMap<key_wallet::Address, Option<OwningAccount>>,
    ctx: &LoadCtx,
) -> Result<(), WalletStorageError> {
    let mut restored = wallet_info.clone();
    restore_indexed_pools(&mut restored, conn, wallet_id, manifest)?;
    let network = restored.network;
    restore_provider_platform_node_pool(&mut restored, conn, wallet_id, network, ctx)?;
    restore_used_addresses(&mut restored, &manifest.ecdsa, used_addresses, ctx)?;
    restore_address_reservations(&mut restored, conn, wallet_id)?;
    *wallet_info = restored;
    Ok(())
}

fn restore_used_addresses(
    wallet_info: &mut ManagedWalletInfo,
    manifest: &[AccountRegistrationEntry],
    used_addresses: &std::collections::HashMap<key_wallet::Address, Option<OwningAccount>>,
    ctx: &LoadCtx,
) -> Result<(), WalletStorageError> {
    let wallet_id = wallet_info.wallet_id;
    let mut funding = wallet_info.accounts.all_funding_accounts_mut();
    if funding.is_empty() {
        return Ok(());
    }
    let account_keys: Vec<_> = funding
        .iter()
        .map(|account| owning_account_of(account))
        .collect();
    let mut per_account = vec![Vec::new(); funding.len()];
    let mut orphaned_owners = Vec::new();
    for (address, owner) in used_addresses {
        let target = route_to_funds_account(&account_keys, owner.as_ref(), &mut orphaned_owners);
        per_account[target].push(address.clone());
    }
    if !orphaned_owners.is_empty() {
        ctx.note_degraded(
            LoadSite::OrphanedUtxoOwner,
            SiteCoords {
                wallet_id: Some(wallet_id),
                account_type: &orphaned_owners,
                affected: orphaned_owners.len(),
                detail: None,
            },
            "used addresses reference an account absent from this wallet's funding accounts",
        );
    }
    for (account, addresses) in funding.iter_mut().zip(per_account) {
        if !addresses.is_empty() {
            extend_pools_for_restored_addresses(account, manifest, &addresses, wallet_id, ctx)?;
        }
    }
    Ok(())
}

fn restore_address_reservations(
    wallet_info: &mut ManagedWalletInfo,
    conn: &rusqlite::Connection,
    wallet_id: &[u8; 32],
) -> Result<(), WalletStorageError> {
    use key_wallet::managed_account::address_pool::AddressState;
    for mut account in wallet_info.accounts.all_accounts_mut() {
        let account_type = account.managed_account_type().to_account_type();
        for pool in account.managed_account_type_mut().address_pools_mut() {
            for (index, reserved_at) in
                core_pool::load_pool_reservations(conn, wallet_id, &account_type, pool.pool_type)?
            {
                let Some(info) = pool.addresses.get_mut(&index) else {
                    continue;
                };
                if !info.is_used() {
                    info.state = AddressState::Reserved {
                        at: info
                            .reserved_at()
                            .map_or(reserved_at, |at| at.max(reserved_at)),
                    };
                }
            }
        }
    }
    Ok(())
}

/// Resolve an owning account to its position among `account_keys`, using the
/// first funds account as a candidate when no owner resolves. Unknown owners
/// are recorded for load diagnostics. Unspent coins must still pass address
/// ownership validation; provider-owned used-address hints can degrade.
fn route_to_funds_account(
    account_keys: &[OwningAccount],
    owner: Option<&OwningAccount>,
    orphaned_owners: &mut Vec<String>,
) -> usize {
    // An unresolved owner uses account 0 as a candidate; coin installation
    // separately verifies the account's address ownership.
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
/// address to its derivation index. Unspent coins that remain unresolved
/// fail ownership validation; used-address-only entries can be rediscovered
/// on a later sync.
const MAX_REHYDRATION_DERIVATION_INDEX: u32 = 10_000;

/// Soft threshold past which a single chain's discovery scan is treated as
/// abnormally deep and worth a `tracing::warn!`. Real funds chains anchor
/// well below this; reaching it means either a corrupt / foreign-heavy UTXO
/// set walking the horizon out, or an approach toward the hard
/// [`MAX_REHYDRATION_DERIVATION_INDEX`] ceiling — both worth surfacing.
const REHYDRATION_DEEP_SCAN_WARN_INDEX: u32 = 1_000;

/// Upper bound on the addresses one gap-limit refill may generate during
/// load. A generated address costs roughly 500 bytes — an `AddressInfo`
/// plus its three pool index entries, two of which clone the
/// `Address` / `ScriptBuf` — so this holds a single refill near 100 MB.
/// Legitimate refills span one gap window (tens of addresses); reaching
/// this cap needs `highest_used` far past `highest_generated`, which
/// `mark_used` — the only writer today — cannot produce. Defense in depth
/// against a future upstream invariant break, not a currently reachable path.
const MAX_REHYDRATION_GAP_REFILL: u32 = 250_000;

/// Highest BIP-32 non-hardened child index. A derivation target at or past
/// `2^31` names a hardened child, which no address pool can derive from a
/// public xpub — such a target is corruption, not a legitimately deep wallet.
const MAX_NORMAL_CHILD_INDEX: u32 = (1u32 << 31) - 1;

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
/// are counted and logged; unresolved unspent coins fail ownership validation.
/// Used-address-only entries can be rediscovered on sync. Every resolved address
/// the pools hold (in-window or
/// deep-resolved) is marked used so a funded or previously-used address is
/// never handed out as a fresh receive address.
///
/// Tested with Standard BIP44 topology (External + Internal pools) and
/// CoinJoin topology (single External pool). The per-chain probe loop has no
/// topology-specific branches, so the non-hardened single-pool type
/// (`Absent`) follows the same code path with a different relative derivation
/// path. `AbsentHardened` pools cannot be derived from a public xpub at all —
/// hardened child derivation needs the private key — so under watch-only
/// rehydration their addresses must already be present to restore their coins.
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

        // Used-address-only entries can remain unresolved; unspent coins must
        // still pass the ownership check when the snapshot is installed.
        if !unresolved.is_empty() {
            ctx.note_degraded(
                LoadSite::UnresolvedUtxoAddress,
                SiteCoords {
                    wallet_id: Some(wallet_id),
                    account_type: &account_type,
                    affected: unresolved.len(),
                    detail: None,
                },
                "restored addresses did not resolve against this account's xpub; \
                 any unspent coins at these addresses require restored ownership",
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
        // NOTE(recovery-mode): defensive site with no reachable seed from
        // the storage layer — the probe resolved this index from the same
        // xpub, so the real pool derives it too. Kept fail-closed because a
        // deferred address is an address that can be re-issued as fresh.
        if let Some(deepest) = *deepest_resolved {
            if let Some(key_source) = key_source.as_ref() {
                if ensure_derived(pool, key_source, deepest).is_none() {
                    ctx.tolerate_at(
                        LoadSite::RehydrationEnsureDerived,
                        SiteCoords {
                            wallet_id: Some(wallet_id),
                            account_type: &account_type,
                            affected: 1,
                            detail: Some(&pool.pool_type),
                        },
                        WalletStorageError::RehydrationEnsureDerivedFailed { index: deepest },
                    )?;
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
            // Bound the refill before it runs, so nothing is allocated on
            // the way out. Validity first: an unrepresentable target has no
            // meaningful cost to weigh against the cap, and upstream panics
            // computing it (debug) or wraps it into a bogus one (release).
            let refill = match ImpliedRefill::of(
                pool.highest_used,
                pool.highest_generated,
                pool.gap_limit,
            ) {
                Ok(refill) => refill,
                Err(error) => {
                    ctx.tolerate_at(
                        LoadSite::RehydrationMaintainGapLimit,
                        SiteCoords {
                            wallet_id: Some(wallet_id),
                            account_type: &account_type,
                            affected: 1,
                            detail: Some(&pool.pool_type),
                        },
                        error,
                    )?;
                    break;
                }
            };
            if refill.implied > MAX_REHYDRATION_GAP_REFILL {
                ctx.tolerate_at(
                    LoadSite::RehydrationGapLimit,
                    SiteCoords {
                        wallet_id: Some(wallet_id),
                        account_type: &account_type,
                        affected: 1,
                        detail: Some(&pool.pool_type),
                    },
                    WalletStorageError::RehydrationGapLimitRefillTooLarge {
                        refill_target: refill.target,
                        already_generated: refill.already_generated,
                        implied: refill.implied,
                        cap: MAX_REHYDRATION_GAP_REFILL,
                    },
                )?;
                break;
            }

            // Defense in depth against an upstream derivation failure. The
            // pre-flight above rejects every target this crate can compute as
            // out of range, so no fixture currently reaches this branch.
            //
            // Kept fail-closed like `ensure_derived` above: a short window
            // means a previously-used address can be re-issued as fresh.
            if let Err(e) = pool.maintain_gap_limit(key_source) {
                ctx.tolerate_at(
                    LoadSite::RehydrationMaintainGapLimit,
                    SiteCoords {
                        wallet_id: Some(wallet_id),
                        account_type: &account_type,
                        affected: 1,
                        detail: Some(&pool.pool_type),
                    },
                    WalletStorageError::RehydrationGapLimitFailed { source: e },
                )?;
                break;
            }
        }
    }
    Ok(())
}

/// What a `maintain_gap_limit` call on a pool would cost, costed before it
/// runs.
///
/// Mirrors the target upstream computes internally (key-wallet rev
/// 393b612) — a private formula, so an upstream change would silently
/// mis-estimate here until `AddressPool` exposes a read-only
/// `refill_target()` (upstream work, out of scope). Upstream computes that
/// target with raw arithmetic — a panic with debug assertions on, a wrapped
/// target without — so the same formula is applied checked here; the cost
/// derived from it still saturates, because over-estimating fails closed.
#[derive(Debug)]
struct ImpliedRefill {
    /// Highest derivation index the refill would have to reach.
    target: u32,
    /// Addresses the pool already holds.
    already_generated: u32,
    /// Addresses the refill would derive.
    implied: u32,
}

impl ImpliedRefill {
    /// # Errors
    ///
    /// [`WalletStorageError::RehydrationGapLimitTargetOutOfRange`] when the
    /// target over- or underflows `u32`, or names a hardened child index.
    fn of(
        highest_used: Option<u32>,
        highest_generated: Option<u32>,
        gap_limit: u32,
    ) -> Result<Self, WalletStorageError> {
        let target = match highest_used {
            None => gap_limit.checked_sub(1),
            Some(highest) => highest.checked_add(gap_limit),
        }
        .filter(|target| *target <= MAX_NORMAL_CHILD_INDEX)
        .ok_or(WalletStorageError::RehydrationGapLimitTargetOutOfRange {
            highest_used,
            gap_limit,
        })?;
        // Both fields are INDICES: a pool with nothing generated holds no
        // addresses, and one generated through index 0 holds one. Counting
        // `None` as index 0 would under-count the work by one and report a
        // generated address that does not exist.
        let already_generated = highest_generated.map_or(0, |highest| highest.saturating_add(1));
        Ok(Self {
            target,
            already_generated,
            implied: target.saturating_add(1).saturating_sub(already_generated),
        })
    }
}

/// Verify one saved index without deriving an attacker-controlled gap before it.
fn restore_indexed_address(
    pool: &mut key_wallet::managed_account::address_pool::AddressPool,
    key_source: &key_wallet::managed_account::address_pool::KeySource,
    index: u32,
) -> Option<key_wallet::Address> {
    use key_wallet::managed_account::address_pool::AddressPool;
    if index > MAX_NORMAL_CHILD_INDEX {
        return None;
    }
    if let Some(address) = pool.address_at_index(index) {
        return Some(address);
    }
    let mut probe = AddressPool::new_without_generation(
        pool.base_path.clone(),
        pool.pool_type,
        pool.gap_limit,
        pool.network,
    );
    probe.set_address_type(pool.address_type);
    // generate_addresses starts immediately after this cursor; only the saved index is derived.
    probe.highest_generated = index.checked_sub(1);
    probe.generate_addresses(1, key_source, true).ok()?;
    let info = probe.addresses.remove(&index)?;
    let address = info.address.clone();
    pool.address_index.insert(address.clone(), index);
    pool.script_pubkey_index
        .insert(info.script_pubkey.clone(), index);
    pool.addresses.insert(index, info);
    pool.highest_generated = Some(
        pool.highest_generated
            .map_or(index, |highest| highest.max(index)),
    );
    Some(address)
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
    restore_indexed_address(pool, key_source, index)
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

    fn provider_keys(
        w: &Wallet,
    ) -> (
        key_wallet::derivation_bls_bip32::ExtendedBLSPubKey,
        key_wallet::derivation_slip10::ExtendedEd25519PubKey,
    ) {
        let bls = w
            .accounts
            .bls_account_of_type(AccountType::ProviderOperatorKeys)
            .expect("Default-created wallet has a BLS provider account")
            .bls_public_key
            .clone();
        let eddsa = w
            .accounts
            .eddsa_account_of_type(AccountType::ProviderPlatformKeys)
            .expect("Default-created wallet has an EdDSA provider account")
            .ed25519_public_key
            .clone();
        (bls, eddsa)
    }

    #[test]
    fn watch_only_rebuild_restores_provider_key_accounts() {
        use platform_wallet::changeset::{ProviderKeyAccountEntry, ProviderKeyExtendedPubKey};

        let w = Wallet::from_seed_bytes(
            [3u8; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let id = w.compute_wallet_id();
        let (bls, eddsa) = provider_keys(&w);
        let manifest = AccountManifest {
            ecdsa: manifest_for(&w),
            provider: vec![
                ProviderKeyAccountEntry {
                    account_type: AccountType::ProviderOperatorKeys,
                    extended_public_key: ProviderKeyExtendedPubKey::Bls(bls.clone()),
                },
                ProviderKeyAccountEntry {
                    account_type: AccountType::ProviderPlatformKeys,
                    extended_public_key: ProviderKeyExtendedPubKey::EdDSA(eddsa.clone()),
                },
            ],
        };

        let restored = build_wallet(Network::Testnet, id, &manifest).unwrap();

        let restored_bls = restored
            .accounts
            .bls_account_of_type(AccountType::ProviderOperatorKeys)
            .expect("BLS provider account must be rebuilt");
        assert_eq!(restored_bls.bls_public_key.to_bytes(), bls.to_bytes());
        assert_eq!(restored_bls.parent_wallet_id.as_deref(), Some(&id[..]));
        assert!(restored_bls.is_watch_only);
        let restored_eddsa = restored
            .accounts
            .eddsa_account_of_type(AccountType::ProviderPlatformKeys)
            .expect("EdDSA provider account must be rebuilt");
        assert_eq!(restored_eddsa.ed25519_public_key, eddsa);
        assert_eq!(restored_eddsa.parent_wallet_id.as_deref(), Some(&id[..]));
        assert!(restored_eddsa.is_watch_only);
    }

    #[test]
    fn watch_only_rebuild_rejects_provider_curve_type_mismatch() {
        use platform_wallet::changeset::{ProviderKeyAccountEntry, ProviderKeyExtendedPubKey};

        let w = Wallet::from_seed_bytes(
            [3u8; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let (bls, _) = provider_keys(&w);
        let manifest = AccountManifest {
            ecdsa: manifest_for(&w),
            provider: vec![ProviderKeyAccountEntry {
                account_type: AccountType::ProviderPlatformKeys,
                extended_public_key: ProviderKeyExtendedPubKey::Bls(bls),
            }],
        };

        let err = build_wallet(Network::Testnet, w.compute_wallet_id(), &manifest)
            .expect_err("a BLS key must not rebuild as the platform-node account");
        assert!(matches!(
            err,
            WalletStorageError::ProviderKeyAccountEntryMismatch
        ));
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
    /// A `Default` watch-only wallet with its first funds account's external
    /// pool high-water marks overwritten (both fields are `pub` upstream), as
    /// a pool whose persisted state implies an oversized refill would look.
    struct GapRefillFixture {
        wallet_info: key_wallet::wallet::managed_wallet_info::ManagedWalletInfo,
        manifest: Vec<AccountRegistrationEntry>,
        /// External index 0 — inside the eager window, so it marks used
        /// without any discovery derivation and drives the mark/refill
        /// fixpoint straight into the guard.
        marked: key_wallet::Address,
        generated_before: Option<u32>,
        gap_limit: u32,
    }

    fn gap_refill_fixture(
        seed: u8,
        highest_used: u32,
        highest_generated: Option<u32>,
    ) -> GapRefillFixture {
        use key_wallet::managed_account::address_pool::AddressPool;
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

        let wallet = Wallet::from_seed_bytes(
            [seed; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&wallet);
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 1);
        let marked = first_external_address(&wallet_info, &manifest);

        let (generated_before, gap_limit) = {
            let mut funding = wallet_info.accounts.all_funding_accounts_mut();
            let account = funding.first_mut().expect("a funds account");
            let mut pools = account.managed_account_type_mut().address_pools_mut();
            let pool: &mut AddressPool = pools
                .iter_mut()
                .find(|p| p.is_external())
                .expect("an external pool");
            pool.highest_used = Some(highest_used);
            if let Some(generated) = highest_generated {
                pool.highest_generated = Some(generated);
            }
            (pool.highest_generated, pool.gap_limit)
        };

        GapRefillFixture {
            wallet_info,
            manifest,
            marked,
            generated_before,
            gap_limit,
        }
    }

    fn ensure_derived_failure_fixture(seed: u8) -> GapRefillFixture {
        use key_wallet::managed_account::address_pool::{AddressPool, KeySource};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

        let wallet = Wallet::from_seed_bytes(
            [seed; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&wallet);
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 1);

        let (marked, generated_before, gap_limit) = {
            let mut funding = wallet_info.accounts.all_funding_accounts_mut();
            let account = funding.first_mut().expect("a funds account");
            let account_type = account.managed_account_type().to_account_type();
            let key_source = KeySource::Public(
                manifest
                    .iter()
                    .find(|entry| entry.account_type == account_type)
                    .expect("manifest entry")
                    .account_xpub,
            );
            let mut pools = account.managed_account_type_mut().address_pools_mut();
            let pool: &mut AddressPool = pools
                .iter_mut()
                .find(|pool| pool.is_external())
                .expect("an external pool");
            let mut probe = pool.clone();
            let marked = probe
                .generate_addresses(1, &key_source, true)
                .expect("derive the next address")
                .pop()
                .expect("one derived address");
            let missing_index = probe.highest_generated.expect("derived index");
            pool.highest_generated = Some(missing_index);
            (marked, pool.highest_generated, pool.gap_limit)
        };

        GapRefillFixture {
            wallet_info,
            manifest,
            marked,
            generated_before,
            gap_limit,
        }
    }

    /// A pool poised at the non-hardened child-index boundary (`2^31`): the
    /// implied cost stays tiny (a handful of addresses), so the size cap
    /// would wave it through, but the target it implies is deeper than any
    /// index derivable from a public xpub.
    fn gap_limit_boundary_fixture(seed: u8) -> GapRefillFixture {
        gap_refill_fixture(
            seed,
            MAX_NORMAL_CHILD_INDEX - 5,
            Some(MAX_NORMAL_CHILD_INDEX - 10),
        )
    }

    /// The first funds account's external pool.
    fn external_pool_state(
        wallet_info: &key_wallet::wallet::managed_wallet_info::ManagedWalletInfo,
    ) -> Option<u32> {
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        wallet_info
            .accounts
            .all_funding_accounts()
            .into_iter()
            .next()
            .expect("a funds account")
            .managed_account_type()
            .address_pools()
            .into_iter()
            .find(|p| p.is_external())
            .expect("an external pool")
            .highest_generated
    }

    fn rehydrate_fixture(
        fixture: &mut GapRefillFixture,
        ctx: &LoadCtx,
    ) -> Result<(), WalletStorageError> {
        let wallet_id = fixture.wallet_info.wallet_id;
        let restored = std::slice::from_ref(&fixture.marked);
        let manifest = fixture.manifest.clone();
        let mut funding = fixture.wallet_info.accounts.all_funding_accounts_mut();
        extend_pools_for_restored_addresses(funding[0], &manifest, restored, wallet_id, ctx)
    }

    /// A pool whose `highest_used` sits far past `highest_generated` implies a
    /// refill of that whole span. Rehydration must reject it on arithmetic
    /// alone rather than let `maintain_gap_limit` allocate its way to an OOM.
    #[test]
    fn gap_refill_over_cap_is_fatal_under_strict() {
        let mut fixture = gap_refill_fixture(21, MAX_REHYDRATION_GAP_REFILL + 1_000, None);
        let err = rehydrate_fixture(&mut fixture, &LoadCtx::strict())
            .expect_err("an over-cap refill must fail a strict load");

        assert!(
            matches!(
                err,
                WalletStorageError::RehydrationGapLimitRefillTooLarge {
                    implied,
                    cap: MAX_REHYDRATION_GAP_REFILL,
                    ..
                } if implied > MAX_REHYDRATION_GAP_REFILL
            ),
            "the cap must report the refill it refused and the cap it applied: {err:?}"
        );
    }

    /// Recovery defers the same pool instead of failing — and, the point of
    /// the pre-flight placement, generates nothing at all on the way out.
    #[test]
    fn gap_refill_over_cap_is_deferred_under_recovery() {
        let mut fixture = gap_refill_fixture(22, MAX_REHYDRATION_GAP_REFILL + 1_000, None);
        let generated_before = fixture.generated_before;
        let ctx = LoadCtx::recovery();
        rehydrate_fixture(&mut fixture, &ctx).expect("recovery must defer, not fail");

        let degradation = ctx.degradation();
        assert!(degradation.degraded);
        assert_eq!(
            degradation.by_site.get(&LoadSite::RehydrationGapLimit),
            Some(&1),
            "the deferred refill must be counted at its own site: {:?}",
            degradation.by_site
        );
        assert_eq!(
            external_pool_state(&fixture.wallet_info),
            generated_before,
            "a rejected refill must derive nothing"
        );
    }

    #[test]
    fn ensure_derived_failure_is_strictly_fatal_and_recovery_is_deferred() {
        let mut strict_fixture = ensure_derived_failure_fixture(24);
        let missing_index = strict_fixture.generated_before.expect("missing index");
        let err = rehydrate_fixture(&mut strict_fixture, &LoadCtx::strict())
            .expect_err("strict must reject the inconsistent pool high-water mark");
        assert!(matches!(
            err,
            WalletStorageError::RehydrationEnsureDerivedFailed { index }
                if index == missing_index
        ));

        let mut recovery_fixture = ensure_derived_failure_fixture(25);
        let generated_before = recovery_fixture.generated_before;
        let ctx = LoadCtx::recovery();
        rehydrate_fixture(&mut recovery_fixture, &ctx)
            .expect("recovery must defer the inconsistent pool");
        let degradation = ctx.degradation();
        assert_eq!(
            degradation.by_site.get(&LoadSite::RehydrationEnsureDerived),
            Some(&1)
        );
        assert_eq!(degradation.by_site.len(), 1);
        assert_eq!(
            external_pool_state(&recovery_fixture.wallet_info),
            generated_before
        );
    }

    /// A pool whose refill target crosses the `2^31` non-hardened
    /// child-index boundary clears the size cap — the implied span is a
    /// handful of addresses — so only the validity pre-flight refuses it.
    #[test]
    fn gap_refill_target_past_normal_child_boundary_is_fatal_under_strict() {
        let mut fixture = gap_limit_boundary_fixture(26);
        let err = rehydrate_fixture(&mut fixture, &LoadCtx::strict())
            .expect_err("an out-of-range refill target must fail a strict load");

        assert!(
            matches!(
                err,
                WalletStorageError::RehydrationGapLimitTargetOutOfRange {
                    highest_used: Some(highest),
                    ..
                } if highest == MAX_NORMAL_CHILD_INDEX - 5
            ),
            "the guard must name the pool state it refused: {err:?}"
        );
    }

    /// Recovery defers the same pool instead of failing, and — the point of
    /// refusing before the call rather than after it — derives nothing on
    /// the way out instead of walking up to the boundary first.
    #[test]
    fn gap_refill_target_past_normal_child_boundary_is_deferred_under_recovery() {
        let mut fixture = gap_limit_boundary_fixture(27);
        let generated_before = fixture.generated_before;
        let ctx = LoadCtx::recovery();
        rehydrate_fixture(&mut fixture, &ctx).expect("recovery must defer, not fail");

        let degradation = ctx.degradation();
        assert!(degradation.degraded);
        assert_eq!(
            degradation
                .by_site
                .get(&LoadSite::RehydrationMaintainGapLimit),
            Some(&1),
            "the deferred refill failure must be counted at its own site: {:?}",
            degradation.by_site
        );
        assert_eq!(
            external_pool_state(&fixture.wallet_info),
            generated_before,
            "a refused refill must derive nothing"
        );
    }

    /// A pool whose `highest_used` sits within one gap window of `u32::MAX`
    /// drives upstream's raw `highest + gap_limit` over the end of the type.
    /// The implied span is a handful of addresses, so the size cap passes it
    /// through; only a checked pre-flight stops it, and it must stop it as an
    /// error rather than the panic no load policy can catch.
    #[test]
    fn gap_refill_target_overflow_is_fatal_under_strict() {
        let mut fixture = gap_refill_fixture(28, u32::MAX - 5, Some(u32::MAX - 10));
        let err = rehydrate_fixture(&mut fixture, &LoadCtx::strict())
            .expect_err("an unrepresentable refill target must fail a strict load");

        assert!(
            matches!(
                err,
                WalletStorageError::RehydrationGapLimitTargetOutOfRange {
                    highest_used: Some(highest),
                    ..
                } if highest == u32::MAX - 5
            ),
            "the guard must name the pool state it refused: {err:?}"
        );
    }

    /// Recovery defers the same pool instead of failing — the contract that
    /// an upstream panic would have broken outright — and derives nothing.
    #[test]
    fn gap_refill_target_overflow_is_deferred_under_recovery() {
        let mut fixture = gap_refill_fixture(29, u32::MAX - 5, Some(u32::MAX - 10));
        let generated_before = fixture.generated_before;
        let ctx = LoadCtx::recovery();
        rehydrate_fixture(&mut fixture, &ctx).expect("recovery must defer, not fail");

        let degradation = ctx.degradation();
        assert!(degradation.degraded);
        assert_eq!(
            degradation
                .by_site
                .get(&LoadSite::RehydrationMaintainGapLimit),
            Some(&1),
            "the refused refill must be counted at the gap-maintenance site: {:?}",
            degradation.by_site
        );
        assert_eq!(
            external_pool_state(&fixture.wallet_info),
            generated_before,
            "a refused refill must derive nothing"
        );
    }

    /// `highest_generated` is an index, so `None` means the pool holds no
    /// addresses at all — not one at index 0. Costing it as index 0
    /// under-counts the work by one and names a generated address that does
    /// not exist, in a guard whose own contract is to over-estimate.
    #[test]
    fn nothing_generated_costs_the_whole_window() {
        let refill = ImpliedRefill::of(None, None, 20).expect("a representable target");
        assert_eq!(refill.target, 19, "indices 0..=19 must exist");
        assert_eq!(refill.already_generated, 0);
        assert_eq!(refill.implied, 20, "twenty addresses, not nineteen");
    }

    /// The boundary the previous arithmetic blurred: a pool generated
    /// through index 0 holds exactly one address, one more than an empty
    /// one, and the two must cost differently.
    #[test]
    fn a_pool_generated_through_index_zero_holds_one_address() {
        let empty = ImpliedRefill::of(None, None, 20).expect("a representable target");
        let one = ImpliedRefill::of(None, Some(0), 20).expect("a representable target");
        assert_eq!(one.already_generated, 1);
        assert_eq!(one.implied, 19);
        assert_eq!(
            empty.implied,
            one.implied + 1,
            "an empty pool must cost exactly one address more"
        );
    }

    /// A deep pool already generated up to its used index owes one gap
    /// window, whatever its absolute depth.
    #[test]
    fn a_deep_pool_owes_only_its_gap_window() {
        let refill =
            ImpliedRefill::of(Some(50_000), Some(49_990), 20).expect("a representable target");
        assert_eq!(refill.target, 50_020);
        assert_eq!(refill.already_generated, 49_991);
        assert_eq!(refill.implied, 30);
    }

    /// Every refill target upstream would compute with raw arithmetic, at the
    /// four boundaries where the raw form breaks: an empty pool with no gap
    /// window (upstream's `gap_limit - 1` underflow), a used index one gap
    /// window from the end of the type (its `highest + gap_limit` overflow),
    /// a target landing exactly on the first hardened index, and the deepest
    /// target that is still legal.
    #[test]
    fn unrepresentable_refill_targets_are_rejected() {
        let rejected = [
            (None, 0),
            (Some(u32::MAX), 1),
            (Some(MAX_NORMAL_CHILD_INDEX), 1),
        ];
        for (highest_used, gap_limit) in rejected {
            let err = ImpliedRefill::of(highest_used, None, gap_limit).expect_err(
                "a target that does not fit a non-hardened child index must be refused",
            );
            assert!(
                matches!(
                    err,
                    WalletStorageError::RehydrationGapLimitTargetOutOfRange {
                        highest_used: got_used,
                        gap_limit: got_gap,
                    } if got_used == highest_used && got_gap == gap_limit
                ),
                "the refusal must carry the inputs it refused: {err:?}"
            );
        }

        let deepest_legal = ImpliedRefill::of(Some(MAX_NORMAL_CHILD_INDEX - 1), None, 1)
            .expect("the last non-hardened index is a legal target");
        assert_eq!(deepest_legal.target, MAX_NORMAL_CHILD_INDEX);
    }

    /// The cap bounds the refill's *span*, not the depth it starts from: a
    /// legitimately deep pool that is already generated up to its used index
    /// implies one gap window of work and must refill normally.
    #[test]
    fn deep_but_shallow_span_gap_refill_is_not_capped() {
        let mut fixture = gap_refill_fixture(23, 50_000, Some(49_990));
        let expected = 50_000 + fixture.gap_limit;
        let ctx = LoadCtx::strict();
        rehydrate_fixture(&mut fixture, &ctx).expect("a one-window refill must not be capped");

        assert!(!ctx.degradation().degraded);
        assert_eq!(
            external_pool_state(&fixture.wallet_info),
            Some(expected),
            "the refill must reach one gap window past the used index"
        );
    }
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
        restore_used_addresses(&mut wallet_info, &manifest, &used, &LoadCtx::strict()).unwrap();

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

    /// A used address whose owning account is not one of this wallet's funds
    /// accounts — what a masternode-operator wallet looks like, since provider
    /// accounts sit on a non-secp256k1 curve and are not funds accounts at all.
    /// Degraded in every policy, fatal in none (dashpay/platform#3968).
    #[test]
    fn rehydration_orphaned_used_address_owner_is_degraded_not_fatal() {
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::Address;
        use std::collections::HashMap;

        let wallet = Wallet::from_seed_bytes(
            [13u8; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&wallet);
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 1);

        // An owner this wallet has no funds account for.
        let mut used: HashMap<Address, Option<OwningAccount>> = HashMap::new();
        used.insert(
            first_external_address(&wallet_info, &manifest),
            Some(OwningAccount {
                account_type: "provider_platform".to_string(),
                account_index: 0,
                user_identity_id: [0u8; 32],
                friend_identity_id: [0u8; 32],
            }),
        );

        let ctx = LoadCtx::strict();
        restore_used_addresses(&mut wallet_info, &manifest, &used, &ctx)
            .expect("an unroutable owner must never brick a strict load");

        let degradation = ctx.degradation();
        assert!(degradation.degraded);
        assert_eq!(
            degradation.by_site.get(&LoadSite::OrphanedUtxoOwner),
            Some(&1),
            "the unroutable owner must be counted: {:?}",
            degradation.by_site
        );
    }

    /// Two unroutable owners are two incidents. Every existing test at this
    /// site seeds exactly one address, which cannot tell "count the rows"
    /// apart from "count the times the reader decided to tolerate them" —
    /// and a rescue operator sizing the damage needs the former.
    #[test]
    fn rehydration_orphaned_used_address_owners_are_counted_per_address() {
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::Address;
        use std::collections::HashMap;

        let wallet = Wallet::from_seed_bytes(
            [14u8; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&wallet);
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 1);

        let orphan_owner = OwningAccount {
            account_type: "provider_platform".to_string(),
            account_index: 0,
            user_identity_id: [0u8; 32],
            friend_identity_id: [0u8; 32],
        };
        let mut used: HashMap<Address, Option<OwningAccount>> = HashMap::new();
        for index in 0..2 {
            used.insert(
                external_address_at(&wallet_info, &manifest, index),
                Some(orphan_owner.clone()),
            );
        }

        let ctx = LoadCtx::strict();
        restore_used_addresses(&mut wallet_info, &manifest, &used, &ctx)
            .expect("unroutable owners must never brick a strict load");

        assert_eq!(
            ctx.degradation().by_site.get(&LoadSite::OrphanedUtxoOwner),
            Some(&2),
            "both unroutable addresses must be counted"
        );
    }

    /// External index-0 address of the wallet's first funds account.
    fn first_external_address(
        wallet_info: &key_wallet::wallet::managed_wallet_info::ManagedWalletInfo,
        manifest: &[AccountRegistrationEntry],
    ) -> key_wallet::Address {
        external_address_at(wallet_info, manifest, 0)
    }

    /// External address at `index` of the wallet's first funds account.
    fn external_address_at(
        wallet_info: &key_wallet::wallet::managed_wallet_info::ManagedWalletInfo,
        manifest: &[AccountRegistrationEntry],
        index: u32,
    ) -> key_wallet::Address {
        use key_wallet::bip32::DerivationPath;
        use key_wallet::gap_limit::DEFAULT_EXTERNAL_GAP_LIMIT;
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

        let account_type = wallet_info
            .accounts
            .all_funding_accounts()
            .into_iter()
            .next()
            .expect("a funds account")
            .managed_account_type()
            .to_account_type();
        let xpub = manifest
            .iter()
            .find(|e| e.account_type == account_type)
            .map(|e| e.account_xpub)
            .expect("funds account xpub");
        let mut pool = AddressPool::new_without_generation(
            DerivationPath::master(),
            AddressPoolType::External,
            DEFAULT_EXTERNAL_GAP_LIMIT,
            Network::Testnet,
        );
        pool.generate_addresses(index + 1, &KeySource::Public(xpub), true)
            .unwrap();
        pool.address_at_index(index).unwrap()
    }

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
        let used: std::collections::HashMap<Address, Option<OwningAccount>> =
            [in_window_used.clone(), wedge_used.clone()]
                .into_iter()
                .map(|a| (a, None))
                .collect();
        restore_used_addresses(&mut wallet_info, &manifest, &used, &LoadCtx::strict()).unwrap();

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
}
