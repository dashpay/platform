package org.dashfoundation.example.ui.dashpay

import org.dashfoundation.dashsdk.errors.DashSdkError

/**
 * Pure decision logic for the invitation-reclaim flow — port of the
 * `nonisolated static` seams on `ReclaimInvitationSheet.swift`
 * (`classifyReclaimFailure` / `shouldClearInFlightMarker` and the message
 * classifiers). Side-effect-free so the outcome matrix is unit-tested; the
 * sheet maps an outcome to `statusRaw` / message / save.
 */
object InvitationReclaimLogic {

    /** The terminal state a reclaim attempt resolves to. */
    enum class ReclaimOutcome {
        /**
         * This wallet retained a tombstone written after its successful
         * local consume, so the interrupted reclaim recovers definitively.
         */
        RECLAIMED,

        /** The voucher was consumed with no local attempt in flight — a foreign claim. */
        CLAIMED,

        /**
         * Provably consumed (deterministic Platform rejection), but our own
         * in-flight attempt makes the consumer ambiguous. Resolves to the
         * conservative terminal Claimed, never an inferred Reclaimed.
         */
        CONSUMED_AMBIGUOUS,

        /**
         * The wallet no longer tracks the voucher lock and our own attempt
         * was in flight — consistent with that attempt having landed, but
         * with no on-chain proof at all. Status and marker stay untouched.
         */
        UNTRACKED_AFTER_OWN_ATTEMPT,

        /** An uncertain, unrelated failure — leave state as-is. */
        ERROR,
    }

    /**
     * Pure decision for the reclaim failure path. A typed wallet
     * [DashSdkError.PlatformWallet.AssetLockAlreadyConsumed] comes from a
     * retained local tombstone written only after this wallet's successful
     * consume, and therefore recovers Reclaimed. Consensus wording is only
     * proof the lock is consumed — not who consumed it — so the prior
     * in-flight marker splits that fallback into a foreign claim vs an
     * explicitly ambiguous consumption.
     */
    fun classifyReclaimFailure(
        error: Throwable,
        hadPriorReclaimInFlight: Boolean,
    ): ReclaimOutcome {
        if (isLocallyConsumedTombstone(error)) return ReclaimOutcome.RECLAIMED
        if (isAlreadyConsumed(error.message.orEmpty())) {
            return if (hadPriorReclaimInFlight) {
                ReclaimOutcome.CONSUMED_AMBIGUOUS
            } else {
                ReclaimOutcome.CLAIMED
            }
        }
        // A retry after our own crash-interrupted consume can also fail
        // LOCALLY ("…is not tracked") — before Platform. Consistent with the
        // consume having landed but not proof of it, so it gets its own
        // explicitly ambiguous outcome instead of a Reclaimed recovery. A
        // first attempt hitting "not tracked" still resolves to ERROR.
        if (hadPriorReclaimInFlight && isLockNoLongerTracked(error.message.orEmpty())) {
            return ReclaimOutcome.UNTRACKED_AFTER_OWN_ATTEMPT
        }
        return ReclaimOutcome.ERROR
    }

    /**
     * Whether the ERROR outcome should also CLEAR the persisted
     * `reclaimInFlight` marker: only when this attempt set the marker itself
     * and then failed the LOCAL pre-broadcast "is not tracked" resume guard —
     * proof the consume never started, so the freshly-set marker is stale
     * (leaving it would degrade an identical retry into the two-attempt
     * false-ambiguity outcome). Every other error keeps the marker so a later
     * "already consumed" stays classified as ambiguous.
     */
    fun shouldClearInFlightMarker(
        error: Throwable,
        hadPriorReclaimInFlight: Boolean,
    ): Boolean = !hadPriorReclaimInFlight && isLockNoLongerTracked(error.message.orEmpty())

    /**
     * The wallet's typed local consumed tombstone — written only after this
     * wallet's own successful consume, so it can safely recover Reclaimed.
     */
    fun isLocallyConsumedTombstone(error: Throwable): Boolean =
        error is DashSdkError.PlatformWallet.AssetLockAlreadyConsumed

    /**
     * The deterministic consensus 10504 rejection. Matched on the exact
     * canonical Display phrase of
     * `IdentityAssetLockTransactionOutPointAlreadyConsumedError` ONLY —
     * broader phrases would widen false-positive risk (misclassifying an
     * unrelated failure as a benign "already claimed" wrongly flips the row
     * to Claimed). The typed tombstone above is the primary signal; this
     * wording is the compatibility fallback for errors originating below the
     * typed wallet boundary. (A typed FFI code for 10504 is a known
     * follow-up shared with iOS.)
     */
    fun isAlreadyConsumed(message: String): Boolean =
        message.lowercase().contains("already completely used")

    /**
     * The wallet's LOCAL "asset lock … is not tracked" resume-guard failure
     * — distinct from the network's already-consumed rejection; used only
     * for our-own-reclaim crash recovery.
     */
    fun isLockNoLongerTracked(message: String): Boolean =
        message.lowercase().contains("is not tracked")

    /**
     * Split a stored 36-byte outpoint (`txid_le ‖ vout_le`) into the 32-byte
     * txid and the vout, read directly from the raw bytes (never re-parsed
     * from the display string).
     */
    fun outPointParts(rawOutPoint: ByteArray): Pair<ByteArray, Int> {
        require(rawOutPoint.size == 36) {
            "rawOutPoint must be 36 bytes (was ${rawOutPoint.size})"
        }
        val txid = rawOutPoint.copyOfRange(0, 32)
        var vout = 0
        for (i in 35 downTo 32) vout = (vout shl 8) or (rawOutPoint[i].toInt() and 0xFF)
        return txid to vout
    }

    /**
     * One past the highest used registration index across [usedIndices],
     * else 0 — the fresh HD slot a claim/reclaim-register targets. Mirrors
     * `ClaimInvitationSheet.nextUnusedIdentityIndex`.
     */
    fun nextUnusedIdentityIndex(usedIndices: List<Int>): Int {
        val highest = usedIndices.maxOrNull() ?: return 0
        return if (highest == Int.MAX_VALUE) Int.MAX_VALUE else highest + 1
    }
}
