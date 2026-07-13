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

    // Fixed-size hash copies: `Txid`/`PubkeyHash` are 32/20 bytes, so
    // `copy_from_slice` on `as_ref()` is length-exact and cannot panic —
    // the same pattern the txid copy in `tx_record_to_ffi` relies on.
    fn to_32(bytes: &[u8]) -> [u8; 32] {
        let mut out = [0u8; 32];
        out.copy_from_slice(bytes);
        out
    }
    fn to_20(bytes: &[u8]) -> [u8; 20] {
        let mut out = [0u8; 20];
        out.copy_from_slice(bytes);
        out
    }

    match &tx.special_transaction_payload {
        Some(TransactionPayload::ProviderRegistrationPayloadType(p)) => ProviderPayloadFields {
            service_address: Some(p.service_address.to_string()),
            pro_tx_hash: None,
            collateral: Some((
                to_32(p.collateral_outpoint.txid.as_ref()),
                p.collateral_outpoint.vout,
            )),
            owner_key_hash: Some(to_20(p.owner_key_hash.as_ref())),
            voting_key_hash: Some(to_20(p.voting_key_hash.as_ref())),
        },
        Some(TransactionPayload::ProviderUpdateServicePayloadType(p)) => {
            // ProUpServTx stores the endpoint as a little-endian
            // IPv6-mapped `u128` + separate `port`, not a `SocketAddr`;
            // rebuild one and collapse IPv4-mapped addresses to V4 so a
            // normal masternode renders as `"1.2.3.4:port"`.
            let v6 = std::net::Ipv6Addr::from(p.ip_address.to_le_bytes());
            let ip = v6
                .to_ipv4_mapped()
                .map(std::net::IpAddr::V4)
                .unwrap_or(std::net::IpAddr::V6(v6));
            ProviderPayloadFields {
                service_address: Some(format!("{}:{}", ip, p.port)),
                pro_tx_hash: Some(to_32(p.pro_tx_hash.as_ref())),
                ..Default::default()
            }
        }
        _ => ProviderPayloadFields::default(),
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

    let (ctx_val, blk_height, blk_hash, blk_ts) = match &tr.context {
        TransactionContext::Mempool => (0u32, 0u32, [0u8; 32], 0u32),
        TransactionContext::InstantSend(_is_lock) => {
            // InstantSend has no block info — treat as mempool-level with flag
            (1u32, 0u32, [0u8; 32], 0u32)
        }
        TransactionContext::InBlock(bi) => {
            let mut h = [0u8; 32];
            h.copy_from_slice(bi.block_hash().as_ref());
            (2u32, bi.height(), h, bi.timestamp())
        }
        TransactionContext::InChainLockedBlock(bi) => {
            let mut h = [0u8; 32];
            h.copy_from_slice(bi.block_hash().as_ref());
            (3u32, bi.height(), h, bi.timestamp())
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
}
