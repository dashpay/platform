package org.dashfoundation.example.ui.funding

import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity

/**
 * Asset-lock display helpers — port of `PersistentAssetLockDisplay.swift`.
 * Consolidates the 0..5 `AssetLockStatus` discriminants (a protocol-
 * mirrored constant from the Rust side) into one place so the funding UIs
 * don't re-implement the same `when` blocks. Extensions on
 * [AssetLockEntity] so every list surface reads the status the same way.
 *
 * The status domain is NOT an ordered severity scale, and no predicate here
 * may treat it as one. `4` (Consumed) is the terminal tombstone; `5`
 * (RecoveredFromChain) sits above it numerically but is very much alive.
 * Every predicate below therefore excludes `4` by name rather than by an
 * upper bound.
 */

/**
 * `true` when the lock should appear on the resumable "Pending Platform Top
 * Ups" orphan surface at all — `statusRaw ∈ [1, 3] ∪ {5}`. Lower bar than
 * [canFundIdentity]: a Broadcast (1) lock isn't submittable yet but the user
 * should still see it (as "Waiting for InstantSend / ChainLock…") so a
 * crash-recovery situation has visible continuity through the IS-lock
 * arrival.
 *
 * Excluding `4` (Consumed) is load-bearing: it is terminal and
 * `resume_asset_lock` rejects it, so a Consumed row must never resurface as
 * a perpetual-spinner dead end. That exclusion is deliberately NOT written
 * as an upper bound of `3` — `5` (RecoveredFromChain) is what the restore
 * scan and the chainlock-promotion path write for a lock with proven Core
 * finality and unknown Platform-side consumption, and a contiguous `1..3`
 * range silently dropped every one of them: a chain-locked top-up the user
 * really funded appeared on no surface at all and read as lost funds.
 * ← Swift `isVisibleAsResumable`.
 */
val AssetLockEntity.isVisibleAsResumable: Boolean
    get() = statusRaw in 1..3 || statusRaw == 5

/**
 * `true` when the lock has a usable IS-lock / chain-lock proof AND hasn't
 * been consumed — `statusRaw ∈ {2, 3, 5}`. Only these can submit the funding
 * ST immediately; Built (0) and Broadcast (1) still await finality.
 *
 * `5` (RecoveredFromChain) qualifies: the restore scan and the
 * chainlock-promotion path attach a real `ChainAssetLockProof` before
 * writing that status, so Core-side finality is PROVEN and the lock is
 * exactly as fundable as a ChainLocked (3) one. What is unknown for a `5` is
 * whether Platform already consumed it — and Platform, not the client, is
 * the arbiter of that: it rejects an already-spent outpoint with a typed
 * error. A user-driven Resume is the surface allowed to ask. (Do not feed
 * `5` into an automatic retry sweep — blind retries of historical locks are
 * the failure mode the status exists to prevent.) Reading `5` as
 * not-yet-final made the UI tell the user to wait for a finality that had
 * already happened. ← Swift `canFundIdentity`.
 */
val AssetLockEntity.canFundIdentity: Boolean
    get() = statusRaw == 2 || statusRaw == 3 || statusRaw == 5

/**
 * Human-readable status label. Mirrors the Rust-side `AssetLockStatus` enum.
 * ← Swift `statusLabel`.
 */
val AssetLockEntity.statusLabel: String
    get() = when (statusRaw) {
        0 -> "Built"
        1 -> "Broadcast"
        2 -> "InstantSendLocked"
        3 -> "ChainLocked"
        4 -> "Consumed"
        // Core finality proven, Platform-side consumption unknown.
        // Rendered as "Unknown(5)" before this branch existed.
        5 -> "RecoveredFromChain"
        else -> "Unknown($statusRaw)"
    }

/**
 * First 8 hex chars of the display-order txid plus the vout, derived from
 * the canonical `<txid display hex>:<vout>` outpoint encoding. Used by every
 * UI that lists asset-lock rows so the txid prefix reads the same way across
 * surfaces. ← Swift `shortOutPointDisplay`.
 */
val AssetLockEntity.shortOutPointDisplay: String
    get() {
        val parts = outPointHex.split(":", limit = 2)
        if (parts.size != 2) return outPointHex
        return "${parts[0].take(8)}:${parts[1]}"
    }
