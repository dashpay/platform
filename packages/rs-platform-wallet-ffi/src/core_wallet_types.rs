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
    pub transaction_type: *mut c_char,
    pub net_amount: i64,
    pub fee: u64,
    pub has_fee: bool,
    pub label: *mut c_char,
    pub first_seen: u64,
}

// ---------------------------------------------------------------------------
// Per-account changeset
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct AccountChangeSetFFI {
    /// Account type name (Debug format of AccountType).
    pub account_type_name: *mut c_char,
    /// Account index (for indexed types, 0 otherwise).
    pub account_index: u32,
    /// UTXOs added.
    pub utxos_added: *mut UtxoEntryFFI,
    pub utxos_added_count: usize,
    /// Outpoints of UTXOs spent.
    pub utxos_spent: *mut OutPointFFI,
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
}

// ---------------------------------------------------------------------------
// Conversion
// ---------------------------------------------------------------------------

impl WalletChangeSetFFI {
    pub fn from_changeset(cs: &key_wallet::changeset::WalletChangeSet) -> Self {
        use key_wallet::managed_account::address_pool::AddressPoolType;
        use std::ffi::CString;

        let (has_chain, chain) = match cs.chain {
            Some(ref c) => (
                true,
                ChainChangeSetFFI {
                    has_synced_height: c.synced_height.is_some(),
                    synced_height: c.synced_height.unwrap_or(0),
                    has_block_hash: c.block_hash.is_some(),
                    block_hash: c.block_hash.map(|h| *h.as_ref()).unwrap_or([0u8; 32]),
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

        let (has_balance, balance) = match cs.balance {
            Some(ref b) => (
                true,
                BalanceChangeSetFFI {
                    confirmed_delta: b.confirmed_delta,
                    unconfirmed_delta: b.unconfirmed_delta,
                    immature_delta: b.immature_delta,
                    locked_delta: b.locked_delta,
                },
            ),
            None => (
                false,
                BalanceChangeSetFFI {
                    confirmed_delta: 0,
                    unconfirmed_delta: 0,
                    immature_delta: 0,
                    locked_delta: 0,
                },
            ),
        };

        let mut ffi_accounts = Vec::new();
        for (account_type, account_cs) in &cs.per_account {
            let type_name_str = format!("{:?}", account_type);
            let type_name =
                CString::new(type_name_str).unwrap_or_else(|_| CString::new("Unknown").unwrap());
            let account_index = account_type.index().unwrap_or(0);

            // UTXOs added
            let utxos_added: Vec<UtxoEntryFFI> = account_cs
                .utxos_added
                .values()
                .map(|utxo| {
                    let addr = CString::new(utxo.address.to_string())
                        .unwrap_or_else(|_| CString::new("").unwrap());
                    let script = utxo.txout.script_pubkey.as_bytes().to_vec();
                    let script_len = script.len();
                    let script_ptr = vec_to_ptr_u8(script, script_len);

                    UtxoEntryFFI {
                        outpoint: outpoint_to_ffi(&utxo.outpoint),
                        amount: utxo.txout.value,
                        address: addr.into_raw(),
                        script_pubkey: script_ptr,
                        script_pubkey_len: script_len,
                        height: utxo.height,
                        is_coinbase: utxo.is_coinbase,
                        is_confirmed: utxo.is_confirmed,
                        is_instantlocked: utxo.is_instantlocked,
                        is_locked: utxo.is_locked,
                    }
                })
                .collect();

            // UTXOs spent
            let utxos_spent: Vec<OutPointFFI> =
                account_cs.utxos_spent.iter().map(outpoint_to_ffi).collect();

            // UTXOs instant-locked
            let utxos_il: Vec<OutPointFFI> = account_cs
                .utxos_instant_locked
                .iter()
                .map(outpoint_to_ffi)
                .collect();

            // Transactions
            let transactions: Vec<TransactionRecordFFI> = account_cs
                .transactions
                .values()
                .map(tx_record_to_ffi)
                .collect();

            let ext_hu = account_cs.highest_used.get(&AddressPoolType::External);
            let int_hu = account_cs.highest_used.get(&AddressPoolType::Internal);

            let utxos_added_count = utxos_added.len();
            let utxos_spent_count = utxos_spent.len();
            let utxos_il_count = utxos_il.len();
            let transactions_count = transactions.len();

            ffi_accounts.push(AccountChangeSetFFI {
                account_type_name: type_name.into_raw(),
                account_index,
                utxos_added: vec_to_ptr(utxos_added),
                utxos_added_count,
                utxos_spent: vec_to_ptr(utxos_spent),
                utxos_spent_count,
                utxos_instant_locked: vec_to_ptr(utxos_il),
                utxos_instant_locked_count: utxos_il_count,
                transactions: vec_to_ptr(transactions),
                transactions_count,
                external_highest_used: ext_hu.map(|&v| v as i32).unwrap_or(-1),
                has_external_highest_used: ext_hu.is_some(),
                internal_highest_used: int_hu.map(|&v| v as i32).unwrap_or(-1),
                has_internal_highest_used: int_hu.is_some(),
            });
        }

        let accounts_count = ffi_accounts.len();
        WalletChangeSetFFI {
            has_chain,
            chain,
            has_balance,
            balance,
            accounts: vec_to_ptr(ffi_accounts),
            accounts_count,
        }
    }
}

fn outpoint_to_ffi(op: &dashcore::OutPoint) -> OutPointFFI {
    let mut txid = [0u8; 32];
    txid.copy_from_slice(op.txid.as_ref());
    OutPointFFI {
        txid,
        vout: op.vout,
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
    let label_str = CString::new(tr.label.clone()).unwrap_or_else(|_| CString::new("").unwrap());

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
        net_amount: tr.net_amount,
        fee: tr.fee.unwrap_or(0),
        has_fee: tr.fee.is_some(),
        label: label_str.into_raw(),
        first_seen: tr.first_seen,
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
