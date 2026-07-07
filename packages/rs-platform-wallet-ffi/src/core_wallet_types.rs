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
                // Highest-used pool indices were a feature of the
                // deleted upstream changeset's per-account bucket.
                // The new event-bus model doesn't surface them; the
                // persister can derive them from monitored addresses
                // if needed.
                external_highest_used: -1,
                has_external_highest_used: false,
                internal_highest_used: -1,
                has_internal_highest_used: false,
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
    }
}

/// Convert a `Vec` into a raw heap pointer for a C out-array: null for
/// empty, `Box::into_raw(boxed_slice)` otherwise. The caller owns the
/// allocation and must free it by reconstructing the boxed slice with
/// the ORIGINAL length.
pub(crate) fn vec_to_ptr<T>(v: Vec<T>) -> *mut T {
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
    // after `is_empty` filters out the other arms) must
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
