package org.dashfoundation.example.ui.funding

import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity

/**
 * Asset-lock display helpers — port of `PersistentAssetLockDisplay.swift`.
 * Consolidates the 0/1/2/3/4 `AssetLockStatus` discriminants (a protocol-
 * mirrored constant from the Rust side) into one place so the funding UIs
 * don't re-implement the same `when` blocks. Extensions on
 * [AssetLockEntity] so every list surface reads the status the same way.
 */

/**
 * `true` when the lock should appear on the resumable "Pending Platform Top
 * Ups" orphan surface at all — `statusRaw ∈ [1, 3]` (Broadcast through
 * ChainLocked). Lower bar than [canFundIdentity]: a Broadcast (1) lock isn't
 * submittable yet but the user should still see it (as "Waiting for
 * InstantSend / ChainLock…") so a crash-recovery situation has visible
 * continuity through the IS-lock arrival. Upper bound at 3 is load-bearing:
 * status 4 (Consumed) is terminal and `resume_asset_lock` rejects it, so a
 * Consumed row must never resurface as a perpetual-spinner dead end.
 * ← Swift `isVisibleAsResumable`.
 */
val AssetLockEntity.isVisibleAsResumable: Boolean
    get() = statusRaw in 1..3

/**
 * `true` when the lock has a usable IS-lock / chain-lock proof AND hasn't
 * been consumed — `statusRaw == 2 || statusRaw == 3`. Only these can submit
 * the funding ST immediately; Built (0) and Broadcast (1) still await
 * finality. ← Swift `canFundIdentity`.
 */
val AssetLockEntity.canFundIdentity: Boolean
    get() = statusRaw == 2 || statusRaw == 3

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
