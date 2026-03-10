# Audit Findings — PR #3220 (feat/zk-drive-abci)

**Date**: 2026-03-10
**Branch**: `feat/zk-drive-abci`
**Base**: `v2.1-dev`
**Auditors**: 5 specialized agents (blockchain security, Rust quality, test coverage, integer safety, pipeline ordering)

## Summary

PR adds shielded pool drive-abci integration (Shield, ShieldedTransfer, Unshield, ShieldedWithdrawal, ShieldFromAssetLock state transitions). 84 files changed, ~10,500 lines added.

## Bug Found & Fixed During Audit

**Platform version config missing `basic_structure` for 2 transitions** — `shield_from_asset_lock_state_transition` and `shielded_withdrawal_state_transition` had `basic_structure: None` in `v8.rs`, causing structure validation to be skipped entirely. Fixed by setting both to `Some(0)` in `packages/rs-platform-version/src/version/drive_abci_versions/drive_abci_validation_versions/v8.rs`. This was causing 6 test failures where structure errors were masked by later validation steps (ECDSA signature check for shield_from_asset_lock, insufficient fee check for shielded_withdrawal).

---

## Findings by Severity

### HIGH

#### H1: Missing `i64::MAX` bound check on `ShieldTransitionV0::amount`

**Status**: FIXED
**Location**: `packages/rs-dpp/src/state_transition/state_transitions/shielded/shield_transition/v0/state_transition_validation.rs`

All other shielded transitions validate their monetary field `<= i64::MAX` before the `as i64` cast, but `ShieldTransitionV0` only checks `amount > 0`. In `shielded_proof.rs:169`, the expression `-(v0.amount as i64)` wraps for values > `i64::MAX` due to two's-complement truncation.

For example, if `amount = i64::MAX as u64 + 1`, then `amount as i64 = i64::MIN`, and `-(amount as i64)` wraps back to `i64::MIN` in release mode. The `value_balance` passed to `reconstruct_and_verify_bundle` would be semantically wrong.

The Orchard `BatchValidator` binding signature check prevents exploitation (an attacker would need to construct a valid proof over the corrupted value_balance, which is cryptographically infeasible), but defense-in-depth requires catching this at structure validation time.

Comparison with peer types:
- `UnshieldTransitionV0`: checks `unshielding_amount > i64::MAX as u64` ✓
- `ShieldedTransferTransitionV0`: checks `value_balance > i64::MAX as u64` ✓
- `ShieldedWithdrawalTransitionV0`: checks `unshielding_amount > i64::MAX as u64` ✓
- `ShieldFromAssetLockTransitionV0`: checks `value_balance > i64::MAX as u64` ✓
- `ShieldTransitionV0`: only checks `amount > 0` ✗

**Fix**: Add `amount > i64::MAX as u64` check to `ShieldTransitionV0::validate_structure`.

---

#### H2: Unshield/ShieldedWithdrawal `fee_amount` hardcoded to 0

**Status**: KNOWN (TODO in code)
**Location**:
- `packages/rs-drive/src/state_transition_action/shielded/unshield/v0/transformer.rs:21`
- `packages/rs-drive/src/state_transition_action/shielded/shielded_withdrawal/v0/transformer.rs:73`

Both transformers set `fee_amount: 0` with `// TODO` comments. This value flows into `ExecutionEvent::PaidFromShieldedPool { fees_to_add_to_pool: 0 }`, causing validators to receive zero fees for processing these transitions.

The execution flow:
1. `validate_minimum_shielded_fee` passes (but checks the wrong value — see M1)
2. `transform_into_action` creates the action with `fee_amount: 0`
3. `execute_event_v0` processes `PaidFromShieldedPool` with `fees_to_add_to_pool = 0`
4. Validators receive zero compensation

This creates an economic DoS vector: attackers can spam Unshield/ShieldedWithdrawal transactions that consume validator resources (ZK proof verification, nullifier insertion, balance updates) without paying fees.

**Fix**: Calculate the actual fee in the transformers. The fee should be derived from the difference between the ZK-proven value_balance and the recipient amount. Requires architectural clarity on how the fee split is represented.

---

### MEDIUM

#### M1: `validate_minimum_shielded_fee` uses total amount instead of fee for Unshield/ShieldedWithdrawal

**Status**: OPEN
**Location**: `packages/rs-drive-abci/src/execution/validation/state_transition/processor/traits/shielded_proof.rs:88-101`

For `Unshield` and `ShieldedWithdrawal`, `unshielding_amount` (total outflow including recipient amount + fee) is used as the `fee` variable. The doc comment (lines 55-56) correctly states `fee = value_balance - amount`, but the implementation passes `unshielding_amount` directly without computing the subtraction.

This means the minimum fee check compares the total withdrawal amount against the minimum fee threshold, which trivially passes for any meaningful withdrawal. A withdrawal of 1,000,000 credits with 1 credit fee would pass a minimum fee of 111,548,800 only if `unshielding_amount >= 111,548,800`, so the check does provide a floor — but it's on the total outflow, not the fee portion.

**Fix**: Restructure to compute `fee = unshielding_amount - recipient_amount` or separate the fields.

---

#### M2: Missing minimum fee check in `check_tx` path

**Status**: OPEN
**Location**: `packages/rs-drive-abci/src/execution/validation/state_transition/check_tx_verification/v0/mod.rs`

The `check_tx` FirstTimeCheck path validates the ZK proof (`validate_shielded_proof` at lines 138-147) but does NOT call `validate_minimum_shielded_fee`. The import for `StateTransitionShieldedMinimumFeeValidationV0` is absent.

In the block proposal path (`processor/v0/mod.rs`), minimum fee validation is deliberately ordered BEFORE proof verification (cheap check before expensive check). The `check_tx` path skips the cheap check and goes straight to expensive proof verification.

An attacker could submit shielded transitions with insufficient fees that trigger expensive ZK proof verification during `check_tx`, wasting validator CPU. The transitions would only be rejected during block processing.

**Fix**: Add `validate_minimum_shielded_fee` to check_tx before `validate_shielded_proof`, mirroring the process_proposal ordering.

---

#### M3: `ShieldedTransferTransition` allows `value_balance == 0`

**Status**: OPEN
**Location**: `packages/rs-dpp/src/state_transition/state_transitions/shielded/shielded_transfer_transition/v0/state_transition_validation.rs`

The structure validation only checks `value_balance <= i64::MAX` but not `value_balance > 0`. Since `value_balance` IS the fee for shielded transfers, a zero value means zero fee. All other shielded transitions validate their monetary field `> 0`:
- `ShieldTransitionV0`: checks `amount == 0` → reject ✓
- `UnshieldTransitionV0`: checks `unshielding_amount == 0` → reject ✓
- `ShieldedWithdrawalTransitionV0`: checks `unshielding_amount == 0` → reject ✓
- `ShieldFromAssetLockTransitionV0`: checks `value_balance == 0` → reject ✓
- `ShieldedTransferTransitionV0`: missing ✗

**Fix**: Add `value_balance == 0` rejection to structure validation.

---

#### M4: Unbounded anchor query in `validate_anchor_exists`

**Status**: OPEN
**Location**: `packages/rs-drive-abci/src/execution/validation/state_transition/state_transitions/shielded_common/mod.rs:232-255`

The function queries ALL anchors from the anchors tree with `Query::new_range_full()` and `limit: None`. As the system ages, the number of stored anchors grows linearly with blocks that modify the commitment tree. This produces an increasingly expensive full-table scan for every shielded spending transition.

A more efficient approach would store anchors by value as the key for O(1) lookup, or limit the search window to recent anchors.

**Fix**: Either restructure anchors storage for key-based lookup, or add a reasonable limit.

---

#### M5: `PaidFromShieldedPool` bypasses fee validation in execution layer

**Status**: OPEN
**Location**: `packages/rs-drive-abci/src/execution/platform_events/state_transition_processing/validate_fees_of_event/v0/mod.rs:267-272`

The `PaidFromShieldedPool` execution event is grouped with `Free` in `validate_fees_of_event`, returning `FeeResult::default()` without any fee validation. Combined with H2 (fee_amount = 0), no fees are ever collected for shielded pool transitions.

**Fix**: When H2 is resolved, add fee validation for `PaidFromShieldedPool` to ensure `fees_to_add_to_pool` covers execution costs.

---

### LOW

#### L1: `signable_bytes_len as u16` truncation in ShieldFromAssetLock

**Status**: FIXED
**Location**: `packages/rs-drive-abci/src/execution/validation/state_transition/state_transitions/shield_from_asset_lock/transform_into_action/v0/mod.rs:172`

The expression `signable_bytes_len as u16` truncates a `usize` to `u16`, silently wrapping for payloads >= 65536 bytes. This affects the `DoubleSha256` fee block count accounting. While the `max_shielded_transition_actions` limit constrains payload size (hitting 65536 bytes would require ~77 actions at ~852 bytes each), the truncation is incorrect.

**Fix**: Use saturating conversion: `(signable_bytes_len / SHA256_BLOCK_SIZE as usize).min(u16::MAX as usize) as u16`.

---

#### L2: Unchecked `tx_out.value * CREDITS_PER_DUFF` overflow

**Status**: FIXED
**Location**: `packages/rs-drive-abci/src/execution/validation/state_transition/state_transitions/shield_from_asset_lock/transform_into_action/v0/mod.rs:149`

Line 124 uses `tx_out.value.saturating_mul(CREDITS_PER_DUFF)` but line 149 uses plain `tx_out.value * CREDITS_PER_DUFF` for the same computation. While `tx_out.value` would need to exceed ~18.4 billion DASH to overflow (exceeding total supply), the inconsistency should be fixed.

**Fix**: Change line 149 to use `saturating_mul`.

---

#### L3: Stale anchor comparison from wrong query direction

**Status**: OPEN
**Location**: `packages/rs-drive-abci/src/execution/platform_events/block_processing_end_events/record_shielded_pool_anchor/v0/mod.rs:51-68`

The function queries with `limit: Some(1)` on an ascending range, returning the OLDEST anchor (lowest block height key) instead of the most recent one. Then compares the current anchor against this oldest value. Works in practice because a Sinsemilla collision between the current and oldest anchor is cryptographically improbable.

**Fix**: Use a descending query or query the latest key explicitly.

---

### INFO

#### I1: `FLAGS_SPENDS_ONLY` defined but never used

**Status**: FIXED (removed)
**Location**: `packages/rs-drive-abci/src/execution/validation/state_transition/state_transitions/shielded_common/mod.rs:34`

The constant `FLAGS_SPENDS_ONLY: u8 = 0x01` is defined but never referenced anywhere. All spending transitions use `FLAGS_SPENDS_AND_OUTPUTS` (0x03) because even unshield transitions create change outputs.

**Fix**: Remove the dead constant.

---

#### I2: 8 query modules fully written but commented out

**Status**: KNOWN (pending dapi-grpc types)
**Location**: `packages/rs-drive-abci/src/query/shielded/mod.rs`

All 8 shielded query endpoint implementations are complete but commented out with TODO: "Re-enable when dapi-grpc shielded protobuf types are available." Note: `encrypted_notes/v0/mod.rs:120` has an `.unwrap()` on a GroveDB cost result that should be addressed when re-enabled.

---

#### I3: Strategy tests feature-gated and disabled

**Status**: KNOWN (pending OperationType enum variants)
**Location**: `packages/rs-drive-abci/tests/strategy_tests/test_cases/shielded_tests.rs`

The `#[cfg(feature = "__shielded_strategy_tests")]` gate prevents integration tests from running. The `OperationType` enum in the strategy-tests crate lacks shielded variants. These are the only multi-block chain execution tests for shielded transitions.

---

## Test Coverage Gaps

| Gap | Transitions Affected | Priority |
|-----|---------------------|----------|
| Zero `unshielding_amount` structure validation test | Unshield, ShieldedWithdrawal | High |
| `amount > i64::MAX` structure validation test | Shield | High |
| `ShieldedTooManyActionsError` (max actions exceeded) | All 5 types | High |
| Minimum fee boundary tests | Unshield, ShieldedWithdrawal | Medium |
| Anchor-not-found with valid ZK proof | ShieldedTransfer, Unshield, ShieldedWithdrawal | Medium |
| Nullifier-already-spent with valid ZK proof | All spending types | Medium |
| Pool-balance-insufficient with valid ZK proof | ShieldedTransfer, Unshield, ShieldedWithdrawal | Medium |
| Zeroed binding signature | Shield, ShieldFromAssetLock | Low |
| Remaining-balance insufficient for ShieldFromAssetLock | ShieldFromAssetLock | Low |

---

## Verified Correct

- ZK proof reconstruction and verification via `BatchValidator` — all fields correctly parsed and passed to `Bundle::from_parts`
- Nullifier double-spend prevention — intra-bundle `HashSet` + cross-state GroveDB `grove_has_raw` check
- ShieldFromAssetLock penalty enforcement — failed ZK proofs produce `PartiallyUseAssetLockAction` that burns penalty from asset lock
- Bundle field completeness in reconstruction (nullifier, rk, cmx, encrypted_note, cv_net, spend_auth_sig, anchor, proof, binding_signature, flags, value_balance)
- Validation pipeline ordering in process_proposal: structure → fee → proof → state
- Exhaustive match arms across all trait implementations (no missing shielded variants)
- Platform version gating pattern consistency with existing transitions
- Clean `PenalizeShieldedPoolAction` removal (no dangling references)
- Correct flags usage: `FLAGS_OUTPUTS_ONLY` for Shield/ShieldFromAssetLock, `FLAGS_SPENDS_AND_OUTPUTS` for ShieldedTransfer/Unshield/ShieldedWithdrawal
- Correct `value_balance` sign handling: negative for shield (money entering pool), positive for unshield (money leaving pool)
- Anchor validation correctly skipped for output-only bundles (Shield, ShieldFromAssetLock use empty tree anchor)
- `i64` cast safety verified for all types except Shield (now fixed)
- `sighash` computation correctly binds transparent fields via `compute_platform_sighash` with `extra_sighash_data`
- Static verifying key with `OnceLock` + background thread warmup in `main.rs`
