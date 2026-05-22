//! Found-021 — `TransactionRecord::update_context` silently drops the
//! `InstantLock` when a transaction is promoted from `InstantSend` to `InBlock`.
//!
//! **Spec**: `tests/e2e/TEST_SPEC.md` (§ Found bugs → Found-021).
//! **Upstream defect site**: `key-wallet/src/managed_account/transaction_record.rs:182-184`
//! (`TransactionRecord::update_context`: `self.context = context;`).
//! **Tracking issue**: dashpay/rust-dashcore#763.
//! **Pinned status**: RED-BY-DESIGN — pure unit test; pins upstream bug until fix lands.
//!
//! ## Bug shape
//!
//! `update_context` is a naive replacement:
//!
//! ```text
//! pub fn update_context(&mut self, context: TransactionContext) {
//!     self.context = context;
//! }
//! ```
//!
//! When a transaction is first seen with `TransactionContext::InstantSend(lock)`
//! and later promoted with `TransactionContext::InBlock(info)`, the IS-lock is
//! unconditionally overwritten. Any downstream consumer that reads the record
//! after block confirmation to use the lock as proof material (e.g. to construct
//! an `InstantAssetLockProof`) finds no lock.
//!
//! ## Related amplifier
//!
//! The production transition `InBlock(info)` → `InChainLockedBlock(info)` lives at
//! `key-wallet/src/managed_account/managed_core_keys_account.rs:129-139`. By the
//! time a record reaches that path, the IS-lock is already gone — it was dropped
//! at the prior IS → InBlock hop pinned by this test. Two compounding lossy hops
//! to chain-lock proof, both caused by `update_context`'s naive replace.
//!
//! ## What this test pins
//!
//! The merging invariant:
//!
//!   * A `TransactionRecord` first updated with `InstantSend(lock)` carries that
//!     lock in `record.context`.
//!   * A subsequent `update_context(InBlock(info))` call MUST NOT silently discard
//!     the lock. After promotion the record EITHER (a) has a dedicated `instant_lock`
//!     field that still holds the original lock, OR (b) its `context` is a merged
//!     variant (`InBlockWithInstantLock { info, lock }`) that preserves both.
//!   * Counter-assertion (today's buggy behaviour): `record.context` is exactly
//!     `InBlock(info)` with no lock accessible — the IS-lock is gone.
//!
//! The test reconstructs the exact call sequence production code uses:
//! two `update_context` calls in order, then inspects the record. No SPV
//! harness or network connection is required.
//!
//! ## Test lifecycle
//!
//! **Today (bug present)**: the final assertion fires — `record.context` is
//! `InBlock(info)` with no lock accessible. This is the bug-pin's success
//! condition (red).
//!
//! **After upstream fix**: `update_context` merges context on IS → InBlock
//! promotion. Either a new `instant_lock` field on `TransactionRecord` persists
//! the lock independently, or a new `TransactionContext::InBlockWithInstantLock`
//! variant carries both. This test must be updated alongside the fix to assert
//! the lock is recoverable.
//!
//! ## Why no SPV harness
//!
//! Constructing a BLS-signed, quorum-validated `InstantLock` for injection
//! through the real SPV pipeline requires a live masternode quorum — that is an
//! e2e infrastructure dependency orthogonal to the defect. The bug lives
//! entirely in `update_context`'s naive field assignment, which this test drives
//! directly through the public `TransactionRecord` API.

use key_wallet::account::{
    AccountType, StandardAccountType, TransactionDirection, TransactionRecord,
};
use key_wallet::dashcore::blockdata::transaction::Transaction;
use key_wallet::dashcore::ephemerealdata::instant_lock::InstantLock;
use key_wallet::dashcore::hashes::Hash;
use key_wallet::dashcore::BlockHash;
use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};

/// Build a minimal, structurally valid (non-special) `Transaction` for use as a
/// fixture. Real on-chain shape is irrelevant to this bug pin.
fn dummy_tx() -> Transaction {
    Transaction {
        version: 2,
        lock_time: 0,
        input: vec![],
        output: vec![],
        special_transaction_payload: None,
    }
}

/// Build a `TransactionRecord` in the `InstantSend` context — analogous to
/// the wallet's first observation of an asset-lock tx after IS-lock receipt.
fn record_with_instant_send(lock: InstantLock) -> TransactionRecord {
    TransactionRecord::new(
        dummy_tx(),
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        TransactionContext::InstantSend(lock),
        TransactionType::AssetLock,
        TransactionDirection::Outgoing,
        vec![],
        vec![],
        -100_000, // net outgoing amount in duffs
    )
}

/// Returns `true` if the `TransactionContext` carries an `InstantLock` in any
/// form. Today only `InstantSend(lock)` matches. After the upstream fix a
/// merged variant (e.g. `InBlockWithInstantLock`) would also match here.
fn context_has_instant_lock(ctx: &TransactionContext) -> bool {
    matches!(ctx, TransactionContext::InstantSend(_))
    // When the fix introduces InBlockWithInstantLock { .. }, extend to:
    // || matches!(ctx, TransactionContext::InBlockWithInstantLock { .. })
}

/// Bug-pin for Found-021.
///
/// **RED today**: after `update_context(InBlock(..))` the IS-lock is silently
/// dropped — `context_has_instant_lock` returns `false`.
///
/// **GREEN after fix**: `update_context` retains the lock on IS→InBlock
/// promotion; `context_has_instant_lock` (or an equivalent accessor) returns
/// `true`. The assertion in this test must be updated alongside the fix.
#[test]
fn found_021_instant_lock_dropped_on_context_promotion() {
    // ── 1. Construct a synthetic InstantLock ────────────────────────────
    //
    // `InstantLock::default()` gives all-zero fields — structurally valid
    // for the purpose of this bug pin; BLS signature verification is not
    // exercised here.
    let lock = InstantLock::default();

    // ── 2. Build a record in InstantSend state ──────────────────────────
    let mut record = record_with_instant_send(lock);

    // Pre-condition: record must start in InstantSend state.
    assert!(
        context_has_instant_lock(&record.context),
        "pre-condition: record.context must be InstantSend after construction"
    );

    // ── 3. Promote to InBlock — the operation that drops the lock ───────
    //
    // This is the call sequence the wallet's sync path uses when a block
    // confirmation event arrives for an already-IS-locked transaction.
    let block_info = BlockInfo::new(
        100_000,                // height
        BlockHash::all_zeros(), // block_hash (synthetic)
        1_700_000_000,          // timestamp
    );
    record.update_context(TransactionContext::InBlock(block_info));

    // ── 4. Assert the lock survives the promotion ───────────────────────
    //
    // This assertion FAILS today (bug present):
    //   record.context == InBlock(..) with no lock accessible.
    //
    // After the upstream fix:
    //   record.context carries both the block info AND the original lock
    //   (via a merged variant or a dedicated field), so this assertion passes.
    assert!(
        context_has_instant_lock(&record.context),
        "Found-021 (RED-by-design): InstantLock was silently dropped on InBlock promotion. \
         record.context after update_context(InBlock(..)) is {:?} — the IS-lock is gone. \
         update_context at key-wallet/src/managed_account/transaction_record.rs:182-184 \
         does `self.context = context;` unconditionally, overwriting the InstantSend(lock). \
         Fix: merge context on IS→InBlock: retain the lock in a dedicated field or a \
         new InBlockWithInstantLock variant. \
         See TEST_SPEC.md Found-021.",
        record.context
    );
}
