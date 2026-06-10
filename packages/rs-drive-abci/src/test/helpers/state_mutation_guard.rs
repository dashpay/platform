//! CheckTx committed-state mutation guard.
//!
//! CheckTx (mempool validation and fee estimation) runs OUTSIDE any block transaction:
//! `check_tx_v0` calls `validate_fees_of_event(..., transaction: None, ...)`, which applies the
//! event's drive operations in estimation mode (`apply: false`). Any code on that path that
//! eagerly writes to GroveDB instead of returning operations therefore commits DIRECTLY to disk
//! on every node that validates the gossiped transition. The on-disk root hash then diverges from
//! the last signed app hash, every proposer panics with "drive and platform state app hash
//! mismatch" (`abci/handler/prepare_proposal.rs`), and restarts panic forever in
//! `abci/handler/info.rs` because the write is durable.
//!
//! This exact class halted devnet paloma at height 788 on 2026-06-10: pre-#3823, the
//! `ShieldedPoolOperationType::InsertNullifiers` low-level converter arm eagerly called
//! `drive.store_nullifiers_for_block(...)` with whatever `TransactionArg` it was handed (`None`
//! during CheckTx fee estimation). #3823 made the converter pure; the helpers here guard the
//! CLASS by asserting the committed root hash is byte-identical around CheckTx-path entry points.

use dpp::version::PlatformVersion;
use drive::drive::Drive;

/// The committed (`transaction: None`, i.e. on-disk) GroveDB root hash.
pub fn committed_root_hash(drive: &Drive, platform_version: &PlatformVersion) -> [u8; 32] {
    drive
        .grove
        .root_hash(None, &platform_version.drive.grove_version)
        .unwrap()
        .expect("expected to fetch the committed grovedb root hash")
}

/// Runs `action` and asserts the committed GroveDB root hash is byte-identical before and after,
/// returning `action`'s result.
///
/// Wrap CheckTx-path entry points (`check_tx`, `state_transition_to_execution_event_for_check_tx`,
/// `validate_fees_of_event` with `transaction: None`) in this guard: CheckTx must NEVER mutate
/// committed state (see the module docs for the paloma height-788 halt this guards against).
pub fn assert_committed_root_hash_unchanged<R>(
    drive: &Drive,
    platform_version: &PlatformVersion,
    context: &str,
    action: impl FnOnce() -> R,
) -> R {
    let before = committed_root_hash(drive, platform_version);
    let result = action();
    let after = committed_root_hash(drive, platform_version);
    assert_eq!(
        before, after,
        "{context} mutated committed grovedb state: CheckTx runs with transaction = None, so an \
         eager write on this path commits straight to disk, diverging the root from the signed \
         app hash and halting the chain (devnet paloma, height 788)"
    );
    result
}
