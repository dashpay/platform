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
use std::collections::HashSet;

use platform_wallet::changeset::provider_key_account::{
    rebuild_provider_key_account, ProviderAccountRebuildError,
};
use platform_wallet::changeset::{AccountRegistrationEntry, CoreChangeSet};

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
    let mut accounts = ecdsa_accounts(network, expected_wallet_id, &manifest.ecdsa)?;
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

/// The watch-only ECDSA accounts of `entries`, rebuilt from their xpubs.
fn ecdsa_accounts(
    network: Network,
    expected_wallet_id: [u8; 32],
    entries: &[AccountRegistrationEntry],
) -> Result<AccountCollection, WalletStorageError> {
    let mut accounts = AccountCollection::new();
    for entry in entries {
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
    Ok(accounts)
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

    // TODO(#4188): `reserved_at` is persisted but deliberately not consumed here;
    // restoring it requires widening `provider_accounts::insert_platform_node_pool_entry`.
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
///   `CoreChangeSet` cannot carry it. A UTXO goes to the account that
///   derives its address (below); the entry only pre-derives a deep address
///   in its account and places a UTXO no account reaches. One naming an
///   account this wallet's funds accounts lack is reported as store drift.
/// - `used_pool_addresses`: addresses the persisted pool snapshot marked
///   used, each mapped to its owning account (`None` when the script matched
///   no pool row). One with an owner among this wallet's funds accounts is
///   marked used there first; every one is then marked used in whichever
///   funds account derives it once the pools are extended, so a
///   previously-used address whose funds were since spent is never
///   re-handed-out as a fresh receive address (address-reuse guard), and
///   extends the reach of the UTXO restore as a rescan's would. Empty = no
///   pool used-state carried.
///
/// # Reconstructed (safety-critical-correct)
///
/// - **Wallet balance** (`wallet_info.balance`, the no-silent-zero
///   guarantee): every persisted UTXO is restored and the per-account
///   and wallet totals are recomputed via `update_balance()`. A UTXO
///   carrying a block height is marked confirmed so it lands in the
///   `confirmed` bucket; the wallet total is exact regardless.
/// - **UTXO set**: every unspent persisted outpoint is restored into the
///   funds account whose pools derive its address
///   (`ManagedWalletInfo::restore_utxos`, any topology — BIP44, BIP32,
///   CoinJoin, DashPay), each pool extended to its gap limit past its used
///   addresses as a rescan would, so per-account balance, coin selection,
///   and reservations are correct after restart, and a later rescan of the
///   coin's block credits the account that already holds it. An outpoint no
///   account reaches falls back to the first funds account, counted at
///   [`LoadSite::UnresolvedUtxoAddress`].
/// - **Address-pool depth**: each pool is forward-derived to cover
///   restored UTXOs at deep derivation indices, then the gap window is
///   refilled beyond the deepest restored index so the per-address view
///   reconciles with the wallet total.
/// - **Address-pool used-state**: every `used_pool_addresses` entry is
///   re-marked used (in union with the unspent-UTXO addresses), so an
///   address whose funds were since spent is not re-handed-out as fresh.
/// - **InstantSend locks**: every `instant_locks_for_non_final_records`
///   entry is replayed through `mark_instant_send_utxos` after the UTXO
///   restore, so instant-locked funds come back instant-locked instead of
///   waiting for the next sync to re-learn them.
/// - **Sync watermarks**: `synced_height` / `last_processed_height`.
///
/// # Reconstructed when the persister supplies it
///
/// - **`last_applied_chain_lock`**: restored from `core` on both backends
///   when the supplied [`CoreChangeSet`](platform_wallet::changeset::CoreChangeSet)
///   carries it, so the asset-lock-resume CL-from-metadata fallback
///   (`proof.rs`) fires at launch instead of waiting for SPV. The FFI/iOS
///   persister round-trips the value Swift held; the SQLite persister
///   reads it from `core_sync_state.last_applied_chain_lock` (present
///   since V001) via a monotonic height-max merge on write and
///   `decode_chain_lock` under the load policy on read. It stays `None`
///   only when the column is NULL or, under
///   [`LoadPolicy::Recovery`](crate::LoadPolicy), when the blob failed to
///   decode and was tolerated as [`LoadSite::ChainLockBlob`].
///
/// # Deferred to the first post-load `sync` (safe re-warm)
///
/// - **Deep-index address visibility**: a stored address no funds pool
///   holds after the marking rounds, a coin's or a used one's, whatever
///   owner its row names, is looked for in every funds account past each
///   pool: through one index past what the pool holds, and one past a
///   `gap_limit` beyond each address found, up to
///   [`MAX_REHYDRATION_DERIVATION_INDEX`]
///   ([`discover_in_funds_accounts`]); what that derives is marked, refilled
///   and matched again, the refills carrying on past that index from each
///   stored address within a gap of the last. An address can still be left
///   unresolved in two distinct cases: (1) it is genuinely foreign (another
///   wallet's key, or corrupt), and (2) it is a *legitimately-owned but
///   deep-and-sparse* address, past that reach with no stored address
///   nearer to it. Both are counted at [`LoadSite::UnresolvedUtxoAddress`]
///   and re-warm on the next full sync; such a coin falls back to its stored
///   owner, else the first funds account. The wallet *total* stays exact
///   (every UTXO is summed regardless of pool visibility); only the
///   per-address view is incomplete until that sync.
/// - **Per-UTXO `is_coinbase` / `is_trusted` flags**: not columns in
///   `core_utxos`; conservatively defaulted (non-coinbase,
///   confirmed-by-height) and refreshed on the next scan.
///   Coinbase-maturity nuance re-warms on sync. `is_instantlocked` is NOT
///   among them: it is rebuilt from `core_instant_locks` above, for every
///   UTXO a replayed lock covers.
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

    // INTENTIONAL(tx-record-rehydration-gap): `core` also carries transaction
    // records, but they cannot be replayed here — injecting one needs the raw
    // `dashcore::Transaction`, and this crate persists only the abstracted
    // `TransactionRecord` blob. History re-warms on the next scan and is not a
    // balance input.

    // Restore the UTXO set into the funds accounts that derive each unspent
    // outpoint's address (`restore_utxos`, below). The `utxo_accounts` side
    // channel (the account identity the writer keyed the pool row on) only
    // pre-derives a deep address in its account and places a UTXO no account
    // reaches. A side-channel owner not among this wallet's funds accounts — a
    // pool row referencing an unregistered account (store drift) — is counted
    // and surfaced via `tracing::warn!` after the loop so it is never silent.
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
    let account_keys: Vec<OwningAccount> = funding.iter().map(|a| owning_account_of(a)).collect();
    // Every stored used address, whatever owner its row names: marked
    // wherever the pools, once extended, derive it. None without a funds
    // account, which has no pool to mark one in.
    let mut used: Vec<key_wallet::Address> = Vec::new();
    // Pools whose gap refill was refused and reported, so a later check
    // refuses them without reporting them again.
    let mut refused = RefusedRefills::new();
    if !funding.is_empty() {
        // Per-account addresses to derive-and-mark-used: each account gets only
        // its own attributed addresses, so `extend_pools_for_restored_addresses`
        // never scans another account's keys as "unresolved".
        let mut per_account_addrs: Vec<Vec<key_wallet::Address>> = vec![Vec::new(); funding.len()];
        used.extend(used_pool_addresses.keys().cloned());

        // Owners that resolve to no funds account (store drift, above), plus
        // used addresses with an owner absent from this wallet — collected
        // across both loops and warned once after them, never per iteration.
        let mut orphaned_owners: Vec<String> = Vec::new();
        for utxo in &unspent {
            if let Some(owner) = utxo_accounts.get(&utxo.outpoint) {
                match account_keys.iter().position(|k| k == owner) {
                    Some(target) => per_account_addrs[target].push(utxo.address.clone()),
                    None => orphaned_owners.push(owner_label(owner)),
                }
            }
        }

        // The persisted pool used-state restores addresses whose funds were
        // since spent — without it a previously-used address comes back marked
        // unused and could be handed out again as a fresh receive address
        // (address-reuse privacy leak). One whose owner is a funds account is
        // first marked used on ITS OWN pool; every one is matched by
        // derivation below. An empty map marks only the unspent-UTXO
        // addresses.
        for (addr, owner) in used_pool_addresses {
            let Some(owner) = owner else {
                continue;
            };
            match account_keys.iter().position(|k| k == owner) {
                Some(target) => per_account_addrs[target].push(addr.clone()),
                None => orphaned_owners.push(owner_label(owner)),
            }
        }

        // Degraded in BOTH policies, never fatal. An owner absent from the
        // funds accounts is not necessarily corruption: provider accounts are
        // first-class in the schema yet sit on a non-secp256k1 curve, so they
        // are not `ManagedCoreFundsAccount` and a used provider-owned address
        // has no funds account to route to. Telling that apart needs an
        // upstream `key-wallet` enumerator over every account kind; until then,
        // failing here would brick a masternode-operator wallet.
        if !orphaned_owners.is_empty() {
            ctx.note_degraded(
                LoadSite::OrphanedUtxoOwner,
                SiteCoords {
                    wallet_id: Some(wallet_id),
                    account_type: &orphaned_owners,
                    affected: orphaned_owners.len(),
                    detail: None,
                },
                "restored UTXOs or used addresses name owning accounts that are not funds \
                 accounts of this wallet; they are placed in the funds account that derives \
                 their address, or the first one when none does",
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
                    &mut refused,
                )?;
            }
        }
    }

    if !unspent.is_empty() || !used.is_empty() {
        let wallet = Wallet::new_external_signable(
            wallet_info.network(),
            wallet_id,
            ecdsa_accounts(wallet_info.network(), wallet_id, manifest)?,
        );
        let (mut unmatched, restorable) = restore_owned_utxos(
            wallet_info,
            &wallet,
            &unspent,
            &used,
            wallet_id,
            ctx,
            &mut refused,
        )?;
        // Addresses still outside every funds pool, a coin's or a used one's,
        // are looked for past each pool as a rescan would reach them, and
        // whatever that derives is marked and refilled. Every coin is then
        // offered to `restore_utxos` again, not only the unmatched ones: a
        // coin the first restore placed in a DashPay contact account, outside
        // the balance, moves to the funds account that now derives its
        // script, which comes first in `restore_utxos` order, and the contact
        // copy is removed. A coin already in its account is taken in again in
        // its own place, keeping its flags (`take_in_output` carries them
        // from the copy it replaces), so it is held and credited once. Up to
        // `MAX_REHYDRATION_DERIVATION_INDEX` one pass reaches as far as
        // repeating it would: past each find it probes one index beyond the
        // window the refill after it opens, which is where a next pass would
        // start and stop. Past that index the probe stops, and the restore's
        // rounds carry on from the refill past the deepest find, as a rescan
        // would. A restore that already failed would only fail, and be
        // reported, again.
        if restorable {
            let mut outside = outside_funds_pools(
                wallet_info,
                (unspent.iter().map(|u| &u.address)).chain(&used),
            );
            if !outside.is_empty()
                && discover_in_funds_accounts(wallet_info, manifest, &mut outside, wallet_id, ctx)?
                    > 0
            {
                (unmatched, _) = restore_owned_utxos(
                    wallet_info,
                    &wallet,
                    &unspent,
                    &used,
                    wallet_id,
                    ctx,
                    &mut refused,
                )?;
            }
        }
        if !unmatched.is_empty() {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                count = unmatched.len(),
                duffs = unmatched.iter().map(|u| u.value()).sum::<u64>(),
                "restored UTXOs whose address no funds account derives within its gap limit \
                 were placed by their stored owner, or in the first funds account"
            );
        }
        // Every stored address no funds pool holds now, counted once: the
        // fallback coins' and the used ones', whatever owner their rows name.
        let unresolved = outside_funds_pools(
            wallet_info,
            (unmatched.iter().map(|u| &u.address)).chain(&used),
        )
        .len();
        let mut funding = wallet_info.accounts.all_funding_accounts_mut();
        for utxo in unmatched {
            let target = fallback_account(&account_keys, utxo_accounts.get(&utxo.outpoint));
            funding[target].utxos.insert(utxo.outpoint, utxo);
        }
        drop(funding);
        // Degraded in BOTH policies, never fatal: a foreign address and a
        // legitimately-owned deep-and-sparse one look the same in the stored
        // rows, and no fix short of an unbounded scan tells them apart. The
        // wallet total is exact either way, except for a coin of ours only a
        // DashPay contact account holds: `restore_utxos` places it there,
        // outside the total. Discovery moves one a funds account derives
        // within its reach; one past that reach stays, as does one placed
        // after a refused or failed refill (Recovery only), where discovery
        // is skipped (`place_held_coins`).
        if unresolved > 0 {
            ctx.note_degraded(
                LoadSite::UnresolvedUtxoAddress,
                SiteCoords {
                    wallet_id: Some(wallet_id),
                    account_type: &"any funds account",
                    affected: unresolved,
                    detail: None,
                },
                "restored coin or used addresses did not resolve against any funds account's \
                 xpub within its reach; they re-warm on the next full sync",
            );
        }
    }

    // Replay persisted InstantSend locks AFTER the UTXO restore: this marks the
    // UTXOs it finds, so running it earlier would record the txid and mark
    // nothing. Without it, instant-locked funds come back as merely confirmed
    // and stay that way until the next sync re-learns the lock.
    for (txid, lock) in &core.instant_locks_for_non_final_records {
        wallet_info.mark_instant_send_utxos(txid, lock);
    }

    // Recompute per-account + wallet balance from the restored set.
    // After this, a non-zero persisted balance is non-zero here — a
    // silent zero would be a hard FAIL of the rehydration contract.
    wallet_info.update_balance();
    Ok(())
}

/// The position among `account_keys` of a UTXO's stored owner, or the first
/// funds account when it has none there. Only for a UTXO no funds account
/// derives (`restore_owned_utxos` returned it).
fn fallback_account(account_keys: &[OwningAccount], owner: Option<&OwningAccount>) -> usize {
    // INTENTIONAL(funds-account-fallback): an unattributable UTXO routes to
    // account 0 rather than failing or being dropped, so the wallet total stays
    // exact. Accepted risk: such a UTXO sits in an account that cannot sign for
    // it, and a rescan that derives its address in its own account credits it
    // there too. Failing closed here would brick wallets whose owners
    // legitimately have no funds account (provider accounts sit on a
    // non-secp256k1 curve), which is the worse outcome.
    owner
        .and_then(|owner| account_keys.iter().position(|k| k == owner))
        .unwrap_or(0)
}

fn owner_label(owner: &OwningAccount) -> String {
    format!("{}[{}]", owner.account_type, owner.account_index)
}

/// Restore `unspent` into the funds accounts whose pools derive each address,
/// after marking `used` and the coins' addresses wherever the pools derive
/// them. Returns the UTXOs no account reaches, and whether `restore_utxos` may
/// run again: false once a refill was refused or a restore failed.
///
/// Used addresses and the coins' own extend the reach as a rescan's would:
/// each round marks the ones the funds pools now hold, and the address each
/// coin's script pays wherever a funds or DashPay contact pool holds it, as
/// the final restore marks it. It then extends every pool to its gap limit
/// past its highest used address, until a round marks nothing and its refill
/// derives nothing in any of those pools, so a run of spent addresses longer
/// than one gap window still leads to the coins past it, and a coin's refill
/// to the spent addresses past it. The final restore then marks nothing the
/// rounds left unused, so its refill derives nothing: every pool it could
/// grow was refilled first by a round, bounded as below. The first round
/// looks up every address those pools hold; each later round only the ones
/// the previous refill derived, so the rounds together look up each derived
/// address once.
///
/// Terminates: every round but the last marks at least one more address of a
/// finite set, or is the call's first and its refill derived addresses no
/// round has looked up. A later round that marks nothing leaves each pool's
/// highest used address where the refill before it found it, so its own
/// refill derives nothing and the loop exits. Each refill is bounded as
/// below.
///
/// Every refill of the marking rounds is bounded first, as
/// [`extend_pools_for_restored_addresses`] bounds its own
/// ([`gap_refills_bounded`]). A pool past those bounds stops the refills; so
/// does a derivation error, which is reported at
/// [`LoadSite::RehydrationMaintainGapLimit`]. Either way the coins whose
/// script a pool already holds are still placed, without growing any pool
/// ([`place_held_coins`]), and only the others are returned unmatched.
fn restore_owned_utxos(
    wallet_info: &mut ManagedWalletInfo,
    wallet: &Wallet,
    unspent: &[&key_wallet::Utxo],
    used: &[key_wallet::Address],
    wallet_id: [u8; 32],
    ctx: &LoadCtx,
    refused: &mut RefusedRefills,
) -> Result<(Vec<key_wallet::Utxo>, bool), WalletStorageError> {
    // The addresses the coins' scripts pay, which `restore_utxos` marks in
    // every pool holding them.
    let paid: HashSet<key_wallet::Address> = (unspent.iter())
        .filter_map(|u| {
            key_wallet::Address::from_script(&u.txout.script_pubkey, wallet.network).ok()
        })
        .collect();
    let paid: HashSet<&key_wallet::Address> = paid.iter().collect();
    let wanted: HashSet<&key_wallet::Address> = (used.iter())
        .chain(unspent.iter().map(|u| &u.address))
        .chain(paid.iter().copied())
        .collect();
    // Per pool, funds pools in `all_funding_accounts_mut` x
    // `address_pools_mut` order and then the contact pools, the highest index
    // already looked up.
    let mut looked_up: Vec<Option<u32>> = Vec::new();
    loop {
        let mut marked_any = false;
        let mut slot = 0;
        let mut look_up = |pool: &mut key_wallet::managed_account::address_pool::AddressPool,
                           wanted: &HashSet<&key_wallet::Address>| {
            if looked_up.len() == slot {
                looked_up.push(None);
            }
            let seen = &mut looked_up[slot];
            slot += 1;
            let from = seen.map_or(0, |seen| seen.saturating_add(1));
            *seen = pool.highest_generated;
            let Some(to) = pool.highest_generated.filter(|to| from <= *to) else {
                return false;
            };
            let hits: Vec<u32> = (pool.addresses.range(from..=to))
                .filter(|(_, info)| wanted.contains(&info.address))
                .map(|(index, _)| *index)
                .collect();
            hits.into_iter()
                .fold(false, |marked, index| pool.mark_index_used(index) | marked)
        };
        for funds in wallet_info.accounts.all_funding_accounts_mut() {
            for pool in funds.managed_account_type_mut().address_pools_mut() {
                marked_any |= look_up(pool, &wanted);
            }
        }
        for contact in wallet_info.accounts.dashpay_external_accounts.values_mut() {
            for pool in contact.managed_account_type_mut().address_pools_mut() {
                marked_any |= look_up(pool, &paid);
            }
        }
        if !gap_refills_bounded(wallet_info, wallet_id, ctx, refused)? {
            return place_held_coins(wallet_info, wallet, unspent, ctx).map(|left| (left, false));
        }
        if let Err(source) = wallet_info.restore_utxos(wallet, std::iter::empty()) {
            restore_failed(ctx, wallet_id, source)?;
            return place_held_coins(wallet_info, wallet, unspent, ctx).map(|left| (left, false));
        }
        if !marked_any && !derived_past(wallet_info, &looked_up) {
            break;
        }
    }
    match wallet_info.restore_utxos(wallet, unspent.iter().map(|u| (*u).clone())) {
        Ok(unmatched) => Ok((unmatched, true)),
        Err(source) => {
            restore_failed(ctx, wallet_id, source)?;
            place_held_coins(wallet_info, wallet, unspent, ctx).map(|left| (left, false))
        }
    }
}

/// Places each of `unspent` whose script an account's pools already hold as
/// `restore_utxos` does: in the first account holding it, in its order, with
/// the address marked used and any copy in another account removed. Derives
/// nothing: the restore runs against `wallet` without its accounts, so no
/// pool has a key source and its refill skips every one. Returns the coins no
/// account holds. The placement can't fail: `restore_utxos` errs only from a
/// refill, and with no key source there is none. Were it to fail, it is
/// reported as a failed restore is and every coin comes back, which is right
/// only while no earlier restore of this load has placed any of them.
///
/// Only the pools as far as the marking rounds grew them are matched, so a
/// coin a DashPay contact account already holds before its funds account's
/// pool reaches it lands in the contact account, outside the balance and not
/// counted as unresolved.
fn place_held_coins(
    wallet_info: &mut ManagedWalletInfo,
    wallet: &Wallet,
    unspent: &[&key_wallet::Utxo],
    ctx: &LoadCtx,
) -> Result<Vec<key_wallet::Utxo>, WalletStorageError> {
    let keyless =
        Wallet::new_external_signable(wallet.network, wallet.wallet_id, AccountCollection::new());
    let coins = || unspent.iter().map(|u| (*u).clone());
    match wallet_info.restore_utxos(&keyless, coins()) {
        Ok(unmatched) => Ok(unmatched),
        Err(source) => restore_failed(ctx, wallet.wallet_id, source).map(|()| coins().collect()),
    }
}

/// Whether a funds or DashPay contact pool holds addresses past the highest
/// index `looked_up` records for it, in [`restore_owned_utxos`]'s pool order.
fn derived_past(wallet_info: &ManagedWalletInfo, looked_up: &[Option<u32>]) -> bool {
    let accounts = &wallet_info.accounts;
    let pools = (accounts.all_funding_accounts().into_iter())
        .chain(accounts.dashpay_external_accounts.values())
        .flat_map(|account| account.managed_account_type().address_pools());
    pools
        .zip(looked_up)
        .any(|(pool, seen)| pool.highest_generated > *seen)
}

/// Pools, by account and pool type, whose gap refill was refused and
/// reported during this load.
type RefusedRefills = HashSet<(AccountType, AddressPoolType)>;

/// Whether a gap-limit refill of every pool `restore_utxos` extends (every
/// funds and DashPay contact pool) is representable and within
/// [`MAX_REHYDRATION_GAP_REFILL`]. Each refused pool not yet in `refused` is
/// reported through the load policy at the site
/// [`extend_pools_for_restored_addresses`] reports it at, with its account
/// and pool type, and added: strict fails the load, recovery counts it once.
fn gap_refills_bounded(
    wallet_info: &ManagedWalletInfo,
    wallet_id: [u8; 32],
    ctx: &LoadCtx,
    refused: &mut RefusedRefills,
) -> Result<bool, WalletStorageError> {
    let accounts = &wallet_info.accounts;
    let mut bounded = true;
    for account in (accounts.all_funding_accounts().into_iter())
        .chain(accounts.dashpay_external_accounts.values())
    {
        let account_type = account.managed_account_type().to_account_type();
        for pool in account.managed_account_type().address_pools() {
            let (site, error) = match ImpliedRefill::of(
                pool.highest_used,
                pool.highest_generated,
                pool.gap_limit,
            ) {
                Ok(refill) if refill.implied <= MAX_REHYDRATION_GAP_REFILL => continue,
                Ok(refill) => (LoadSite::RehydrationGapLimit, refill.too_large()),
                Err(error) => (LoadSite::RehydrationMaintainGapLimit, error),
            };
            bounded = false;
            if refused.insert((account_type, pool.pool_type)) {
                ctx.tolerate_at(
                    site,
                    SiteCoords {
                        wallet_id: Some(wallet_id),
                        account_type: &account_type,
                        affected: 1,
                        detail: Some(&pool.pool_type),
                    },
                    error,
                )?;
            }
        }
    }
    Ok(bounded)
}

/// The `addresses` no funds pool of `wallet_info` holds, once each.
fn outside_funds_pools<'a>(
    wallet_info: &ManagedWalletInfo,
    addresses: impl IntoIterator<Item = &'a key_wallet::Address>,
) -> HashSet<key_wallet::Address> {
    let funding = wallet_info.accounts.all_funding_accounts();
    addresses
        .into_iter()
        .filter(|address| {
            !(funding.iter()).any(|a| a.managed_account_type().contains_address(address))
        })
        .cloned()
        .collect()
}

/// Derives into the funds pools the `wanted` addresses they derive past what
/// they hold, from each account's xpub in `manifest`, removing each found
/// from `wanted`, and marks them used. Each pool is probed through one index
/// past what it holds (its gap limit when it holds less), and past each
/// address found through one index past the gap-limit window a refill would
/// then hold, up to [`MAX_REHYDRATION_DERIVATION_INDEX`], so addresses a gap
/// limit and one apart are found in one call. The probe derives on a copy of
/// the pool, so the pool itself grows only to the deepest address found. Every
/// funds account is probed, whatever owner a coin's row names, since only
/// derivation proves which account can sign for an address. Returns how many
/// were derived.
///
/// A pool that does not derive its deepest find reports it at
/// [`LoadSite::RehydrationEnsureDerived`] (strict fails the load); its finds
/// are then not counted.
fn discover_in_funds_accounts(
    wallet_info: &mut ManagedWalletInfo,
    manifest: &[AccountRegistrationEntry],
    wanted: &mut HashSet<key_wallet::Address>,
    wallet_id: [u8; 32],
    ctx: &LoadCtx,
) -> Result<usize, WalletStorageError> {
    use key_wallet::managed_account::address_pool::KeySource;

    let mut derived = 0;
    for account in wallet_info.accounts.all_funding_accounts_mut() {
        if wanted.is_empty() {
            break;
        }
        let account_type = account.managed_account_type().to_account_type();
        let Some(key_source) = (manifest.iter())
            .find(|e| e.account_type == account_type)
            .map(|e| KeySource::Public(e.account_xpub))
        else {
            continue;
        };
        for pool in account.managed_account_type_mut().address_pools_mut() {
            if wanted.is_empty() {
                break;
            }
            if pool.pool_type == AddressPoolType::AbsentHardened {
                continue;
            }
            let gap = pool.gap_limit;
            let start = pool.highest_generated.map_or(0, |h| h.saturating_add(1));
            let mut horizon = start.max(gap);
            let mut index = start;
            let mut probe = pool.clone();
            let mut found = Vec::new();
            let mut deepest = None;
            while index <= horizon
                && index <= MAX_REHYDRATION_DERIVATION_INDEX
                && !wanted.is_empty()
            {
                let Some(address) = ensure_derived(&mut probe, &key_source, index) else {
                    break;
                };
                if wanted.remove(&address) {
                    found.push(address);
                    deepest = Some(index);
                    horizon = horizon.max(index.saturating_add(gap).saturating_add(1));
                }
                index = index.saturating_add(1);
            }
            if index.saturating_sub(start) > REHYDRATION_DEEP_SCAN_WARN_INDEX {
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    account_type = ?account_type,
                    pool_type = ?pool.pool_type,
                    scanned_to = index.saturating_sub(1),
                    "rehydration: discovery past the pool scanned abnormally deep — \
                     likely a foreign-heavy or sparse UTXO set"
                );
            }
            let Some(deepest) = deepest else {
                continue;
            };
            if ensure_derived(pool, &key_source, deepest).is_none() {
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
                continue;
            }
            for address in &found {
                pool.mark_used(address);
            }
            derived += found.len();
        }
    }
    Ok(derived)
}

fn restore_failed(
    ctx: &LoadCtx,
    wallet_id: [u8; 32],
    source: key_wallet::Error,
) -> Result<(), WalletStorageError> {
    ctx.tolerate_at(
        LoadSite::RehydrationMaintainGapLimit,
        SiteCoords {
            wallet_id: Some(wallet_id),
            account_type: &"restore_utxos",
            affected: 1,
            detail: None,
        },
        WalletStorageError::RehydrationGapLimitFailed { source },
    )
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

/// Upper bound on the addresses one gap-limit refill may generate during
/// load. A generated address costs roughly 500 bytes — an `AddressInfo`
/// plus its three pool index entries, two of which clone the
/// `Address` / `ScriptBuf` — so this holds a single refill near 100 MB.
/// Legitimate refills span one gap window (tens of addresses); reaching
/// this cap needs `highest_used` far past `highest_generated`, which
/// `mark_used` — the only writer today — cannot produce, or a gap limit near
/// the cap, which key-wallet's own pools never have. Defense in depth
/// against a future upstream invariant break or a caller-built pool, not a
/// path a stored wallet reaches.
#[cfg(not(test))]
const MAX_REHYDRATION_GAP_REFILL: u32 = 250_000;
/// The same bound, smaller in unit tests, so a test that crosses it derives
/// little on the way.
#[cfg(test)]
const MAX_REHYDRATION_GAP_REFILL: u32 = 2_000;

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
/// are left to the caller, which looks for them past every funds pool and
/// counts those still unresolved once. Every resolved address the pools *do*
/// hold (in-window or deep-resolved) is marked used so a funded or
/// previously-used address is
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
    refused: &mut RefusedRefills,
) -> Result<(), WalletStorageError> {
    use key_wallet::managed_account::address_pool::{AddressPool, KeySource};
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

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

        // Still-unresolved addresses are left to the caller: discovery past
        // every funds pool may still resolve them, in this account or
        // another, and the caller counts what it does not.
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
            let refusal = match ImpliedRefill::of(
                pool.highest_used,
                pool.highest_generated,
                pool.gap_limit,
            ) {
                Ok(refill) if refill.implied <= MAX_REHYDRATION_GAP_REFILL => None,
                Ok(refill) => Some((LoadSite::RehydrationGapLimit, refill.too_large())),
                Err(error) => Some((LoadSite::RehydrationMaintainGapLimit, error)),
            };
            if let Some((site, error)) = refusal {
                if refused.insert((account_type, pool.pool_type)) {
                    ctx.tolerate_at(
                        site,
                        SiteCoords {
                            wallet_id: Some(wallet_id),
                            account_type: &account_type,
                            affected: 1,
                            detail: Some(&pool.pool_type),
                        },
                        error,
                    )?;
                }
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

    /// The refusal of this refill for exceeding [`MAX_REHYDRATION_GAP_REFILL`].
    fn too_large(&self) -> WalletStorageError {
        WalletStorageError::RehydrationGapLimitRefillTooLarge {
            refill_target: self.target,
            already_generated: self.already_generated,
            implied: self.implied,
            cap: MAX_REHYDRATION_GAP_REFILL,
        }
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

        let core = platform_wallet::changeset::CoreChangeSet {
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };
        let ctx = LoadCtx::strict();
        apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &used,
            &ctx,
        )
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

        let core = platform_wallet::changeset::CoreChangeSet {
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };
        let ctx = LoadCtx::strict();
        apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &used,
            &ctx,
        )
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

        // Strict, deliberately: an unresolved address cannot be told apart
        // from a legitimately sparse wallet, so it degrades but never fails.
        let ctx = LoadCtx::strict();
        apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &Default::default(),
            &ctx,
        )
        .expect("an unresolved address must never brick a strict load");
        let degradation = ctx.degradation();
        assert!(degradation.degraded);
        assert_eq!(
            degradation.by_site.get(&LoadSite::UnresolvedUtxoAddress),
            Some(&1),
            "the deep-sparse address must be counted: {:?}",
            degradation.by_site
        );

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
        extend_pools_for_restored_addresses(
            funding[0],
            &manifest,
            restored,
            wallet_id,
            ctx,
            &mut RefusedRefills::new(),
        )
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

    /// A restored UTXO whose transaction carried an InstantSend lock must come
    /// back instant-locked, not wait for the next sync to re-learn it. The
    /// persisted `core_instant_locks` rows arrive in
    /// `CoreChangeSet::instant_locks_for_non_final_records`, and replaying them
    /// has to happen AFTER the UTXO restore — `mark_instant_send_utxos` marks
    /// the UTXOs it can find, so calling it first would insert the txid and
    /// mark nothing.
    #[test]
    fn rehydration_restores_instant_send_locks_onto_restored_utxos() {
        use dashcore::blockdata::transaction::txout::TxOut;
        use dashcore::ephemerealdata::instant_lock::InstantLock;
        use dashcore::{OutPoint, Txid};
        use key_wallet::bip32::DerivationPath;
        use key_wallet::gap_limit::DEFAULT_EXTERNAL_GAP_LIMIT;
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::{Address, Utxo};

        let seed = [37u8; 64];
        let wallet = Wallet::from_seed_bytes(
            seed,
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&wallet);
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 1);

        let bip44_type = wallet_info
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .unwrap()
            .managed_account_type()
            .to_account_type();
        let xpub = manifest
            .iter()
            .find(|e| e.account_type == bip44_type)
            .map(|e| e.account_xpub)
            .expect("account xpub in manifest");
        let address: Address = {
            let mut pool = AddressPool::new_without_generation(
                DerivationPath::master(),
                AddressPoolType::External,
                DEFAULT_EXTERNAL_GAP_LIMIT,
                Network::Testnet,
            );
            pool.generate_addresses(1, &KeySource::Public(xpub), true)
                .unwrap();
            pool.address_at_index(0).unwrap()
        };

        let txid = Txid::from([0x5Au8; 32]);
        let outpoint = OutPoint { txid, vout: 0 };
        // `is_instantlocked: false` is the persisted shape: the flag is not a
        // stored column, it is re-derived from the `core_instant_locks` row.
        let utxo = Utxo {
            outpoint,
            txout: TxOut {
                value: 12_345,
                script_pubkey: address.script_pubkey(),
            },
            address,
            height: 1,
            is_coinbase: false,
            is_confirmed: true,
            is_instantlocked: false,
            is_locked: false,
            is_trusted: false,
        };

        // A real IS-lock always carries at least one input; `default()` leaves
        // the vec empty.
        let lock = InstantLock {
            inputs: vec![OutPoint {
                txid: Txid::from([0xA1u8; 32]),
                vout: 7,
            }],
            txid,
            ..Default::default()
        };

        let core = platform_wallet::changeset::CoreChangeSet {
            new_utxos: vec![utxo],
            instant_locks_for_non_final_records: [(txid, lock)].into_iter().collect(),
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

        assert!(
            wallet_info.instant_send_locks().contains(&txid),
            "the persisted InstantSend lock must be replayed onto the wallet"
        );
        let restored = wallet_info
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .and_then(|a| a.utxos.get(&outpoint))
            .expect("the restored UTXO must be present on the BIP44 account");
        assert!(
            restored.is_instantlocked,
            "the restored UTXO must carry instant-locked status, not wait for the next sync"
        );
    }

    mod restore_owned {
        use super::*;
        use dashcore::blockdata::transaction::txout::TxOut;
        use dashcore::{BlockHash, OutPoint, Transaction, TxIn, Txid};
        use key_wallet::managed_account::address_pool::AddressPool;
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::transaction_checking::{
            BlockInfo, TransactionContext, WalletTransactionChecker,
        };
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
        use key_wallet::{Address, Utxo};
        use std::collections::{BTreeSet, HashMap};

        const VALUE: u64 = 100_001_000;
        const HEIGHT: u32 = 1_000;

        fn cj() -> AccountType {
            AccountType::CoinJoin { index: 0 }
        }

        fn bip44() -> AccountType {
            AccountType::Standard {
                index: 0,
                standard_account_type: key_wallet::account::StandardAccountType::BIP44Account,
            }
        }

        /// A BIP44 and a CoinJoin account; the CoinJoin pools hold their
        /// registration window (0..=99) only, as a reloaded store's rows do.
        fn fixture(seed: u8) -> (Wallet, Vec<AccountRegistrationEntry>) {
            let opts = WalletAccountCreationOptions::SpecificAccounts(
                BTreeSet::from([0]),
                BTreeSet::new(),
                BTreeSet::from([0]),
                BTreeSet::new(),
                BTreeSet::new(),
                None,
            );
            let wallet = Wallet::from_seed_bytes([seed; 64], Network::Testnet, opts).unwrap();
            let manifest = manifest_for(&wallet);
            (wallet, manifest)
        }

        /// The CoinJoin external address at `index`, derived on a copy of
        /// the account's pool.
        fn coinjoin_address(wallet: &Wallet, index: u32) -> Address {
            external_address(wallet, &cj(), index)
        }

        /// `account`'s external address at `index`, derived on a copy of its
        /// pool.
        fn external_address(wallet: &Wallet, account: &AccountType, index: u32) -> Address {
            let info = ManagedWalletInfo::from_wallet(wallet, 1);
            let pool = external_pool(&info, account);
            let mut probe = AddressPool::new_without_generation(
                pool.base_path.clone(),
                pool.pool_type,
                pool.gap_limit,
                Network::Testnet,
            );
            probe
                .generate_addresses(index + 1, &wallet.key_source_for_account(*account), true)
                .unwrap();
            probe.address_at_index(index).unwrap()
        }

        fn external_pool(info: &ManagedWalletInfo, account: &AccountType) -> AddressPool {
            info.accounts
                .funds_account(account)
                .unwrap()
                .managed_account_type()
                .address_pools()
                .into_iter()
                .find(|p| p.pool_type == AddressPoolType::External)
                .unwrap()
                .clone()
        }

        fn txid(n: u32) -> Txid {
            let mut bytes = [0xA5u8; 32];
            bytes[..4].copy_from_slice(&n.to_le_bytes());
            Txid::from_byte_array(bytes)
        }

        fn utxo(n: u32, to: &Address) -> Utxo {
            let mut utxo = Utxo::new(
                OutPoint::new(txid(n), 0),
                TxOut {
                    value: VALUE,
                    script_pubkey: to.script_pubkey(),
                },
                to.clone(),
                HEIGHT,
                false,
            );
            utxo.is_confirmed = true;
            utxo
        }

        fn load(
            wallet: &Wallet,
            manifest: &[AccountRegistrationEntry],
            utxos: Vec<Utxo>,
            utxo_accounts: &HashMap<OutPoint, OwningAccount>,
            used: &HashMap<Address, Option<OwningAccount>>,
        ) -> ManagedWalletInfo {
            let mut info = ManagedWalletInfo::from_wallet(wallet, 1);
            let core = CoreChangeSet {
                new_utxos: utxos,
                last_processed_height: Some(HEIGHT),
                synced_height: Some(HEIGHT),
                ..Default::default()
            };
            apply_persisted_core_state(
                &mut info,
                manifest,
                &core,
                utxo_accounts,
                used,
                &LoadCtx::strict(),
            )
            .unwrap();
            info
        }

        fn holders(info: &ManagedWalletInfo, outpoint: &OutPoint) -> Vec<AccountType> {
            info.accounts
                .all_funding_accounts()
                .into_iter()
                .filter(|a| a.utxos.contains_key(outpoint))
                .map(|a| a.managed_account_type().to_account_type())
                .collect()
        }

        /// A CoinJoin coin past the stored window, with no pool row, lands
        /// in CoinJoin, where its key is found.
        #[test]
        fn a_coinjoin_utxo_past_its_stored_window_restores_into_coinjoin() {
            let (wallet, manifest) = fixture(21);
            let near = coinjoin_address(&wallet, 60);
            let deep = coinjoin_address(&wallet, 150);
            let coins = vec![utxo(1, &near), utxo(2, &deep)];
            let info = load(
                &wallet,
                &manifest,
                coins.clone(),
                &HashMap::new(),
                &HashMap::new(),
            );
            for coin in &coins {
                assert_eq!(holders(&info, &coin.outpoint), vec![cj()]);
            }
            assert!(external_pool(&info, &cj()).contains_address(&deep));
            assert_eq!(info.balance.total(), 2 * VALUE);
        }

        /// Spent addresses with no pool row chain the reach as a rescan
        /// would: three spent coins 90 apart lead to the coin past them,
        /// though no unspent coin sits within one gap window (100) of it.
        #[test]
        fn a_spent_run_longer_than_the_gap_still_reaches_the_coin_past_it() {
            let (wallet, manifest) = fixture(22);
            let spent: Vec<Address> = [60, 150, 240]
                .iter()
                .map(|i| coinjoin_address(&wallet, *i))
                .collect();
            let used = spent.iter().map(|a| (a.clone(), None)).collect();
            let coin = utxo(1, &coinjoin_address(&wallet, 330));
            let info = load(
                &wallet,
                &manifest,
                vec![coin.clone()],
                &HashMap::new(),
                &used,
            );
            assert_eq!(holders(&info, &coin.outpoint), vec![cj()]);
            let pool = external_pool(&info, &cj());
            for address in &spent {
                assert!(
                    pool.address_info(address).is_some_and(|i| i.is_used()),
                    "a spent address must come back used"
                );
            }
            assert_eq!(info.balance.total(), VALUE);
        }

        /// Coins chained 90 apart reach past index 10,000, the old discovery
        /// ceiling, and the pool keeps its gap past the deepest one.
        #[test]
        fn chained_coins_past_index_ten_thousand_restore_into_coinjoin_and_keep_the_gap() {
            let (wallet, manifest) = fixture(23);
            let indexes: Vec<u32> = (60..=10_200).step_by(90).collect();
            let deepest = *indexes.last().unwrap();
            assert!(deepest > 10_000);
            let probe = {
                let info = ManagedWalletInfo::from_wallet(&wallet, 1);
                let pool = external_pool(&info, &cj());
                let mut probe = AddressPool::new_without_generation(
                    pool.base_path.clone(),
                    pool.pool_type,
                    pool.gap_limit,
                    Network::Testnet,
                );
                probe
                    .generate_addresses(deepest + 1, &wallet.key_source_for_account(cj()), true)
                    .unwrap();
                probe
            };
            let coins: Vec<Utxo> = indexes
                .iter()
                .map(|i| utxo(*i, &probe.address_at_index(*i).unwrap()))
                .collect();
            let info = load(
                &wallet,
                &manifest,
                coins.clone(),
                &HashMap::new(),
                &HashMap::new(),
            );
            for coin in &coins {
                assert_eq!(holders(&info, &coin.outpoint), vec![cj()]);
            }
            let pool = external_pool(&info, &cj());
            assert_eq!(pool.highest_used, Some(deepest));
            assert!(pool.highest_generated >= Some(deepest + pool.gap_limit));
            assert_eq!(info.balance.total(), VALUE * coins.len() as u64);
        }

        /// A coin no account derives within its gap limit keeps today's
        /// placement: in the account its pool row names, else the first.
        #[test]
        fn a_utxo_no_account_reaches_goes_to_its_stored_owner_or_the_first_account() {
            let (wallet, manifest) = fixture(24);
            let (other, _) = fixture(25);
            let foreign = utxo(1, &coinjoin_address(&other, 0));
            let isolated = utxo(2, &coinjoin_address(&wallet, 5_000));
            let cj_owner = {
                let info = ManagedWalletInfo::from_wallet(&wallet, 1);
                owning_account_of(info.accounts.funds_account(&cj()).unwrap())
            };
            let side = HashMap::from([(isolated.outpoint, cj_owner)]);
            let info = load(
                &wallet,
                &manifest,
                vec![foreign.clone(), isolated.clone()],
                &side,
                &HashMap::new(),
            );
            assert_eq!(holders(&info, &foreign.outpoint), vec![bip44()]);
            assert_eq!(holders(&info, &isolated.outpoint), vec![cj()]);
            assert_eq!(info.balance.total(), 2 * VALUE);
        }

        /// A coin whose stale stored owner names another account lands in
        /// the account that derives it once the horizon walk from a coin with
        /// no stored owner (BIP44 index 30, one past the generated window)
        /// reaches it (index 45).
        #[test]
        fn a_coin_only_another_coins_walk_reaches_restores_into_that_account() {
            let (wallet, manifest) = fixture(27);
            let walked = utxo(1, &external_address(&wallet, &bip44(), 30));
            let reached = utxo(2, &external_address(&wallet, &bip44(), 45));
            let owner_of = |account: &AccountType| {
                let info = ManagedWalletInfo::from_wallet(&wallet, 1);
                owning_account_of(info.accounts.funds_account(account).unwrap())
            };
            let side = HashMap::from([(reached.outpoint, owner_of(&cj()))]);
            let info = load(
                &wallet,
                &manifest,
                vec![walked.clone(), reached.clone()],
                &side,
                &HashMap::new(),
            );
            assert_eq!(holders(&info, &walked.outpoint), vec![bip44()]);
            assert_eq!(holders(&info, &reached.outpoint), vec![bip44()]);
            assert_eq!(info.balance.total(), 2 * VALUE);
        }

        /// Two loads of the same store place every coin and grow every pool
        /// the same way.
        #[test]
        fn every_load_places_the_coins_the_same_way() {
            let (wallet, manifest) = fixture(26);
            let used: HashMap<Address, Option<OwningAccount>> = [60, 150]
                .iter()
                .map(|i| (coinjoin_address(&wallet, *i), None))
                .collect();
            let coins = vec![
                utxo(1, &coinjoin_address(&wallet, 240)),
                utxo(2, &coinjoin_address(&wallet, 3)),
            ];
            let state = |info: &ManagedWalletInfo| {
                info.accounts
                    .all_funding_accounts()
                    .into_iter()
                    .map(|a| {
                        let pools: Vec<_> = a
                            .managed_account_type()
                            .address_pools()
                            .iter()
                            .map(|p| (p.highest_generated, p.highest_used))
                            .collect();
                        (a.utxos.keys().copied().collect::<Vec<_>>(), pools)
                    })
                    .collect::<Vec<_>>()
            };
            let first = load(&wallet, &manifest, coins.clone(), &HashMap::new(), &used);
            let second = load(&wallet, &manifest, coins, &HashMap::new(), &used);
            assert_eq!(state(&first), state(&second));
        }

        fn pays(n: u32, to: &Address) -> Transaction {
            Transaction {
                version: 2,
                lock_time: 0,
                input: vec![TxIn {
                    previous_output: OutPoint::new(txid(n), 7),
                    ..Default::default()
                }],
                output: vec![TxOut {
                    value: VALUE,
                    script_pubkey: to.script_pubkey(),
                }],
                special_transaction_payload: None,
            }
        }

        fn restored(tx: &Transaction, to: &Address) -> Utxo {
            let mut utxo = Utxo::new(
                OutPoint::new(tx.txid(), 0),
                tx.output[0].clone(),
                to.clone(),
                HEIGHT,
                false,
            );
            utxo.is_confirmed = true;
            utxo
        }

        /// A rescan of the coins' blocks after the load (the DIP-15 rescan, a
        /// user rescan, a crash catch-up) credits the account that already
        /// holds them: the balance counts each once (rust-dashcore#1294).
        #[tokio::test]
        async fn a_rescan_after_the_load_counts_each_restored_coin_once() {
            let (mut wallet, manifest) = fixture(27);
            let near = coinjoin_address(&wallet, 60);
            let deep = coinjoin_address(&wallet, 150);
            let txs = [pays(1, &near), pays(2, &deep)];
            let coins = vec![restored(&txs[0], &near), restored(&txs[1], &deep)];
            let mut info = load(
                &wallet,
                &manifest,
                coins.clone(),
                &HashMap::new(),
                &HashMap::new(),
            );
            assert_eq!(info.balance.total(), 2 * VALUE);
            for tx in &txs {
                let context = TransactionContext::InChainLockedBlock(BlockInfo::new(
                    HEIGHT,
                    BlockHash::from_byte_array([1; 32]),
                    1_793_598_712,
                ));
                info.check_core_transaction(tx, context, &mut wallet, true, true)
                    .await;
            }
            info.update_balance();
            assert_eq!(info.balance.total(), 2 * VALUE);
            for coin in &coins {
                assert_eq!(holders(&info, &coin.outpoint), vec![cj()]);
            }
        }

        /// A coin no account reaches at load stays in the first funds
        /// account, and a rescan of its block reaches it no further, so it
        /// is still counted once.
        #[tokio::test]
        async fn a_rescan_after_the_load_leaves_an_unreached_coin_counted_once() {
            let (mut wallet, manifest) = fixture(28);
            let isolated = coinjoin_address(&wallet, 5_000);
            let tx = pays(3, &isolated);
            let coin = restored(&tx, &isolated);
            let mut info = load(
                &wallet,
                &manifest,
                vec![coin.clone()],
                &HashMap::new(),
                &HashMap::new(),
            );
            assert_eq!(holders(&info, &coin.outpoint), vec![bip44()]);
            let context = TransactionContext::InChainLockedBlock(BlockInfo::new(
                HEIGHT,
                BlockHash::from_byte_array([1; 32]),
                1_793_598_712,
            ));
            info.check_core_transaction(&tx, context, &mut wallet, true, true)
                .await;
            info.update_balance();
            assert_eq!(holders(&info, &coin.outpoint), vec![bip44()]);
            assert_eq!(info.balance.total(), VALUE);
        }

        /// A stored coin on an address of a DashPay contact's chain (theirs,
        /// watched) restores into that contact's account: no funds account
        /// holds it and the balance leaves it out.
        #[test]
        fn a_coin_on_a_contacts_address_stays_out_of_the_funds_accounts() {
            let (mut wallet, _) = fixture(29);
            let contact = AccountType::DashpayExternalAccount {
                index: 0,
                user_identity_id: [0xaa; 32],
                friend_identity_id: [0xbb; 32],
            };
            wallet.add_account(contact, None).unwrap();
            let manifest = manifest_for(&wallet);
            let theirs = {
                let info = ManagedWalletInfo::from_wallet(&wallet, 1);
                let account = info.accounts.dashpay_external_accounts.values().next();
                let pools = account.unwrap().managed_account_type().address_pools();
                pools[0].address_at_index(0).unwrap()
            };
            let ours = external_address(&wallet, &bip44(), 0);
            let contact_coin = utxo(1, &theirs);
            let own_coin = utxo(2, &ours);
            let info = load(
                &wallet,
                &manifest,
                vec![contact_coin.clone(), own_coin.clone()],
                &HashMap::new(),
                &HashMap::new(),
            );
            assert!(holders(&info, &contact_coin.outpoint).is_empty());
            assert!(info
                .accounts
                .dashpay_external_accounts
                .values()
                .any(|a| a.utxos.contains_key(&contact_coin.outpoint)));
            assert_eq!(holders(&info, &own_coin.outpoint), vec![bip44()]);
            assert_eq!(info.balance.total(), VALUE);
        }

        /// The used set as the persister loads it: every stored script, spent
        /// or unspent, with no owner (no pool row covers any of them).
        fn ownerless(addresses: &[&Address]) -> HashMap<Address, Option<OwningAccount>> {
            addresses.iter().map(|a| ((*a).clone(), None)).collect()
        }

        /// Whether `account`'s pools hold `address`, marked used.
        fn used_in(info: &ManagedWalletInfo, account: &AccountType, address: &Address) -> bool {
            let funds = info.accounts.funds_account(account).unwrap();
            (funds.managed_account_type().address_pools().iter())
                .any(|p| p.address_info(address).is_some_and(|i| i.is_used()))
        }

        /// Whether `account` can sign for `address`: coin selection finds its
        /// derivation path there.
        fn derives(info: &ManagedWalletInfo, account: &AccountType, address: &Address) -> bool {
            let funds = info.accounts.funds_account(account).unwrap();
            (funds.managed_account_type())
                .get_address_derivation_path(address)
                .is_some()
        }

        /// `account`'s identity as its pool rows name it.
        fn owner(wallet: &Wallet, account: &AccountType) -> OwningAccount {
            let info = ManagedWalletInfo::from_wallet(wallet, 1);
            owning_account_of(info.accounts.funds_account(account).unwrap())
        }

        /// [`load`] under `ctx`, onto `info` as the caller prepared it.
        fn load_into(
            info: &mut ManagedWalletInfo,
            manifest: &[AccountRegistrationEntry],
            utxos: Vec<Utxo>,
            utxo_accounts: &HashMap<OutPoint, OwningAccount>,
            used: &HashMap<Address, Option<OwningAccount>>,
            ctx: &LoadCtx,
        ) -> Result<(), WalletStorageError> {
            let core = CoreChangeSet {
                new_utxos: utxos,
                last_processed_height: Some(HEIGHT),
                synced_height: Some(HEIGHT),
                ..Default::default()
            };
            apply_persisted_core_state(info, manifest, &core, utxo_accounts, used, ctx)
        }

        /// Loads `coin` and `used` strictly, then checks the coin is held
        /// once, by `account`, which signs for it; that every `used` address
        /// comes back used there; that the total is exact; and that nothing
        /// was left unresolved.
        fn assert_restored(
            wallet: &Wallet,
            manifest: &[AccountRegistrationEntry],
            coin: &Utxo,
            utxo_accounts: &HashMap<OutPoint, OwningAccount>,
            used: &HashMap<Address, Option<OwningAccount>>,
            account: AccountType,
        ) {
            let mut info = ManagedWalletInfo::from_wallet(wallet, 1);
            let ctx = LoadCtx::strict();
            load_into(
                &mut info,
                manifest,
                vec![coin.clone()],
                utxo_accounts,
                used,
                &ctx,
            )
            .unwrap();
            assert_eq!(holders(&info, &coin.outpoint), vec![account]);
            assert!(
                derives(&info, &account, &coin.address),
                "the coin is signable"
            );
            for address in used.keys() {
                assert!(
                    used_in(&info, &account, address),
                    "{address} comes back used"
                );
            }
            assert!(used_in(&info, &account, &coin.address));
            assert_eq!(info.balance.total(), VALUE);
            assert_eq!(
                ctx.degradation()
                    .by_site
                    .get(&LoadSite::UnresolvedUtxoAddress),
                None
            );
        }

        /// A spent BIP44 address one past the generated window (30) with no
        /// pool row leads to the coin past it (45), with no stored owner for
        /// either, whether or not the coin's own script is in the used set.
        #[test]
        fn a_spent_address_one_past_the_pool_leads_to_the_coin_past_it() {
            let (wallet, manifest) = fixture(31);
            let spent = external_address(&wallet, &bip44(), 30);
            let coin = utxo(1, &external_address(&wallet, &bip44(), 45));
            for used in [ownerless(&[&spent]), ownerless(&[&spent, &coin.address])] {
                assert_restored(&wallet, &manifest, &coin, &HashMap::new(), &used, bip44());
            }
        }

        /// The reverse: the coin one past the window (30) brings the spent
        /// address past it (45) back used, never as a fresh receive address.
        #[test]
        fn a_coin_one_past_the_pool_brings_the_spent_address_past_it_back_used() {
            let (wallet, manifest) = fixture(32);
            let spent = external_address(&wallet, &bip44(), 45);
            let coin = utxo(1, &external_address(&wallet, &bip44(), 30));
            let used = ownerless(&[&spent, &coin.address]);
            assert_restored(&wallet, &manifest, &coin, &HashMap::new(), &used, bip44());
        }

        /// Spent addresses past the window bridge to a deeper coin: 30, then
        /// 55 (within a gap of 30), then the coin at 84 (within a gap of 55).
        #[test]
        fn spent_addresses_past_the_pool_bridge_to_a_deeper_coin() {
            let (wallet, manifest) = fixture(33);
            let spent: Vec<Address> = [30, 55]
                .iter()
                .map(|i| external_address(&wallet, &bip44(), *i))
                .collect();
            let coin = utxo(1, &external_address(&wallet, &bip44(), 84));
            let used = ownerless(&[&spent[0], &spent[1], &coin.address]);
            assert_restored(&wallet, &manifest, &coin, &HashMap::new(), &used, bip44());
        }

        /// A lone CoinJoin coin one past its pool (100, the pool generated
        /// through 99), with no stored owner and no other coin to extend
        /// CoinJoin, restores into CoinJoin, not the first funds account:
        /// with and without its script in the used set.
        #[test]
        fn a_lone_coinjoin_coin_one_past_its_pool_restores_into_coinjoin() {
            let (wallet, manifest) = fixture(34);
            assert_eq!(
                external_pool(&ManagedWalletInfo::from_wallet(&wallet, 1), &cj()).highest_generated,
                Some(99)
            );
            let coin = utxo(1, &coinjoin_address(&wallet, 100));
            for used in [HashMap::new(), ownerless(&[&coin.address])] {
                assert_restored(&wallet, &manifest, &coin, &HashMap::new(), &used, cj());
            }
        }

        /// A spent CoinJoin address one past its pool (100) leads to the
        /// coin past it (180), neither with a stored owner.
        #[test]
        fn a_spent_coinjoin_address_one_past_its_pool_leads_to_the_coin_past_it() {
            let (wallet, manifest) = fixture(35);
            let spent = coinjoin_address(&wallet, 100);
            let coin = utxo(1, &coinjoin_address(&wallet, 180));
            let used = ownerless(&[&spent, &coin.address]);
            assert_restored(&wallet, &manifest, &coin, &HashMap::new(), &used, cj());
        }

        /// A wallet whose DashPay contact pool has a gap limit of 0, a pool
        /// no gap refill can be computed for, with one shallow BIP44 coin.
        fn unrefillable_contact_fixture() -> (Wallet, Vec<AccountRegistrationEntry>, Utxo) {
            let (mut wallet, _) = fixture(36);
            let contact = AccountType::DashpayExternalAccount {
                index: 0,
                user_identity_id: [0xaa; 32],
                friend_identity_id: [0xbb; 32],
            };
            wallet.add_account(contact, None).unwrap();
            let manifest = manifest_for(&wallet);
            let coin = utxo(1, &external_address(&wallet, &bip44(), 3));
            (wallet, manifest, coin)
        }

        fn with_unrefillable_contact_pool(wallet: &Wallet) -> ManagedWalletInfo {
            let mut info = ManagedWalletInfo::from_wallet(wallet, 1);
            let contact = info.accounts.dashpay_external_accounts.values_mut().next();
            for pool in contact
                .unwrap()
                .managed_account_type_mut()
                .address_pools_mut()
            {
                pool.gap_limit = 0;
            }
            info
        }

        /// The refill preflight's refusal of a pool no extension visits (the
        /// contact pool) fails a strict load with its typed error.
        #[test]
        fn an_unrefillable_contact_pool_fails_a_strict_load() {
            let (wallet, manifest, coin) = unrefillable_contact_fixture();
            let mut info = with_unrefillable_contact_pool(&wallet);
            let err = load_into(
                &mut info,
                &manifest,
                vec![coin],
                &HashMap::new(),
                &HashMap::new(),
                &LoadCtx::strict(),
            )
            .expect_err("a refused refill must fail a strict load");
            assert!(
                matches!(
                    err,
                    WalletStorageError::RehydrationGapLimitTargetOutOfRange { gap_limit: 0, .. }
                ),
                "{err:?}"
            );
        }

        /// Under recovery the same refusal is counted, and the coin stays
        /// counted.
        #[test]
        fn an_unrefillable_contact_pool_is_counted_under_recovery() {
            let (wallet, manifest, coin) = unrefillable_contact_fixture();
            let mut info = with_unrefillable_contact_pool(&wallet);
            let ctx = LoadCtx::recovery();
            load_into(
                &mut info,
                &manifest,
                vec![coin.clone()],
                &HashMap::new(),
                &ownerless(&[&coin.address]),
                &ctx,
            )
            .expect("recovery tolerates the refusal");
            let degradation = ctx.degradation();
            assert_eq!(
                degradation
                    .by_site
                    .get(&LoadSite::RehydrationMaintainGapLimit),
                Some(&1),
                "{:?}",
                degradation.by_site
            );
            assert_eq!(holders(&info, &coin.outpoint), vec![bip44()]);
            assert_eq!(info.balance.total(), VALUE);
        }

        /// Every account holding `outpoint`, the DashPay contact accounts
        /// included.
        fn every_holder(info: &ManagedWalletInfo, outpoint: &OutPoint) -> Vec<AccountType> {
            (info.accounts.all_funding_accounts().into_iter())
                .chain(info.accounts.dashpay_external_accounts.values())
                .filter(|a| a.utxos.contains_key(outpoint))
                .map(|a| a.managed_account_type().to_account_type())
                .collect()
        }

        /// The address `coin`'s script pays.
        fn paid_address(coin: &Utxo) -> Address {
            Address::from_script(&coin.txout.script_pubkey, Network::Testnet).unwrap()
        }

        /// Loads `coins` under recovery onto [`with_unrefillable_contact_pool`]
        /// prepared by `prepare`, checking the contact pool's refusal was
        /// counted once.
        fn load_past_refused_refill(
            wallet: &Wallet,
            manifest: &[AccountRegistrationEntry],
            coins: &[Utxo],
            utxo_accounts: &HashMap<OutPoint, OwningAccount>,
            used: &HashMap<Address, Option<OwningAccount>>,
            prepare: impl FnOnce(&mut ManagedWalletInfo),
        ) -> (ManagedWalletInfo, LoadCtx) {
            let mut info = with_unrefillable_contact_pool(wallet);
            prepare(&mut info);
            let ctx = LoadCtx::recovery();
            load_into(
                &mut info,
                manifest,
                coins.to_vec(),
                utxo_accounts,
                used,
                &ctx,
            )
            .expect("recovery tolerates the refusal");
            assert_eq!(
                ctx.degradation()
                    .by_site
                    .get(&LoadSite::RehydrationMaintainGapLimit),
                Some(&1)
            );
            (info, ctx)
        }

        /// Checks `coin` is held by CoinJoin alone, which signs for the
        /// address its script pays and has it marked used.
        fn assert_held_by_coinjoin(info: &ManagedWalletInfo, coin: &Utxo) {
            let address = paid_address(coin);
            assert_eq!(every_holder(info, &coin.outpoint), vec![cj()]);
            assert!(derives(info, &cj(), &address), "the coin is signable");
            assert!(used_in(info, &cj(), &address), "{address} comes back used");
        }

        /// An ownerless CoinJoin coin whose script CoinJoin already holds
        /// (50) stays in CoinJoin when an unrelated contact pool's refill is
        /// refused, with or without its script in the used set.
        #[test]
        fn a_held_coin_keeps_its_account_past_a_refused_refill() {
            let (wallet, manifest, _) = unrefillable_contact_fixture();
            let coin = utxo(1, &coinjoin_address(&wallet, 50));
            for used in [HashMap::new(), ownerless(&[&coin.address])] {
                let (info, ctx) = load_past_refused_refill(
                    &wallet,
                    &manifest,
                    std::slice::from_ref(&coin),
                    &HashMap::new(),
                    &used,
                    |_| {},
                );
                assert_held_by_coinjoin(&info, &coin);
                assert_eq!(info.balance.total(), VALUE);
                assert_eq!(
                    ctx.degradation()
                        .by_site
                        .get(&LoadSite::UnresolvedUtxoAddress),
                    None
                );
            }
        }

        /// Past a refused refill, a held coin goes to the account holding
        /// its script, not to the account its row names (BIP44).
        #[test]
        fn a_held_coin_goes_by_its_script_not_its_stored_owner_past_a_refused_refill() {
            let (wallet, manifest, _) = unrefillable_contact_fixture();
            let coin = utxo(1, &coinjoin_address(&wallet, 50));
            let side = HashMap::from([(coin.outpoint, owner(&wallet, &bip44()))]);
            for used in [HashMap::new(), ownerless(&[&coin.address])] {
                let (info, _) = load_past_refused_refill(
                    &wallet,
                    &manifest,
                    std::slice::from_ref(&coin),
                    &side,
                    &used,
                    |_| {},
                );
                assert_held_by_coinjoin(&info, &coin);
                assert_eq!(info.balance.total(), VALUE);
            }
        }

        /// Past a refused refill, a coin is matched by the script it pays,
        /// not by the address stored with it (here a BIP44 one).
        #[test]
        fn a_held_coin_goes_by_its_script_not_its_stored_address_past_a_refused_refill() {
            let (wallet, manifest, _) = unrefillable_contact_fixture();
            let mut coin = utxo(1, &coinjoin_address(&wallet, 50));
            coin.address = external_address(&wallet, &bip44(), 5);
            let (info, _) = load_past_refused_refill(
                &wallet,
                &manifest,
                std::slice::from_ref(&coin),
                &HashMap::new(),
                &HashMap::new(),
                |_| {},
            );
            assert_held_by_coinjoin(&info, &coin);
            assert_eq!(info.balance.total(), VALUE);
        }

        /// Past a refused refill, only the coin no account holds (another
        /// wallet's address) takes the fallback and is counted unresolved;
        /// the held CoinJoin coin stays in CoinJoin.
        #[test]
        fn only_a_coin_no_account_holds_falls_back_past_a_refused_refill() {
            let (wallet, manifest, _) = unrefillable_contact_fixture();
            let (foreign_wallet, _) = fixture(99);
            let held = utxo(1, &coinjoin_address(&wallet, 50));
            let foreign = utxo(2, &external_address(&foreign_wallet, &bip44(), 0));
            for used in [
                HashMap::new(),
                ownerless(&[&held.address, &foreign.address]),
            ] {
                let (info, ctx) = load_past_refused_refill(
                    &wallet,
                    &manifest,
                    &[held.clone(), foreign.clone()],
                    &HashMap::new(),
                    &used,
                    |_| {},
                );
                assert_held_by_coinjoin(&info, &held);
                assert_eq!(every_holder(&info, &foreign.outpoint), vec![bip44()]);
                assert_eq!(info.balance.total(), 2 * VALUE);
                assert_eq!(
                    ctx.degradation()
                        .by_site
                        .get(&LoadSite::UnresolvedUtxoAddress),
                    Some(&1)
                );
            }
        }

        /// Past a refused refill, a script two accounts hold (CoinJoin's,
        /// also put in the contact pool) goes to the first of them in
        /// `restore_utxos` order: CoinJoin before the contact account.
        #[test]
        fn a_script_two_accounts_hold_goes_to_the_first_past_a_refused_refill() {
            let (wallet, manifest, _) = unrefillable_contact_fixture();
            let coin = utxo(1, &coinjoin_address(&wallet, 50));
            let (info, _) = load_past_refused_refill(
                &wallet,
                &manifest,
                std::slice::from_ref(&coin),
                &HashMap::new(),
                &HashMap::new(),
                |info| {
                    let funds = info.accounts.funds_account(&cj()).unwrap();
                    let pools = funds.managed_account_type().address_pools();
                    let mut held = (pools.iter())
                        .find_map(|p| p.address_info(&coin.address))
                        .unwrap()
                        .clone();
                    let contact = info.accounts.dashpay_external_accounts.values_mut().next();
                    let mut pools = contact
                        .unwrap()
                        .managed_account_type_mut()
                        .address_pools_mut();
                    let pool = &mut pools[0];
                    let index = pool.highest_generated.map_or(0, |h| h + 1);
                    held.index = index;
                    pool.address_index.insert(held.address.clone(), index);
                    pool.script_pubkey_index
                        .insert(held.script_pubkey.clone(), index);
                    pool.addresses.insert(index, held);
                    pool.highest_generated = Some(index);
                    // Unrefillable whether or not the coin marks it.
                    pool.gap_limit = u32::MAX;
                },
            );
            assert_held_by_coinjoin(&info, &coin);
            assert_eq!(info.balance.total(), VALUE);
        }

        /// Past a refused refill, a copy of a held coin that another account
        /// (BIP44) holds is removed when the coin is placed.
        #[test]
        fn a_copy_another_account_holds_is_removed_past_a_refused_refill() {
            let (wallet, manifest, _) = unrefillable_contact_fixture();
            let coin = utxo(1, &coinjoin_address(&wallet, 50));
            let (info, _) = load_past_refused_refill(
                &wallet,
                &manifest,
                std::slice::from_ref(&coin),
                &HashMap::new(),
                &HashMap::new(),
                |info| {
                    let funds = info.accounts.funds_account_mut(&bip44()).unwrap();
                    funds.utxos.insert(coin.outpoint, coin.clone());
                },
            );
            assert_held_by_coinjoin(&info, &coin);
            assert_eq!(info.balance.total(), VALUE);
        }

        /// Every pool's highest generated index, the contact pools included.
        fn pool_heights(info: &ManagedWalletInfo) -> Vec<Option<u32>> {
            (info.accounts.all_funding_accounts().into_iter())
                .chain(info.accounts.dashpay_external_accounts.values())
                .flat_map(|a| a.managed_account_type().address_pools())
                .map(|p| p.highest_generated)
                .collect()
        }

        /// The held-coin placement grows no pool, refused or not: the coin on
        /// CoinJoin's last generated address (99), which a refill would grow
        /// CoinJoin past, is placed in CoinJoin and every pool keeps its
        /// height.
        #[test]
        fn placing_held_coins_grows_no_pool() {
            let (wallet, _, _) = unrefillable_contact_fixture();
            let coin = utxo(1, &coinjoin_address(&wallet, 99));
            for refused in [false, true] {
                let mut info = match refused {
                    true => with_unrefillable_contact_pool(&wallet),
                    false => ManagedWalletInfo::from_wallet(&wallet, 1),
                };
                let heights = pool_heights(&info);
                let left =
                    place_held_coins(&mut info, &wallet, &[&coin], &LoadCtx::strict()).unwrap();
                assert!(left.is_empty());
                assert_held_by_coinjoin(&info, &coin);
                assert_eq!(pool_heights(&info), heights, "refused: {refused}");
            }
        }

        /// The address at `index` of `account`'s first pool, the DashPay
        /// contact accounts included, derived on a copy of the pool.
        fn first_pool_address(wallet: &Wallet, account: &AccountType, index: u32) -> Address {
            let info = ManagedWalletInfo::from_wallet(wallet, 1);
            let holder = (info.accounts.all_funding_accounts().into_iter())
                .chain(info.accounts.dashpay_external_accounts.values())
                .find(|a| a.managed_account_type().to_account_type() == *account)
                .unwrap();
            let mut probe = holder.managed_account_type().address_pools()[0].clone();
            ensure_derived(&mut probe, &wallet.key_source_for_account(*account), index).unwrap()
        }

        /// A wallet with the DashPay receiving account of a friendship between
        /// two of its identities and the reverse contact account it also
        /// holds, added as key-wallet's `befriend_both_ways` adds them: both
        /// derive the same addresses. Returns the wallet, its manifest, both
        /// account types and their shared address one past the receiving
        /// pool's window.
        fn befriended_both_ways(
            seed: u8,
        ) -> (
            Wallet,
            Vec<AccountRegistrationEntry>,
            AccountType,
            AccountType,
            Address,
        ) {
            let (mut wallet, _) = fixture(seed);
            let (us, them) = ([0xaa; 32], [0xbb; 32]);
            let receiving = AccountType::DashpayReceivingFunds {
                index: 0,
                user_identity_id: us,
                friend_identity_id: them,
            };
            let reverse = AccountType::DashpayExternalAccount {
                index: 0,
                user_identity_id: them,
                friend_identity_id: us,
            };
            for account in [receiving, reverse] {
                wallet.add_account(account, None).unwrap();
            }
            let manifest = manifest_for(&wallet);
            let past = key_wallet::gap_limit::DEFAULT_DASHPAY_GAP_LIMIT;
            let shared = first_pool_address(&wallet, &receiving, past);
            assert_eq!(first_pool_address(&wallet, &reverse, past), shared);
            (wallet, manifest, receiving, reverse, shared)
        }

        /// `wallet`'s managed state with the reverse contact pool derived one
        /// past the receiving pool's window, through their shared address.
        fn with_deeper_contact_pool(
            wallet: &Wallet,
            receiving: &AccountType,
            reverse: &AccountType,
        ) -> ManagedWalletInfo {
            let mut info = ManagedWalletInfo::from_wallet(wallet, 1);
            let past = key_wallet::gap_limit::DEFAULT_DASHPAY_GAP_LIMIT;
            let funds = info.accounts.funds_account(receiving).unwrap();
            let window = funds.managed_account_type().address_pools()[0].highest_generated;
            assert_eq!(window, Some(past - 1));
            let contact = info.accounts.dashpay_external_accounts.values_mut().next();
            let mut pools = contact
                .unwrap()
                .managed_account_type_mut()
                .address_pools_mut();
            let key_source = wallet.key_source_for_account(*reverse);
            ensure_derived(&mut *pools[0], &key_source, past).unwrap();
            info
        }

        /// Loads a coin on the address the receiving and the reverse contact
        /// accounts share, past the receiving pool's window but inside the
        /// contact pool, under `ctx`, with its address in the used set or
        /// not. Checks it ends in the receiving account, which discovery
        /// derives it in: signable, in the balance, gone from the contact
        /// account, and nothing left unresolved.
        fn assert_ends_in_the_receiving_account(seed: u8, ctx: LoadCtx, coin_used: bool) {
            let (wallet, manifest, receiving, reverse, shared) = befriended_both_ways(seed);
            let coin = utxo(1, &shared);
            let used = if coin_used {
                ownerless(&[&coin.address])
            } else {
                HashMap::new()
            };
            let mut info = with_deeper_contact_pool(&wallet, &receiving, &reverse);
            load_into(
                &mut info,
                &manifest,
                vec![coin.clone()],
                &HashMap::new(),
                &used,
                &ctx,
            )
            .unwrap();
            assert_eq!(every_holder(&info, &coin.outpoint), vec![receiving]);
            assert!(
                derives(&info, &receiving, &coin.address),
                "the coin is signable"
            );
            assert_eq!(info.balance.total(), VALUE);
            assert_eq!(
                ctx.degradation()
                    .by_site
                    .get(&LoadSite::UnresolvedUtxoAddress),
                None
            );
        }

        #[test]
        fn a_coin_a_deeper_contact_pool_holds_ends_in_the_receiving_account() {
            assert_ends_in_the_receiving_account(42, LoadCtx::strict(), true);
        }

        #[test]
        fn a_coin_a_deeper_contact_pool_holds_ends_in_the_receiving_account_unused() {
            assert_ends_in_the_receiving_account(44, LoadCtx::strict(), false);
        }

        #[test]
        fn a_coin_a_deeper_contact_pool_holds_ends_in_the_receiving_account_in_recovery() {
            assert_ends_in_the_receiving_account(45, LoadCtx::recovery(), true);
        }

        #[test]
        fn a_coin_a_deeper_contact_pool_holds_ends_in_the_receiving_account_unused_in_recovery() {
            assert_ends_in_the_receiving_account(46, LoadCtx::recovery(), false);
        }

        /// After discovery derives something every coin is offered to
        /// key-wallet again, and a coin already in its account comes back
        /// once, unchanged: a BIP44 coin, instant-locked, trusted and locked,
        /// beside a CoinJoin coin one past its pool that only discovery
        /// reaches.
        #[test]
        fn a_placed_coin_offered_again_after_discovery_is_held_once_unchanged() {
            let (wallet, manifest) = fixture(43);
            let mut placed = utxo(1, &external_address(&wallet, &bip44(), 3));
            placed.is_instantlocked = true;
            placed.is_trusted = true;
            placed.is_locked = true;
            let deep = utxo(2, &coinjoin_address(&wallet, 100));
            for used in [HashMap::new(), ownerless(&[&placed.address, &deep.address])] {
                let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
                load_into(
                    &mut info,
                    &manifest,
                    vec![placed.clone(), deep.clone()],
                    &HashMap::new(),
                    &used,
                    &LoadCtx::strict(),
                )
                .unwrap();
                assert_eq!(every_holder(&info, &placed.outpoint), vec![bip44()]);
                let funds = info.accounts.funds_account(&bip44()).unwrap();
                assert_eq!(funds.utxos.get(&placed.outpoint), Some(&placed));
                assert_eq!(every_holder(&info, &deep.outpoint), vec![cj()]);
                assert_eq!(info.balance.total(), 2 * VALUE);
            }
        }

        fn contact() -> AccountType {
            AccountType::DashpayExternalAccount {
                index: 0,
                user_identity_id: [0xaa; 32],
                friend_identity_id: [0xbb; 32],
            }
        }

        /// The first pool of [`unrefillable_contact_fixture`]'s contact
        /// account.
        fn contact_pool(info: &ManagedWalletInfo) -> &AddressPool {
            let contact = info.accounts.dashpay_external_accounts.values().next();
            contact.unwrap().managed_account_type().address_pools()[0]
        }

        /// [`unrefillable_contact_fixture`]'s managed state with its contact
        /// pool's gap limit one past the refill budget, derived through
        /// `through` when given.
        fn with_wide_contact_pool(wallet: &Wallet, through: Option<u32>) -> ManagedWalletInfo {
            let mut info = ManagedWalletInfo::from_wallet(wallet, 1);
            let account = info.accounts.dashpay_external_accounts.values_mut().next();
            let mut pools = account
                .unwrap()
                .managed_account_type_mut()
                .address_pools_mut();
            let pool = &mut *pools[0];
            pool.gap_limit = MAX_REHYDRATION_GAP_REFILL + 1;
            if let Some(index) = through {
                let key_source = wallet.key_source_for_account(contact());
                ensure_derived(pool, &key_source, index).unwrap();
            }
            info
        }

        /// A coin on the last address a contact pool holds, whose gap limit
        /// is one past the refill budget, would have that pool refilled past
        /// the budget once marked. A strict load fails at the refill site,
        /// and the pool derives nothing more.
        #[test]
        fn a_coin_cannot_push_a_contact_pool_refill_past_the_budget() {
            let (wallet, manifest, own) = unrefillable_contact_fixture();
            let budget = MAX_REHYDRATION_GAP_REFILL;
            let mut info = with_wide_contact_pool(&wallet, Some(budget));
            let theirs = utxo(2, &contact_pool(&info).address_at_index(budget).unwrap());
            let err = load_into(
                &mut info,
                &manifest,
                vec![own, theirs],
                &HashMap::new(),
                &HashMap::new(),
                &LoadCtx::strict(),
            )
            .expect_err("a refill past the budget must fail a strict load");
            assert!(
                matches!(
                    err,
                    WalletStorageError::RehydrationGapLimitRefillTooLarge { .. }
                ),
                "{err:?}"
            );
            assert_eq!(contact_pool(&info).highest_generated, Some(budget));
        }

        /// Under recovery the same refusal is counted once, the contact pool
        /// derives nothing more, and both coins are still placed: the BIP44
        /// coin in BIP44 and the contact coin in the contact account.
        #[test]
        fn a_coin_cannot_push_a_contact_pool_refill_past_the_budget_in_recovery() {
            let (wallet, manifest, own) = unrefillable_contact_fixture();
            let budget = MAX_REHYDRATION_GAP_REFILL;
            let mut info = with_wide_contact_pool(&wallet, Some(budget));
            let theirs = utxo(2, &contact_pool(&info).address_at_index(budget).unwrap());
            let ctx = LoadCtx::recovery();
            load_into(
                &mut info,
                &manifest,
                vec![own.clone(), theirs.clone()],
                &HashMap::new(),
                &HashMap::new(),
                &ctx,
            )
            .expect("recovery tolerates the refusal");
            assert_eq!(
                ctx.degradation()
                    .by_site
                    .get(&LoadSite::RehydrationGapLimit),
                Some(&1)
            );
            assert_eq!(contact_pool(&info).highest_generated, Some(budget));
            assert_eq!(every_holder(&info, &own.outpoint), vec![bip44()]);
            assert_eq!(every_holder(&info, &theirs.outpoint), vec![contact()]);
            assert_eq!(info.balance.total(), VALUE);
        }

        /// A contact pool whose own refill first derives a coin's address
        /// (its window, 0..=9, grows to the gap limit, one past the budget)
        /// cannot then be refilled past the budget by that coin's mark: the
        /// rounds look up what the refill derived before the final restore,
        /// although the round before marked nothing.
        #[test]
        fn a_coin_a_contact_refill_reaches_cannot_push_it_past_the_budget() {
            let (wallet, manifest, _) = unrefillable_contact_fixture();
            let budget = MAX_REHYDRATION_GAP_REFILL;
            let mut info = with_wide_contact_pool(&wallet, None);
            assert_eq!(contact_pool(&info).highest_generated, Some(9));
            let theirs = utxo(2, &first_pool_address(&wallet, &contact(), budget));
            let err = load_into(
                &mut info,
                &manifest,
                vec![theirs],
                &HashMap::new(),
                &HashMap::new(),
                &LoadCtx::strict(),
            )
            .expect_err("a refill past the budget must fail a strict load");
            assert!(
                matches!(
                    err,
                    WalletStorageError::RehydrationGapLimitRefillTooLarge { .. }
                ),
                "{err:?}"
            );
            assert_eq!(contact_pool(&info).highest_generated, Some(budget));
        }

        /// A coin whose stored address (BIP44's at 5) differs from the one
        /// its script pays (CoinJoin's last, in a pool whose gap limit is one
        /// past the budget) cannot push the CoinJoin pool past the budget: the
        /// rounds mark the script's address before any refill.
        #[test]
        fn a_coin_cannot_push_its_scripts_pool_past_the_budget_by_its_stored_address() {
            let (wallet, manifest) = fixture(47);
            let budget = MAX_REHYDRATION_GAP_REFILL;
            let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
            let funds = info.accounts.funds_account_mut(&cj()).unwrap();
            let mut pools = funds.managed_account_type_mut().address_pools_mut();
            let pool = (pools.iter_mut())
                .find(|p| p.pool_type == AddressPoolType::External)
                .unwrap();
            pool.gap_limit = budget + 1;
            let key_source = wallet.key_source_for_account(cj());
            let paid = ensure_derived(pool, &key_source, budget).unwrap();
            let mut coin = utxo(1, &paid);
            coin.address = external_address(&wallet, &bip44(), 5);
            let err = load_into(
                &mut info,
                &manifest,
                vec![coin],
                &HashMap::new(),
                &HashMap::new(),
                &LoadCtx::strict(),
            )
            .expect_err("a refill past the budget must fail a strict load");
            assert!(
                matches!(
                    err,
                    WalletStorageError::RehydrationGapLimitRefillTooLarge { .. }
                ),
                "{err:?}"
            );
            assert_eq!(external_pool(&info, &cj()).highest_generated, Some(budget));
        }

        /// A coin on a contact address within the pool (9, its last) still
        /// has the contact pool refilled one gap past it, marked used.
        #[test]
        fn a_contact_coin_keeps_its_pool_refilled_past_it() {
            let (wallet, manifest, own) = unrefillable_contact_fixture();
            let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
            let theirs = utxo(2, &contact_pool(&info).address_at_index(9).unwrap());
            load_into(
                &mut info,
                &manifest,
                vec![own, theirs.clone()],
                &HashMap::new(),
                &HashMap::new(),
                &LoadCtx::strict(),
            )
            .unwrap();
            let pool = contact_pool(&info);
            assert_eq!(pool.highest_generated, Some(9 + pool.gap_limit));
            assert!(pool.address_info(&theirs.address).unwrap().is_used());
            assert_eq!(every_holder(&info, &theirs.outpoint), vec![contact()]);
        }

        /// A funds pool whose refill both its owner's extension and the
        /// preflight before the restore refuse (a gap limit past
        /// [`MAX_REHYDRATION_GAP_REFILL`]) is counted once under recovery,
        /// and its coin stays in its account.
        #[test]
        fn a_refill_refused_by_the_extension_and_the_preflight_is_counted_once() {
            let (wallet, manifest) = fixture(41);
            let coin = utxo(1, &external_address(&wallet, &bip44(), 3));
            let side = HashMap::from([(coin.outpoint, owner(&wallet, &bip44()))]);
            let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
            let funds = info.accounts.funds_account_mut(&bip44()).unwrap();
            for pool in funds.managed_account_type_mut().address_pools_mut() {
                if pool.pool_type == AddressPoolType::External {
                    pool.gap_limit = MAX_REHYDRATION_GAP_REFILL + 1_000;
                }
            }
            let ctx = LoadCtx::recovery();
            load_into(
                &mut info,
                &manifest,
                vec![coin.clone()],
                &side,
                &HashMap::new(),
                &ctx,
            )
            .expect("recovery tolerates the refusal");
            let degradation = ctx.degradation();
            assert_eq!(
                degradation.by_site.get(&LoadSite::RehydrationGapLimit),
                Some(&1),
                "{:?}",
                degradation.by_site
            );
            assert_eq!(holders(&info, &coin.outpoint), vec![bip44()]);
            assert_eq!(info.balance.total(), VALUE);
        }

        /// A used address whose pool row names its account (55) behind one
        /// with no row (30, one past the window) comes back used, never as a
        /// fresh receive address, whether the coin sits on the unrowed
        /// address or past both (70).
        #[test]
        fn a_rowed_used_address_behind_an_unrowed_one_comes_back_used() {
            let (wallet, manifest) = fixture(37);
            let unrowed = external_address(&wallet, &bip44(), 30);
            let rowed = external_address(&wallet, &bip44(), 55);
            for coin in [
                utxo(1, &unrowed),
                utxo(2, &external_address(&wallet, &bip44(), 70)),
            ] {
                let mut used = ownerless(&[&unrowed, &coin.address]);
                used.insert(rowed.clone(), Some(owner(&wallet, &bip44())));
                assert_restored(&wallet, &manifest, &coin, &HashMap::new(), &used, bip44());
            }
        }

        /// A coin its stored owner's extension misses (45, past that
        /// account's first window), but which discovery past the pool
        /// reaches through a spent address (30), is restored and not counted
        /// as unresolved.
        #[test]
        fn a_coin_its_owner_misses_but_discovery_reaches_is_not_counted() {
            let (wallet, manifest) = fixture(38);
            let spent = external_address(&wallet, &bip44(), 30);
            let coin = utxo(1, &external_address(&wallet, &bip44(), 45));
            let side = HashMap::from([(coin.outpoint, owner(&wallet, &bip44()))]);
            let used = ownerless(&[&spent]);
            assert_restored(&wallet, &manifest, &coin, &side, &used, bip44());
        }

        /// Stored addresses a gap limit and one apart (BIP44 gap 30: spent
        /// 30 and 61, then the coin at 92) lead to the coin past them.
        #[test]
        fn stored_addresses_a_gap_and_one_apart_lead_to_the_coin_past_them() {
            let (wallet, manifest) = fixture(39);
            let spent: Vec<Address> = [30, 61]
                .iter()
                .map(|i| external_address(&wallet, &bip44(), *i))
                .collect();
            let coin = utxo(1, &external_address(&wallet, &bip44(), 92));
            let used = ownerless(&[&spent[0], &spent[1], &coin.address]);
            assert_restored(&wallet, &manifest, &coin, &HashMap::new(), &used, bip44());
        }

        /// A coin no funds account reaches (BIP44 5000) is counted once as
        /// unresolved, with no stored owner, also in the used set, or with a
        /// stored owner whose extension misses it too; it falls back to
        /// BIP44 and the total stays exact.
        #[test]
        fn a_coin_past_every_reach_is_counted_once() {
            let (wallet, manifest) = fixture(40);
            let coin = utxo(1, &external_address(&wallet, &bip44(), 5_000));
            let bip44_owner = owner(&wallet, &bip44());
            let cases = [
                ("no owner", HashMap::new(), HashMap::new()),
                (
                    "no owner, in the used set",
                    HashMap::new(),
                    ownerless(&[&coin.address]),
                ),
                (
                    "stored owner",
                    HashMap::from([(coin.outpoint, bip44_owner.clone())]),
                    HashMap::from([(coin.address.clone(), Some(bip44_owner))]),
                ),
            ];
            for (case, side, used) in cases {
                let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
                let ctx = LoadCtx::strict();
                load_into(&mut info, &manifest, vec![coin.clone()], &side, &used, &ctx).unwrap();
                assert_eq!(holders(&info, &coin.outpoint), vec![bip44()], "{case}");
                assert_eq!(info.balance.total(), VALUE, "{case}");
                assert_eq!(
                    ctx.degradation()
                        .by_site
                        .get(&LoadSite::UnresolvedUtxoAddress),
                    Some(&1),
                    "{case}"
                );
            }
        }

        /// Loads `coins` strictly with only the `spent` addresses as the used
        /// set, none with a stored owner, then checks each coin is held once,
        /// by its account, which signs for it; that every coin and spent
        /// address comes back used in its account; that the total is exact;
        /// and that nothing was left unresolved.
        fn assert_live_and_spent(
            wallet: &Wallet,
            manifest: &[AccountRegistrationEntry],
            coins: &[(Utxo, AccountType)],
            spent: &[(Address, AccountType)],
        ) {
            let used: HashMap<Address, Option<OwningAccount>> =
                spent.iter().map(|(a, _)| (a.clone(), None)).collect();
            let utxos = coins.iter().map(|(coin, _)| coin.clone()).collect();
            let mut info = ManagedWalletInfo::from_wallet(wallet, 1);
            let ctx = LoadCtx::strict();
            load_into(&mut info, manifest, utxos, &HashMap::new(), &used, &ctx).unwrap();
            for (coin, account) in coins {
                assert_eq!(holders(&info, &coin.outpoint), vec![*account]);
                assert!(
                    derives(&info, account, &coin.address),
                    "{} is signable",
                    coin.address
                );
                assert!(used_in(&info, account, &coin.address));
            }
            for (address, account) in spent {
                assert!(
                    used_in(&info, account, address),
                    "{address} comes back used"
                );
            }
            assert_eq!(info.balance.total(), VALUE * coins.len() as u64);
            assert_eq!(
                ctx.degradation()
                    .by_site
                    .get(&LoadSite::UnresolvedUtxoAddress),
                None
            );
        }

        /// A used set holding only a spent address (45) and not the live
        /// coin's, as the API allows: the coin in the window (20) brings the
        /// spent address into the pool, and it comes back used.
        #[test]
        fn a_live_coin_in_the_window_brings_a_spent_address_back_used() {
            let (wallet, manifest) = fixture(42);
            let spent = external_address(&wallet, &bip44(), 45);
            let coin = utxo(1, &external_address(&wallet, &bip44(), 20));
            assert_live_and_spent(&wallet, &manifest, &[(coin, bip44())], &[(spent, bip44())]);
        }

        /// The spent address a live coin brings in (45, from the coin at 20)
        /// leads on to a deeper live coin (70), which restores signable.
        #[test]
        fn a_spent_address_a_live_coin_brings_in_leads_to_the_coin_past_it() {
            let (wallet, manifest) = fixture(43);
            let spent = external_address(&wallet, &bip44(), 45);
            let coins =
                [20, 70].map(|i| (utxo(i, &external_address(&wallet, &bip44(), i)), bip44()));
            assert_live_and_spent(&wallet, &manifest, &coins, &[(spent, bip44())]);
        }

        /// The same in two accounts in one load: BIP44 (coin 20, spent 45)
        /// and CoinJoin (coin 50, spent 140; gap 100).
        #[test]
        fn live_coins_in_two_accounts_bring_their_spent_addresses_back_used() {
            let (wallet, manifest) = fixture(44);
            let coins = [
                (utxo(1, &external_address(&wallet, &bip44(), 20)), bip44()),
                (utxo(2, &coinjoin_address(&wallet, 50)), cj()),
            ];
            let spent = [
                (external_address(&wallet, &bip44(), 45), bip44()),
                (coinjoin_address(&wallet, 140), cj()),
            ];
            assert_live_and_spent(&wallet, &manifest, &coins, &spent);
        }

        /// A live CoinJoin coin (50) brings the spent address its refill
        /// derives (140; gap 100) back used, with only that spent address in
        /// the used set.
        #[test]
        fn a_live_coinjoin_coin_brings_its_spent_address_back_used() {
            let (wallet, manifest) = fixture(48);
            let coin = utxo(1, &coinjoin_address(&wallet, 50));
            let spent = coinjoin_address(&wallet, 140);
            assert_live_and_spent(&wallet, &manifest, &[(coin, cj())], &[(spent, cj())]);
        }

        /// `account`'s `pool_type` pool, derived through `through` on a copy.
        fn pool_through(
            wallet: &Wallet,
            account: AccountType,
            pool_type: AddressPoolType,
            through: u32,
        ) -> AddressPool {
            let info = ManagedWalletInfo::from_wallet(wallet, 1);
            let funds = info.accounts.funds_account(&account).unwrap();
            let pools = funds.managed_account_type().address_pools();
            let pool = pools
                .into_iter()
                .find(|p| p.pool_type == pool_type)
                .unwrap();
            let mut probe = AddressPool::new_without_generation(
                pool.base_path.clone(),
                pool.pool_type,
                pool.gap_limit,
                Network::Testnet,
            );
            probe
                .generate_addresses(through + 1, &wallet.key_source_for_account(account), true)
                .unwrap();
            probe
        }

        /// Ownerless spent addresses a gap apart in `account`'s `pool_type`
        /// pool, from one gap through `last`, then a coin at last + gap/2, a
        /// spent address at last + gap + gap/5 and a coin at last + 2 gaps:
        /// each within a gap of the one before, as a rescan reaches them.
        /// With and without the coins in the used set (the SQLite loader
        /// passes both), every stored address comes back used, both coins
        /// restore signable into `account`, and nothing is unresolved.
        fn assert_chain_restored(
            seed: u8,
            account: AccountType,
            pool_type: AddressPoolType,
            last: u32,
        ) {
            let (wallet, manifest) = fixture(seed);
            let gap = pool_through(&wallet, account, pool_type, 0).gap_limit;
            let probe = pool_through(&wallet, account, pool_type, last + 2 * gap);
            let at = |i: u32| probe.address_at_index(i).unwrap();
            let mut spent: Vec<Address> = (gap..=last).step_by(gap as usize).map(at).collect();
            spent.push(at(last + gap + gap / 5));
            let coins = [utxo(1, &at(last + gap / 2)), utxo(2, &at(last + 2 * gap))];
            for coins_in_used in [false, true] {
                let mut used = ownerless(&spent.iter().collect::<Vec<_>>());
                if coins_in_used {
                    used.extend(ownerless(
                        &coins.iter().map(|c| &c.address).collect::<Vec<_>>(),
                    ));
                }
                let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
                let ctx = LoadCtx::strict();
                let utxos = coins.to_vec();
                load_into(&mut info, &manifest, utxos, &HashMap::new(), &used, &ctx).unwrap();
                for coin in &coins {
                    assert_eq!(holders(&info, &coin.outpoint), vec![account]);
                    assert!(
                        derives(&info, &account, &coin.address),
                        "{} is signable (coins in used: {coins_in_used})",
                        coin.address
                    );
                }
                for address in &spent {
                    assert!(
                        used_in(&info, &account, address),
                        "{address} comes back used (coins in used: {coins_in_used})"
                    );
                }
                assert_eq!(info.balance.total(), 2 * VALUE);
                assert_eq!(
                    ctx.degradation()
                        .by_site
                        .get(&LoadSite::UnresolvedUtxoAddress),
                    None
                );
            }
        }

        /// The chain's last stored address (10000) sits at the discovery
        /// ceiling, so the coin and spent address past it (10050, 10120) and
        /// the coin past those (10200) are reached by the refills after
        /// discovery, not by its probe.
        #[test]
        fn a_coinjoin_chain_to_the_discovery_ceiling_reaches_the_coins_past_it() {
            assert_chain_restored(45, cj(), AddressPoolType::External, 10_000);
        }

        /// The same on a BIP44 change chain (gap 30, through 9990): the coin
        /// past the ceiling (10005), the spent address past it (10026) and
        /// the coin past that (10050).
        #[test]
        fn a_change_chain_to_the_discovery_ceiling_reaches_the_coins_past_it() {
            assert_chain_restored(46, bip44(), AddressPoolType::Internal, 9_990);
        }

        /// The same CoinJoin chain ending well below the ceiling (5000),
        /// which discovery's probe covers on its own.
        #[test]
        fn a_coinjoin_chain_below_the_discovery_ceiling_reaches_the_coins_past_it() {
            assert_chain_restored(47, cj(), AddressPoolType::External, 5_000);
        }
    }
}
