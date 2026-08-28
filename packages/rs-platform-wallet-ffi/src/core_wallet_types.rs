//! C-compatible types for core wallet changeset FFI.

use platform_wallet::masternode::{provider_payload_fields, MasternodeRecord};
use std::os::raw::c_char;

// ---------------------------------------------------------------------------
// OutPoint
// ---------------------------------------------------------------------------

/// Fixed-size outpoint: 32-byte txid + u32 vout.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OutPointFFI {
    pub txid: [u8; 32],
    pub vout: u32,
}

/// Outpoint of a TXO that was spent, paired with the spending
/// transaction's txid. Replaces the bare `OutPointFFI` on
/// `AccountChangeSetFFI.utxos_spent` so the Swift persister can
/// populate `PersistentTxo.spendingTransaction` (the column that
/// drives "Spent By" in the storage explorer and any per-tx
/// drill-down from the spent side of the chain).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SpentOutPointFFI {
    pub outpoint: OutPointFFI,
    pub spending_txid: [u8; 32],
}

// ---------------------------------------------------------------------------
// Chain state
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct ChainChangeSetFFI {
    pub has_synced_height: bool,
    pub synced_height: u32,
    pub has_block_hash: bool,
    pub block_hash: [u8; 32],
}

// ---------------------------------------------------------------------------
// Balance
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct BalanceChangeSetFFI {
    pub confirmed_delta: i64,
    pub unconfirmed_delta: i64,
    pub immature_delta: i64,
    pub locked_delta: i64,
}

// ---------------------------------------------------------------------------
// UTXO
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct UtxoEntryFFI {
    pub outpoint: OutPointFFI,
    pub amount: u64,
    pub address: *mut c_char,
    pub script_pubkey: *mut u8,
    pub script_pubkey_len: usize,
    pub height: u32,
    pub is_coinbase: bool,
    pub is_confirmed: bool,
    pub is_instantlocked: bool,
    pub is_locked: bool,
}

// ---------------------------------------------------------------------------
// Transaction
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct TransactionRecordFFI {
    pub txid: [u8; 32],
    pub tx_data: *mut u8,
    pub tx_data_len: usize,
    /// 0=mempool, 1=instantSend, 2=inBlock, 3=inChainLockedBlock.
    pub context: u32,
    pub block_height: u32,
    pub block_hash: [u8; 32],
    pub block_timestamp: u32,
    /// The transaction's index within its block (`block.vtx` order),
    /// meaningful only when `has_block_position` (stamped by block
    /// processing since rust-dashcore#891; absent on records confirmed
    /// before the field existed and on unconfirmed contexts). Persisted
    /// so restored provider special transactions keep Core's same-block
    /// apply order in the masternode aggregation.
    pub block_position: u32,
    pub has_block_position: bool,
    /// 0=incoming, 1=outgoing, 2=internal, 3=coinJoin.
    pub direction: u32,
    /// `transaction_type` rendered as a `Debug`-formatted string for
    /// human display (`"Standard"`, `"AssetLock"`, etc.). NOT stable
    /// across Rust version bumps — the typed discriminant is
    /// `transaction_type_kind`; the string is for UI only and should
    /// never be matched against.
    pub transaction_type: *mut c_char,
    /// Typed discriminant of `key_wallet::transaction_checking::
    /// transaction_router::TransactionType`:
    /// 0=Standard, 1=CoinJoin, 2=ProviderRegistration,
    /// 3=ProviderUpdateRegistrar, 4=ProviderUpdateService,
    /// 5=ProviderUpdateRevocation, 6=AssetLock, 7=AssetUnlock,
    /// 8=Coinbase, 9=Ignored. Stable wire shape — Swift
    /// `PersistentTransaction.isAssetLock` matches on this byte
    /// instead of regex-matching `transaction_type`'s Debug string.
    /// `0xFF` is the sentinel for "pre-feature row whose discriminant
    /// hasn't been populated yet"; treat as unknown.
    pub transaction_type_kind: u8,
    pub net_amount: i64,
    pub fee: u64,
    pub has_fee: bool,
    pub label: *mut c_char,
    pub first_seen: u64,
    /// Outpoints of every input in this transaction, in input index
    /// order. Always populated from `tx.input.iter()` directly so the
    /// list survives even when the wallet's in-memory `self.utxos`
    /// map didn't classify the input as "ours" at processing time.
    /// Required so the Swift persister can reconcile `(spending tx)
    /// ↔ (spent TXO)` even when the funding tx is processed after
    /// the spending tx (in-Swift out-of-order arrival), or when the
    /// funding output was persisted but never re-loaded into
    /// `self.utxos`. Iterating `input_details` (which only has
    /// entries for inputs that hit `self.utxos`) was the silent-drop
    /// path that left `PersistentTxo.isSpent` stuck at false. Empty
    /// for coinbase transactions.
    pub input_outpoints: *mut OutPointFFI,
    pub input_outpoints_count: usize,
    // --- Provider (masternode) special-transaction payload, UI only ---
    // Parsed once here from the DIP-3 special-tx payload so the Swift
    // side never decodes it. Populated only for ProRegTx
    // (`transaction_type_kind == 2`) and ProUpServTx (`== 4`); for every
    // other tx the string is null, the byte arrays are zeroed, and the
    // `has_*` flags are false.
    /// Masternode service endpoint rendered as `"ip:port"`, or null.
    pub provider_service_address: *mut c_char,
    /// ProUpServTx `pro_tx_hash` (32 bytes) linking the update to its
    /// registration. `has_provider_pro_tx_hash == false` for ProRegTx
    /// (whose own `txid` is the proTxHash) and for non-provider txs.
    pub provider_pro_tx_hash: [u8; 32],
    pub has_provider_pro_tx_hash: bool,
    /// ProRegTx collateral outpoint (`txid` in raw wire order + `vout`),
    /// gated by `has_provider_collateral`.
    pub provider_collateral_txid: [u8; 32],
    pub provider_collateral_vout: u32,
    pub has_provider_collateral: bool,
    /// ProRegTx owner / voting key hashes (hash160, 20 bytes each),
    /// each gated by its `has_*` flag.
    pub provider_owner_key_hash: [u8; 20],
    pub has_provider_owner_key_hash: bool,
    pub provider_voting_key_hash: [u8; 20],
    pub has_provider_voting_key_hash: bool,
}

// ---------------------------------------------------------------------------
// Per-account changeset
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct AccountChangeSetFFI {
    /// Account type name. Currently emitted as the `Debug` form of
    /// `AccountType` (e.g. `"Standard { index: 0,
    /// standard_account_type: BIP44Account }"`); kept for one extra
    /// release so any caller still string-matching against it
    /// doesn't break, but **not** used for upsert identity any more
    /// — Swift derives the display name from the typed tag fields
    /// below via the same helper the load path uses, so a single
    /// canonical name appears in the SwiftData row regardless of
    /// which path emitted it.
    pub account_type_name: *mut c_char,
    /// Account index (for indexed types, 0 otherwise).
    pub account_index: u32,
    /// `AccountType` discriminant. Stable across releases — the
    /// Swift persister keys upsert on `(wallet_id, type_tag,
    /// account_index, ...)` rather than on the legacy `Debug`
    /// `account_type_name` string, so a load-path emit and a
    /// changeset-path emit for the same account collapse onto a
    /// single SwiftData row.
    pub type_tag: crate::wallet_restore_types::AccountTypeTagFFI,
    /// Sub-discriminant for `type_tag == Standard`. Splits BIP44
    /// (0) from BIP32 (1). `Bip44` for non-Standard variants
    /// (ignored by Swift in that case).
    pub standard_tag: crate::wallet_restore_types::StandardAccountTypeTagFFI,
    /// `IdentityTopUp.registration_index`. `0` for other variants.
    pub registration_index: u32,
    /// `PlatformPayment.key_class`. `0` for other variants.
    pub key_class: u32,
    /// `Dashpay*.user_identity_id` (32 bytes). Zeroed for non-
    /// Dashpay variants.
    pub user_identity_id: [u8; 32],
    /// `Dashpay*.friend_identity_id` (32 bytes). Zeroed for non-
    /// Dashpay variants.
    pub friend_identity_id: [u8; 32],
    /// UTXOs added.
    pub utxos_added: *mut UtxoEntryFFI,
    pub utxos_added_count: usize,
    /// Outpoints of UTXOs spent.
    pub utxos_spent: *mut SpentOutPointFFI,
    pub utxos_spent_count: usize,
    /// Outpoints that became InstantSend-locked.
    pub utxos_instant_locked: *mut OutPointFFI,
    pub utxos_instant_locked_count: usize,
    /// Transactions added/updated.
    pub transactions: *mut TransactionRecordFFI,
    pub transactions_count: usize,
    /// Highest used address index per pool.
    pub external_highest_used: i32,
    pub has_external_highest_used: bool,
    pub internal_highest_used: i32,
    pub has_internal_highest_used: bool,
}

// ---------------------------------------------------------------------------
// Top-level wallet changeset
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct WalletChangeSetFFI {
    pub has_chain: bool,
    pub chain: ChainChangeSetFFI,
    pub has_balance: bool,
    pub balance: BalanceChangeSetFFI,
    pub accounts: *mut AccountChangeSetFFI,
    pub accounts_count: usize,
    /// Bincode-serialised `dashcore::ephemerealdata::chain_lock::ChainLock`
    /// (`bincode::config::standard()`) representing the wallet's
    /// `metadata.last_applied_chain_lock` after this round. `null` /
    /// `0` length when this changeset didn't advance the chainlock
    /// watermark. Persister writes the bytes to a dedicated SwiftData
    /// column on `PersistentWallet` so the wallet's CL metadata
    /// survives app restarts (otherwise it starts as `None` every
    /// launch and the asset-lock-resume CL-from-metadata fallback in
    /// `proof.rs` can't fire until SPV re-applies a fresh CL).
    pub last_applied_chain_lock_bytes: *mut u8,
    pub last_applied_chain_lock_bytes_len: usize,
}

// ---------------------------------------------------------------------------
// Conversion
// ---------------------------------------------------------------------------

impl WalletChangeSetFFI {
    /// Convert a platform-wallet [`CoreChangeSet`] into the C-ABI struct
    /// the Swift persister consumes.
    ///
    /// Buckets the changeset's flat `records` list by `record.account_type`,
    /// then derives per-account UTXO deltas at the time of conversion
    /// (using each record's `input_details` / `output_details`). The
    /// changeset's own `spent_utxos` / `new_utxos` vecs aren't read here:
    /// they're informationally redundant with the records themselves
    /// (the records are authoritative), and re-deriving keeps the
    /// per-account routing self-contained.
    ///
    /// `chain` carries `synced_height` from the changeset's
    /// `synced_height` field; `block_hash` is omitted because
    /// `WalletEvent::SyncHeightAdvanced` doesn't carry it (the upstream
    /// event is just a height watermark). `balance` is left absent —
    /// the new event-bus model derives balance from per-event balance
    /// snapshots delivered through the `BalanceUpdateHandler`, not as
    /// a delta on the persistence path.
    ///
    /// TODO(events): wire `instant_locks_for_non_final_records` through
    /// the FFI surface. Today the Swift side learns about IS-lock state
    /// only when a re-emitted `TransactionRecord` flows through `records`
    /// with `context = InstantSend(..)`. The standalone IS-lock map is
    /// dropped here. Acceptable as long as the event adapter re-emits
    /// affected records (it currently does for `TransactionDetected`
    /// and `BlockProcessed` but NOT for `TransactionInstantLocked`).
    /// When the standalone IS-lock event needs to flow to Swift, add
    /// a `BTreeMap`-shaped FFI field here and populate it.
    pub fn from_changeset(cs: &platform_wallet::changeset::CoreChangeSet) -> Self {
        use key_wallet::account::AccountType;
        use std::ffi::CString;

        // Chain — only synced_height flows through SyncHeightAdvanced.
        let (has_chain, chain) = match cs.synced_height {
            Some(h) => (
                true,
                ChainChangeSetFFI {
                    has_synced_height: true,
                    synced_height: h,
                    has_block_hash: false,
                    block_hash: [0u8; 32],
                },
            ),
            None => (
                false,
                ChainChangeSetFFI {
                    has_synced_height: false,
                    synced_height: 0,
                    has_block_hash: false,
                    block_hash: [0u8; 32],
                },
            ),
        };

        // Balance is no longer carried as a delta on the persistence
        // path; the BalanceUpdateHandler keeps wallet atomics current
        // from the post-event balance snapshot upstream embeds in each
        // event variant.
        let (has_balance, balance) = (
            false,
            BalanceChangeSetFFI {
                confirmed_delta: 0,
                unconfirmed_delta: 0,
                immature_delta: 0,
                locked_delta: 0,
            },
        );

        // Bucket records by account_type. Record sequence is preserved
        // within each bucket so the persister sees them in arrival
        // order (matters for the `inserted` -> `updated` transition
        // ordering inside a single BlockProcessed event).
        //
        // Two record sources fill each account's bucket:
        //  - transaction rows come from `records` — wallet-level,
        //    same-txid slices folded (dashpay/platform#4387). Each
        //    folded row is emitted into the bucket of EVERY account
        //    that owns a slice of its txid, not just the funding
        //    account's: the Swift/Kotlin per-account transaction
        //    callback is the sole writer of the tx↔account involvement
        //    join (`involvedAccounts` / `transaction_account_
        //    involvements`), which payload-only matches — a ProReg/
        //    ProUp payload hitting a provider owner or voting key, no
        //    TXO in the account — depend on for restart restoration.
        //    Funding-bucket-only emission dropped that involvement and
        //    provider transactions vanished from restoration until a
        //    rescan. The row's VALUES are identical in every bucket
        //    (the persisted row is txid-keyed and account-agnostic),
        //    so duplicate upserts converge; only the enclosing bucket
        //    differs, which is exactly what the involvement join
        //    records.
        //  - TXO deltas come from `account_records` — the raw
        //    per-account slices — so every UTXO lands in its OWNING
        //    account's bucket. Deriving TXOs from the folded record
        //    filed a sibling account's change under the funding
        //    account (`OutputDetail` carries no owning account), and
        //    the Swift/Kotlin stores then restored it into the wrong
        //    account's map. Changesets whose producer doesn't
        //    populate `account_records` fall back to `records`.
        //
        // `AccountType` doesn't implement `Ord` upstream (the
        // 256-bit `[u8; 32]` fields on the Dashpay variants would make
        // a derived ordering arbitrary), so a `Vec<(key, rows, slices)>`
        // with a linear "find or insert" walk is the path of least
        // resistance. Wallets typically have well under a hundred
        // accounts, so the linear search is cheap.
        let utxo_source: &Vec<key_wallet::managed_account::transaction_record::TransactionRecord> =
            if cs.account_records.is_empty() {
                &cs.records
            } else {
                &cs.account_records
            };
        #[allow(clippy::type_complexity)]
        let mut by_account: Vec<(
            AccountType,
            Vec<&key_wallet::managed_account::transaction_record::TransactionRecord>,
            Vec<&key_wallet::managed_account::transaction_record::TransactionRecord>,
        )> = Vec::new();
        for rec in &cs.records {
            // Every account with a slice of this txid is involved; the
            // record's own account (the funder) is a target even in
            // the no-slices fallback. Dedup keeps a bucket from
            // receiving the same row twice if a producer ever carries
            // a duplicate slice.
            let mut targets: Vec<AccountType> = vec![rec.account_type];
            for slice in utxo_source.iter().filter(|s| s.txid == rec.txid) {
                if !targets.contains(&slice.account_type) {
                    targets.push(slice.account_type);
                }
            }
            for target in targets {
                if let Some(bucket) = by_account.iter_mut().find(|(at, _, _)| at == &target) {
                    bucket.1.push(rec);
                } else {
                    by_account.push((target, vec![rec], Vec::new()));
                }
            }
        }
        for rec in utxo_source {
            if let Some(bucket) = by_account
                .iter_mut()
                .find(|(at, _, _)| at == &rec.account_type)
            {
                bucket.2.push(rec);
            } else {
                by_account.push((rec.account_type, Vec::new(), vec![rec]));
            }
        }

        // Watermark-only accounts still need an `AccountChangeSetFFI`
        // row. A batch can carry a highest-used advance for an account
        // with no record of its own this round — e.g. upstream's
        // `confirm_transaction` returning `None` for an idempotent
        // re-confirmation while `mark_address_used` still ran, or the
        // bridge's re-check resolving a sibling account of the same
        // category. Without an empty bucket the watermark would be
        // silently dropped below.
        for account_type in cs.account_highest_used.keys() {
            if !by_account.iter().any(|(at, _, _)| at == account_type) {
                by_account.push((*account_type, Vec::new(), Vec::new()));
            }
        }

        let mut ffi_accounts = Vec::with_capacity(by_account.len());
        for (account_type, tx_rows, utxo_slices) in by_account {
            let type_name = CString::new(format!("{:?}", account_type))
                .unwrap_or_else(|_| CString::new("Unknown").unwrap());
            let account_index = account_index_of(&account_type);

            // Derive UTXO add/spend lists from this account's SLICES.
            // Each slice carries its own account's input_details and
            // output_details; we walk them once per record to project
            // the UTXOs the persister should add or remove.
            let mut utxos_added: Vec<UtxoEntryFFI> = Vec::new();
            let mut utxos_spent: Vec<SpentOutPointFFI> = Vec::new();
            for rec in &utxo_slices {
                utxos_added.extend(record_new_utxos_ffi(rec));
                utxos_spent.extend(record_spent_outpoints_ffi(rec));
            }

            // Transaction rows for this account (wallet-level,
            // folded — see the bucketing comment above).
            let transactions: Vec<TransactionRecordFFI> =
                tx_rows.into_iter().map(tx_record_to_ffi).collect();

            let utxos_added_count = utxos_added.len();
            let utxos_spent_count = utxos_spent.len();
            let transactions_count = transactions.len();

            // Project the typed `AccountType` into the same flat tag
            // layout the load path's `AccountSpecFFI` already uses.
            // The Swift persister upserts on these typed fields
            // rather than on the legacy `Debug`-formatted
            // `account_type_name` string, so a load-path emit and a
            // sync-path emit for the same account collapse onto a
            // single SwiftData row.
            let tags = account_type_to_tags(&account_type);
            // Post-batch highest-used watermarks, captured by the
            // event bridge from the authoritative in-memory pools for
            // every account this batch marked an address used on
            // (`CoreChangeSet::account_highest_used`). `has_* = false`
            // means "no update this batch" — the Swift persister only
            // overwrites its row when the flag is set, so batches
            // without usage never regress a stored watermark.
            let highest = cs.account_highest_used.get(&account_type);
            let external_highest_used = highest.and_then(|h| h.external);
            let internal_highest_used = highest.and_then(|h| h.internal);
            ffi_accounts.push(AccountChangeSetFFI {
                account_type_name: type_name.into_raw(),
                account_index,
                type_tag: tags.type_tag,
                standard_tag: tags.standard_tag,
                registration_index: tags.registration_index,
                key_class: tags.key_class,
                user_identity_id: tags.user_identity_id,
                friend_identity_id: tags.friend_identity_id,
                utxos_added: vec_to_ptr(utxos_added),
                utxos_added_count,
                utxos_spent: vec_to_ptr(utxos_spent),
                utxos_spent_count,
                // IS-locked outpoints aren't carried as a separate
                // bucket on the new path — see TODO above.
                utxos_instant_locked: std::ptr::null_mut(),
                utxos_instant_locked_count: 0,
                transactions: vec_to_ptr(transactions),
                transactions_count,
                external_highest_used: external_highest_used.map_or(-1, |v| v as i32),
                has_external_highest_used: external_highest_used.is_some(),
                internal_highest_used: internal_highest_used.map_or(-1, |v| v as i32),
                has_internal_highest_used: internal_highest_used.is_some(),
            });
        }

        let accounts_count = ffi_accounts.len();

        // Bincode-serialise `last_applied_chain_lock` if present.
        // `ChainLock` derives `Encode` under upstream's `bincode`
        // feature; `bincode::encode_to_vec` cannot fail for plain
        // POD-shaped types, so a serialisation error here would
        // indicate an upstream `ChainLock` derive regression — fall
        // back to null and log so the persister round still
        // succeeds for the rest of the changeset.
        let (last_applied_chain_lock_bytes, last_applied_chain_lock_bytes_len) =
            match cs.last_applied_chain_lock.as_ref() {
                Some(cl) => match dpp::bincode::encode_to_vec(cl, dpp::bincode::config::standard())
                {
                    Ok(bytes) => {
                        let len = bytes.len();
                        let boxed = bytes.into_boxed_slice();
                        (Box::into_raw(boxed) as *mut u8, len)
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "Failed to bincode-encode last_applied_chain_lock; \
                             persister round will skip the watermark for this batch"
                        );
                        (std::ptr::null_mut(), 0)
                    }
                },
                None => (std::ptr::null_mut(), 0),
            };

        WalletChangeSetFFI {
            has_chain,
            chain,
            has_balance,
            balance,
            accounts: vec_to_ptr(ffi_accounts),
            accounts_count,
            last_applied_chain_lock_bytes,
            last_applied_chain_lock_bytes_len,
        }
    }
}

/// Returns the account "index" the FFI surfaces in `account_index`.
///
/// For variants with a natural index field (`Standard`, `CoinJoin`,
/// `IdentityTopUp`, `DashpayReceivingFunds`, `DashpayExternalAccount`,
/// `PlatformPayment`), returns that field. For singleton variants
/// (`IdentityRegistration`, `IdentityInvitation`, etc.), returns 0.
/// Matches the pre-event-bus behaviour where `AccountType::index()`
/// returned `Option<u32>` and singletons mapped to `None` → 0.
fn account_index_of(at: &key_wallet::account::AccountType) -> u32 {
    use key_wallet::account::AccountType;
    match at {
        AccountType::Standard { index, .. }
        | AccountType::CoinJoin { index }
        | AccountType::DashpayReceivingFunds { index, .. }
        | AccountType::DashpayExternalAccount { index, .. } => *index,
        AccountType::IdentityTopUp { registration_index } => *registration_index,
        AccountType::PlatformPayment { account, .. } => *account,
        _ => 0,
    }
}

/// Per-account balance entry returned by the query FFI. Carries the
/// same `AccountTypeTagFFI` discriminants as `AccountSpecFFI`, the four
/// balance fields from `WalletCoreBalance`, and address-pool key-usage
/// totals (`keys_used` / `keys_total`) summed across every pool on the
/// account. The pool counts are meaningful for both funds and keys
/// variants; the explorer surfaces them as the headline number on
/// keys-only rows where balance reads zero by construction.
#[repr(C)]
pub struct AccountBalanceEntryFFI {
    pub type_tag: crate::wallet_restore_types::AccountTypeTagFFI,
    pub standard_tag: crate::wallet_restore_types::StandardAccountTypeTagFFI,
    pub index: u32,
    pub registration_index: u32,
    pub key_class: u32,
    pub user_identity_id: [u8; 32],
    pub friend_identity_id: [u8; 32],
    pub confirmed: u64,
    pub unconfirmed: u64,
    pub immature: u64,
    pub locked: u64,
    pub keys_used: u32,
    pub keys_total: u32,
}

// ---------------------------------------------------------------------------
// Diagnostic snapshot FFI types
// ---------------------------------------------------------------------------
//
// All structs here are read-only diagnostic surfaces consumed by the
// iOS memory explorer. Each struct mirrors a `*Snapshot` type in
// `platform-wallet`'s `manager::accessors` module 1:1.

/// Snapshot of [`PlatformAddressSyncManager`] configuration / last-pass
/// timestamp. `last_event_wallet_count` was dropped — it aliased
/// `watch_list_size` and rendering it as an independent field invited
/// confused interpretation.
#[repr(C)]
pub struct PlatformAddressSyncConfigFFI {
    pub interval_seconds: u64,
    pub watch_list_size: usize,
    pub last_event_unix_seconds: u64,
}

/// Snapshot of [`IdentitySyncManager`] configuration / queue depth.
#[repr(C)]
pub struct IdentitySyncConfigFFI {
    pub interval_seconds: u64,
    pub queue_depth: usize,
}

/// Per-wallet core SPV state.
#[repr(C)]
pub struct CoreWalletStateFFI {
    pub synced_height: u32,
    pub last_processed_height: u32,
    pub monitor_revision: u64,
}

/// Per-wallet identity scan state.
#[repr(C)]
pub struct IdentityWalletStateFFI {
    pub last_scanned_index: u32,
    pub scan_pending: bool,
}

/// Per-wallet platform address provider state.
#[repr(C)]
pub struct PlatformAddressProviderStateFFI {
    pub initialized: bool,
    pub accounts_watched: usize,
    pub found_count: usize,
    pub known_balances_count: usize,
    pub watermark_height: u32,
}

// `WalletInfoMetadataFFI` was removed in lockstep with the explorer's
// "PlatformWalletInfo Metadata" section — every meaningful field
// duplicated `CoreWalletStateFFI` or had nothing populating it.

/// One row of the tracked-asset-lock list.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrackedAssetLockEntryFFI {
    pub outpoint_txid: [u8; 32],
    pub outpoint_vout: u32,
    /// 0=IdentityRegistration, 1=IdentityTopUp, 2=IdentityTopUpNotBound,
    /// 3=IdentityInvitation, 4=AssetLockAddressTopUp,
    /// 5=AssetLockShieldedAddressTopUp.
    pub lock_type: u8,
    /// 0=Built, 1=Broadcast, 2=InstantSendLocked, 3=ChainLocked.
    pub status: u8,
    pub registration_index: u32,
    pub instant_lock_present: bool,
    pub chain_lock_height: u32,
}

/// Snapshot of the per-account metadata for one account. Strings are
/// Per-account metadata snapshot.
///
/// `is_watch_only` and `custom_name` were dropped in lockstep with
/// upstream removing both fields from `ManagedCoreFundsAccount` /
/// `ManagedCoreKeysAccount`. Watch-only is now wallet-level (read off
/// `Wallet.wallet_type`); `AccountMetadata` no longer exists. The
/// struct is now plain-data — no heap-owned fields, no paired free fn
/// strictly required (kept as a stable no-op).
#[repr(C)]
pub struct AccountMetadataFFI {
    pub total_transactions: u64,
    pub total_utxos: u64,
    pub monitor_revision: u64,
}

/// One address row inside [`AccountAddressPoolEntryFFI`]. The pool's
/// own free fn walks the nested array and reclaims it.
///
/// `address` is a heap-owned NUL-terminated UTF-8 string;
/// `public_key_bytes` is a heap-owned byte buffer (`null` +
/// `public_key_bytes_len = 0` when the pool entry didn't retain the
/// derivation source). Both are freed by the parent pool's free fn.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AddressInfoFFI {
    pub pubkey_hash: [u8; 20],
    pub address_index: u32,
    pub is_used: bool,
    /// `last_used_height` is reserved on the FFI — upstream
    /// `AddressInfo` doesn't currently track per-address height. Set
    /// to `0`; will be populated when upstream gains the field.
    pub last_used_height: u32,
    pub address: *mut c_char,
    pub public_key_bytes: *mut u8,
    pub public_key_bytes_len: usize,
}

/// One pool-level entry inside the per-account address pool snapshot.
/// `addresses` is a heap-owned slice of `AddressInfoFFI`, freed by the
/// paired free fn (which walks every pool first).
#[repr(C)]
pub struct AccountAddressPoolEntryFFI {
    /// 0=External, 1=Internal, 2=Absent, 3=AbsentHardened.
    pub pool_type: u8,
    pub gap_limit: u32,
    /// `i64`-encoded; `-1` signals "no addresses used yet".
    pub last_used_index: i64,
    pub addresses: *mut AddressInfoFFI,
    pub addresses_count: usize,
}

/// One UTXO row in the per-account drill-down. `script_pubkey` is
/// heap-owned and freed by the paired free fn.
#[repr(C)]
pub struct AccountUtxoEntryFFI {
    pub outpoint_txid: [u8; 32],
    pub outpoint_vout: u32,
    pub value_duffs: u64,
    pub script_pubkey: *mut u8,
    pub script_pubkey_len: usize,
    pub height: u32,
    pub is_locked: bool,
}

/// One transaction row in the per-account paginated drill-down.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AccountTransactionEntryFFI {
    pub txid: [u8; 32],
    pub height: u32,
    pub timestamp: u64,
    pub value_delta_duffs: i64,
    pub fee_duffs: u64,
    pub is_coinbase: bool,
}

/// One row of the wallet-bound identity list (registration index +
/// identity id).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WalletIdentityRowFFI {
    pub registration_index: u32,
    pub identity_id: [u8; 32],
}

/// One row of the DAPI address ban-list snapshot.
///
/// `address` is a heap-owned NUL-terminated UTF-8 string (the node
/// URI); `reason` is a heap-owned NUL-terminated UTF-8 string or
/// `null` when no ban reason was recorded. Both are freed by the
/// paired `platform_wallet_manager_address_ban_info_free`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AddressBanInfoFFI {
    /// Heap-owned node URI string. Always non-null on a successful row.
    pub address: *mut c_char,
    /// Whether the address is currently effectively banned.
    pub banned: bool,
    /// Total number of times the address has been banned.
    pub ban_count: u32,
    /// Unix-epoch millisecond timestamp until which the address is
    /// banned; `0` when there is no active ban window.
    pub banned_until_ms: i64,
    /// Heap-owned human-readable ban reason, or `null` when none was
    /// recorded.
    pub reason: *mut c_char,
}

/// Subset of [`crate::wallet_restore_types::AccountSpecFFI`] carrying
/// only the tag/discriminator fields — no xpub. Used by the
/// changeset emit path to populate
/// [`AccountChangeSetFFI`]'s typed tags so the Swift persister can
/// upsert on the same composite key the load path uses. Also used by
/// [`AccountBalanceEntryFFI`] to carry per-account routing context
/// for balance queries.
pub struct AccountChangeSetTags {
    pub type_tag: crate::wallet_restore_types::AccountTypeTagFFI,
    pub standard_tag: crate::wallet_restore_types::StandardAccountTypeTagFFI,
    pub index: u32,
    pub registration_index: u32,
    pub key_class: u32,
    pub user_identity_id: [u8; 32],
    pub friend_identity_id: [u8; 32],
}

/// Project an upstream [`AccountType`] into the flat FFI tag layout.
///
/// Mirrors [`build_account_spec_ffi`](crate::persistence::build_account_spec_ffi)'s
/// match arms but emits only the tag/discriminator fields — the
/// xpub is load-path-only and not relevant on the changeset emit
/// path.
pub fn account_type_to_tags(at: &key_wallet::account::AccountType) -> AccountChangeSetTags {
    use crate::wallet_restore_types::{AccountTypeTagFFI, StandardAccountTypeTagFFI};
    use key_wallet::account::{AccountType, StandardAccountType};
    let mut tags = AccountChangeSetTags {
        type_tag: AccountTypeTagFFI::Standard,
        standard_tag: StandardAccountTypeTagFFI::Bip44,
        index: 0,
        registration_index: 0,
        key_class: 0,
        user_identity_id: [0u8; 32],
        friend_identity_id: [0u8; 32],
    };
    match at {
        AccountType::Standard {
            index,
            standard_account_type,
        } => {
            tags.index = *index;
            tags.type_tag = AccountTypeTagFFI::Standard;
            tags.standard_tag = match standard_account_type {
                StandardAccountType::BIP44Account => StandardAccountTypeTagFFI::Bip44,
                StandardAccountType::BIP32Account => StandardAccountTypeTagFFI::Bip32,
            };
        }
        AccountType::CoinJoin { index } => {
            tags.type_tag = AccountTypeTagFFI::CoinJoin;
            tags.index = *index;
        }
        AccountType::IdentityRegistration => {
            tags.type_tag = AccountTypeTagFFI::IdentityRegistration;
        }
        AccountType::IdentityTopUp { registration_index } => {
            tags.type_tag = AccountTypeTagFFI::IdentityTopUp;
            tags.registration_index = *registration_index;
        }
        AccountType::IdentityTopUpNotBoundToIdentity => {
            tags.type_tag = AccountTypeTagFFI::IdentityTopUpNotBoundToIdentity;
        }
        AccountType::IdentityInvitation => {
            tags.type_tag = AccountTypeTagFFI::IdentityInvitation;
        }
        AccountType::AssetLockAddressTopUp => {
            tags.type_tag = AccountTypeTagFFI::AssetLockAddressTopUp;
        }
        AccountType::AssetLockShieldedAddressTopUp => {
            tags.type_tag = AccountTypeTagFFI::AssetLockShieldedAddressTopUp;
        }
        AccountType::ProviderVotingKeys => {
            tags.type_tag = AccountTypeTagFFI::ProviderVotingKeys;
        }
        AccountType::ProviderOwnerKeys => {
            tags.type_tag = AccountTypeTagFFI::ProviderOwnerKeys;
        }
        AccountType::ProviderOperatorKeys => {
            tags.type_tag = AccountTypeTagFFI::ProviderOperatorKeys;
        }
        AccountType::ProviderPlatformKeys => {
            tags.type_tag = AccountTypeTagFFI::ProviderPlatformKeys;
        }
        AccountType::DashpayReceivingFunds {
            index,
            user_identity_id,
            friend_identity_id,
        } => {
            tags.type_tag = AccountTypeTagFFI::DashpayReceivingFunds;
            tags.index = *index;
            tags.user_identity_id = *user_identity_id;
            tags.friend_identity_id = *friend_identity_id;
        }
        AccountType::DashpayExternalAccount {
            index,
            user_identity_id,
            friend_identity_id,
        } => {
            tags.type_tag = AccountTypeTagFFI::DashpayExternalAccount;
            tags.index = *index;
            tags.user_identity_id = *user_identity_id;
            tags.friend_identity_id = *friend_identity_id;
        }
        AccountType::PlatformPayment { account, key_class } => {
            tags.type_tag = AccountTypeTagFFI::PlatformPayment;
            tags.index = *account;
            tags.key_class = *key_class;
        }
    }
    tags
}

/// Project the "ours" outputs of a `TransactionRecord` into FFI UTXO
/// entries. Mirrors `derive_new_utxos` in
/// `platform_wallet::changeset::core_bridge` but stops one layer
/// further down the stack so the FFI conversion stays self-contained.
fn record_new_utxos_ffi(
    rec: &key_wallet::managed_account::transaction_record::TransactionRecord,
) -> Vec<UtxoEntryFFI> {
    use key_wallet::managed_account::transaction_record::OutputRole;
    use key_wallet::transaction_checking::TransactionContext;
    use std::ffi::CString;

    let height = rec.context.block_info().map(|b| b.height()).unwrap_or(0);
    let is_confirmed = matches!(
        rec.context,
        TransactionContext::InBlock(_) | TransactionContext::InChainLockedBlock(_)
    );
    let is_instant = matches!(rec.context, TransactionContext::InstantSend(_));
    let is_coinbase = rec.transaction.is_coin_base();

    rec.output_details
        .iter()
        .filter_map(|d| {
            if !matches!(d.role, OutputRole::Received | OutputRole::Change) {
                return None;
            }
            let txout = rec.transaction.output.get(d.index as usize)?;
            let address_str = d
                .address
                .as_ref()
                .map(|a| a.to_string())
                .unwrap_or_default();
            let address = CString::new(address_str).unwrap_or_else(|_| CString::new("").unwrap());
            let script_bytes = txout.script_pubkey.as_bytes().to_vec();
            let script_len = script_bytes.len();
            let script_ptr = vec_to_ptr_u8(script_bytes, script_len);
            let mut txid = [0u8; 32];
            txid.copy_from_slice(rec.txid.as_ref());
            Some(UtxoEntryFFI {
                outpoint: OutPointFFI {
                    txid,
                    vout: d.index,
                },
                amount: txout.value,
                address: address.into_raw(),
                script_pubkey: script_ptr,
                script_pubkey_len: script_len,
                height,
                is_coinbase,
                is_confirmed,
                is_instantlocked: is_instant,
                is_locked: false,
            })
        })
        .collect()
}

/// Project the outpoints spent by a `TransactionRecord` (i.e. the
/// outpoints whose UTXO rows the persister should mark spent),
/// paired with the spending transaction's txid so the Swift
/// persister can populate `PersistentTxo.spendingTransaction`.
fn record_spent_outpoints_ffi(
    rec: &key_wallet::managed_account::transaction_record::TransactionRecord,
) -> Vec<SpentOutPointFFI> {
    let mut spending_txid = [0u8; 32];
    spending_txid.copy_from_slice(rec.txid.as_ref());
    rec.input_details
        .iter()
        .filter_map(|d| {
            let input = rec.transaction.input.get(d.index as usize)?;
            let mut txid = [0u8; 32];
            txid.copy_from_slice(input.previous_output.txid.as_ref());
            Some(SpentOutPointFFI {
                outpoint: OutPointFFI {
                    txid,
                    vout: input.previous_output.vout,
                },
                spending_txid,
            })
        })
        .collect()
}

/// Map upstream `TransactionType` to a stable `u8` discriminant for
/// the FFI wire shape. Order mirrors the enum declaration in
/// `key_wallet::transaction_checking::transaction_router::mod.rs`,
/// pinned here so a future variant addition surfaces as a compile
/// error (the exhaustive match has no wildcard arm).
///
/// Stable contract: this byte is the Swift side's typed discriminant
/// for `PersistentTransaction.isAssetLock` / `isAssetUnlock`. Keep
/// it in sync with `TransactionTypeKind` in
/// `swift-sdk/Sources/SwiftDashSDK/Persistence/Models/PersistentTransaction.swift`
/// — every new variant added here must also gain a Swift enum case.
fn transaction_type_to_u8(
    ty: &key_wallet::transaction_checking::transaction_router::TransactionType,
) -> u8 {
    use key_wallet::transaction_checking::transaction_router::TransactionType;
    match ty {
        TransactionType::Standard => 0,
        TransactionType::CoinJoin => 1,
        TransactionType::ProviderRegistration => 2,
        TransactionType::ProviderUpdateRegistrar => 3,
        TransactionType::ProviderUpdateService => 4,
        TransactionType::ProviderUpdateRevocation => 5,
        TransactionType::AssetLock => 6,
        TransactionType::AssetUnlock => 7,
        TransactionType::Coinbase => 8,
        TransactionType::Ignored => 9,
    }
}

/// Flat, C-ABI masternode entity — the wire shape of one
/// [`MasternodeRecord`], built by [`masternode_entry_ffi`] and
/// returned by `platform_wallet_manager_list_masternodes`. Inline
/// fixed-size hashes with `has_*` gates (mirroring `TransactionRecordFFI`)
/// keep heap ownership to the C strings.
///
/// # ABI stability
///
/// This is the original, frozen layout returned by the unversioned
/// `platform_wallet_manager_list_masternodes` entry point. Do not add, remove,
/// or reorder fields. New projections belong in a versioned wrapper such as
/// [`MasternodeEntryV2FFI`].
#[repr(C)]
pub struct MasternodeEntryFFI {
    /// proTxHash (32 wire bytes) — group key; also the registration txid.
    pub pro_tx_hash: [u8; 32],
    /// Whether a ProRegTx was in the aggregated set (vs update-only).
    pub has_registration: bool,
    /// ProRegTx core height (0 when unseen).
    pub registration_height: u32,
    /// Stable cross-type registration-order position (sort key).
    pub order_index: u32,
    /// 1-based index within this masternode's type — `is_evonode`
    /// selects the "Evonode N" vs "Masternode N" sequence.
    pub type_index: u32,
    /// evonode / HPMN flag.
    pub is_evonode: bool,
    /// A ProUpRevTx was seen ⇒ revoked (data only; NOT the displayed
    /// status — see `status`).
    pub revoked: bool,
    pub revocation_reason: u16,
    /// DML-derived status discriminant: 0 Active, 1 Inactive, 2 Retired,
    /// 3 Unknown (DML unavailable ⇒ persist layer keeps the prior value).
    pub status: u8,
    /// Provider-tx count for this proTxHash.
    pub tx_count: u32,
    /// Collateral outpoint, gated by `has_collateral`.
    pub collateral_txid: [u8; 32],
    pub collateral_vout: u32,
    pub has_collateral: bool,
    /// Owner / voting key hashes (hash160), each gated by its `has_*`.
    pub owner_key_hash: [u8; 20],
    pub has_owner_key_hash: bool,
    pub voting_key_hash: [u8; 20],
    pub has_voting_key_hash: bool,
    /// Service endpoint `"ip:port"`, or null.
    pub service_address: *mut c_char,
    /// Platform HTTP (DAPI gRPC) port from the latest ProRegTx / ProUpServTx,
    /// gated by `has_platform_http_port` (evonodes only). Together with the
    /// `service_address` host this addresses the node's DAPI.
    pub platform_http_port: u16,
    pub has_platform_http_port: bool,
    /// Base58 owner / voting P2PKH addresses for the wallet's network
    /// (null when the hash is absent) — the app-layer join key against a
    /// provider-key account's persisted base58 address, so Swift never
    /// hashes keys.
    pub owner_address: *mut c_char,
    pub voting_address: *mut c_char,
    /// Operator BLS public key (48 bytes), gated by `has_operator_key`.
    pub operator_public_key: [u8; 48],
    pub has_operator_key: bool,
    /// Platform node id (SHA256[..20] Tenderdash, #884, 20 bytes) — evonodes only — gated by
    /// `has_platform_node_id`.
    pub platform_node_id: [u8; 20],
    pub has_platform_node_id: bool,
    /// Base58 payout address for the network, or null (non-standard
    /// payout script, or none seen).
    pub payout_address: *mut c_char,
    // --- Base58 P2PKH addresses of the operator / platform keys ---
    // Owner / voting addresses are `owner_address` / `voting_address`
    // above. These two complete the set so the app can join ALL FOUR key
    // kinds against persisted `PersistentCoreAddress` rows (address ⇒
    // account type + index) — the durable ownership source. Pure encoding,
    // no in-wallet lookup on the Rust side.
    /// Base58 P2PKH pseudo-address of `hash160(operator BLS key)`, or null.
    pub operator_pseudo_address: *mut c_char,
    /// Base58 P2PKH address of the platform node id (evonode), or null.
    pub platform_node_address: *mut c_char,
    // --- Operator / platform ownership (derive-and-compare) ---
    // These key kinds live only in payloads (no on-chain address), so
    // their ownership is derived in Rust and matched here. `*_account_type`
    // is the AccountTypeTagFFI value (10 ProviderOperatorKeys,
    // 11 ProviderPlatformKeys); meaningful only when `*_in_wallet`.
    pub operator_in_wallet: bool,
    pub operator_account_type: u8,
    pub operator_key_index: u32,
    pub platform_in_wallet: bool,
    pub platform_account_type: u8,
    pub platform_key_index: u32,
    /// Whether the platform-node ownership check was actually *possible* for
    /// this query: `true` when the wallet's derived platform-node index had
    /// entries to compare against, `false` when it was empty/unavailable (no
    /// platform pool, or a seedless restore before the persisted key batch
    /// rehydrated it). Lets the persister distinguish a definitive
    /// `platform_in_wallet == false` (checked, not ours — e.g. an on-chain
    /// key rotation to an external node) from "couldn't check yet", so it
    /// never leaves stale ownership set. See `MasternodeSync`.
    pub platform_ownership_checked: bool,
}

/// Version 2 masternode projection. The frozen V1 entry remains the first
/// field, preserving one canonical definition for all established fields;
/// V2 adds record provenance and the optional tracked-node label.
#[repr(C)]
pub struct MasternodeEntryV2FFI {
    pub v1: MasternodeEntryFFI,
    /// Where this record came from: 0 = one of the wallet's own masternodes
    /// (aggregated from its provider transactions), 1 = tracked by the user
    /// independently of every wallet.
    pub source: u8,
    /// User label of a tracked masternode, or null.
    pub label: *mut c_char,
}

/// Encode a hash160 as a network-specific base58 P2PKH address string
/// (heap C string), or null on the (impossible-for-a-valid-hash) CString
/// interior-nul error.
fn masternode_p2pkh_cstring(hash: [u8; 20], network: dashcore::Network) -> *mut c_char {
    use dashcore::address::Payload;
    use dashcore::hashes::Hash;
    use dashcore::PubkeyHash;
    let address = dashcore::Address::new(
        network,
        Payload::PubkeyHash(PubkeyHash::from_byte_array(hash)),
    );
    std::ffi::CString::new(address.to_string())
        .map(std::ffi::CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// Encode a payout `script` to its base58 address for `network`, or null
/// when the script is non-standard (not addressable).
fn masternode_payout_cstring(script_bytes: &[u8], network: dashcore::Network) -> *mut c_char {
    let script = dashcore::ScriptBuf::from_bytes(script_bytes.to_vec());
    match dashcore::Address::from_script(&script, network) {
        Ok(address) => std::ffi::CString::new(address.to_string())
            .map(std::ffi::CString::into_raw)
            .unwrap_or(std::ptr::null_mut()),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Flatten one record into its C-ABI entry, encoding the owner / voting /
/// payout / operator / platform-node base58 addresses for `network`. Pure
/// marshalling: ordering, status and operator / platform key ownership are
/// already resolved on the record by
/// `PlatformWalletManager::wallet_masternodes_blocking`; owner / voting key
/// ownership is resolved app-side (persisted-address join).
pub(crate) fn masternode_entry_ffi(
    mn: &MasternodeRecord,
    network: dashcore::Network,
) -> MasternodeEntryFFI {
    use dashcore::hashes::{hash160, Hash};
    use std::ffi::CString;

    // Operator pseudo-address = P2PKH of hash160(BLS key); platform-node
    // address = P2PKH of the 20-byte node id.
    let operator_pseudo_address = mn
        .operator_public_key
        .map(|k| {
            let h: [u8; 20] = hash160::Hash::hash(&k).to_byte_array();
            masternode_p2pkh_cstring(h, network)
        })
        .unwrap_or(std::ptr::null_mut());
    let platform_node_address = mn
        .platform_node_id
        .map(|h| masternode_p2pkh_cstring(h, network))
        .unwrap_or(std::ptr::null_mut());

    // Ownership flags from the record's resolved indexes. `*_account_type`
    // is the AccountTypeTagFFI value (10 ProviderOperatorKeys,
    // 11 ProviderPlatformKeys); meaningful only when `*_in_wallet`.
    let (operator_in_wallet, operator_account_type, operator_key_index) = mn
        .operator_key_index
        .map(|index| (true, 10u8, index))
        .unwrap_or((false, 0, 0));
    let (platform_in_wallet, platform_account_type, platform_key_index) = mn
        .platform_key_index
        .map(|index| (true, 11u8, index))
        .unwrap_or((false, 0, 0));

    let service_address = match &mn.service_address {
        Some(s) => CString::new(s.clone())
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut()),
        None => std::ptr::null_mut(),
    };
    let (collateral_txid, collateral_vout, has_collateral) = match mn.collateral {
        Some((txid, vout)) => (txid, vout, true),
        None => ([0u8; 32], 0, false),
    };
    let owner_address = mn
        .owner_key_hash
        .map(|h| masternode_p2pkh_cstring(h, network))
        .unwrap_or(std::ptr::null_mut());
    let voting_address = mn
        .voting_key_hash
        .map(|h| masternode_p2pkh_cstring(h, network))
        .unwrap_or(std::ptr::null_mut());
    let payout_address = mn
        .payout_script
        .as_deref()
        .map(|s| masternode_payout_cstring(s, network))
        .unwrap_or(std::ptr::null_mut());

    MasternodeEntryFFI {
        pro_tx_hash: mn.pro_tx_hash,
        has_registration: mn.has_registration,
        registration_height: mn.registration_height,
        order_index: mn.order_index,
        type_index: mn.type_index,
        is_evonode: mn.is_evonode,
        revoked: mn.revoked,
        revocation_reason: mn.revocation_reason,
        status: mn.status.as_u8(),
        tx_count: mn.tx_count,
        collateral_txid,
        collateral_vout,
        has_collateral,
        owner_key_hash: mn.owner_key_hash.unwrap_or([0u8; 20]),
        has_owner_key_hash: mn.owner_key_hash.is_some(),
        voting_key_hash: mn.voting_key_hash.unwrap_or([0u8; 20]),
        has_voting_key_hash: mn.voting_key_hash.is_some(),
        service_address,
        platform_http_port: mn.platform_http_port.unwrap_or(0),
        has_platform_http_port: mn.platform_http_port.is_some(),
        owner_address,
        voting_address,
        operator_public_key: mn.operator_public_key.unwrap_or([0u8; 48]),
        has_operator_key: mn.operator_public_key.is_some(),
        platform_node_id: mn.platform_node_id.unwrap_or([0u8; 20]),
        has_platform_node_id: mn.platform_node_id.is_some(),
        payout_address,
        operator_pseudo_address,
        platform_node_address,
        operator_in_wallet,
        operator_account_type,
        operator_key_index,
        platform_in_wallet,
        platform_account_type,
        platform_key_index,
        platform_ownership_checked: mn.platform_ownership_checked,
    }
}

/// Flatten one record into the additive V2 C-ABI entry.
pub(crate) fn masternode_entry_v2_ffi(
    mn: &MasternodeRecord,
    network: dashcore::Network,
) -> MasternodeEntryV2FFI {
    use std::ffi::CString;

    MasternodeEntryV2FFI {
        v1: masternode_entry_ffi(mn, network),
        source: mn.source.as_u8(),
        label: mn
            .label
            .clone()
            .and_then(|label| CString::new(label).ok())
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut()),
    }
}

fn tx_record_to_ffi(
    tr: &key_wallet::managed_account::transaction_record::TransactionRecord,
) -> TransactionRecordFFI {
    use key_wallet::transaction_checking::TransactionContext;
    use std::ffi::CString;

    let tx_bytes = dashcore::consensus::encode::serialize(&tr.transaction);
    let tx_len = tx_bytes.len();
    let tx_ptr = Box::into_raw(tx_bytes.into_boxed_slice()) as *mut u8;

    let mut txid = [0u8; 32];
    txid.copy_from_slice(tr.txid.as_ref());

    let (ctx_val, blk_height, blk_hash, blk_ts, blk_position, has_blk_position) = match &tr.context
    {
        TransactionContext::Mempool => (0u32, 0u32, [0u8; 32], 0u32, 0u32, false),
        TransactionContext::InstantSend(_is_lock) => {
            // InstantSend has no block info — treat as mempool-level with flag
            (1u32, 0u32, [0u8; 32], 0u32, 0u32, false)
        }
        TransactionContext::InBlock(bi) => {
            let mut h = [0u8; 32];
            h.copy_from_slice(bi.block_hash().as_ref());
            (
                2u32,
                bi.height(),
                h,
                bi.timestamp(),
                bi.position().unwrap_or(0),
                bi.position().is_some(),
            )
        }
        TransactionContext::InChainLockedBlock(bi) => {
            let mut h = [0u8; 32];
            h.copy_from_slice(bi.block_hash().as_ref());
            (
                3u32,
                bi.height(),
                h,
                bi.timestamp(),
                bi.position().unwrap_or(0),
                bi.position().is_some(),
            )
        }
    };

    let dir_val = match tr.direction {
        key_wallet::managed_account::transaction_record::TransactionDirection::Incoming => 0u32,
        key_wallet::managed_account::transaction_record::TransactionDirection::Outgoing => 1,
        key_wallet::managed_account::transaction_record::TransactionDirection::Internal => 2,
        key_wallet::managed_account::transaction_record::TransactionDirection::CoinJoin => 3,
    };

    let type_str = CString::new(format!("{:?}", tr.transaction_type))
        .unwrap_or_else(|_| CString::new("Unknown").unwrap());
    let type_kind = transaction_type_to_u8(&tr.transaction_type);
    let label_str = CString::new(tr.label.clone()).unwrap_or_else(|_| CString::new("").unwrap());

    // Build the input-outpoint slice from `tx.input` directly. NOT from
    // `input_details` — that field only carries entries the wallet
    // recognized as "ours" at processing time (i.e., entries whose
    // `previous_output` was already in `self.utxos`), so it silently
    // drops the spent-outpoint signal whenever the funding tx hadn't
    // populated the UTXO map yet (in-Swift out-of-order arrival, or
    // a load_from_persistor that didn't fully repopulate). The Swift
    // persister reconciles this list against its own `PersistentTxo`
    // table to mark the spend, so emitting every input — even ones
    // we don't currently classify as ours — is correct: outpoint is
    // a globally-unique key and Swift's lookup is a no-op when no
    // matching row exists. Coinbase inputs are skipped (the previous
    // output of the synthetic coinbase outpoint is never one of ours).
    let input_outpoints_vec: Vec<OutPointFFI> = if tr.transaction.is_coin_base() {
        Vec::new()
    } else {
        tr.transaction
            .input
            .iter()
            .map(|input| {
                let mut prev_txid = [0u8; 32];
                prev_txid.copy_from_slice(input.previous_output.txid.as_ref());
                OutPointFFI {
                    txid: prev_txid,
                    vout: input.previous_output.vout,
                }
            })
            .collect()
    };
    let input_outpoints_count = input_outpoints_vec.len();
    let input_outpoints = vec_to_ptr(input_outpoints_vec);

    // Provider (masternode) payload projection. Non-provider txs yield
    // all-`None`, i.e. null string / zeroed arrays / `has_* = false`.
    let provider = provider_payload_fields(&tr.transaction);
    let provider_service_address = match provider.service_address {
        Some(s) => CString::new(s)
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut()),
        None => std::ptr::null_mut(),
    };
    let (provider_collateral_txid, provider_collateral_vout, has_provider_collateral) =
        match provider.collateral {
            Some((txid, vout)) => (txid, vout, true),
            None => ([0u8; 32], 0, false),
        };

    TransactionRecordFFI {
        txid,
        tx_data: tx_ptr,
        tx_data_len: tx_len,
        context: ctx_val,
        block_height: blk_height,
        block_hash: blk_hash,
        block_timestamp: blk_ts,
        block_position: blk_position,
        has_block_position: has_blk_position,
        direction: dir_val,
        transaction_type: type_str.into_raw(),
        transaction_type_kind: type_kind,
        net_amount: tr.net_amount,
        fee: tr.fee.unwrap_or(0),
        has_fee: tr.fee.is_some(),
        label: label_str.into_raw(),
        // `first_seen` was removed from upstream `TransactionRecord` in
        // the event-bus refactor — there's no equivalent timestamp on
        // the new type. The Swift persister still indexes by it, so we
        // surface the block timestamp when the record is confirmed
        // (a usable proxy for "first seen" in the in-block case) and 0
        // for mempool / instant-send records, which the Swift side can
        // refresh from `Date.now()` on insert if it needs a real
        // observation timestamp.
        first_seen: blk_ts as u64,
        input_outpoints,
        input_outpoints_count,
        provider_service_address,
        provider_pro_tx_hash: provider.pro_tx_hash.unwrap_or([0u8; 32]),
        has_provider_pro_tx_hash: provider.pro_tx_hash.is_some(),
        provider_collateral_txid,
        provider_collateral_vout,
        has_provider_collateral,
        provider_owner_key_hash: provider.owner_key_hash.unwrap_or([0u8; 20]),
        has_provider_owner_key_hash: provider.owner_key_hash.is_some(),
        provider_voting_key_hash: provider.voting_key_hash.unwrap_or([0u8; 20]),
        has_provider_voting_key_hash: provider.voting_key_hash.is_some(),
    }
}

fn vec_to_ptr<T>(v: Vec<T>) -> *mut T {
    if v.is_empty() {
        std::ptr::null_mut()
    } else {
        Box::into_raw(v.into_boxed_slice()) as *mut T
    }
}

fn vec_to_ptr_u8(v: Vec<u8>, _len: usize) -> *mut u8 {
    if v.is_empty() {
        std::ptr::null_mut()
    } else {
        Box::into_raw(v.into_boxed_slice()) as *mut u8
    }
}

// ---------------------------------------------------------------------------
// Free
// ---------------------------------------------------------------------------

/// Free all heap allocations in a `WalletChangeSetFFI`.
///
/// # Safety
/// Must only be called once per changeset.
pub unsafe fn free_wallet_changeset_ffi(cs: &WalletChangeSetFFI) {
    use std::ffi::CString;

    // Top-level chain-lock bytes free path runs regardless of
    // whether the changeset carried any account-level deltas — a
    // round that ONLY advances the watermark (chain-lock arm with
    // empty per_account on the WalletEvent side wouldn't fire, but
    // a coalesced round may still resolve down to "just the CL"
    // after `is_empty_no_records` filters out the other arms) must
    // still release the heap allocation we made in `from_changeset`.
    if !cs.last_applied_chain_lock_bytes.is_null() && cs.last_applied_chain_lock_bytes_len > 0 {
        drop(Vec::from_raw_parts(
            cs.last_applied_chain_lock_bytes,
            cs.last_applied_chain_lock_bytes_len,
            cs.last_applied_chain_lock_bytes_len,
        ));
    }

    if cs.accounts.is_null() || cs.accounts_count == 0 {
        return;
    }
    let accounts = std::slice::from_raw_parts(cs.accounts, cs.accounts_count);
    for acc in accounts {
        if !acc.account_type_name.is_null() {
            let _ = CString::from_raw(acc.account_type_name);
        }
        // UTXOs added
        if !acc.utxos_added.is_null() && acc.utxos_added_count > 0 {
            let utxos = std::slice::from_raw_parts(acc.utxos_added, acc.utxos_added_count);
            for u in utxos {
                if !u.address.is_null() {
                    let _ = CString::from_raw(u.address);
                }
                if !u.script_pubkey.is_null() && u.script_pubkey_len > 0 {
                    drop(Vec::from_raw_parts(
                        u.script_pubkey,
                        u.script_pubkey_len,
                        u.script_pubkey_len,
                    ));
                }
            }
            drop(Vec::from_raw_parts(
                acc.utxos_added,
                acc.utxos_added_count,
                acc.utxos_added_count,
            ));
        }
        if !acc.utxos_spent.is_null() && acc.utxos_spent_count > 0 {
            drop(Vec::from_raw_parts(
                acc.utxos_spent,
                acc.utxos_spent_count,
                acc.utxos_spent_count,
            ));
        }
        if !acc.utxos_instant_locked.is_null() && acc.utxos_instant_locked_count > 0 {
            drop(Vec::from_raw_parts(
                acc.utxos_instant_locked,
                acc.utxos_instant_locked_count,
                acc.utxos_instant_locked_count,
            ));
        }
        // Transactions
        if !acc.transactions.is_null() && acc.transactions_count > 0 {
            let txs = std::slice::from_raw_parts(acc.transactions, acc.transactions_count);
            for tx in txs {
                if !tx.tx_data.is_null() && tx.tx_data_len > 0 {
                    drop(Vec::from_raw_parts(
                        tx.tx_data,
                        tx.tx_data_len,
                        tx.tx_data_len,
                    ));
                }
                if !tx.transaction_type.is_null() {
                    let _ = CString::from_raw(tx.transaction_type);
                }
                if !tx.label.is_null() {
                    let _ = CString::from_raw(tx.label);
                }
                if !tx.provider_service_address.is_null() {
                    let _ = CString::from_raw(tx.provider_service_address);
                }
                if !tx.input_outpoints.is_null() && tx.input_outpoints_count > 0 {
                    drop(Vec::from_raw_parts(
                        tx.input_outpoints,
                        tx.input_outpoints_count,
                        tx.input_outpoints_count,
                    ));
                }
            }
            drop(Vec::from_raw_parts(
                acc.transactions,
                acc.transactions_count,
                acc.transactions_count,
            ));
        }
    }
    drop(Vec::from_raw_parts(
        cs.accounts,
        cs.accounts_count,
        cs.accounts_count,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use key_wallet::account::{AccountType, StandardAccountType};
    use platform_wallet::changeset::{CoreChangeSet, HighestUsedIndexes};

    /// A batch can carry a highest-used advance for an account with no
    /// record of its own in the same `store()` round (idempotent
    /// re-confirmation, sibling-account resolution). `from_changeset`
    /// must still surface it as an (otherwise empty) account bucket —
    /// regression for the shape where the watermark was silently
    /// dropped because buckets were built from `cs.records` alone.
    #[test]
    fn watermark_only_account_survives_from_changeset() {
        let account = AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        };
        let mut cs = CoreChangeSet::default();
        cs.account_highest_used.insert(
            account,
            HighestUsedIndexes {
                external: Some(7),
                internal: Some(3),
            },
        );

        let ffi = WalletChangeSetFFI::from_changeset(&cs);
        assert_eq!(ffi.accounts_count, 1, "watermark-only account must emit");
        let bucket = unsafe { &*ffi.accounts };
        assert!(bucket.has_external_highest_used);
        assert_eq!(bucket.external_highest_used, 7);
        assert!(bucket.has_internal_highest_used);
        assert_eq!(bucket.internal_highest_used, 3);
        assert_eq!(bucket.transactions_count, 0);
        assert_eq!(bucket.utxos_added_count, 0);
        unsafe { free_wallet_changeset_ffi(&ffi) };
    }

    /// The `has_*` flags stay false (values -1) for accounts with
    /// records but no watermark update this batch, so the Swift
    /// persister never regresses a stored value on a no-usage round.
    #[test]
    fn accounts_without_watermarks_emit_unset_flags() {
        let cs = CoreChangeSet::default();
        let ffi = WalletChangeSetFFI::from_changeset(&cs);
        assert_eq!(ffi.accounts_count, 0);
        unsafe { free_wallet_changeset_ffi(&ffi) };
    }

    /// A folded wallet-level record files the transaction row under the
    /// FUNDING account while carrying the sibling account's owned
    /// outputs (dashpay/platform#4387), and `OutputDetail` has no
    /// owning-account field — so deriving TXOs from the folded record
    /// persisted the sibling's change under the funding account, and
    /// the Swift/Kotlin stores restored it into the wrong account's
    /// map. TXO deltas must instead come from `account_records` (the
    /// raw per-account slices), with only the transaction rows read
    /// from the folded `records`.
    #[test]
    fn txos_route_to_their_owning_accounts_bucket() {
        use dashcore::{Address, Network, OutPoint, ScriptBuf, TxIn, TxOut, Witness};
        use key_wallet::managed_account::transaction_record::{
            InputDetail, OutputDetail, OutputRole, TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::transaction_router::TransactionType;
        use key_wallet::transaction_checking::TransactionContext;

        let coinjoin = AccountType::CoinJoin { index: 0 };
        let bip44 = AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        };
        let dest = Address::dummy(Network::Testnet, 1);
        let change_addr = Address::dummy(Network::Testnet, 2);
        let funded_addr = Address::dummy(Network::Testnet, 3);
        let tx = dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::default(),
                script_sig: ScriptBuf::new(),
                sequence: 0xffffffff,
                witness: Witness::new(),
            }],
            output: vec![
                TxOut {
                    value: 900_000,
                    script_pubkey: dest.script_pubkey(),
                },
                TxOut {
                    value: 99_000,
                    script_pubkey: change_addr.script_pubkey(),
                },
            ],
            special_transaction_payload: None,
        };
        let our_input = InputDetail {
            index: 0,
            value: 1_000_000,
            address: funded_addr.clone(),
        };
        let sent = OutputDetail {
            index: 0,
            role: OutputRole::Sent,
            address: Some(dest.clone()),
            value: 900_000,
        };
        let change = OutputDetail {
            index: 1,
            role: OutputRole::Change,
            address: Some(change_addr.clone()),
            value: 99_000,
        };
        let rec = |account, direction, inputs: Vec<InputDetail>, outputs, net| {
            TransactionRecord::new(
                tx.clone(),
                account,
                TransactionContext::Mempool,
                TransactionType::Standard,
                direction,
                inputs,
                outputs,
                net,
            )
        };
        // The CoinJoin slice funds the spend; its account-local view of
        // the sibling's change is `Sent`. The BIP44 slice owns the
        // change. The folded row carries the union with the owned role.
        let coinjoin_slice = rec(
            coinjoin,
            TransactionDirection::Outgoing,
            vec![our_input.clone()],
            vec![
                sent.clone(),
                OutputDetail {
                    role: OutputRole::Sent,
                    ..change.clone()
                },
            ],
            -1_000_000,
        );
        let bip44_slice = rec(
            bip44,
            TransactionDirection::Incoming,
            vec![],
            vec![change.clone()],
            99_000,
        );
        let folded = rec(
            coinjoin,
            TransactionDirection::Outgoing,
            vec![our_input],
            vec![sent, change],
            -901_000,
        );

        let cs = CoreChangeSet {
            records: vec![folded],
            account_records: vec![coinjoin_slice, bip44_slice],
            ..CoreChangeSet::default()
        };
        let ffi = WalletChangeSetFFI::from_changeset(&cs);
        assert_eq!(ffi.accounts_count, 2, "one bucket per involved account");
        let buckets = unsafe { std::slice::from_raw_parts(ffi.accounts, ffi.accounts_count) };
        let coinjoin_bucket = buckets
            .iter()
            .find(|b| b.type_tag == account_type_to_tags(&coinjoin).type_tag)
            .expect("coinjoin bucket");
        let bip44_bucket = buckets
            .iter()
            .find(|b| b.type_tag == account_type_to_tags(&bip44).type_tag)
            .expect("bip44 bucket");

        assert_eq!(
            coinjoin_bucket.transactions_count, 1,
            "the folded wallet-level row files under the funding account"
        );
        assert_eq!(
            coinjoin_bucket.utxos_added_count, 0,
            "the funding slice owns no outputs — the sibling's change \
             must NOT be derived from the folded record into this bucket"
        );
        assert_eq!(
            coinjoin_bucket.utxos_spent_count, 1,
            "the spend stays with the account that owned the coin"
        );
        assert_eq!(
            bip44_bucket.transactions_count, 1,
            "every involved account's bucket carries the folded row — the \
             per-account transaction callback is the sole writer of the \
             tx↔account involvement join"
        );
        let coinjoin_row = unsafe { &*coinjoin_bucket.transactions };
        let bip44_row = unsafe { &*bip44_bucket.transactions };
        assert_eq!(
            coinjoin_row.net_amount, bip44_row.net_amount,
            "the row's wallet-level values are identical in every bucket"
        );
        assert_eq!(coinjoin_row.net_amount, -901_000);
        assert_eq!(
            bip44_bucket.utxos_added_count, 1,
            "the change TXO lands in its OWNING account's bucket"
        );
        unsafe { free_wallet_changeset_ffi(&ffi) };
    }

    /// The exact shape behind the provider-restoration P1: a ProReg-like
    /// transaction funded by a Standard account whose payload ALSO
    /// matches a provider owner-keys account. The provider slice is
    /// payload-only — no TXO in the account — so the tx↔account
    /// involvement join written by the per-bucket transaction callback
    /// is the ONLY thing linking the tx to the provider account, and
    /// restart restoration selects provider transactions through it.
    /// The provider bucket must therefore receive the folded row even
    /// though it contributes no TXO deltas.
    #[test]
    fn payload_only_provider_account_still_receives_the_transaction_row() {
        use dashcore::{Address, Network, OutPoint, ScriptBuf, TxIn, TxOut, Witness};
        use key_wallet::managed_account::transaction_record::{
            InputDetail, OutputDetail, OutputRole, TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::transaction_router::TransactionType;
        use key_wallet::transaction_checking::TransactionContext;

        let bip44 = AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        };
        let provider = AccountType::ProviderOwnerKeys;
        let funded_addr = Address::dummy(Network::Testnet, 4);
        let dest = Address::dummy(Network::Testnet, 5);
        let tx = dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::default(),
                script_sig: ScriptBuf::new(),
                sequence: 0xffffffff,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: 100_000_000,
                script_pubkey: dest.script_pubkey(),
            }],
            special_transaction_payload: None,
        };
        let rec =
            |account, direction, inputs: Vec<InputDetail>, outputs: Vec<OutputDetail>, net| {
                TransactionRecord::new(
                    tx.clone(),
                    account,
                    TransactionContext::Mempool,
                    TransactionType::Standard,
                    direction,
                    inputs,
                    outputs,
                    net,
                )
            };
        let funding_slice = rec(
            bip44,
            TransactionDirection::Outgoing,
            vec![InputDetail {
                index: 0,
                value: 100_001_000,
                address: funded_addr,
            }],
            vec![OutputDetail {
                index: 0,
                role: OutputRole::Sent,
                address: Some(dest),
                value: 100_000_000,
            }],
            -100_001_000,
        );
        // Payload-only provider match: no input details, no output
        // details — the owner key appears in the special-tx payload.
        let provider_slice = rec(provider, TransactionDirection::Outgoing, vec![], vec![], 0);
        let folded = funding_slice.clone();

        let cs = CoreChangeSet {
            records: vec![folded],
            account_records: vec![funding_slice, provider_slice],
            ..CoreChangeSet::default()
        };
        let ffi = WalletChangeSetFFI::from_changeset(&cs);
        assert_eq!(ffi.accounts_count, 2);
        let buckets = unsafe { std::slice::from_raw_parts(ffi.accounts, ffi.accounts_count) };
        let provider_bucket = buckets
            .iter()
            .find(|b| b.type_tag == account_type_to_tags(&provider).type_tag)
            .expect("provider bucket");
        assert_eq!(
            provider_bucket.transactions_count, 1,
            "the payload-only provider account must receive the folded row, \
             or its involvement join is never written and the transaction \
             disappears from provider restoration after restart"
        );
        assert_eq!(provider_bucket.utxos_added_count, 0);
        assert_eq!(provider_bucket.utxos_spent_count, 0);
        unsafe { free_wallet_changeset_ffi(&ffi) };
    }

    /// The FFI entry carries the platform HTTP port gated by
    /// `has_platform_http_port`, and releases its heap C strings through the
    /// public free routine.
    #[test]
    fn masternode_entry_gates_platform_http_port() {
        let mut mn = MasternodeRecord::default();
        mn.platform_http_port = Some(1443);
        mn.service_address = Some("1.2.3.4:19999".to_string());
        let entry = masternode_entry_ffi(&mn, dashcore::Network::Testnet);
        assert!(entry.has_platform_http_port);
        assert_eq!(entry.platform_http_port, 1443);
        assert!(
            !entry.platform_ownership_checked,
            "default record: unchecked"
        );
        // Release the entry's heap C strings through the public free routine.
        let entries = Box::into_raw(vec![entry].into_boxed_slice()) as *mut MasternodeEntryFFI;
        unsafe { crate::wallet::platform_wallet_manager_free_masternodes(entries, 1) };
    }

    /// Pin the original array element layout used by already-built C/Swift
    /// consumers. A field addition or reorder here is an ABI break even when
    /// all Rust callers are recompiled together.
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn masternode_entry_v1_layout_is_frozen() {
        assert_eq!(std::mem::size_of::<MasternodeEntryFFI>(), 296);
        assert_eq!(std::mem::align_of::<MasternodeEntryFFI>(), 8);
        assert_eq!(
            std::mem::offset_of!(MasternodeEntryFFI, service_address),
            144
        );
        assert_eq!(std::mem::offset_of!(MasternodeEntryFFI, owner_address), 160);
        assert_eq!(
            std::mem::offset_of!(MasternodeEntryFFI, operator_public_key),
            176
        );
        assert_eq!(
            std::mem::offset_of!(MasternodeEntryFFI, payout_address),
            248
        );
        assert_eq!(
            std::mem::offset_of!(MasternodeEntryFFI, platform_key_index),
            284
        );
        assert_eq!(
            std::mem::offset_of!(MasternodeEntryFFI, platform_ownership_checked),
            288
        );
        assert_eq!(std::mem::offset_of!(MasternodeEntryV2FFI, v1), 0);
    }

    #[test]
    fn masternode_entry_v2_carries_additive_fields_and_frees_them() {
        let mut mn = MasternodeRecord::default();
        mn.source = platform_wallet::masternode::MasternodeSource::Tracked;
        mn.label = Some("tracked label".to_string());
        let entry = masternode_entry_v2_ffi(&mn, dashcore::Network::Testnet);
        assert_eq!(entry.source, 1);
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(entry.label) }
                .to_str()
                .unwrap(),
            "tracked label"
        );
        let entries = Box::into_raw(vec![entry].into_boxed_slice()) as *mut MasternodeEntryV2FFI;
        unsafe { crate::wallet::platform_wallet_manager_free_masternodes_v2(entries, 1) };
    }

    #[test]
    fn masternode_v1_and_v2_arrays_preserve_second_element_stride() {
        let mut first = MasternodeRecord::default();
        first.pro_tx_hash = [1; 32];
        first.service_address = Some("1.1.1.1:9999".to_string());
        first.source = platform_wallet::masternode::MasternodeSource::Tracked;
        first.label = Some("first".to_string());
        let mut second = MasternodeRecord::default();
        second.pro_tx_hash = [2; 32];
        second.service_address = Some("2.2.2.2:9999".to_string());
        second.source = platform_wallet::masternode::MasternodeSource::Tracked;
        second.label = Some("second".to_string());

        let v1 = vec![
            masternode_entry_ffi(&first, dashcore::Network::Testnet),
            masternode_entry_ffi(&second, dashcore::Network::Testnet),
        ];
        let v1 = Box::into_raw(v1.into_boxed_slice()) as *mut MasternodeEntryFFI;
        let v1_slice = unsafe { std::slice::from_raw_parts(v1, 2) };
        assert_eq!(v1_slice[1].pro_tx_hash, [2; 32]);
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(v1_slice[1].service_address) }
                .to_str()
                .unwrap(),
            "2.2.2.2:9999"
        );
        unsafe { crate::wallet::platform_wallet_manager_free_masternodes(v1, 2) };

        let v2 = vec![
            masternode_entry_v2_ffi(&first, dashcore::Network::Testnet),
            masternode_entry_v2_ffi(&second, dashcore::Network::Testnet),
        ];
        let v2 = Box::into_raw(v2.into_boxed_slice()) as *mut MasternodeEntryV2FFI;
        let v2_slice = unsafe { std::slice::from_raw_parts(v2, 2) };
        assert_eq!(v2_slice[1].v1.pro_tx_hash, [2; 32]);
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(v2_slice[1].label) }
                .to_str()
                .unwrap(),
            "second"
        );
        unsafe { crate::wallet::platform_wallet_manager_free_masternodes_v2(v2, 2) };
    }
}
