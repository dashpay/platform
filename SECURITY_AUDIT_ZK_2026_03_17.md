# Security Audit: Dash Platform Shielded/ZK Pool
**Date**: 2026-03-17
**Branch**: v3.1-dev (commit 95bdf2c08)
**Scope**: All shielded pool code across rs-dpp, rs-drive, rs-drive-abci, rs-sdk-ffi

## Executive Summary

Five specialized security auditors independently examined the shielded pool codebase. The overall architecture is sound — ZK proof verification uses the Orchard `BatchValidator` correctly, all balance arithmetic uses checked operations, nullifier double-spend prevention is thorough, and sighash domain separation prevents cross-protocol replay.

However, the audit revealed **1 critical cryptographic bug**, a **cluster of fee-related issues** (known TODOs that collectively form a critical gap), and several medium/low findings.

---

## CRITICAL Findings

### C1: Sighash Mismatch Between Client Builders and Server Verification (Unshield & ShieldedWithdrawal)

**Severity**: Critical
**Files**:
- Client: `rs-dpp/src/shielded/builder/unshield.rs:84`
- Client: `rs-dpp/src/shielded/builder/shielded_withdrawal.rs:92`
- Client FFI: `rs-sdk-ffi/src/shielded/crypto/bundle_build.rs:848,1021`
- Server: `rs-drive-abci/.../traits/shielded_proof.rs:210-228`

**Description**: The `extra_sighash_data` used during Orchard bundle signing on the client differs from what the server uses during verification. The client only binds the destination (address or script), while the server binds BOTH the destination AND the `unshielding_amount`.

```
Client (signing):   sighash = SHA256(domain || commitment || output_address)
Server (verifying): sighash = SHA256(domain || commitment || output_address || unshielding_amount_le_bytes)
```

**Impact**: Every Unshield and ShieldedWithdrawal built with the `rs-dpp` builder or `rs-sdk-ffi` will **fail proof verification** on the platform. Integration tests pass because they construct bundles with the server-side sighash. This is a liveness bug — legitimate wallet transactions will be rejected.

**Fix**: Update client builders to append `unshielding_amount.to_le_bytes()` to `extra_sighash_data`, matching the server.

---

### C2: Zero Fee Collection for Unshield & ShieldedWithdrawal (Cluster of Related Issues)

This is a cluster of 4 interrelated issues that together mean **validators receive zero compensation** for processing the two most expensive shielded transition types:

#### C2a: `fee_amount` Hardcoded to 0 in Transformers
- `rs-drive/.../unshield/v0/transformer.rs:21` — `fee_amount: 0, // TODO`
- `rs-drive/.../shielded_withdrawal/v0/transformer.rs:73` — `fee_amount: 0, // TODO`

The entire `unshielding_amount` goes to the recipient. Proposers get nothing.

#### C2b: `PaidFromShieldedPool` Skips Fee Validation
- `rs-drive-abci/.../validate_fees_of_event/v0/mod.rs:267-272` — grouped with `Free` events
- `rs-drive-abci/.../execute_event/v0/mod.rs:345-348` — returns `None` for fee validation

Unlike every other paid event type, `PaidFromShieldedPool` performs zero execution-time fee validation.

#### C2c: `validate_minimum_shielded_fee` Uses Wrong Semantics
- `rs-drive-abci/.../traits/shielded_proof.rs:107-118`

For Unshield/ShieldedWithdrawal, it checks `unshielding_amount >= minimum_fee`. But `unshielding_amount` is the TOTAL leaving the pool (recipient + fee), not the fee alone. Any unshield > ~1.23 DASH trivially passes regardless of actual fee.

#### C2d: Client `calculate_min_required_fee` Returns 0
- `rs-dpp/.../unshield_transition/state_transition_estimated_fee_validation.rs`
- `rs-dpp/.../shielded_transfer_transition/state_transition_estimated_fee_validation.rs`
- `rs-dpp/.../shielded_withdrawal_transition/state_transition_estimated_fee_validation.rs`

All return `Ok(0)`, allowing zero-fee transitions into the mempool.

**Note**: ShieldedTransfer is partially correct — its `fee_amount` is properly set from `value_balance`. But it still lacks execution-time fee validation (C2b).

---

## HIGH Findings

### H1: ShieldedWithdrawal L1 Document Includes Fee in Withdrawal Amount

**File**: `rs-drive/.../shielded_withdrawal/v0/transformer.rs:39`

The withdrawal document's `AMOUNT` field is set to `unshielding_amount` (the total value_balance including fee). Combined with `fee_amount: 0`, this means the full value_balance exits the system to L1 and zero stays as proposer payment. When fees are fixed, this field must use `unshielding_amount - fee_amount`.

### H2: Missing `core_fee_per_byte` and `output_script` Validation in ShieldedWithdrawal

**File**: `rs-dpp/.../shielded_withdrawal_transition/v0/state_transition_validation.rs`

The `validate_structure` for ShieldedWithdrawal does NOT validate `core_fee_per_byte` or `output_script`. A `core_fee_per_byte: 0` creates an unminable L1 withdrawal. An empty `output_script` creates an unspendable withdrawal. Credits are deducted from the pool but never arrive.

---

## MEDIUM Findings

### M1: Potential Pool Balance Desynchronization (TOCTOU)

**Files**: All shielded `transform_into_action` functions

Each transition reads `current_total_balance` from GroveDB and stores it in the action. If multiple shielded transitions are in the same block, each reads the same snapshot. The `UpdateTotalBalance` operation uses absolute set (not delta), so the last writer wins.

This is **likely mitigated** by GroveDB's transaction semantics if state transitions are applied sequentially within the same transaction. Needs confirmation from the team.

**Fix**: Use delta operations (`SubtractFromBalance`) instead of absolute set, or verify sequential application.

### M2: Panic-Prone `.unwrap()` in Consensus-Critical Path

**Files**:
- `rs-drive/.../record_anchor_if_changed/v0/mod.rs:38,52,83,97,110`
- `rs-drive/.../prune_anchors/v0/mod.rs:65,78`

GroveDB operations return nested `Result<Result<T,E>,E>`. The outer result is `.unwrap()`ed, meaning any storage-level error crashes the node during block processing.

### M3: Unshield Balance Check Doesn't Account for Fee (Latent)

**File**: `rs-drive-abci/.../unshield/transform_into_action/v0/mod.rs:99-115`

Checks `pool_balance >= amount` but drive operations subtract `amount + fee_amount`. Currently masked by `fee_amount=0`, but will cause `CorruptedDriveState` errors when fees are fixed.

### M4: Non-Atomic Nullifier Storage

**File**: `rs-drive/.../insert_nullifiers/v0/mod.rs:27-51`

Permanent nullifier ops are batched (returned for later application), but `store_nullifiers_for_block()` writes immediately. If the batch fails, sync storage has nullifiers that the permanent tree doesn't.

### M5: Cross-Transaction Nullifier Deduplication Within a Block

**File**: `rs-drive-abci/.../shielded_common/mod.rs:234-263`

`has_nullifier` checks GroveDB state, but pending nullifier insertions from earlier transactions in the same block may not yet be committed. The `insert_only_known_to_not_already_exist_op` provides a safety net at batch application time, but failure there could be a hard error rather than a consensus rejection.

---

## LOW Findings

### L1: Missing `encrypted_note` Length Validation at DPP Layer
All 5 transition types validate actions count, amounts, proof, and anchor, but not `encrypted_note` length (must be exactly 216 bytes). Oversized notes waste bandwidth before being rejected at the ABCI layer. Bounded by 20KB `max_state_transition_size`.

### L2: Missing Upper Fee Bound in Unshield/ShieldedWithdrawal Builders
`build_shielded_transfer_transition` has a `fee > min_fee * 1000` guard; `build_unshield_transition` and `build_shielded_withdrawal_transition` do not.

### L3: Silent Type Coercion in Anchor Retrieval
`record_anchor_if_changed_v0` returns `[0u8; 32]` for non-Item elements instead of an error, masking potential state corruption.

### L4: Stale Most-Recent-Anchor After Full Pruning
`prune_shielded_pool_anchors` doesn't update `MOST_RECENT_ANCHOR_KEY` when pruning removes the most recent anchor.

### L5: Integer Overflow in Fee Calculation (Theoretical)
`rs-drive-abci/.../shielded_proof.rs:132-140` uses unchecked multiplication. Not practically exploitable with current constants but should use `checked_mul`.

### L6: No Max Proof Size Validation
Proof bytes are accepted with no size check beyond "not empty". The 20KB global limit caps this, but a proof-specific check (expected ~7,500 bytes) would be defense-in-depth.

### L7: Bincode `NoLimit` Deserialization for Nullifier Types
`rs-drive/.../nullifiers/types.rs` uses `with_no_limit()` for bincode config. A corrupted DB entry could cause OOM. Should use `with_limit()`.

### L8: `saturating_sub` Masks Invariant Violation in ShieldFromAssetLock Fee
`execution_event/mod.rs:499-503` — if `shield_amount > asset_lock_value`, fee saturates to 0 instead of erroring.

---

## Positive Observations

1. **ZK Proof Verification**: `BatchValidator` correctly verifies Halo2 proof + spend auth + binding signature together
2. **Verifying Key Immutability**: Lazily initialized via `OnceLock` from deterministic `VerifyingKey::build()`
3. **Sighash Domain Separation**: `b"DashPlatformSighash"` prefix prevents cross-protocol replay
4. **Checked Arithmetic**: All balance operations use `checked_add`/`checked_sub`
5. **Intra-Bundle Nullifier Dedup**: HashSet check prevents duplicate nullifiers within a bundle
6. **Anchor Existence Validation**: Spending transitions correctly verify anchor exists in state
7. **i64 Bounds Checking**: All u64→i64 casts are validated beforehand
8. **`OsRng` for Randomness**: Cryptographically secure source for proofs and signatures
9. **ShieldFromAssetLock Penalty**: Failed ZK proofs partially consume the asset lock, preventing free spam
10. **Minimum Pool Notes Threshold**: Enforces anonymity set size before allowing withdrawals
11. **Orchard Flags Enforcement**: `FLAGS_OUTPUTS_ONLY` for Shield, `FLAGS_SPENDS_AND_OUTPUTS` for spending

---

## Recommended Priority

| Priority | Finding | Action |
|----------|---------|--------|
| **P0** | C1 — Sighash mismatch | Fix client builders immediately — all wallet Unshield/Withdrawal transactions will fail |
| **P1** | C2a-d — Fee cluster | Implement fee computation in transformers, add `PaidFromShieldedPool` validation, fix client estimation |
| **P1** | H1 — Withdrawal document amount | Fix when implementing C2a (use `amount` not `unshielding_amount`) |
| **P1** | H2 — Missing core_fee/script validation | Add to ShieldedWithdrawal `validate_structure` |
| **P2** | M1 — Pool balance TOCTOU | Verify GroveDB transaction semantics; consider delta ops |
| **P2** | M2 — Panicking unwraps | Replace with proper error propagation |
| **P2** | M3 — Balance check latent bug | Fix when implementing C2a |
| **P2** | M4-M5 — Nullifier atomicity | Verify execution model; add in-memory pending set |
| **P3** | L1-L8 — Defense-in-depth improvements | |
