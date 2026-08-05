package org.dashfoundation.example.ui.dashpay

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.example.ui.dashpay.InvitationReclaimLogic.ReclaimOutcome
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Port of `ReclaimInvitationClassifierTests.swift` — the full outcome
 * matrix of the pure reclaim-failure classifier, including the
 * marker/no-marker ambiguity split, the stale-marker clearing rule, and
 * the exact-phrase false-positive safety of the consensus-message match.
 */
class InvitationReclaimLogicTest {

    // The canonical Display of consensus 10504 as it surfaces through the SDK.
    private val consumedMessage =
        "SDK error: Protocol error: Asset lock transaction 35c0… output 0 " +
            "already completely used"

    private val notTrackedMessage =
        "Asset lock 35c0…:0 is not tracked by this wallet"

    private fun typedTombstone(): Throwable =
        DashSdkError.PlatformWallet.AssetLockAlreadyConsumed("asset lock already consumed")

    // ── Outcome matrix ────────────────────────────────────────────────

    @Test
    fun typedTombstoneRecoversReclaimedRegardlessOfMarker() {
        assertEquals(
            ReclaimOutcome.RECLAIMED,
            InvitationReclaimLogic.classifyReclaimFailure(typedTombstone(), false),
        )
        assertEquals(
            ReclaimOutcome.RECLAIMED,
            InvitationReclaimLogic.classifyReclaimFailure(typedTombstone(), true),
        )
    }

    @Test
    fun consensusConsumedWithoutMarkerIsAForeignClaim() {
        assertEquals(
            ReclaimOutcome.CLAIMED,
            InvitationReclaimLogic.classifyReclaimFailure(
                RuntimeException(consumedMessage), hadPriorReclaimInFlight = false,
            ),
        )
    }

    @Test
    fun consensusConsumedWithMarkerIsExplicitlyAmbiguousNeverReclaimed() {
        assertEquals(
            ReclaimOutcome.CONSUMED_AMBIGUOUS,
            InvitationReclaimLogic.classifyReclaimFailure(
                RuntimeException(consumedMessage), hadPriorReclaimInFlight = true,
            ),
        )
    }

    @Test
    fun notTrackedWithMarkerIsUntrackedAfterOwnAttempt() {
        assertEquals(
            ReclaimOutcome.UNTRACKED_AFTER_OWN_ATTEMPT,
            InvitationReclaimLogic.classifyReclaimFailure(
                RuntimeException(notTrackedMessage), hadPriorReclaimInFlight = true,
            ),
        )
    }

    @Test
    fun notTrackedOnAFirstAttemptIsAPlainError() {
        assertEquals(
            ReclaimOutcome.ERROR,
            InvitationReclaimLogic.classifyReclaimFailure(
                RuntimeException(notTrackedMessage), hadPriorReclaimInFlight = false,
            ),
        )
    }

    @Test
    fun unrelatedErrorsResolveToErrorWithEitherMarkerState() {
        val boring = RuntimeException("connection reset by peer")
        assertEquals(
            ReclaimOutcome.ERROR,
            InvitationReclaimLogic.classifyReclaimFailure(boring, false),
        )
        assertEquals(
            ReclaimOutcome.ERROR,
            InvitationReclaimLogic.classifyReclaimFailure(boring, true),
        )
    }

    // ── Stale-marker clearing ─────────────────────────────────────────

    @Test
    fun markerClearsOnlyForOwnAttemptLocalNotTrackedFailure() {
        assertTrue(
            InvitationReclaimLogic.shouldClearInFlightMarker(
                RuntimeException(notTrackedMessage), hadPriorReclaimInFlight = false,
            ),
        )
        // Prior marker: keep it — a later "already consumed" must stay ambiguous.
        assertFalse(
            InvitationReclaimLogic.shouldClearInFlightMarker(
                RuntimeException(notTrackedMessage), hadPriorReclaimInFlight = true,
            ),
        )
        // Any other error: keep it (may have reached the network).
        assertFalse(
            InvitationReclaimLogic.shouldClearInFlightMarker(
                RuntimeException("timeout"), hadPriorReclaimInFlight = false,
            ),
        )
    }

    // ── Message-match false-positive safety ───────────────────────────

    @Test
    fun alreadyConsumedMatchesTheExactCanonicalPhraseOnly() {
        assertTrue(InvitationReclaimLogic.isAlreadyConsumed(consumedMessage))
        assertTrue(InvitationReclaimLogic.isAlreadyConsumed(consumedMessage.uppercase()))
        // Broader wordings must NOT match — a false positive would wrongly
        // flip the row to Claimed.
        assertFalse(InvitationReclaimLogic.isAlreadyConsumed("asset lock already consumed"))
        assertFalse(InvitationReclaimLogic.isAlreadyConsumed("output already used"))
        assertFalse(InvitationReclaimLogic.isAlreadyConsumed("completely unrelated"))
    }

    @Test
    fun notTrackedMatchIsCaseInsensitiveContains() {
        assertTrue(InvitationReclaimLogic.isLockNoLongerTracked(notTrackedMessage))
        assertTrue(InvitationReclaimLogic.isLockNoLongerTracked("Lock IS NOT TRACKED"))
        assertFalse(InvitationReclaimLogic.isLockNoLongerTracked("lock is untracked"))
    }

    // ── Outpoint + index helpers ──────────────────────────────────────

    @Test
    fun outPointPartsSplitsTxidAndLittleEndianVout() {
        val raw = ByteArray(36)
        for (i in 0 until 32) raw[i] = i.toByte()
        raw[32] = 0x01 // vout = 0x04030201 little-endian
        raw[33] = 0x02
        raw[34] = 0x03
        raw[35] = 0x04
        val (txid, vout) = InvitationReclaimLogic.outPointParts(raw)
        assertArrayEquals(ByteArray(32) { it.toByte() }, txid)
        assertEquals(0x04030201, vout)
    }

    @Test
    fun outPointPartsRejectsWrongLength() {
        val failure = runCatching { InvitationReclaimLogic.outPointParts(ByteArray(35)) }
        assertTrue(failure.exceptionOrNull() is IllegalArgumentException)
    }

    @Test
    fun nextUnusedIdentityIndexIsOnePastTheHighestUsedElseZero() {
        assertEquals(0, InvitationReclaimLogic.nextUnusedIdentityIndex(emptyList()))
        assertEquals(1, InvitationReclaimLogic.nextUnusedIdentityIndex(listOf(0)))
        assertEquals(8, InvitationReclaimLogic.nextUnusedIdentityIndex(listOf(0, 3, 7)))
        assertEquals(
            Int.MAX_VALUE,
            InvitationReclaimLogic.nextUnusedIdentityIndex(listOf(Int.MAX_VALUE)),
        )
    }

    @Test
    fun completedReclaimCannotSubmitAgainFromAStaleCreatedRowSnapshot() {
        assertFalse(
            canSubmitInvitationReclaim(
                isReclaiming = false,
                statusRaw = 0,
                targetTopUp = false,
                hasSelectedIdentity = false,
                completedForRow = true,
            ),
        )
    }
}
