//! Synchronous host transaction adapter for wallet-owned history replay.

use std::collections::BTreeMap;
use std::os::raw::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};

use dashcore::{hashes::Hash, OutPoint, ScriptBuf, TxOut, Txid};
use parking_lot::Mutex;
use platform_wallet::changeset::recorded_history::ReplayEvidence;
use platform_wallet::changeset::{PersistenceError, RecordedHistory};
use platform_wallet::manager::history_replay::{
    replay_wallet_history_blocking, HistoryReplayError, HistoryReplayLoad, ReplayClaimEffect,
    StoredReplayClaim,
};
use platform_wallet::wallet::platform_wallet::WalletId;

use super::FFIPersister;
use crate::core_wallet_types::OutPointFFI;
use crate::dashpay_payment::{build_payment_persist_entries, DashpayPaymentPersistEntryFFI};

/// Scope-only callback. The host must invoke it once synchronously and never retain it.
pub type ReplayOperationFn = unsafe extern "C" fn(operation_context: *mut c_void) -> i32;
/// Run one wallet's replay inside a host storage transaction; nonzero means rollback.
pub type ReplayWalletFn = unsafe extern "C" fn(
    context: *mut c_void,
    wallet_id: *const u8,
    operation: ReplayOperationFn,
    operation_context: *mut c_void,
) -> i32;
/// Load owned coin material and durable claims from the current wallet snapshot.
pub type LoadReplayClaimsFn = unsafe extern "C" fn(
    context: *mut c_void,
    wallet_id: *const u8,
    out_rows: *mut *const ReplayClaimRestoreFFI,
    out_count: *mut usize,
) -> i32;
/// Release the complete host-owned array and its nested script buffers.
pub type FreeReplayClaimsFn =
    unsafe extern "C" fn(context: *mut c_void, rows: *const ReplayClaimRestoreFFI, count: usize);
/// Apply exactly the owner-computed effects inside the current wallet transaction.
pub type ApplyReplayEffectsFn = unsafe extern "C" fn(
    context: *mut c_void,
    wallet_id: *const u8,
    effects: *const HistoryReplayEffectsFFI,
) -> i32;

/// Coin material and optional hold; repeated outpoints may carry distinct claims.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ReplayClaimRestoreFFI {
    pub txid: [u8; 32],
    pub vout: u32,
    pub has_output: bool,
    pub value: u64,
    pub script: *const u8,
    pub script_len: usize,
    pub is_held: bool,
    pub has_claimant: bool,
    pub claimant: [u8; 32],
    pub has_mined_height: bool,
    pub mined_height: u32,
}

/// One Rust-owned claim verdict. The host must not infer its own release policy.
#[repr(C)]
pub struct ReplayClaimEffectFFI {
    pub txid: [u8; 32],
    pub vout: u32,
    /// 0 removes the output/placeholder, 1 releases it, 2 holds it.
    pub action: u8,
    pub has_claimant: bool,
    pub claimant: [u8; 32],
    pub has_mined_height: bool,
    pub mined_height: u32,
}

/// Borrowed only for ApplyReplayEffectsFn; all removals concern the supplied wallet.
#[repr(C)]
pub struct HistoryReplayEffectsFFI {
    pub removed_records: *const [u8; 32],
    pub removed_records_count: usize,
    pub removed_evidence: *const [u8; 32],
    pub removed_evidence_count: usize,
    pub claims: *const ReplayClaimEffectFFI,
    pub claims_count: usize,
    pub removed_asset_locks: *const OutPointFFI,
    pub removed_asset_locks_count: usize,
    pub payments: *const DashpayPaymentPersistEntryFFI,
    pub payments_count: usize,
}

#[derive(Clone, Copy, Default)]
pub(super) struct ReplayCallbacks {
    pub transaction: Option<ReplayWalletFn>,
    pub load_claims: Option<LoadReplayClaimsFn>,
    pub free_claims: Option<FreeReplayClaimsFn>,
    pub apply: Option<ApplyReplayEffectsFn>,
}

#[derive(Debug, thiserror::Error)]
enum ReplayAdapterError {
    #[error("wallet history replay requires all atomic transaction, claim load/free and effect callbacks")]
    IncompleteCallbacks,
    #[error("host must invoke the wallet replay operation exactly once before its transaction callback returns")]
    InvocationProtocol,
    #[error("host wallet replay transaction failed (code {0})")]
    HostFailure(i32),
    #[error("host wallet replay transaction failed (code {code}): {operation}")]
    HostAndOperationFailure {
        code: i32,
        #[source]
        operation: PersistenceError,
    },
    #[error("wallet replay operation panicked; host must roll back the transaction")]
    Panicked,
    #[error(
        "wallet replay claim callback returned invalid buffers or conflicting output material"
    )]
    InvalidClaims,
    #[error("wallet replay claim callback failed (code {0})")]
    ClaimRead(i32),
    #[error("wallet replay effect callback failed (code {0})")]
    EffectWrite(i32),
    #[error("wallet disappeared during atomic history replay")]
    MissingWallet,
}

impl ReplayCallbacks {
    fn any(self) -> bool {
        self.transaction.is_some()
            || self.load_claims.is_some()
            || self.free_claims.is_some()
            || self.apply.is_some()
    }
    pub(super) fn complete(self) -> bool {
        self.transaction.is_some()
            && self.load_claims.is_some()
            && self.free_claims.is_some()
            && self.apply.is_some()
    }
}

struct ReplayInvocation<'a> {
    persister: &'a FFIPersister,
    wallet_id: WalletId,
    evidence: &'a ReplayEvidence,
    calls: AtomicUsize,
    result: Mutex<Option<Result<HistoryReplayLoad, PersistenceError>>>,
}

unsafe extern "C" fn invoke_replay(context: *mut c_void) -> i32 {
    // SAFETY: the host contract permits this borrowed context only during its callback.
    let invocation = &*context.cast::<ReplayInvocation<'_>>();
    let previous = invocation.calls.fetch_add(1, Ordering::SeqCst);
    if previous != 0 {
        return 1;
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        invocation
            .persister
            .replay_in_scope(invocation.wallet_id, invocation.evidence)
    }))
    .unwrap_or_else(|_| Err(PersistenceError::backend(ReplayAdapterError::Panicked)));
    let failed = result.is_err();
    *invocation.result.lock() = Some(result);
    i32::from(failed)
}

impl FFIPersister {
    pub(super) fn load_wallet_replay_scope(
        &self,
        wallet_id: WalletId,
        evidence: &ReplayEvidence,
    ) -> Result<HistoryReplayLoad, PersistenceError> {
        let round = self.round_lock.lock();
        if round.in_round {
            return Err(PersistenceError::backend(
                "cannot replay an unclosed persistence round",
            ));
        }
        if !self.replay_callbacks.any() {
            return Ok(HistoryReplayLoad {
                state: self.load_snapshot_unlocked(Some(wallet_id))?,
                outcomes: BTreeMap::new(),
            });
        }
        if !self.replay_callbacks.complete() {
            return Err(PersistenceError::backend(
                ReplayAdapterError::IncompleteCallbacks,
            ));
        }
        let Some(transaction) = self.replay_callbacks.transaction else {
            return Err(PersistenceError::backend(
                ReplayAdapterError::IncompleteCallbacks,
            ));
        };
        let invocation = ReplayInvocation {
            persister: self,
            wallet_id,
            evidence,
            calls: AtomicUsize::new(0),
            result: Mutex::new(None),
        };
        // Synchronous host -> Rust -> private unlocked reads; no recursive round mutex.
        let code = unsafe {
            transaction(
                self.callbacks.context,
                wallet_id.as_ptr(),
                invoke_replay,
                (&invocation as *const ReplayInvocation<'_>)
                    .cast_mut()
                    .cast(),
            )
        };
        let result = invocation.result.into_inner();
        if invocation.calls.load(Ordering::SeqCst) != 1 || result.is_none() {
            return Err(PersistenceError::backend(
                ReplayAdapterError::InvocationProtocol,
            ));
        }
        match (code, result) {
            (0, Some(result)) => result,
            (code, Some(Err(operation))) => Err(PersistenceError::backend(
                ReplayAdapterError::HostAndOperationFailure { code, operation },
            )),
            (code, _) => Err(PersistenceError::backend(ReplayAdapterError::HostFailure(
                code,
            ))),
        }
    }

    fn replay_in_scope(
        &self,
        wallet_id: WalletId,
        evidence: &ReplayEvidence,
    ) -> Result<HistoryReplayLoad, PersistenceError> {
        let mut state = self.load_snapshot_unlocked(Some(wallet_id))?;
        let Some(wallet) = state.wallets.get_mut(&wallet_id) else {
            return Ok(HistoryReplayLoad {
                state,
                outcomes: BTreeMap::new(),
            });
        };
        let mut claims = self.load_replay_claims(wallet_id, &mut wallet.recorded_history)?;
        let outcome =
            replay_wallet_history_blocking(wallet, evidence).map_err(PersistenceError::backend)?;
        let records = wallet.recorded_history.transactions.clone();
        let effects = outcome
            .storage_effects(&records, &claims)
            .map_err(PersistenceError::backend)?;
        let has_effects = !effects.removed_records.is_empty()
            || !effects.removed_evidence.is_empty()
            || !effects.claims.is_empty();
        if (!outcome.sweeps.is_empty() || !outcome.authorization.rejected.is_empty())
            && !has_effects
        {
            return Err(PersistenceError::backend(
                HistoryReplayError::UnsettledAfterReplay,
            ));
        }
        if has_effects {
            let Some(apply) = self.replay_callbacks.apply else {
                return Err(PersistenceError::backend(
                    ReplayAdapterError::IncompleteCallbacks,
                ));
            };
            let removed_records: Vec<_> = effects
                .removed_records
                .iter()
                .map(|id| id.to_byte_array())
                .collect();
            let removed_evidence: Vec<_> = effects
                .removed_evidence
                .iter()
                .map(|id| id.to_byte_array())
                .collect();
            let claim_effects: Vec<_> = effects
                .claims
                .iter()
                .map(|(outpoint, effect)| {
                    let (action, claimant, height) = match effect {
                        ReplayClaimEffect::Remove => (0, None, None),
                        ReplayClaimEffect::Release => (1, None, None),
                        ReplayClaimEffect::Hold {
                            claimant,
                            mined_height,
                        } => (2, *claimant, *mined_height),
                    };
                    ReplayClaimEffectFFI {
                        txid: outpoint.txid.to_byte_array(),
                        vout: outpoint.vout,
                        action,
                        has_claimant: claimant.is_some(),
                        claimant: claimant.map(|id| id.to_byte_array()).unwrap_or_default(),
                        has_mined_height: height.is_some(),
                        mined_height: height.unwrap_or_default(),
                    }
                })
                .collect();
            let operations = effects.operation_effects(wallet);
            let asset_locks: Vec<_> = operations
                .asset_locks
                .removed
                .iter()
                .map(OutPointFFI::from)
                .collect();
            let (payments, _payment_strings) = build_payment_persist_entries(&operations.payments);
            let ffi = HistoryReplayEffectsFFI {
                removed_records: slice_pointer(&removed_records),
                removed_records_count: removed_records.len(),
                removed_evidence: slice_pointer(&removed_evidence),
                removed_evidence_count: removed_evidence.len(),
                claims: slice_pointer(&claim_effects),
                claims_count: claim_effects.len(),
                removed_asset_locks: slice_pointer(&asset_locks),
                removed_asset_locks_count: asset_locks.len(),
                payments: slice_pointer(&payments),
                payments_count: payments.len(),
            };
            let code = unsafe { apply(self.callbacks.context, wallet_id.as_ptr(), &ffi) };
            if code != 0 {
                return Err(PersistenceError::backend(ReplayAdapterError::EffectWrite(
                    code,
                )));
            }
            state = self.load_snapshot_unlocked(Some(wallet_id))?;
            let wallet = state
                .wallets
                .get_mut(&wallet_id)
                .ok_or_else(|| PersistenceError::backend(ReplayAdapterError::MissingWallet))?;
            claims = self.load_replay_claims(wallet_id, &mut wallet.recorded_history)?;
            let remaining = replay_wallet_history_blocking(wallet, evidence)
                .map_err(PersistenceError::backend)?;
            if !remaining.sweeps.is_empty()
                || !remaining.authorization.rejected.is_empty()
                || !remaining.withheld_asset_lock_inputs.is_empty()
            {
                return Err(PersistenceError::backend(
                    HistoryReplayError::UnsettledAfterReplay,
                ));
            }
            wallet.unconfirmed_outgoing_txs =
                outcome.resendable(std::mem::take(&mut wallet.unconfirmed_outgoing_txs));
        }
        if let Some(wallet) = state.wallets.get_mut(&wallet_id) {
            wallet.wallet_info.restore_spent_outpoints(
                &claims
                    .iter()
                    .map(|claim| (claim.outpoint, claim.claimant))
                    .collect::<Vec<_>>(),
            );
        }
        Ok(HistoryReplayLoad {
            state,
            outcomes: [(wallet_id, outcome)].into(),
        })
    }

    fn load_replay_claims(
        &self,
        wallet_id: WalletId,
        history: &mut RecordedHistory,
    ) -> Result<Vec<StoredReplayClaim>, PersistenceError> {
        let (Some(load), Some(free)) = (
            self.replay_callbacks.load_claims,
            self.replay_callbacks.free_claims,
        ) else {
            return Err(PersistenceError::backend(
                ReplayAdapterError::IncompleteCallbacks,
            ));
        };
        let mut rows = std::ptr::null();
        let mut count = 0;
        let code = unsafe {
            load(
                self.callbacks.context,
                wallet_id.as_ptr(),
                &mut rows,
                &mut count,
            )
        };
        let _guard = ClaimRowsGuard {
            context: self.callbacks.context,
            free,
            rows,
            count,
        };
        if code != 0 {
            return Err(PersistenceError::backend(ReplayAdapterError::ClaimRead(
                code,
            )));
        }
        if count
            > dashcore::consensus::encode::MAX_VEC_SIZE
                / std::mem::size_of::<ReplayClaimRestoreFFI>()
            || (rows.is_null() && count != 0)
        {
            return Err(PersistenceError::backend(ReplayAdapterError::InvalidClaims));
        }
        let rows = if count == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(rows, count) }
        };
        let mut claims = Vec::new();
        for row in rows {
            let outpoint = OutPoint::new(Txid::from_byte_array(row.txid), row.vout);
            if row.has_output {
                if row.script_len > dashcore::consensus::encode::MAX_VEC_SIZE
                    || (row.script.is_null() && row.script_len != 0)
                {
                    return Err(PersistenceError::backend(ReplayAdapterError::InvalidClaims));
                }
                let script = if row.script_len == 0 {
                    Vec::new()
                } else {
                    unsafe { std::slice::from_raw_parts(row.script, row.script_len) }.to_vec()
                };
                let output = TxOut {
                    value: row.value,
                    script_pubkey: ScriptBuf::from_bytes(script),
                };
                if history
                    .owned_prevouts
                    .insert(outpoint, output.clone())
                    .is_some_and(|old| old != output)
                {
                    return Err(PersistenceError::backend(ReplayAdapterError::InvalidClaims));
                }
            }
            if row.is_held {
                claims.push(StoredReplayClaim {
                    outpoint,
                    claimant: row
                        .has_claimant
                        .then(|| Txid::from_byte_array(row.claimant)),
                    mined_height: row.has_mined_height.then_some(row.mined_height),
                });
            }
        }
        Ok(claims)
    }
}

fn slice_pointer<T>(values: &[T]) -> *const T {
    if values.is_empty() {
        std::ptr::null()
    } else {
        values.as_ptr()
    }
}

struct ClaimRowsGuard {
    context: *mut c_void,
    free: FreeReplayClaimsFn,
    rows: *const ReplayClaimRestoreFFI,
    count: usize,
}
impl Drop for ClaimRowsGuard {
    fn drop(&mut self) {
        unsafe { (self.free)(self.context, self.rows, self.count) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_lock_persistence::AssetLockEntryFFI;
    use crate::persistence::{FFIPersister, PersistenceCallbacks, PersistenceExtensionCallbacks};
    use crate::types::FFINetwork;
    use crate::wallet_restore_types::RecordedTransactionRestoreFFI;
    use crate::wallet_restore_types::{UtxoRestoreEntryFFI, WalletRestoreEntryFFI};
    use dashcore::{Transaction, TxIn};
    use platform_wallet::changeset::recorded_history::ReplayEvidence;
    use platform_wallet::changeset::{PersistenceCapabilities, PlatformWalletPersistence};
    use std::cell::{Cell, RefCell};

    type AppliedClaim = (OutPoint, u8, Option<Txid>, Option<u32>);

    #[derive(Default)]
    struct Host {
        scopes: Cell<usize>,
        inside: Cell<bool>,
        scoped_reads: Cell<usize>,
        mode: Cell<u8>,
        fail_read: Cell<bool>,
        transaction: RefCell<Option<Transaction>>,
        replacement: RefCell<Option<Transaction>>,
        claim_frees: Cell<usize>,
        claim_mode: Cell<u8>,
        effects: Cell<usize>,
        additional: RefCell<Vec<(Transaction, u32)>>,
        account_xpub: Vec<u8>,
        applied_claims: RefCell<Vec<AppliedClaim>>,
        terminal_lock: Option<Transaction>,
        coin: Option<(OutPoint, TxOut)>,
        coin_held: Cell<bool>,
    }

    unsafe extern "C" fn scope(
        ctx: *mut c_void,
        _: *const u8,
        operation: ReplayOperationFn,
        operation_ctx: *mut c_void,
    ) -> i32 {
        let host = &*ctx.cast::<Host>();
        host.scopes.set(host.scopes.get() + 1);
        host.inside.set(true);
        let saved = host.additional.borrow().clone();
        let saved_coin = host.coin_held.get();
        let saved_claims = host.applied_claims.borrow().clone();
        if let Some(replacement) = host.replacement.borrow_mut().take() {
            *host.transaction.borrow_mut() = Some(replacement);
        }
        let first = if host.mode.get() == 5 {
            let address = operation_ctx as usize;
            std::thread::scope(|scope| {
                scope
                    .spawn(move || operation(address as *mut c_void))
                    .join()
                    .unwrap()
            })
        } else if host.mode.get() == 1 {
            0
        } else {
            operation(operation_ctx)
        };
        if host.mode.get() == 2 {
            let _ = operation(operation_ctx);
        }
        host.inside.set(false);
        if first != 0 || host.mode.get() == 4 {
            *host.additional.borrow_mut() = saved;
            host.coin_held.set(saved_coin);
            *host.applied_claims.borrow_mut() = saved_claims;
        }
        match host.mode.get() {
            1..=3 => 0,
            4 => 71,
            _ => first,
        }
    }

    unsafe extern "C" fn load(
        ctx: *mut c_void,
        rows: *mut *const WalletRestoreEntryFFI,
        count: *mut usize,
    ) -> i32 {
        let host = &*ctx.cast::<Host>();
        if host.inside.get() {
            host.scoped_reads.set(host.scoped_reads.get() + 1);
        }
        if host.fail_read.get() {
            return 73;
        }
        let transactions: Vec<_> = host
            .transaction
            .borrow()
            .iter()
            .cloned()
            .map(|tx| (tx, 1))
            .chain(host.additional.borrow().iter().cloned())
            .collect();
        let history: Vec<_> = transactions
            .iter()
            .map(|(transaction, context)| {
                let bytes = dashcore::consensus::serialize(transaction).into_boxed_slice();
                let len = bytes.len();
                RecordedTransactionRestoreFFI {
                    txid: transaction.txid().to_byte_array(),
                    tx_bytes: Box::into_raw(bytes).cast(),
                    tx_bytes_len: len,
                    context: *context,
                    block_height: if *context >= 2 { 42 } else { 0 },
                    block_hash: [1; 32],
                    block_timestamp: 0,
                    block_position: 0,
                    has_block_position: false,
                    net_amount: 0,
                    direction: 0,
                }
            })
            .collect();
        let history_len = history.len();
        let history = Box::into_raw(history.into_boxed_slice()).cast();
        let account = (!host.account_xpub.is_empty()).then(|| {
            let kind = key_wallet::account::AccountType::Standard {
                index: 0,
                standard_account_type: key_wallet::account::StandardAccountType::BIP44Account,
            };
            Box::into_raw(Box::new(super::super::build_account_spec_ffi(
                &kind,
                &host.account_xpub,
            )))
        });
        let coin =
            host.coin
                .as_ref()
                .filter(|_| !host.coin_held.get())
                .map(|(outpoint, output)| {
                    let spec = &*account.unwrap();
                    Box::into_raw(Box::new(UtxoRestoreEntryFFI {
                        type_tag: spec.type_tag,
                        standard_tag: spec.standard_tag,
                        account_index: spec.index,
                        registration_index: 0,
                        key_class: 0,
                        user_identity_id: [0; 32],
                        friend_identity_id: [0; 32],
                        prev_txid: outpoint.txid.to_byte_array(),
                        vout: outpoint.vout,
                        value_duffs: output.value,
                        script_pubkey: output.script_pubkey.as_bytes().as_ptr(),
                        script_pubkey_len: output.script_pubkey.len(),
                        height: 100,
                        is_coinbase: false,
                        is_confirmed: true,
                        is_instantlocked: false,
                        is_locked: false,
                        parent_context: 2,
                        parent_block_height: 100,
                    }))
                });
        let terminal = host.terminal_lock.as_ref().map(|transaction| {
            let bytes = dashcore::consensus::serialize(transaction).into_boxed_slice();
            let mut out_point = [0; 36];
            out_point[..32].copy_from_slice(transaction.txid().as_byte_array());
            out_point[32..].copy_from_slice(&1_u32.to_le_bytes());
            Box::into_raw(Box::new(AssetLockEntryFFI {
                out_point,
                transaction_bytes_len: bytes.len(),
                transaction_bytes: Box::into_raw(bytes).cast(),
                account_index: 0,
                funding_type: 0,
                identity_index: 0,
                amount_duffs: 50_000,
                status: 4,
                proof_bytes: std::ptr::null(),
                proof_bytes_len: 0,
            }))
        });
        *rows = Box::into_raw(Box::new(WalletRestoreEntryFFI {
            wallet_id: [41; 32],
            accounts: account.unwrap_or(std::ptr::null_mut()),
            accounts_count: usize::from(account.is_some()),
            network: FFINetwork::Testnet,
            recorded_transactions: history,
            recorded_transactions_count: history_len,
            utxos: coin.unwrap_or(std::ptr::null_mut()),
            utxos_count: usize::from(coin.is_some()),
            tracked_asset_locks: terminal.unwrap_or(std::ptr::null_mut()),
            tracked_asset_locks_count: usize::from(terminal.is_some()),
            ..Default::default()
        }));
        *count = 1;
        0
    }
    unsafe extern "C" fn free(_: *mut c_void, rows: *const WalletRestoreEntryFFI, _: usize) {
        if !rows.is_null() {
            let entry = Box::from_raw(rows.cast_mut());
            if !entry.accounts.is_null() {
                drop(Box::from_raw(entry.accounts.cast_mut()));
            }
            if !entry.utxos.is_null() {
                drop(Box::from_raw(entry.utxos.cast_mut()));
            }
            if !entry.tracked_asset_locks.is_null() {
                let lock = Box::from_raw(entry.tracked_asset_locks.cast_mut());
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    lock.transaction_bytes.cast_mut(),
                    lock.transaction_bytes_len,
                )));
            }
            if !entry.recorded_transactions.is_null() {
                let records = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    entry.recorded_transactions.cast_mut(),
                    entry.recorded_transactions_count,
                ));
                for record in &records {
                    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                        record.tx_bytes,
                        record.tx_bytes_len,
                    )));
                }
            }
        }
    }
    unsafe extern "C" fn claims(
        ctx: *mut c_void,
        _: *const u8,
        rows: *mut *const ReplayClaimRestoreFFI,
        count: *mut usize,
    ) -> i32 {
        *rows = std::ptr::null();
        let host = &*ctx.cast::<Host>();
        *count = usize::from(host.claim_mode.get() == 2);
        if host.claim_mode.get() == 1 {
            79
        } else {
            0
        }
    }
    unsafe extern "C" fn free_claims(ctx: *mut c_void, _: *const ReplayClaimRestoreFFI, _: usize) {
        let host = &*ctx.cast::<Host>();
        host.claim_frees.set(host.claim_frees.get() + 1);
    }
    unsafe extern "C" fn apply(
        ctx: *mut c_void,
        _: *const u8,
        effects: *const HistoryReplayEffectsFFI,
    ) -> i32 {
        let host = &*ctx.cast::<Host>();
        assert!(host.inside.get());
        host.effects.set(host.effects.get() + 1);
        let effects = &*effects;
        if effects.removed_records_count != 0 {
            let removed =
                std::slice::from_raw_parts(effects.removed_records, effects.removed_records_count);
            host.additional
                .borrow_mut()
                .retain(|(tx, _)| !removed.contains(&tx.txid().to_byte_array()));
        }
        if effects.claims_count != 0 {
            let claims = std::slice::from_raw_parts(effects.claims, effects.claims_count);
            *host.applied_claims.borrow_mut() = claims
                .iter()
                .map(|claim| {
                    (
                        OutPoint::new(Txid::from_byte_array(claim.txid), claim.vout),
                        claim.action,
                        claim
                            .has_claimant
                            .then(|| Txid::from_byte_array(claim.claimant)),
                        claim.has_mined_height.then_some(claim.mined_height),
                    )
                })
                .collect();
        }
        if host.mode.get() != 7 && host.coin.is_some() && effects.claims_count > 0 {
            host.coin_held.set(true);
        }
        if host.mode.get() == 6 {
            81
        } else {
            0
        }
    }

    fn host_with_funds_account() -> Host {
        let wallet = key_wallet::Wallet::from_seed_bytes(
            [0x51; 64],
            dashcore::Network::Testnet,
            key_wallet::wallet::initialization::WalletAccountCreationOptions::Default,
        )
        .unwrap();
        Host {
            account_xpub: bincode::encode_to_vec(
                wallet.get_bip44_account(0).unwrap().account_xpub,
                bincode::config::standard(),
            )
            .unwrap(),
            ..Default::default()
        }
    }

    #[test]
    fn should_apply_exact_owner_claims_and_reread_inside_the_host_transaction() {
        let winner = transaction(1);
        let loser = transaction(2);
        let host = host_with_funds_account();
        *host.additional.borrow_mut() = vec![(winner.clone(), 3), (loser, 0)];
        let loaded = persister(&host)
            .load_with_replay(&ReplayEvidence::default())
            .unwrap();
        assert_eq!(loaded.state.wallets.len(), 1);
        assert_eq!(host.effects.get(), 1);
        assert_eq!(*host.additional.borrow(), vec![(winner.clone(), 3)]);
        assert_eq!(
            *host.applied_claims.borrow(),
            vec![(
                winner.input[0].previous_output,
                2,
                Some(winner.txid()),
                Some(42)
            )]
        );
        assert!(host.scoped_reads.get() >= 2);
    }

    #[test]
    fn should_rollback_host_row_deletion_when_effect_application_fails() {
        let original = vec![(transaction(1), 3), (transaction(2), 0)];
        let host = host_with_funds_account();
        *host.additional.borrow_mut() = original.clone();
        host.mode.set(6);
        let error = persister(&host)
            .load_with_replay(&ReplayEvidence::default())
            .unwrap_err();
        assert!(format!("{error:?}").contains("81"), "{error:?}");
        assert_eq!(host.effects.get(), 1);
        assert_eq!(*host.additional.borrow(), original);
    }

    #[test]
    fn should_apply_terminal_input_hold_atomically_and_reject_a_noop_host() {
        use dashcore::transaction::{
            special_transaction::asset_lock::AssetLockPayload, TransactionPayload,
        };
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        for mode in [0, 6, 7] {
            let wallet = key_wallet::Wallet::from_seed_bytes(
                [0x51; 64],
                dashcore::Network::Testnet,
                WalletAccountCreationOptions::Default,
            )
            .unwrap();
            let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
            let xpub = wallet.accounts.standard_bip44_accounts[&0].account_xpub;
            let address = info
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .unwrap()
                .next_receive_address(Some(&xpub), true)
                .unwrap();
            let mut lock = transaction(90);
            lock.version = 3;
            lock.special_transaction_payload = Some(TransactionPayload::AssetLockPayloadType(
                AssetLockPayload::new(
                    [49_000, 50_000]
                        .map(|value| TxOut {
                            value,
                            script_pubkey: address.script_pubkey(),
                        })
                        .to_vec(),
                ),
            ));
            let spent = lock.input[0].previous_output;
            let mut host = host_with_funds_account();
            host.coin = Some((
                spent,
                TxOut {
                    value: 100_000,
                    script_pubkey: address.script_pubkey(),
                },
            ));
            host.terminal_lock = Some(lock.clone());
            host.mode.set(mode);
            let result = persister(&host).load_with_replay(&ReplayEvidence::default());
            if mode == 0 {
                let loaded = result.unwrap();
                assert_eq!(
                    loaded.state.wallets[&[41; 32]].wallet_info.balance.total(),
                    0
                );
                assert!(host.coin_held.get());
                assert_eq!(
                    *host.applied_claims.borrow(),
                    vec![(spent, 2, Some(lock.txid()), None)]
                );
                let again = persister(&host)
                    .load_with_replay(&ReplayEvidence::default())
                    .unwrap();
                assert_eq!(
                    again.state.wallets[&[41; 32]].wallet_info.balance.total(),
                    0
                );
            } else {
                assert!(
                    result.is_err(),
                    "a failed or unapplied hold must not publish a wallet"
                );
                assert!(
                    !host.coin_held.get(),
                    "transaction rollback preserves the original host state"
                );
                assert!(host.applied_claims.borrow().is_empty());
            }
        }
    }

    fn persister(host: &Host) -> FFIPersister {
        FFIPersister::new_with_persistence_capabilities_and_extensions(
            PersistenceCallbacks {
                context: (host as *const Host).cast_mut().cast(),
                on_load_wallet_list_fn: Some(load),
                on_load_wallet_list_free_fn: Some(free),
                ..Default::default()
            },
            PersistenceCapabilities::NONE,
            PersistenceExtensionCallbacks {
                replay_wallet: Some(scope),
                load_replay_claims: Some(claims),
                free_replay_claims: Some(free_claims),
                apply_replay_effects: Some(apply),
                ..Default::default()
            },
        )
    }

    fn transaction(marker: u32) -> Transaction {
        Transaction {
            version: 1,
            lock_time: marker,
            input: vec![TxIn {
                previous_output: OutPoint::new(Txid::from_byte_array([7; 32]), 0),
                ..Default::default()
            }],
            output: vec![],
            special_transaction_payload: None,
        }
    }

    async fn checked_evidence(transaction: &Transaction) -> ReplayEvidence {
        use dash_spv::test_utils::{InstantLockFixture, MockNetworkManager};
        use dash_spv::{ClientConfig, DashSpvClient};
        use key_wallet_manager::test_utils::MockWallet;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        struct Storage(std::path::PathBuf);
        impl Drop for Storage {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let storage = Storage(std::env::temp_dir().join(format!(
            "ffi-replay-proof-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&storage.0).unwrap();
        let client = DashSpvClient::new(
            ClientConfig::new(dashcore::Network::Testnet)
                .without_filters()
                .with_storage_path(&storage.0),
            MockNetworkManager::new(),
            Arc::new(RwLock::new(MockWallet::new())),
            Vec::new(),
        )
        .await
        .unwrap();
        let fixture = InstantLockFixture::new(transaction).unwrap();
        fixture.install(&client).await.unwrap();
        let proof = client
            .verify_instant_lock(fixture.instant_lock())
            .await
            .unwrap()
            .unwrap();
        ReplayEvidence {
            instant_locks: [(proof.txid(), proof)].into(),
        }
    }

    #[tokio::test]
    async fn should_allow_explicit_replay_retry_only_with_checked_matching_evidence() {
        let host = Host::default();
        let transaction = transaction(2);
        *host.transaction.borrow_mut() = Some(transaction.clone());
        let persister = persister(&host);
        assert!(persister
            .load_wallet_with_replay([41; 32], &ReplayEvidence::default())
            .is_err());
        let evidence = checked_evidence(&transaction).await;
        let restored = persister
            .load_wallet_with_replay([41; 32], &evidence)
            .unwrap();
        assert!(restored.state.wallets.contains_key(&[41; 32]));
        assert_eq!(host.effects.get(), 0);
        assert_eq!(host.scopes.get(), 2);
    }

    #[tokio::test]
    async fn should_rebind_prefetched_evidence_to_the_fresh_host_transaction() {
        let host = Host::default();
        let old = transaction(3);
        *host.transaction.borrow_mut() = Some(old.clone());
        let persister = persister(&host);
        let _discovery = persister.load_read_snapshot().unwrap();
        let evidence = checked_evidence(&old).await;
        let fresh = transaction(4);
        let fresh_id = fresh.txid();
        *host.replacement.borrow_mut() = Some(fresh);
        let error = persister
            .load_wallet_with_replay([41; 32], &evidence)
            .unwrap_err();
        assert!(error.to_string().contains(&fresh_id.to_string()));
        assert_eq!(host.effects.get(), 0);
    }

    #[test]
    fn should_fail_missing_required_proof_before_any_host_effects() {
        let host = Host::default();
        let transaction = transaction(1);
        let txid = transaction.txid();
        *host.transaction.borrow_mut() = Some(transaction);
        let error = persister(&host)
            .load_wallet_with_replay([41; 32], &ReplayEvidence::default())
            .unwrap_err();
        assert!(error.to_string().contains(&txid.to_string()));
        assert_eq!(host.effects.get(), 0);
        assert_eq!(host.claim_frees.get(), 1);
        assert!(host.transaction.borrow().is_some());
    }

    #[test]
    fn should_free_claim_buffers_on_callback_and_decode_failure() {
        for mode in [1, 2] {
            let host = Host::default();
            host.claim_mode.set(mode);
            assert!(persister(&host)
                .load_wallet_with_replay([41; 32], &ReplayEvidence::default())
                .is_err());
            assert_eq!(host.claim_frees.get(), 1);
            assert_eq!(host.effects.get(), 0);
        }
    }

    #[test]
    fn should_require_complete_atomic_callbacks_for_wallet_restore_capability() {
        let host = Host::default();
        let mut persister = persister(&host);
        persister.declared_capabilities = PersistenceCapabilities::WALLET_RESTORE;
        assert!(persister
            .persistence_capabilities()
            .contains(PersistenceCapabilities::WALLET_RESTORE));
        for callbacks in [
            ReplayCallbacks {
                transaction: None,
                ..persister.replay_callbacks
            },
            ReplayCallbacks {
                load_claims: None,
                ..persister.replay_callbacks
            },
            ReplayCallbacks {
                free_claims: None,
                ..persister.replay_callbacks
            },
            ReplayCallbacks {
                apply: None,
                ..persister.replay_callbacks
            },
            ReplayCallbacks::default(),
        ] {
            persister.replay_callbacks = callbacks;
            assert!(!persister
                .persistence_capabilities()
                .contains(PersistenceCapabilities::WALLET_RESTORE));
        }
    }

    #[test]
    fn should_read_wallet_inside_exactly_one_atomic_replay_scope() {
        let host = Host::default();
        let loaded = persister(&host)
            .load_wallet_with_replay([41; 32], &ReplayEvidence::default())
            .unwrap();
        assert!(loaded.state.wallets.contains_key(&[41; 32]));
        assert_eq!(host.scopes.get(), 1);
        assert!(host.scoped_reads.get() > 0);
        assert!(!host.inside.get());
    }

    #[test]
    fn should_accept_a_synchronous_replay_thunk_on_the_hosts_serial_thread() {
        let host = Host::default();
        host.mode.set(5);
        let loaded = persister(&host)
            .load_wallet_with_replay([41; 32], &ReplayEvidence::default())
            .unwrap();
        assert!(loaded.state.wallets.contains_key(&[41; 32]));
        assert_eq!(host.scopes.get(), 1);
        assert_eq!(host.scoped_reads.get(), 1);
    }

    #[test]
    fn should_reject_missing_or_repeated_atomic_replay_thunk() {
        for mode in [1, 2] {
            let host = Host::default();
            host.mode.set(mode);
            assert!(
                persister(&host)
                    .load_wallet_with_replay([41; 32], &ReplayEvidence::default())
                    .is_err(),
                "mode={mode}"
            );
        }
    }

    #[test]
    fn should_preserve_rust_failure_when_host_reports_replay_success() {
        let host = Host::default();
        host.mode.set(3);
        host.fail_read.set(true);
        let error = persister(&host)
            .load_wallet_with_replay([41; 32], &ReplayEvidence::default())
            .unwrap_err();
        assert!(error.to_string().contains("73"));
        assert_eq!(host.scopes.get(), 1);
    }

    #[test]
    fn should_reject_host_failure_after_successful_replay() {
        let host = Host::default();
        host.mode.set(4);
        assert!(persister(&host)
            .load_wallet_with_replay([41; 32], &ReplayEvidence::default())
            .is_err());
    }
}
