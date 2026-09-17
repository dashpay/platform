//! External-signable wallet reconstruction
//!
//! Load is seedless — each wallet is rebuilt watch-only from its manifest and
//! the manager consumes the carried snapshot directly, so no wrong-seed check
//! runs here; that gate lives in the resolver-backed signing entrypoints.

use key_wallet::account::account_collection::AccountCollection;
use key_wallet::account::{Account, AccountType};
use key_wallet::managed_account::address_pool::{AddressPoolType, PublicKeyType};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::transaction_record::{
    OutputRole, TransactionDirection, TransactionRecord,
};
use key_wallet::transaction_checking::{TransactionContext, TransactionType};
use key_wallet::wallet::managed_wallet_info::{ManagedWalletInfo, PersistedWalletState};
use key_wallet::wallet::Wallet;
use key_wallet::Network;

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
///   `CoreChangeSet` cannot carry it. Each restored unspent UTXO is routed
///   to the funds account whose identity matches its entry; a UTXO absent
///   from the map (its script matched no pool row) falls back to the first
///   funds account, which must establish address ownership before installation.
/// - `additional_spent_outpoints`: durable spend evidence, including sweep
///   placeholders without an owning account or a complete transaction record.
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
///   outpoint with no account hint is tried against the first account, but
///   installation requires that account to own its address.
/// - **Address-pool depth**: each pool is forward-derived to cover
///   restored UTXOs at deep derivation indices, then the gap window is
///   refilled beyond the deepest restored index so the per-address view
///   reconciles with the wallet total.
/// - **Address-pool used-state**: every `used_pool_addresses` entry is
///   re-marked used (in union with the unspent-UTXO addresses), so an
///   address whose funds were since spent is not re-handed-out as fresh.
/// - **InstantSend locks**: separately persisted locks reconcile unconfirmed
///   records and coins before validation. Mined contexts retain their block
///   information; lock replay also restores transaction-level lock tracking.
/// - **Sync watermarks**: `synced_height` / `last_processed_height`.
///
/// # Reconstructed when the persister supplies it
///
/// - **`last_applied_chain_lock`**: restored from `core` on both backends
///   when the supplied [`CoreChangeSet`](platform_wallet::changeset::CoreChangeSet)
///   carries it, promoting covered records before returning so finality is
///   effective at open. The asset-lock-resume CL-from-metadata fallback
///   (`proof.rs`) also fires at launch instead of waiting for SPV. The FFI/iOS
///   persister round-trips the value Swift held; the SQLite persister
///   reads it from `core_sync_state.last_applied_chain_lock` (present
///   since V001) via a monotonic height-max merge on write and
///   `decode_chain_lock` under the load policy on read. It stays `None`
///   only when the column is NULL or, under
///   [`LoadPolicy::Recovery`](crate::LoadPolicy), when the blob failed to
///   decode and was tolerated as [`LoadSite::ChainLockBlob`].
///
/// # Address ownership and coin metadata
///
/// Pool discovery is bounded by the gap limit and [`MAX_REHYDRATION_DERIVATION_INDEX`].
/// A coin whose address remains unresolved is rejected, including a legitimate
/// deep-and-sparse address outside the restored pools. Restore its derivation
/// range before retrying; unverified coins cannot contribute to the balance.
/// SQLite derives `is_coinbase` from the funding record when available.
/// `is_trusted` is refreshed on sync; `is_instantlocked` is rebuilt from stored locks.
///
/// # Errors
///
/// [`WalletStorageError::MissingAccount`] if there are persisted UTXOs to
/// restore but the reconstructed account collection has **no**
/// funds-bearing account to hold them. Fail-closed rather than
/// reconstructing a silent zero balance (the no-silent-zero mandate).
/// [`WalletStorageError::CoreStateRestore`] if the snapshot is inconsistent
/// or the receiving wallet already contains transaction or UTXO state.
///
/// On error the supplied wallet is unchanged. This never touches key material.
pub fn apply_persisted_core_state(
    wallet_info: &mut ManagedWalletInfo,
    manifest: &[AccountRegistrationEntry],
    core: &CoreChangeSet,
    utxo_accounts: &std::collections::HashMap<dashcore::OutPoint, OwningAccount>,
    used_pool_addresses: &std::collections::HashMap<key_wallet::Address, Option<OwningAccount>>,
    additional_spent_outpoints: &std::collections::BTreeMap<dashcore::OutPoint, Option<u32>>,
    ctx: &LoadCtx,
) -> Result<(), WalletStorageError> {
    let mut restored = wallet_info.clone();
    restore_core_state(
        &mut restored,
        manifest,
        core,
        utxo_accounts,
        used_pool_addresses,
        additional_spent_outpoints,
        ctx,
    )?;
    *wallet_info = restored;
    Ok(())
}

fn restore_core_state(
    wallet_info: &mut ManagedWalletInfo,
    manifest: &[AccountRegistrationEntry],
    core: &CoreChangeSet,
    utxo_accounts: &std::collections::HashMap<dashcore::OutPoint, OwningAccount>,
    used_pool_addresses: &std::collections::HashMap<key_wallet::Address, Option<OwningAccount>>,
    additional_spent_outpoints: &std::collections::BTreeMap<dashcore::OutPoint, Option<u32>>,
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

    // Restore the UTXO set, routing each materialized outpoint to its true owning
    // funds account via `utxo_accounts` (matched on the same account identity
    // the writer keyed the pool row on). Two miss cases share the first-account
    // best-effort fallback but differ in signal:
    //   (a) no side-channel entry — the UTXO's script matched no pool row; an
    //       fallback whose address must resolve in that account's pools.
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

    let mut persisted = PersistedWalletState {
        transactions: core.account_records.clone(),
        additional_spent_outpoints: additional_spent_outpoints.clone(),
        ..Default::default()
    };
    for utxo in &core.spent_utxos {
        persisted
            .additional_spent_outpoints
            .entry(utxo.outpoint)
            .or_insert(None);
    }
    let instantlocked_txids: std::collections::HashSet<_> = core
        .records
        .iter()
        .filter(|record| matches!(record.context, TransactionContext::InstantSend(_)))
        .map(|record| record.txid)
        .chain(core.instant_locks_for_non_final_records.keys().copied())
        .collect();
    let mut funding = wallet_info.accounts.all_funding_accounts_mut();
    if (!unspent.is_empty() || !core.spent_utxos.is_empty()) && funding.is_empty() {
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
            let mut restored_utxo = (*utxo).clone();
            restored_utxo.is_instantlocked |= instantlocked_txids.contains(&utxo.outpoint.txid);
            persisted.utxos.push((
                funding[target].managed_account_type().to_account_type(),
                restored_utxo,
            ));
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

        // An owner absent from the
        // funds accounts is not necessarily corruption: provider accounts are
        // first-class in the schema yet sit on a non-secp256k1 curve, so they
        // are not `ManagedCoreFundsAccount` and a used provider-owned address
        // has no funds account to route to. Telling that apart needs an
        // upstream `key-wallet` enumerator over every account kind; until then,
        // used-address-only rows can remain unresolved. Unspent coins still
        // require verified ownership before installation.
        if !orphaned_owners.is_empty() {
            ctx.note_degraded(
                LoadSite::OrphanedUtxoOwner,
                SiteCoords {
                    wallet_id: Some(wallet_id),
                    account_type: &orphaned_owners,
                    affected: orphaned_owners.len(),
                    detail: None,
                },
                "restored UTXOs or used addresses were routed to the first funds account \
                 because their own owning accounts are not funds accounts of this wallet; \
                 unspent coins still require verified address ownership",
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
    drop(funding);

    let exact_txids: std::collections::HashSet<_> = persisted
        .transactions
        .iter()
        .map(|record| record.txid)
        .collect();
    for record in &core.records {
        if !exact_txids.contains(&record.txid) {
            persisted
                .transactions
                .extend(reconstruct_legacy_account_records(wallet_info, record)?);
        }
    }

    // Lock events persist independently of records; validation requires their
    // unconfirmed lifecycle to agree with the restored coin flags.
    for record in &mut persisted.transactions {
        if matches!(record.context, TransactionContext::Mempool) {
            if let Some(lock) = core.instant_locks_for_non_final_records.get(&record.txid) {
                record.update_context(TransactionContext::InstantSend(lock.clone()));
            }
        }
    }

    wallet_info.restore_persisted_state(persisted)?;

    // Replay lock metadata after restoring records. Coins are already marked
    // above because records with InstantSend context pre-register the txid and
    // make this method return early.
    for (txid, lock) in &core.instant_locks_for_non_final_records {
        wallet_info.mark_instant_send_utxos(txid, lock);
    }
    if let Some(chain_lock) = &core.last_applied_chain_lock {
        wallet_info.apply_chain_lock(chain_lock.clone());
    }

    // Recompute per-account + wallet balance from the restored set.
    // After this, a non-zero persisted balance is non-zero here — a
    // silent zero would be a hard FAIL of the rehydration contract.
    wallet_info.update_balance();
    Ok(())
}

fn reconstruct_legacy_account_records(
    wallet_info: &ManagedWalletInfo,
    record: &TransactionRecord,
) -> Result<Vec<TransactionRecord>, WalletStorageError> {
    use key_wallet::wallet::managed_wallet_info::RestoreError;

    let accounts = wallet_info.accounts.all_funding_accounts();
    let mut slices = Vec::new();
    for account in &accounts {
        let inputs: Vec<_> = record
            .input_details
            .iter()
            .filter(|detail| account.contains_address(&detail.address))
            .cloned()
            .collect();
        let has_inputs = !inputs.is_empty();
        let outputs: Vec<_> = record
            .output_details
            .iter()
            .filter_map(|detail| {
                if detail
                    .address
                    .as_ref()
                    .is_some_and(|address| account.contains_address(address))
                {
                    Some(detail.clone())
                } else if has_inputs {
                    let mut sent = detail.clone();
                    if sent.role != OutputRole::Unspendable {
                        sent.role = OutputRole::Sent;
                    }
                    Some(sent)
                } else {
                    None
                }
            })
            .collect();
        if inputs.is_empty() && outputs.is_empty() {
            continue;
        }
        let received: i128 = outputs
            .iter()
            .filter(|detail| matches!(detail.role, OutputRole::Received | OutputRole::Change))
            .map(|detail| i128::from(detail.value))
            .sum();
        let sent: i128 = inputs.iter().map(|detail| i128::from(detail.value)).sum();
        let net_amount =
            i64::try_from(received - sent).map_err(|_| RestoreError::InvalidRecord(record.txid))?;
        let has_sent = outputs.iter().any(|detail| detail.role == OutputRole::Sent);
        let has_our_outputs = outputs
            .iter()
            .any(|detail| matches!(detail.role, OutputRole::Received | OutputRole::Change));
        let direction = if record.transaction_type == TransactionType::CoinJoin {
            TransactionDirection::CoinJoin
        } else if has_inputs && !has_sent && has_our_outputs {
            TransactionDirection::Internal
        } else if has_inputs {
            TransactionDirection::Outgoing
        } else {
            TransactionDirection::Incoming
        };
        let mut slice = record.clone();
        slice.account_type = account.managed_account_type().to_account_type();
        slice.input_details = inputs;
        slice.output_details = outputs;
        slice.net_amount = net_amount;
        slice.direction = direction;
        slices.push(slice);
    }
    if record.input_details.iter().any(|detail| {
        accounts
            .iter()
            .filter(|account| account.contains_address(&detail.address))
            .count()
            != 1
    }) || record.output_details.iter().any(|detail| {
        let matches = detail.address.as_ref().map_or(0, |address| {
            accounts
                .iter()
                .filter(|account| account.contains_address(address))
                .count()
        });
        (matches!(detail.role, OutputRole::Received | OutputRole::Change) && matches != 1)
            || (detail.role == OutputRole::Sent && matches > 0)
    }) {
        return Err(RestoreError::InvalidRecord(record.txid).into());
    }
    if slices.is_empty() {
        slices.push(record.clone());
    }
    Ok(slices)
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
            &Default::default(),
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
            &Default::default(),
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
            &Default::default(),
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

    #[test]
    fn should_reject_foreign_utxo_without_mutating_wallet() {
        use dashcore::blockdata::transaction::txout::TxOut;
        use dashcore::{OutPoint, Txid};
        use key_wallet::bip32::DerivationPath;
        use key_wallet::gap_limit::DEFAULT_EXTERNAL_GAP_LIMIT;
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::{Address, Utxo};

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

        let core = platform_wallet::changeset::CoreChangeSet {
            new_utxos: vec![
                utxo(normal_addr, normal_val, 1),
                utxo(foreign_addr, foreign_val, 2),
            ],
            last_processed_height: Some(1),
            synced_height: Some(1),
            ..Default::default()
        };

        let before =
            bincode::serde::encode_to_vec(&wallet_info, bincode::config::standard()).unwrap();
        let error = apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            WalletStorageError::CoreStateRestore(
                key_wallet::wallet::managed_wallet_info::RestoreError::InvalidUtxo(_)
            )
        ));
        assert_eq!(
            bincode::serde::encode_to_vec(&wallet_info, bincode::config::standard()).unwrap(),
            before
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
            &Default::default(),
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

    #[test]
    fn should_reject_unresolved_sparse_utxo_until_pool_is_restored() {
        use dashcore::blockdata::transaction::txout::TxOut;
        use dashcore::{OutPoint, Txid};
        use key_wallet::bip32::DerivationPath;
        use key_wallet::gap_limit::DEFAULT_EXTERNAL_GAP_LIMIT;
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::{Address, Utxo};

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

        let before =
            bincode::serde::encode_to_vec(&wallet_info, bincode::config::standard()).unwrap();
        let ctx = LoadCtx::strict();
        let error = apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &Default::default(),
            &Default::default(),
            &ctx,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            WalletStorageError::CoreStateRestore(
                key_wallet::wallet::managed_wallet_info::RestoreError::InvalidUtxo(_)
            )
        ));
        assert_eq!(
            bincode::serde::encode_to_vec(&wallet_info, bincode::config::standard()).unwrap(),
            before
        );
        assert_eq!(
            ctx.degradation()
                .by_site
                .get(&LoadSite::UnresolvedUtxoAddress),
            Some(&1)
        );

        // Restoring the known derivation range establishes ownership before retrying.
        let account = wallet_info.accounts.all_funding_accounts_mut().remove(0);
        let pool = account
            .managed_account_type_mut()
            .address_pools_mut()
            .into_iter()
            .find(|pool| pool.is_external())
            .unwrap();
        pool.generate_addresses(46, &KeySource::Public(xpub), true)
            .unwrap();
        apply_persisted_core_state(
            &mut wallet_info,
            &manifest,
            &core,
            &Default::default(),
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .unwrap();
        assert_eq!(wallet_info.balance.total(), value);
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

    // Lock events persist separately from transaction records and must restore
    // the record and coin to the same lifecycle before validation.
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

        let transaction = dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![dashcore::TxIn {
                previous_output: OutPoint {
                    txid: Txid::from([0xA1u8; 32]),
                    vout: 7,
                },
                ..Default::default()
            }],
            output: vec![TxOut {
                value: 12_345,
                script_pubkey: address.script_pubkey(),
            }],
            special_transaction_payload: None,
        };
        let txid = transaction.txid();
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
            height: 0,
            is_coinbase: false,
            is_confirmed: false,
            is_instantlocked: false,
            is_locked: false,
            is_trusted: false,
        };

        let already_locked = Utxo {
            is_instantlocked: true,
            ..utxo.clone()
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
        let record = key_wallet::managed_account::transaction_record::TransactionRecord::new(
            transaction,
            bip44_type,
            key_wallet::transaction_checking::TransactionContext::Mempool,
            key_wallet::transaction_checking::TransactionType::Standard,
            key_wallet::managed_account::transaction_record::TransactionDirection::Incoming,
            Vec::new(),
            Vec::new(),
            12_345,
        );

        use key_wallet::transaction_checking::BlockInfo;
        let block = BlockInfo::new(1, dashcore::BlockHash::from([0xB1; 32]), 123);
        for context in [
            TransactionContext::Mempool,
            TransactionContext::InstantSend(lock.clone()),
            TransactionContext::InBlock(block),
            TransactionContext::InChainLockedBlock(block),
        ] {
            for exact_records in [false, true] {
                let mut record = record.clone();
                record.update_context(context.clone());
                let mut utxo = utxo.clone();
                if let Some(block) = record.block_info() {
                    utxo.height = block.height();
                    utxo.is_confirmed = true;
                }
                let wallet_id = wallet_info.wallet_id;
                let mut conn = rusqlite::Connection::open_in_memory().unwrap();
                crate::sqlite::migrations::run(&mut conn).unwrap();
                conn.execute(
                "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
                rusqlite::params![wallet_id.as_slice()],
            ).unwrap();
                let tx = conn.transaction().unwrap();
                crate::sqlite::schema::core_state::apply(
                    &tx,
                    &wallet_id,
                    &CoreChangeSet {
                        records: vec![record.clone()],
                        account_records: if exact_records {
                            vec![record.clone()]
                        } else {
                            vec![]
                        },
                        new_utxos: vec![utxo.clone()],
                        ..Default::default()
                    },
                )
                .unwrap();
                // TransactionInstantLocked persists a lock without rewriting the record.
                crate::sqlite::schema::core_state::apply(
                    &tx,
                    &wallet_id,
                    &CoreChangeSet {
                        instant_locks_for_non_final_records: [(txid, lock.clone())]
                            .into_iter()
                            .collect(),
                        ..Default::default()
                    },
                )
                .unwrap();
                tx.commit().unwrap();
                let (core, owners, spent) = crate::sqlite::schema::core_state::load_state(
                    &conn,
                    &wallet_id,
                    Network::Testnet,
                    &LoadCtx::strict(),
                )
                .unwrap();
                assert_eq!(core.records[0].context, context);
                assert_eq!(!core.account_records.is_empty(), exact_records);
                wallet_info = ManagedWalletInfo::from_wallet(&wallet, 1);
                apply_persisted_core_state(
                    &mut wallet_info,
                    &manifest,
                    &core,
                    &owners,
                    &Default::default(),
                    &spent,
                    &LoadCtx::strict(),
                )
                .unwrap();
                assert!(wallet_info.instant_send_locks().contains(&txid));
                let account = &wallet_info.accounts.standard_bip44_accounts[&0];
                let restored = &account.utxos[&outpoint];
                assert!(restored.is_instantlocked);
                assert_eq!(restored.is_confirmed, record.block_info().is_some());
                assert_eq!(restored.height, utxo.height);
                if context.is_chain_locked() {
                    assert!(account.transaction_is_finalized(&txid));
                    if let Some(retained) = account.transactions().get(&txid) {
                        assert_eq!(retained.context, context);
                    }
                } else {
                    let expected = if record.block_info().is_some() {
                        context.clone()
                    } else {
                        TransactionContext::InstantSend(lock.clone())
                    };
                    assert_eq!(account.transactions()[&txid].context, expected);
                }
            }
        }

        let mut already_locked_info = ManagedWalletInfo::from_wallet(&wallet, 1);
        apply_persisted_core_state(
            &mut already_locked_info,
            &manifest,
            &CoreChangeSet {
                new_utxos: vec![already_locked],
                ..Default::default()
            },
            &Default::default(),
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .unwrap();
        assert!(
            already_locked_info.accounts.standard_bip44_accounts[&0].utxos[&outpoint]
                .is_instantlocked
        );
    }

    #[test]
    fn rehydration_applies_saved_chainlock_to_restored_records() {
        use dashcore::hashes::Hash;
        use key_wallet::account::StandardAccountType;
        use key_wallet::transaction_checking::BlockInfo;
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
        let wallet = Wallet::from_seed_bytes(
            [0x57; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let account_type = AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        };
        let tx = dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        };
        let record = TransactionRecord::new(
            tx,
            account_type,
            TransactionContext::InBlock(BlockInfo::new(80, dashcore::BlockHash::all_zeros(), 0)),
            TransactionType::Standard,
            TransactionDirection::Incoming,
            Vec::new(),
            Vec::new(),
            0,
        );
        let txid = record.txid;
        let mut later_record = record.clone();
        later_record.transaction.lock_time = 1;
        later_record.txid = later_record.transaction.txid();
        later_record.update_context(TransactionContext::InBlock(BlockInfo::new(
            101,
            dashcore::BlockHash::all_zeros(),
            0,
        )));
        let later_txid = later_record.txid;
        let chainlock = dashcore::ChainLock {
            block_height: 100,
            block_hash: dashcore::BlockHash::all_zeros(),
            signature: [0; 96].into(),
        };
        let mut restored = ManagedWalletInfo::from_wallet(&wallet, 1);
        apply_persisted_core_state(
            &mut restored,
            &manifest_for(&wallet),
            &CoreChangeSet {
                records: vec![record, later_record.clone()],
                last_applied_chain_lock: Some(chainlock.clone()),
                synced_height: Some(100),
                ..Default::default()
            },
            &Default::default(),
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .unwrap();
        assert!(
            restored.accounts.standard_bip44_accounts[&0].transaction_is_finalized(&txid),
            "saved ChainLock must already finalize the restored record at open"
        );
        let account = &restored.accounts.standard_bip44_accounts[&0];
        assert!(!account.transaction_is_finalized(&later_txid));
        assert_eq!(
            account.transactions()[&later_txid].context,
            later_record.context
        );
        let repeated = restored.apply_chain_lock(chainlock);
        assert!(!repeated.metadata_advanced);
        assert!(
            repeated.locked_transactions.is_empty(),
            "nothing remains to promote on replay"
        );
    }

    #[test]
    fn rehydration_accepts_legacy_outgoing_contact_payment() {
        use key_wallet::account::StandardAccountType;
        use key_wallet::managed_account::transaction_record::{InputDetail, OutputDetail};
        let mut wallet = Wallet::from_seed_bytes(
            [0x58; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let contact_wallet = Wallet::from_seed_bytes(
            [0x59; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let contact_type = AccountType::DashpayExternalAccount {
            index: 0,
            user_identity_id: [1; 32],
            friend_identity_id: [2; 32],
        };
        let contact_account = Account::from_xpub(
            None,
            contact_type,
            contact_wallet.accounts.standard_bip44_accounts[&0].account_xpub,
            Network::Testnet,
        )
        .unwrap();
        wallet.accounts.insert(contact_account).unwrap();
        let mut restored = ManagedWalletInfo::from_wallet(&wallet, 1);
        let address = restored.accounts.standard_bip44_accounts[&0].all_addresses()[0].clone();
        let contact_address = restored
            .accounts
            .dashpay_external_accounts
            .values()
            .next()
            .unwrap()
            .all_addresses()[0]
            .clone();
        let transaction = dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![dashcore::TxIn {
                previous_output: dashcore::OutPoint {
                    txid: dashcore::Txid::from([0x60; 32]),
                    vout: 0,
                },
                ..Default::default()
            }],
            output: vec![dashcore::TxOut {
                value: 900,
                script_pubkey: contact_address.script_pubkey(),
            }],
            special_transaction_payload: None,
        };
        let record = TransactionRecord::new(
            transaction,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            vec![InputDetail {
                index: 0,
                value: 1000,
                address,
            }],
            vec![OutputDetail {
                index: 0,
                value: 900,
                address: Some(contact_address),
                role: OutputRole::Sent,
            }],
            -1000,
        );
        let txid = record.txid;
        apply_persisted_core_state(
            &mut restored,
            &manifest_for(&wallet),
            &CoreChangeSet {
                records: vec![record],
                ..Default::default()
            },
            &Default::default(),
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .expect("legacy outgoing contact payment must load without treating contact coins as ours");
        let outgoing = &restored.accounts.standard_bip44_accounts[&0].transactions()[&txid];
        assert_eq!(outgoing.net_amount, -1000);
        assert_eq!(outgoing.direction, TransactionDirection::Outgoing);
        assert_eq!(outgoing.output_details[0].role, OutputRole::Sent);
        assert!(restored
            .accounts
            .dashpay_external_accounts
            .values()
            .all(|account| { account.transactions().is_empty() && account.utxos.is_empty() }));
    }

    #[tokio::test]
    async fn legacy_folded_record_restores_each_account_amount() {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, TxOut};
        use key_wallet::account::{AccountType, StandardAccountType};
        use key_wallet::managed_account::transaction_record::OutputDetail;
        use key_wallet::transaction_checking::{BlockInfo, WalletTransactionChecker};
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        let mut wallet = Wallet::from_seed_bytes(
            [0x47; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&wallet);
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
        let standard = AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        };
        let coinjoin = AccountType::CoinJoin { index: 0 };
        let monitored = WalletInfoInterface::monitored_addresses(&info);
        let addresses: Vec<_> = [standard, coinjoin]
            .into_iter()
            .map(|kind| {
                monitored
                    .iter()
                    .find(|address| {
                        info.accounts.all_accounts().iter().any(|account| {
                            account.managed_account_type().to_account_type() == kind
                                && account.contains_address(address)
                        })
                    })
                    .expect("account receive address")
                    .clone()
            })
            .collect();
        let transaction = dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: [5_000, 7_000]
                .into_iter()
                .zip(&addresses)
                .map(|(value, address)| TxOut {
                    value,
                    script_pubkey: address.script_pubkey(),
                })
                .collect(),
            special_transaction_payload: None,
        };
        let details = [5_000, 7_000]
            .into_iter()
            .zip(&addresses)
            .enumerate()
            .map(|(index, (value, address))| OutputDetail {
                index: index as u32,
                role: OutputRole::Received,
                address: Some(address.clone()),
                value,
            })
            .collect();
        let record = TransactionRecord::new(
            transaction.clone(),
            standard,
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Incoming,
            Vec::new(),
            details,
            12_000,
        );
        let txid = record.txid;
        apply_persisted_core_state(
            &mut info,
            &manifest,
            &CoreChangeSet {
                records: vec![record],
                ..Default::default()
            },
            &Default::default(),
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .unwrap();
        let restored: Vec<_> = [standard, coinjoin]
            .into_iter()
            .map(|kind| {
                info.accounts
                    .all_accounts()
                    .into_iter()
                    .find(|account| account.managed_account_type().to_account_type() == kind)
                    .and_then(|account| account.transactions().get(&txid))
                    .expect("restored account record")
                    .net_amount
            })
            .collect();
        assert_eq!(restored, [5_000, 7_000]);
        let result = info
            .check_core_transaction(
                &transaction,
                TransactionContext::InBlock(BlockInfo::new(
                    200,
                    BlockHash::from_byte_array([0x4Du8; 32]),
                    1_700_000_000,
                )),
                &mut wallet,
                true,
                true,
            )
            .await;
        assert_eq!(result.updated_records.len(), 2);
        assert_eq!(
            result
                .updated_records
                .iter()
                .map(|record| record.net_amount)
                .sum::<i64>(),
            12_000,
            "confirmation must not count the restored 7,000 twice"
        );
    }

    #[test]
    fn exact_account_slice_does_not_require_a_spent_only_address_in_the_restored_pool() {
        use dashcore::address::Payload;
        use dashcore::hashes::Hash;
        use dashcore::{PubkeyHash, Transaction, TxOut};
        use key_wallet::account::{AccountType, StandardAccountType};
        use key_wallet::managed_account::transaction_record::OutputDetail;

        let wallet = Wallet::from_seed_bytes(
            [0x4Bu8; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&wallet);
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
        let account_type = AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        };
        let address = dashcore::Address::new(
            Network::Testnet,
            Payload::PubkeyHash(PubkeyHash::from_byte_array([0x4Cu8; 20])),
        );
        assert!(!info.accounts.all_accounts().iter().any(|account| {
            account.managed_account_type().to_account_type() == account_type
                && account.contains_address(&address)
        }));
        let record = TransactionRecord::new(
            Transaction {
                version: 2,
                lock_time: 0,
                input: vec![],
                output: vec![TxOut {
                    value: 2_000,
                    script_pubkey: address.script_pubkey(),
                }],
                special_transaction_payload: None,
            },
            account_type,
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Incoming,
            Vec::new(),
            vec![OutputDetail {
                index: 0,
                role: OutputRole::Received,
                address: Some(address),
                value: 2_000,
            }],
            2_000,
        );
        apply_persisted_core_state(
            &mut info,
            &manifest,
            &CoreChangeSet {
                records: vec![record.clone()],
                account_records: vec![record],
                ..Default::default()
            },
            &Default::default(),
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .unwrap();
        assert_eq!(info.balance.total(), 0);
    }

    #[tokio::test]
    async fn doomed_delivery_then_confirmation_restores_the_same_balance_as_clean_confirmation() {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, OutPoint, Transaction, TxIn, TxOut, Txid};
        use key_wallet::account::{AccountType, StandardAccountType};
        use key_wallet::managed_account::transaction_record::OutputDetail;
        use key_wallet::transaction_checking::BlockInfo;
        use key_wallet::transaction_checking::WalletTransactionChecker;
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
        use key_wallet::Utxo;
        use platform_wallet::changeset::changeset::UtxoCreditVerdict;
        use rusqlite::params;

        let mut wallet = Wallet::from_seed_bytes(
            [0x48; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let manifest = manifest_for(&wallet);
        let skeleton = ManagedWalletInfo::from_wallet(&wallet, 1);
        let wallet_id = skeleton.wallet_id;
        let account_type = AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        };
        let address = WalletInfoInterface::monitored_addresses(&skeleton)
            .into_iter()
            .find(|address| {
                skeleton.accounts.all_accounts().iter().any(|account| {
                    account.managed_account_type().to_account_type() == account_type
                        && account.contains_address(address)
                })
            })
            .expect("BIP44 address");
        let transaction = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_byte_array([0x49; 32]),
                    vout: 0,
                },
                ..Default::default()
            }],
            output: vec![TxOut {
                value: 5_000,
                script_pubkey: address.script_pubkey(),
            }],
            special_transaction_payload: None,
        };
        let outpoint = OutPoint {
            txid: transaction.txid(),
            vout: 0,
        };
        let record = TransactionRecord::new(
            transaction.clone(),
            account_type,
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Incoming,
            Vec::new(),
            vec![OutputDetail {
                index: 0,
                role: OutputRole::Received,
                address: Some(address.clone()),
                value: 5_000,
            }],
            5_000,
        );
        let unconfirmed = Utxo {
            outpoint,
            txout: transaction.output[0].clone(),
            address: address.clone(),
            height: 0,
            is_coinbase: false,
            is_confirmed: false,
            is_instantlocked: false,
            is_locked: false,
            is_trusted: false,
        };
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![wallet_id.as_slice()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO core_address_pool \
             (wallet_id, account_type, account_index, script, pool_type, address_index, used) \
             VALUES (?1, 'standard_bip44', 0, ?2, 0, 0, 1)",
            params![wallet_id.as_slice(), address.script_pubkey().as_bytes()],
        )
        .unwrap();
        let tx = conn.transaction().unwrap();
        crate::sqlite::schema::core_state::apply(
            &tx,
            &wallet_id,
            &CoreChangeSet {
                records: vec![record.clone()],
                new_utxos: vec![unconfirmed.clone()],
                utxo_credit_verdicts: [(outpoint, UtxoCreditVerdict::Doomed)]
                    .into_iter()
                    .collect(),
                ..Default::default()
            },
        )
        .unwrap();
        tx.commit().unwrap();
        let (doomed, _, blocked) = crate::sqlite::schema::core_state::load_state(
            &conn,
            &wallet_id,
            Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        assert!(
            !blocked.contains_key(&outpoint),
            "Doomed is not a spend claim after restart"
        );
        let mut restored = skeleton.clone();
        apply_persisted_core_state(
            &mut restored,
            &manifest,
            &doomed,
            &Default::default(),
            &Default::default(),
            &blocked,
            &LoadCtx::strict(),
        )
        .unwrap();
        assert_eq!(restored.balance.total(), 0);

        let confirmed_context = TransactionContext::InChainLockedBlock(BlockInfo::new(
            101,
            BlockHash::from_byte_array([0x4A; 32]),
            1_700_000_000,
        ));
        let checked = restored
            .check_core_transaction(
                &transaction,
                confirmed_context.clone(),
                &mut wallet,
                true,
                true,
            )
            .await;
        assert!(checked.is_relevant);
        assert_eq!(restored.balance.total(), 5_000);
        let mut confirmed_record = record;
        confirmed_record.context = confirmed_context;
        let mut confirmed_utxo = unconfirmed;
        confirmed_utxo.height = 101;
        confirmed_utxo.is_confirmed = true;
        let clean = CoreChangeSet {
            records: vec![confirmed_record.clone()],
            new_utxos: vec![confirmed_utxo.clone()],
            ..Default::default()
        };
        let tx = conn.transaction().unwrap();
        crate::sqlite::schema::core_state::apply(&tx, &wallet_id, &clean).unwrap();
        tx.commit().unwrap();
        let (loaded, owners, spent) = crate::sqlite::schema::core_state::load_state(
            &conn,
            &wallet_id,
            Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        let mut reopened = skeleton.clone();
        apply_persisted_core_state(
            &mut reopened,
            &manifest,
            &loaded,
            &owners,
            &Default::default(),
            &spent,
            &LoadCtx::strict(),
        )
        .unwrap();
        let mut uninterrupted = skeleton;
        apply_persisted_core_state(
            &mut uninterrupted,
            &manifest,
            &clean,
            &owners,
            &Default::default(),
            &Default::default(),
            &LoadCtx::strict(),
        )
        .unwrap();
        assert_eq!(reopened.balance.total(), uninterrupted.balance.total());
        assert_eq!(reopened.balance.total(), restored.balance.total());
        assert_eq!(reopened.balance.total(), 5_000);
    }
}
