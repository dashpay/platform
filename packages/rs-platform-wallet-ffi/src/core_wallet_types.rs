//! C-compatible types for core wallet changeset FFI.

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
        // `AccountType` doesn't implement `Ord` upstream (the
        // 256-bit `[u8; 32]` fields on the Dashpay variants would make
        // a derived ordering arbitrary), so a `Vec<(key, bucket)>`
        // with a linear "find or insert" walk is the path of least
        // resistance. Wallets typically have well under a hundred
        // accounts, so the linear search is cheap.
        let mut by_account: Vec<(
            AccountType,
            Vec<&key_wallet::managed_account::transaction_record::TransactionRecord>,
        )> = Vec::new();
        for rec in &cs.records {
            if let Some(bucket) = by_account
                .iter_mut()
                .find(|(at, _)| at == &rec.account_type)
            {
                bucket.1.push(rec);
            } else {
                by_account.push((rec.account_type, vec![rec]));
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
            if !by_account.iter().any(|(at, _)| at == account_type) {
                by_account.push((*account_type, Vec::new()));
            }
        }

        let mut ffi_accounts = Vec::with_capacity(by_account.len());
        for (account_type, recs) in by_account {
            let type_name = CString::new(format!("{:?}", account_type))
                .unwrap_or_else(|_| CString::new("Unknown").unwrap());
            let account_index = account_index_of(&account_type);

            // Derive UTXO add/spend lists from this account's records.
            // Each record carries its own input_details and
            // output_details; we walk them once per record to project
            // the UTXOs the persister should add or remove.
            let mut utxos_added: Vec<UtxoEntryFFI> = Vec::new();
            let mut utxos_spent: Vec<SpentOutPointFFI> = Vec::new();
            for rec in &recs {
                utxos_added.extend(record_new_utxos_ffi(rec));
                utxos_spent.extend(record_spent_outpoints_ffi(rec));
            }

            // Transactions for this account.
            let transactions: Vec<TransactionRecordFFI> =
                recs.into_iter().map(tx_record_to_ffi).collect();

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

/// Fixed-size hash copies. `Txid` / `PubkeyHash` are exactly 32 / 20
/// bytes, so `copy_from_slice` on `as_ref()` is length-exact and cannot
/// panic — the same pattern `tx_record_to_ffi`'s txid copy relies on.
fn provider_hash_to_32(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(bytes);
    out
}

fn provider_hash_to_20(bytes: &[u8]) -> [u8; 20] {
    let mut out = [0u8; 20];
    out.copy_from_slice(bytes);
    out
}

/// Rebuild an `"ip:port"` string from a ProUpServTx-style little-endian
/// IPv6-mapped `u128` address + `port`, collapsing IPv4-mapped addresses
/// to V4 so a normal masternode renders as `"1.2.3.4:port"`.
fn provider_ip_port(ip_address: u128, port: u16) -> String {
    let v6 = std::net::Ipv6Addr::from(ip_address.to_le_bytes());
    let ip = v6
        .to_ipv4_mapped()
        .map(std::net::IpAddr::V4)
        .unwrap_or(std::net::IpAddr::V6(v6));
    format!("{}:{}", ip, port)
}

/// Provider (masternode) special-transaction payload fields lifted for
/// the Swift UI. All optional / gated — only a ProRegTx or ProUpServTx
/// populates them. The single seam where the DIP-3 payload is decoded;
/// Swift only marshals the flat results.
#[derive(Default)]
struct ProviderPayloadFields {
    /// Service endpoint as `"ip:port"`.
    service_address: Option<String>,
    /// ProUpServTx registration linkage. `None` for ProRegTx (its own
    /// txid is the proTxHash).
    pro_tx_hash: Option<[u8; 32]>,
    /// ProRegTx collateral outpoint (`txid` wire bytes, `vout`).
    collateral: Option<([u8; 32], u32)>,
    /// ProRegTx owner / voting key hashes (hash160, 20 bytes).
    owner_key_hash: Option<[u8; 20]>,
    voting_key_hash: Option<[u8; 20]>,
}

/// Extract provider-registration (ProRegTx) / provider-update-service
/// (ProUpServTx) payload fields from a transaction for display. Returns
/// all-`None` for any other transaction. Pure; the only allocation is
/// the returned service-address `String`.
fn provider_payload_fields(tx: &dashcore::Transaction) -> ProviderPayloadFields {
    use dashcore::transaction::TransactionPayload;

    match &tx.special_transaction_payload {
        Some(TransactionPayload::ProviderRegistrationPayloadType(p)) => ProviderPayloadFields {
            service_address: Some(p.service_address.to_string()),
            pro_tx_hash: None,
            collateral: Some((
                provider_hash_to_32(p.collateral_outpoint.txid.as_ref()),
                p.collateral_outpoint.vout,
            )),
            owner_key_hash: Some(provider_hash_to_20(p.owner_key_hash.as_ref())),
            voting_key_hash: Some(provider_hash_to_20(p.voting_key_hash.as_ref())),
        },
        Some(TransactionPayload::ProviderUpdateServicePayloadType(p)) => ProviderPayloadFields {
            service_address: Some(provider_ip_port(p.ip_address, p.port)),
            pro_tx_hash: Some(provider_hash_to_32(p.pro_tx_hash.as_ref())),
            ..Default::default()
        },
        _ => ProviderPayloadFields::default(),
    }
}

/// Membership of a proTxHash in the current deterministic masternode
/// list (DML), the authoritative status source. Injected into
/// [`aggregate_masternodes`] as a closure so the aggregation stays
/// source-agnostic and unit-testable without a live SPV engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListMembership {
    /// In the DML and valid / enabled.
    ValidEntry,
    /// In the DML but flagged invalid (PoSe-banned / `is_valid == false`).
    InvalidEntry,
    /// Not in the DML (collateral spent / revoked / expired).
    Absent,
    /// The DML isn't available yet (SPV not running / masternode sync
    /// incomplete) — status is indeterminate.
    ListUnavailable,
}

/// Displayed masternode status, derived from [`ListMembership`]. The
/// `u8` discriminant is the FFI wire value; `Unknown` (DML unavailable)
/// tells the persist layer to KEEP the previously stored status rather
/// than overwrite it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum MasternodeStatus {
    Active,
    Inactive,
    Retired,
    #[default]
    Unknown,
}

impl MasternodeStatus {
    fn from_membership(membership: ListMembership) -> Self {
        match membership {
            ListMembership::ValidEntry => Self::Active,
            ListMembership::InvalidEntry => Self::Inactive,
            ListMembership::Absent => Self::Retired,
            ListMembership::ListUnavailable => Self::Unknown,
        }
    }

    pub(crate) fn as_u8(self) -> u8 {
        match self {
            Self::Active => 0,
            Self::Inactive => 1,
            Self::Retired => 2,
            Self::Unknown => 3,
        }
    }
}

/// One aggregated masternode, grouped by proTxHash across a wallet's
/// provider special transactions. Pure/testable output of
/// [`aggregate_masternodes`]; the FFI query layer flattens it into
/// `MasternodeEntryFFI` and owns the record source.
#[derive(Default, Debug, Clone)]
pub(crate) struct MasternodeAggregate {
    /// proTxHash (32 wire bytes). For a ProRegTx this is its own txid;
    /// updates / revocations link to it via their `pro_tx_hash`.
    pub pro_tx_hash: [u8; 32],
    /// Whether a ProRegTx for this proTxHash was in the input set.
    pub has_registration: bool,
    /// Core height of the ProRegTx (0 when unseen) — the stable
    /// registration-order sort key.
    pub registration_height: u32,
    /// Latest known service endpoint `"ip:port"` (latest-height update
    /// wins; seeded by the ProRegTx address).
    pub service_address: Option<String>,
    /// Platform HTTP (DAPI gRPC) port from the same ProRegTx / ProUpServTx
    /// that set `service_address` — evonodes only, `None` for a regular
    /// masternode or a pre-v19 payload without platform fields. With the
    /// service IP this addresses the node's DAPI (`https://<ip>:<port>`).
    pub platform_http_port: Option<u16>,
    /// Height that set `service_address` / `platform_http_port` (drives
    /// latest-wins).
    service_height: u32,
    /// evonode / HPMN flag from the ProRegTx `masternode_type`.
    pub is_evonode: bool,
    /// Owner key hash (hash160) from the ProRegTx.
    pub owner_key_hash: Option<[u8; 20]>,
    /// Voting key hash (hash160) — follows the latest ProRegTx / ProUpReg.
    pub voting_key_hash: Option<[u8; 20]>,
    /// Height that set `voting_key_hash` (drives latest-wins).
    voting_height: u32,
    /// Operator BLS public key (48 bytes) — follows the latest ProRegTx /
    /// ProUpReg.
    pub operator_public_key: Option<[u8; 48]>,
    operator_height: u32,
    /// Platform node id (SHA256[..20] Tenderdash, #884, 20 bytes) for evonodes — follows the
    /// latest ProRegTx / ProUpServ.
    pub platform_node_id: Option<[u8; 20]>,
    platform_node_height: u32,
    /// Payout script (raw bytes) — follows the latest ProRegTx / ProUpReg
    /// (owner payout). Encoded to a base58 address by `masternode_entry_ffi`
    /// where the network is available.
    pub payout_script: Option<Vec<u8>>,
    payout_height: u32,
    /// Collateral outpoint (`txid` wire bytes, `vout`) from the ProRegTx.
    pub collateral: Option<([u8; 32], u32)>,
    /// A ProUpRevTx was seen ⇒ the masternode was revoked ("previously
    /// had"). `revocation_reason` keeps the latest reason for reference.
    pub revoked: bool,
    pub revocation_reason: u16,
    /// Count of provider txs seen for this proTxHash.
    pub tx_count: u32,
    /// 1-based index WITHIN this masternode's type, in registration order —
    /// evonodes and regular masternodes each get their own sequence
    /// ("Evonode 1, 2, …" / "Masternode 1, 2, …"). `orderIndex` remains the
    /// cross-type stable sort key.
    pub type_index: u32,
    /// Status against the current DML (authoritative). `Unknown` when the
    /// DML isn't available. Note: this is NOT `revoked`-derived — a
    /// ProUpRevTx merely tends to make the node `Absent` (⇒ `Retired`);
    /// the DML is the source of truth. `revoked` / `revocation_reason`
    /// are retained as separate data.
    pub status: MasternodeStatus,
}

/// Aggregate a wallet's provider special transactions into masternode
/// entities, grouped by proTxHash. Each input is `(core_height, tx)`;
/// height drives latest-wins for the mutable fields (service address,
/// voting key), so callers may feed records in any order. Non-provider
/// txs are ignored.
///
/// Output is sorted by registration height then proTxHash for stable
/// "Masternode N" numbering; entities seen only via an update
/// (registration not in the input set — e.g. the ProRegTx was evicted or
/// isn't ours) sort last.
///
/// Status is resolved against the DML via the injected `list_lookup`
/// closure (`proTxHash -> ListMembership`), keeping this function free of
/// any live SPV dependency so tests can stub the lookup.
///
/// Pure — no I/O; allocation is limited to the aggregate strings. The
/// record source (which txs to feed) is the caller's concern (see the
/// query fn), which is why this is decoupled and unit-testable.
pub(crate) fn aggregate_masternodes<'a, F>(
    txs: impl Iterator<Item = (u32, u32, &'a dashcore::Transaction)>,
    list_lookup: F,
) -> Vec<MasternodeAggregate>
where
    F: Fn(&[u8; 32]) -> ListMembership,
{
    use dashcore::blockdata::transaction::special_transaction::provider_registration::ProviderMasternodeType;
    use dashcore::transaction::TransactionPayload;

    // Each input item is `(height, in_block_position, tx)`. Core's
    // `RebuildListFromBlock` applies same-block provider updates in
    // `block.vtx` order, so the per-field latest-wins below must resolve
    // ties by `(height, position)`, not by the arbitrary txid order the
    // caller's `BTreeMap<Txid, _>` dedup produces. Process ascending
    // `(height, position)` so the block-latest write for each field lands
    // last and wins under the `>= *_height` guards. Stable so equal keys
    // keep their incoming order.
    //
    // The position is stamped onto `BlockInfo` during block processing
    // (rust-dashcore#891) and round-tripped through persistence; legacy
    // rows confirmed before the field existed come back as 0 and fall
    // back to feed order among themselves.
    let mut ordered: Vec<(u32, u32, &'a dashcore::Transaction)> = txs.collect();
    ordered.sort_by_key(|(height, position, _)| (*height, *position));

    let mut order: Vec<[u8; 32]> = Vec::new();
    let mut by_hash: std::collections::HashMap<[u8; 32], MasternodeAggregate> =
        std::collections::HashMap::new();

    for (height, _position, tx) in ordered {
        // proTxHash key: a ProRegTx's own txid, else the update's link.
        let key = match &tx.special_transaction_payload {
            Some(TransactionPayload::ProviderRegistrationPayloadType(_)) => {
                provider_hash_to_32(tx.txid().as_ref())
            }
            Some(TransactionPayload::ProviderUpdateServicePayloadType(p)) => {
                provider_hash_to_32(p.pro_tx_hash.as_ref())
            }
            Some(TransactionPayload::ProviderUpdateRegistrarPayloadType(p)) => {
                provider_hash_to_32(p.pro_tx_hash.as_ref())
            }
            Some(TransactionPayload::ProviderUpdateRevocationPayloadType(p)) => {
                provider_hash_to_32(p.pro_tx_hash.as_ref())
            }
            _ => continue,
        };

        let agg = by_hash.entry(key).or_insert_with(|| {
            order.push(key);
            MasternodeAggregate {
                pro_tx_hash: key,
                ..Default::default()
            }
        });
        agg.tx_count = agg.tx_count.saturating_add(1);

        match &tx.special_transaction_payload {
            Some(TransactionPayload::ProviderRegistrationPayloadType(p)) => {
                agg.has_registration = true;
                agg.registration_height = height;
                agg.is_evonode = p.masternode_type == ProviderMasternodeType::HighPerformance;
                agg.owner_key_hash = Some(provider_hash_to_20(p.owner_key_hash.as_ref()));
                agg.collateral = Some((
                    provider_hash_to_32(p.collateral_outpoint.txid.as_ref()),
                    p.collateral_outpoint.vout,
                ));
                // Registration seeds the service address and voting key;
                // treat both as updates observed at this height.
                if agg.service_address.is_none() || height >= agg.service_height {
                    agg.service_address = Some(p.service_address.to_string());
                    agg.platform_http_port = p.platform_http_port;
                    agg.service_height = height;
                }
                if agg.voting_key_hash.is_none() || height >= agg.voting_height {
                    agg.voting_key_hash = Some(provider_hash_to_20(p.voting_key_hash.as_ref()));
                    agg.voting_height = height;
                }
                if agg.operator_public_key.is_none() || height >= agg.operator_height {
                    let bls: &[u8; 48] = p.operator_public_key.as_ref();
                    agg.operator_public_key = Some(*bls);
                    agg.operator_height = height;
                }
                if agg.platform_node_id.is_none() || height >= agg.platform_node_height {
                    // Evonode-only; `None` on a regular masternode.
                    // `platform_node_id` is a `PlatformNodeId` newtype
                    // (rust-dashcore #885) whose `consensus_decode` normalizes
                    // the wire's reversed uint160-internal bytes to the
                    // canonical Tenderdash `SHA256(pubkey)[..20]` order
                    // (rust-dashcore #887/#889), so `to_byte_array()` here is
                    // already canonical and matches the derived ownership
                    // index (`accessors.rs`) and dashmate display directly —
                    // do NOT reverse platform-side.
                    if let Some(node_id) = p.platform_node_id {
                        agg.platform_node_id = Some(node_id.to_byte_array());
                        agg.platform_node_height = height;
                    }
                }
                if agg.payout_script.is_none() || height >= agg.payout_height {
                    agg.payout_script = Some(p.script_payout.as_bytes().to_vec());
                    agg.payout_height = height;
                }
            }
            Some(TransactionPayload::ProviderUpdateServicePayloadType(p)) => {
                if agg.service_address.is_none() || height >= agg.service_height {
                    agg.service_address = Some(provider_ip_port(p.ip_address, p.port));
                    agg.platform_http_port = p.platform_http_port;
                    agg.service_height = height;
                }
                // ProUpServ's `platform_node_id` is now `Option<PlatformNodeId>`
                // (rust-dashcore #885, was `Option<[u8; 20]>`); decoded bytes
                // are canonical forward order (see the ProRegTx arm above).
                if let Some(node_id) = p.platform_node_id {
                    if agg.platform_node_id.is_none() || height >= agg.platform_node_height {
                        agg.platform_node_id = Some(node_id.to_byte_array());
                        agg.platform_node_height = height;
                    }
                }
            }
            Some(TransactionPayload::ProviderUpdateRegistrarPayloadType(p)) => {
                if agg.voting_key_hash.is_none() || height >= agg.voting_height {
                    agg.voting_key_hash = Some(provider_hash_to_20(p.voting_key_hash.as_ref()));
                    agg.voting_height = height;
                }
                if agg.operator_public_key.is_none() || height >= agg.operator_height {
                    let bls: &[u8; 48] = p.operator_public_key.as_ref();
                    agg.operator_public_key = Some(*bls);
                    agg.operator_height = height;
                }
                if agg.payout_script.is_none() || height >= agg.payout_height {
                    agg.payout_script = Some(p.script_payout.as_bytes().to_vec());
                    agg.payout_height = height;
                }
            }
            Some(TransactionPayload::ProviderUpdateRevocationPayloadType(p)) => {
                agg.revoked = true;
                agg.revocation_reason = p.reason;
            }
            _ => {}
        }
    }

    let mut result: Vec<MasternodeAggregate> = order
        .into_iter()
        .filter_map(|k| by_hash.remove(&k))
        .collect();
    // Stable registration-order numbering: registered masternodes by
    // ascending registration height then proTxHash; update-only entities
    // (no ProRegTx seen) sort last via a MAX height sentinel.
    result.sort_by(|a, b| {
        let ha = if a.has_registration {
            a.registration_height
        } else {
            u32::MAX
        };
        let hb = if b.has_registration {
            b.registration_height
        } else {
            u32::MAX
        };
        ha.cmp(&hb).then_with(|| a.pro_tx_hash.cmp(&b.pro_tx_hash))
    });

    // Resolve authoritative status against the DML and assign per-type
    // numbering (separate Evonode / Masternode sequences), both in the
    // stable registration order established above.
    let mut evonode_n: u32 = 0;
    let mut masternode_n: u32 = 0;
    for agg in result.iter_mut() {
        agg.status = MasternodeStatus::from_membership(list_lookup(&agg.pro_tx_hash));
        if agg.is_evonode {
            evonode_n += 1;
            agg.type_index = evonode_n;
        } else {
            masternode_n += 1;
            agg.type_index = masternode_n;
        }
    }
    result
}

/// Flat, C-ABI masternode entity — the wire shape of one
/// [`MasternodeAggregate`], built by [`masternode_entry_ffi`] and
/// returned by `platform_wallet_manager_list_masternodes`. Inline
/// fixed-size hashes with `has_*` gates (mirroring `TransactionRecordFFI`)
/// keep heap ownership to the three C strings.
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

/// Flatten one aggregate into its C-ABI entry, encoding the owner /
/// voting / payout / operator / platform-node base58 addresses for
/// `network`. `order_index` is the caller's stable position in the sorted
/// aggregate list.
///
/// Owner / voting key ownership is resolved app-side (persisted-address
/// join). Operator / platform key ownership is resolved HERE via the
/// derive-and-compare maps (`operator_index`: BLS pubkey ⇒ index,
/// `platform_index`: node id ⇒ index) — those keys have no on-chain
/// address to join against.
pub(crate) fn masternode_entry_ffi(
    mn: &MasternodeAggregate,
    order_index: u32,
    network: dashcore::Network,
    operator_index: &std::collections::HashMap<[u8; 48], u32>,
    platform_index: &std::collections::HashMap<[u8; 20], u32>,
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

    // Derive-and-compare ownership: match the masternode's payload key
    // against the wallet's derived provider keys.
    let (operator_in_wallet, operator_account_type, operator_key_index) = mn
        .operator_public_key
        .and_then(|k| operator_index.get(&k))
        .map(|index| (true, 10u8, *index))
        .unwrap_or((false, 0, 0));
    let (platform_in_wallet, platform_account_type, platform_key_index) = mn
        .platform_node_id
        .and_then(|id| platform_index.get(&id))
        .map(|index| (true, 11u8, *index))
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
        order_index,
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
        // The check was possible iff the wallet's derived platform-node index
        // had entries to compare against. Empty index ⇒ no platform pool / not
        // yet rehydrated ⇒ ownership is "unchecked", and the persister must
        // retain any prior value rather than clobber it to false.
        platform_ownership_checked: !platform_index.is_empty(),
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

    /// ProRegTx provider payload is lifted from the DIP-3 special-tx
    /// body for the UI. Fixture is the testnet
    /// collateral-provider-registration transaction from rust-dashcore's
    /// own `provider_registration` tests
    /// (`test_collateral_provider_registration_transaction`), whose
    /// service address is `1.2.5.6:19999` and whose owner/voting key
    /// hashes are asserted below. ProRegTx carries no explicit
    /// `pro_tx_hash` (its own txid is the proTxHash), so that field
    /// stays `None`.
    #[test]
    fn provider_registration_payload_fields_extracted() {
        let raw = "0300010001ca9a43051750da7c5f858008f2ff7732d15691e48eb7f845c791e5dca78bab58010000006b483045022100fe8fec0b3880bcac29614348887769b0b589908e3f5ec55a6cf478a6652e736502202f30430806a6690524e4dd599ba498e5ff100dea6a872ebb89c2fd651caa71ed012103d85b25d6886f0b3b8ce1eef63b720b518fad0b8e103eba4e85b6980bfdda2dfdffffffff018e37807e090000001976a9144ee1d4e5d61ac40a13b357ac6e368997079678c888ac00000000fd1201010000000000ca9a43051750da7c5f858008f2ff7732d15691e48eb7f845c791e5dca78bab580000000000000000000000000000ffff010205064e1f3dd03f9ec192b5f275a433bfc90f468ee1a3eb4c157b10706659e25eb362b5d902d809f9160b1688e201ee6e94b40f9b5062d7074683ef05a2d5efb7793c47059c878dfad38a30fafe61575db40f05ab0a08d55119b0aad300001976a9144fbc8fb6e11e253d77e5a9c987418e89cf4a63d288ac3477990b757387cb0406168c2720acf55f83603736a314a37d01b135b873a27b411fb37e49c1ff2b8057713939a5513e6e711a71cff2e517e6224df724ed750aef1b7f9ad9ec612b4a7250232e1e400da718a9501e1d9a5565526e4b1ff68c028763";
        let bytes = hex::decode(raw).expect("valid fixture hex");
        let tx: dashcore::Transaction =
            dashcore::consensus::encode::deserialize(&bytes).expect("decode ProRegTx");

        let fields = provider_payload_fields(&tx);

        assert_eq!(
            fields.service_address.as_deref(),
            Some("1.2.5.6:19999"),
            "service address must be lifted from the ProRegTx payload"
        );
        assert!(
            fields.collateral.is_some(),
            "ProRegTx carries a collateral outpoint"
        );
        assert_eq!(
            hex::encode(fields.owner_key_hash.expect("owner key hash")),
            "3dd03f9ec192b5f275a433bfc90f468ee1a3eb4c"
        );
        assert_eq!(
            hex::encode(fields.voting_key_hash.expect("voting key hash")),
            "d38a30fafe61575db40f05ab0a08d55119b0aad3"
        );
        assert!(
            fields.pro_tx_hash.is_none(),
            "ProRegTx has no explicit pro_tx_hash"
        );
    }

    /// ProUpServTx (provider-update-service) also carries a service
    /// address — reconstructed here from its little-endian IPv6-mapped
    /// `ip_address` + `port` — plus an explicit `pro_tx_hash` linking it
    /// to the registration. Fixture is rust-dashcore's own
    /// `test_provider_update_service_transaction` vector, whose endpoint
    /// is `52.36.64.148:19999`. The `pro_tx_hash` is asserted in raw
    /// wire order (what `to_32(txid.as_ref())` stores) — the reverse of
    /// the block-explorer display form.
    #[test]
    fn provider_update_service_payload_fields_extracted() {
        let raw = "03000200018f3fe6683e36326669b6e34876fb2a2264e8327e822f6fec304b66f47d61b3e1010000006b48304502210082af6727408f0f2ec16c7da1c42ccf0a026abea6a3a422776272b03c8f4e262a022033b406e556f6de980b2d728e6812b3ae18ee1c863ae573ece1cbdf777ca3e56101210351036c1192eaf763cd8345b44137482ad24b12003f23e9022ce46752edf47e6effffffff0180220e43000000001976a914123cbc06289e768ca7d743c8174b1e6eeb610f1488ac00000000b501003a72099db84b1c1158568eec863bea1b64f90eccee3304209cebe1df5e7539fd00000000000000000000ffff342440944e1f00e6725f799ea20480f06fb105ebe27e7c4845ab84155e4c2adf2d6e5b73a998b1174f9621bbeda5009c5a6487bdf75edcf602b67fe0da15c275cc91777cb25f5fd4bb94e84fd42cb2bb547c83792e57c80d196acd47020e4054895a0640b7861b3729c41dd681d4996090d5750f65c4b649a5cd5b2bdf55c880459821e53d91c9";
        let bytes = hex::decode(raw).expect("valid fixture hex");
        let tx: dashcore::Transaction =
            dashcore::consensus::encode::deserialize(&bytes).expect("decode ProUpServTx");

        let fields = provider_payload_fields(&tx);

        assert_eq!(
            fields.service_address.as_deref(),
            Some("52.36.64.148:19999"),
            "ProUpServTx endpoint must be rebuilt from ip_address + port"
        );
        assert_eq!(
            fields.pro_tx_hash.map(hex::encode).as_deref(),
            Some("3a72099db84b1c1158568eec863bea1b64f90eccee3304209cebe1df5e7539fd"),
            "ProUpServTx carries an explicit pro_tx_hash (wire order)"
        );
        assert!(
            fields.collateral.is_none(),
            "ProUpServTx has no collateral outpoint"
        );
        assert!(fields.owner_key_hash.is_none());
        assert!(fields.voting_key_hash.is_none());
    }

    /// A plain (non-provider) transaction yields no provider fields, so
    /// the FFI record emits null/zeroed/`false` for all of them.
    #[test]
    fn non_provider_tx_has_no_provider_fields() {
        let tx = dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        };
        let fields = provider_payload_fields(&tx);
        assert!(fields.service_address.is_none());
        assert!(fields.pro_tx_hash.is_none());
        assert!(fields.collateral.is_none());
        assert!(fields.owner_key_hash.is_none());
        assert!(fields.voting_key_hash.is_none());
    }

    // rust-dashcore's own test vectors (see the payload extraction tests
    // above). Both are unrelated masternodes, so they aggregate into
    // distinct proTxHash buckets.
    const PROREG_HEX: &str = "0300010001ca9a43051750da7c5f858008f2ff7732d15691e48eb7f845c791e5dca78bab58010000006b483045022100fe8fec0b3880bcac29614348887769b0b589908e3f5ec55a6cf478a6652e736502202f30430806a6690524e4dd599ba498e5ff100dea6a872ebb89c2fd651caa71ed012103d85b25d6886f0b3b8ce1eef63b720b518fad0b8e103eba4e85b6980bfdda2dfdffffffff018e37807e090000001976a9144ee1d4e5d61ac40a13b357ac6e368997079678c888ac00000000fd1201010000000000ca9a43051750da7c5f858008f2ff7732d15691e48eb7f845c791e5dca78bab580000000000000000000000000000ffff010205064e1f3dd03f9ec192b5f275a433bfc90f468ee1a3eb4c157b10706659e25eb362b5d902d809f9160b1688e201ee6e94b40f9b5062d7074683ef05a2d5efb7793c47059c878dfad38a30fafe61575db40f05ab0a08d55119b0aad300001976a9144fbc8fb6e11e253d77e5a9c987418e89cf4a63d288ac3477990b757387cb0406168c2720acf55f83603736a314a37d01b135b873a27b411fb37e49c1ff2b8057713939a5513e6e711a71cff2e517e6224df724ed750aef1b7f9ad9ec612b4a7250232e1e400da718a9501e1d9a5565526e4b1ff68c028763";
    const PROUPSERV_HEX: &str = "03000200018f3fe6683e36326669b6e34876fb2a2264e8327e822f6fec304b66f47d61b3e1010000006b48304502210082af6727408f0f2ec16c7da1c42ccf0a026abea6a3a422776272b03c8f4e262a022033b406e556f6de980b2d728e6812b3ae18ee1c863ae573ece1cbdf777ca3e56101210351036c1192eaf763cd8345b44137482ad24b12003f23e9022ce46752edf47e6effffffff0180220e43000000001976a914123cbc06289e768ca7d743c8174b1e6eeb610f1488ac00000000b501003a72099db84b1c1158568eec863bea1b64f90eccee3304209cebe1df5e7539fd00000000000000000000ffff342440944e1f00e6725f799ea20480f06fb105ebe27e7c4845ab84155e4c2adf2d6e5b73a998b1174f9621bbeda5009c5a6487bdf75edcf602b67fe0da15c275cc91777cb25f5fd4bb94e84fd42cb2bb547c83792e57c80d196acd47020e4054895a0640b7861b3729c41dd681d4996090d5750f65c4b649a5cd5b2bdf55c880459821e53d91c9";

    fn decode_tx(hex: &str) -> dashcore::Transaction {
        let bytes = hex::decode(hex).expect("valid fixture hex");
        dashcore::consensus::encode::deserialize(&bytes).expect("decode tx")
    }

    /// Stub DML lookup: the list is never available (⇒ every entity is
    /// `Unknown`). Mirrors "SPV not running / masternode sync incomplete".
    fn unavailable_dml(_pro_tx_hash: &[u8; 32]) -> ListMembership {
        ListMembership::ListUnavailable
    }

    /// A lone ProRegTx aggregates into one active masternode carrying its
    /// service address, key hashes, and collateral, keyed by its own txid.
    #[test]
    fn aggregate_single_registration() {
        let reg = decode_tx(PROREG_HEX);
        let expected_pro_tx = provider_hash_to_32(reg.txid().as_ref());

        let mns = aggregate_masternodes([(100u32, 0u32, &reg)].into_iter(), unavailable_dml);
        assert_eq!(mns.len(), 1);
        let mn = &mns[0];
        assert_eq!(mn.pro_tx_hash, expected_pro_tx);
        assert_eq!(mn.status, MasternodeStatus::Unknown, "no DML ⇒ Unknown");
        assert!(mn.has_registration);
        assert!(!mn.revoked);
        assert!(!mn.is_evonode, "legacy ProRegTx fixture is a regular MN");
        assert_eq!(mn.service_address.as_deref(), Some("1.2.5.6:19999"));
        assert!(mn.owner_key_hash.is_some());
        assert!(mn.voting_key_hash.is_some());
        assert!(mn.collateral.is_some());
        // #4116 key-ownership extraction: operator BLS key + payout script
        // are lifted; the legacy (v1) fixture is a regular MN so it has no
        // platform node id.
        assert!(
            mn.operator_public_key.is_some(),
            "ProRegTx carries a 48-byte operator BLS key"
        );
        assert!(
            mn.payout_script.as_ref().is_some_and(|s| !s.is_empty()),
            "ProRegTx carries a payout script"
        );
        assert!(
            mn.platform_node_id.is_none(),
            "legacy regular-MN fixture has no platform node id"
        );
        assert!(
            mn.platform_http_port.is_none(),
            "legacy regular-MN fixture has no platform HTTP port"
        );
        assert_eq!(mn.tx_count, 1);
    }

    /// A ProUpServTx whose registration isn't in the input set still
    /// yields a masternode (keyed by its `pro_tx_hash`) with the updated
    /// service address but no registration-only fields.
    #[test]
    fn aggregate_update_only_masternode() {
        let ups = decode_tx(PROUPSERV_HEX);
        let mns = aggregate_masternodes([(50u32, 0u32, &ups)].into_iter(), unavailable_dml);
        assert_eq!(mns.len(), 1);
        let mn = &mns[0];
        assert!(!mn.has_registration);
        assert_eq!(mn.service_address.as_deref(), Some("52.36.64.148:19999"));
        assert!(mn.owner_key_hash.is_none());
        assert!(mn.collateral.is_none());
        assert_eq!(mn.tx_count, 1);
    }

    /// Two unrelated provider txs bucket into two masternodes.
    #[test]
    fn aggregate_groups_by_pro_tx_hash() {
        let reg = decode_tx(PROREG_HEX);
        let ups = decode_tx(PROUPSERV_HEX);
        let mns = aggregate_masternodes(
            [(100u32, 0u32, &reg), (200u32, 0u32, &ups)].into_iter(),
            unavailable_dml,
        );
        assert_eq!(mns.len(), 2, "distinct proTxHashes ⇒ two masternodes");
    }

    /// A ProUpRevTx linked to a registration flips the masternode to
    /// revoked ("previously had") while its service address and count
    /// reflect the full provider-tx set. Built programmatically because
    /// rust-dashcore ships no ProUpRevTx raw-hex vector.
    #[test]
    fn aggregate_revocation_marks_revoked() {
        use dashcore::blockdata::transaction::special_transaction::provider_update_revocation::ProviderUpdateRevocationPayload;
        use dashcore::transaction::TransactionPayload;

        let reg = decode_tx(PROREG_HEX);
        let pro_tx_hash = reg.txid();

        let rev_payload = ProviderUpdateRevocationPayload {
            version: 1,
            pro_tx_hash,
            reason: 2,
            inputs_hash: [3u8; 32].into(),
            payload_sig: [0u8; 96].into(),
        };
        let rev = dashcore::Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: Some(
                TransactionPayload::ProviderUpdateRevocationPayloadType(rev_payload),
            ),
        };

        // A ProUpRevTx'd node is Absent from the DML here ⇒ Retired.
        let revoked_pro_tx = provider_hash_to_32(pro_tx_hash.as_ref());
        let lookup = |pt: &[u8; 32]| {
            if *pt == revoked_pro_tx {
                ListMembership::Absent
            } else {
                ListMembership::ListUnavailable
            }
        };

        // Revocation feed order shouldn't matter (height drives merges).
        let mns = aggregate_masternodes(
            [(300u32, 0u32, &rev), (100u32, 0u32, &reg)].into_iter(),
            lookup,
        );
        assert_eq!(mns.len(), 1);
        let mn = &mns[0];
        assert_eq!(mn.pro_tx_hash, revoked_pro_tx);
        assert!(mn.has_registration);
        assert!(mn.revoked, "a ProUpRevTx marks the revoked-data flag");
        assert_eq!(mn.revocation_reason, 2);
        assert_eq!(
            mn.status,
            MasternodeStatus::Retired,
            "absent from the DML ⇒ Retired (status is DML-derived, not revoked-derived)"
        );
        assert_eq!(mn.service_address.as_deref(), Some("1.2.5.6:19999"));
        assert_eq!(mn.tx_count, 2);
    }

    /// Status is derived from the injected DML lookup, not from tx history:
    /// a valid entry ⇒ Active, a present-but-invalid entry ⇒ Inactive, an
    /// absent entry ⇒ Retired — all for the same (unrevoked) ProRegTx.
    #[test]
    fn aggregate_status_follows_dml_membership() {
        let reg = decode_tx(PROREG_HEX);
        let pro_tx = provider_hash_to_32(reg.txid().as_ref());

        for (membership, expected) in [
            (ListMembership::ValidEntry, MasternodeStatus::Active),
            (ListMembership::InvalidEntry, MasternodeStatus::Inactive),
            (ListMembership::Absent, MasternodeStatus::Retired),
            (ListMembership::ListUnavailable, MasternodeStatus::Unknown),
        ] {
            let lookup = |pt: &[u8; 32]| {
                assert_eq!(*pt, pro_tx);
                membership
            };
            let mns = aggregate_masternodes([(100u32, 0u32, &reg)].into_iter(), lookup);
            assert_eq!(mns.len(), 1);
            assert_eq!(mns[0].status, expected);
            assert!(!mns[0].revoked, "no ProUpRevTx ⇒ revoked flag stays false");
        }
    }

    /// Evonodes and regular masternodes get INDEPENDENT 1-based per-type
    /// sequences: an evonode + a regular in one aggregation each get
    /// `type_index == 1`. Built by cloning the regular ProRegTx fixture and
    /// flipping its `masternode_type` (plus `lock_time`, so the txid — and
    /// thus the proTxHash group key — differs).
    #[test]
    fn aggregate_per_type_numbering() {
        use dashcore::blockdata::transaction::special_transaction::provider_registration::ProviderMasternodeType;
        use dashcore::transaction::TransactionPayload;

        let regular = decode_tx(PROREG_HEX);

        let mut evonode = decode_tx(PROREG_HEX);
        evonode.lock_time = 4242; // change the txid ⇒ distinct proTxHash
        if let Some(TransactionPayload::ProviderRegistrationPayloadType(p)) =
            &mut evonode.special_transaction_payload
        {
            p.masternode_type = ProviderMasternodeType::HighPerformance;
        }

        let mns = aggregate_masternodes(
            [(100u32, 0u32, &regular), (200u32, 0u32, &evonode)].into_iter(),
            unavailable_dml,
        );
        assert_eq!(mns.len(), 2, "distinct proTxHashes ⇒ two masternodes");

        let evo = mns.iter().find(|m| m.is_evonode).expect("evonode present");
        let reg = mns.iter().find(|m| !m.is_evonode).expect("regular present");
        assert_eq!(evo.type_index, 1, "first (only) evonode ⇒ Evonode 1");
        assert_eq!(reg.type_index, 1, "first (only) regular ⇒ Masternode 1");
    }

    /// Two provider updates for one masternode in the SAME block must resolve
    /// the per-field latest-wins by in-block `position`, matching Core's
    /// `block.vtx` order — NOT by the arbitrary txid order the caller's
    /// `BTreeMap<Txid>` dedup would otherwise impose. Feed the same pair in
    /// both orders; the higher-positioned (block-latest) update wins each time,
    /// proving position — not feed/txid order — decides the outcome.
    #[test]
    fn same_block_updates_resolve_by_position_not_txid() {
        use dashcore::blockdata::transaction::special_transaction::provider_update_service::ProviderUpdateServicePayload;
        use dashcore::transaction::TransactionPayload;

        // Shared registration linkage ⇒ both updates land in one bucket.
        let pro_tx_hash = decode_tx(PROREG_HEX).txid();
        let group_key = provider_hash_to_32(pro_tx_hash.as_ref());

        // Build a ProUpServTx directly (no raw-hex vector needed); `port`
        // distinguishes the resulting service address, `inputs` perturbs the
        // txid so the two txs are genuinely distinct.
        let make_upserv = |port: u16, inputs: u8| -> dashcore::Transaction {
            let payload = ProviderUpdateServicePayload {
                version: 1,
                mn_type: None,
                pro_tx_hash,
                ip_address: 42,
                port,
                script_payout: dashcore::ScriptBuf::new(),
                inputs_hash: [inputs; 32].into(),
                platform_node_id: None,
                platform_p2p_port: None,
                platform_http_port: None,
                payload_sig: [0u8; 96].into(),
            };
            dashcore::Transaction {
                version: 3,
                lock_time: 0,
                input: vec![],
                output: vec![],
                special_transaction_payload: Some(
                    TransactionPayload::ProviderUpdateServicePayloadType(payload),
                ),
            }
        };

        let low = make_upserv(19000, 3); // in-block position 0
        let high = make_upserv(19999, 4); // in-block position 1 (block-latest)

        for feed in [
            [(500u32, 0u32, &low), (500u32, 1u32, &high)],
            // Reversed feed order (block-latest fed first): position, not feed
            // order, must still pick the winner.
            [(500u32, 1u32, &high), (500u32, 0u32, &low)],
        ] {
            let mns = aggregate_masternodes(feed.into_iter(), unavailable_dml);
            assert_eq!(mns.len(), 1, "same proTxHash ⇒ one bucket");
            assert_eq!(mns[0].pro_tx_hash, group_key);
            assert!(
                mns[0]
                    .service_address
                    .as_deref()
                    .unwrap_or_default()
                    .ends_with(":19999"),
                "higher in-block position (block-latest) must win; got {:?}",
                mns[0].service_address
            );
            assert_eq!(mns[0].tx_count, 2, "both updates counted");
        }
    }

    /// The platform HTTP port travels with the service endpoint: the ProRegTx
    /// seeds it and a later ProUpServTx replaces it (latest-wins), so the
    /// DAPI address the wallet builds follows the node's current config.
    #[test]
    fn platform_http_port_follows_the_service_update() {
        use dashcore::blockdata::transaction::special_transaction::provider_update_service::ProviderUpdateServicePayload;
        use dashcore::transaction::special_transaction::provider_registration::ProviderMasternodeType;
        use dashcore::transaction::TransactionPayload;

        let mut reg = decode_tx(PROREG_HEX);
        if let Some(TransactionPayload::ProviderRegistrationPayloadType(p)) =
            &mut reg.special_transaction_payload
        {
            p.masternode_type = ProviderMasternodeType::HighPerformance;
            p.platform_http_port = Some(443);
        }
        let pro_tx_hash = reg.txid();

        let upserv = dashcore::Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: Some(
                TransactionPayload::ProviderUpdateServicePayloadType(
                    ProviderUpdateServicePayload {
                        version: 2,
                        mn_type: Some(1), // HighPerformance (evonode)
                        pro_tx_hash,
                        ip_address: 42,
                        port: 19999,
                        script_payout: dashcore::ScriptBuf::new(),
                        inputs_hash: [7u8; 32].into(),
                        platform_node_id: None,
                        platform_p2p_port: Some(36656),
                        platform_http_port: Some(1443),
                        payload_sig: [0u8; 96].into(),
                    },
                ),
            ),
        };

        // Registration alone ⇒ the ProRegTx port.
        let mns = aggregate_masternodes([(100u32, 0u32, &reg)].into_iter(), unavailable_dml);
        assert_eq!(mns.len(), 1);
        assert_eq!(mns[0].platform_http_port, Some(443));

        // A later ProUpServTx replaces it along with the service address.
        let mns = aggregate_masternodes(
            [(100u32, 0u32, &reg), (200u32, 0u32, &upserv)].into_iter(),
            unavailable_dml,
        );
        assert_eq!(mns.len(), 1, "same proTxHash ⇒ one bucket");
        assert_eq!(mns[0].platform_http_port, Some(1443));
        assert!(
            mns[0]
                .service_address
                .as_deref()
                .unwrap_or_default()
                .ends_with(":19999"),
            "service address and platform port move together"
        );

        // The FFI entry carries it gated by `has_platform_http_port`.
        let entry = masternode_entry_ffi(
            &mns[0],
            0,
            dashcore::Network::Testnet,
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
        );
        assert!(entry.has_platform_http_port);
        assert_eq!(entry.platform_http_port, 1443);
        // Release the entry's heap C strings through the public free routine.
        let entries = Box::into_raw(vec![entry].into_boxed_slice()) as *mut MasternodeEntryFFI;
        unsafe { crate::wallet::platform_wallet_manager_free_masternodes(entries, 1) };
    }
}
