package org.dashfoundation.example.ui.shielded

import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.wallet.ShieldedIdentityDebitRecoveryStatus
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test

/**
 * How the app reports a shield from an identity's balance (type 21) and a
 * top-up of an identity from the shielded pool (type 22). Port of the cases
 * in `SendViewModelIdentityShieldTests.swift` and
 * `ShieldedIdentityFlowsPresentationTests.swift`: a call that returns is
 * only submitted, one that may have gone through is never offered as a
 * retry, and the wallet's refusals before anything is sent say so.
 */
class ShieldedIdentityFlowsTest {

    @Test
    fun `should report a returned identity shield as submitted, never as complete`() {
        assertEquals(IdentityShieldOutcome.Submitted, IdentityShieldMessages.outcome(null))
        assertTrue(IdentityShieldMessages.SUBMITTED.contains("submitted"))
        assertTrue(IdentityShieldMessages.SUBMITTED.contains("pending"))
    }

    @Test
    fun `should not offer a retry for an identity shield that may have gone through`() {
        for (error in listOf(
            DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed("not proved"),
            DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed("not confirmed"),
        )) {
            assertEquals(IdentityShieldOutcome.MayHaveGoneThrough, IdentityShieldMessages.outcome(error))
        }
        assertTrue(IdentityShieldMessages.MAY_HAVE_GONE_THROUGH.contains("Do not retry"))
        assertTrue(IdentityShieldMessages.MAY_HAVE_GONE_THROUGH.contains("In-Progress Identity Shields"))
    }

    @Test
    fun `should word the wallet's refusals made before anything was sent`() {
        val pending = IdentityShieldMessages.outcome(
            DashSdkError.PlatformWallet.ShieldedIdentityDebitPending("unresolved"),
        )
        assertEquals(IdentityShieldOutcome.Failed(IdentityShieldMessages.DEBIT_PENDING), pending)

        val tooLow = IdentityShieldMessages.outcome(
            DashSdkError.PlatformWallet.InsufficientIdentityCredits(
                identityId = "7y3Q",
                requiredCredits = 150_000_000_000uL,
                availableCredits = 100_000_000_000uL,
            ),
        ) as IdentityShieldOutcome.Failed
        assertTrue(tooLow.message.contains("too low"))
        assertTrue(tooLow.message.contains("1.5 DASH"))
        assertTrue(tooLow.message.contains("1 DASH"))
        assertTrue(tooLow.message.endsWith("Nothing was sent."))

        for (error in listOf(
            DashSdkError.PlatformWallet.IdentityBalanceUnavailable("no proof"),
            DashSdkError.PlatformWallet.ProtocolVersionNotActive("pv 13"),
            DashSdkError.PlatformWallet.SdkPinnedBelowProtocolVersion("pinned"),
        )) {
            val outcome = IdentityShieldMessages.outcome(error) as IdentityShieldOutcome.Failed
            assertTrue(outcome.message, outcome.message.endsWith("Nothing was sent."))
        }
    }

    @Test
    fun `should show any other failure's own message`() {
        val outcome = IdentityShieldMessages.outcome(IllegalStateException("signer missing"))
        assertEquals(IdentityShieldOutcome.Failed("signer missing"), outcome)
    }

    @Test
    fun `should rethrow a cancellation instead of reporting it`() {
        try {
            IdentityShieldMessages.outcome(CancellationException("left"))
            fail("expected the cancellation to be rethrown")
        } catch (_: CancellationException) {
        }
        try {
            ShieldedTopUpMessages.outcome(1, CancellationException("left"))
            fail("expected the cancellation to be rethrown")
        } catch (_: CancellationException) {
        }
    }

    @Test
    fun `should label an unreadable record apart and block nothing once stopped`() {
        val id = ByteArray(32) { 3 }
        assertEquals("Retrying", IdentityShieldRecoveryDisplay.statusTitle(ShieldedIdentityDebitRecoveryStatus.RETRYING, id))
        assertEquals(
            "Waiting for the note",
            IdentityShieldRecoveryDisplay.statusTitle(ShieldedIdentityDebitRecoveryStatus.PARKED, id),
        )
        assertEquals(
            "Outcome unknown",
            IdentityShieldRecoveryDisplay.statusTitle(ShieldedIdentityDebitRecoveryStatus.UNKNOWN, id),
        )
        assertEquals(
            "Unreadable record",
            IdentityShieldRecoveryDisplay.statusTitle(ShieldedIdentityDebitRecoveryStatus.RETRYING, null),
        )
        assertEquals(
            "Outcome unknown",
            IdentityShieldRecoveryDisplay.statusTitle(ShieldedIdentityDebitRecoveryStatus.UNKNOWN, null),
        )
        assertTrue(
            IdentityShieldRecoveryDisplay.statusDetail(ShieldedIdentityDebitRecoveryStatus.PARKED, null)
                .contains("blocks new identity shields"),
        )
        assertTrue(IdentityShieldRecoveryDisplay.canStopRetrying(ShieldedIdentityDebitRecoveryStatus.RETRYING))
        assertTrue(IdentityShieldRecoveryDisplay.canStopRetrying(ShieldedIdentityDebitRecoveryStatus.PARKED))
        assertFalse(IdentityShieldRecoveryDisplay.canStopRetrying(ShieldedIdentityDebitRecoveryStatus.UNKNOWN))
    }

    @Test
    fun `should warn that stopping does not cancel the payment`() {
        for (id in listOf(ByteArray(32), null)) {
            val warning = IdentityShieldRecoveryDisplay.stopRetryingWarning(id)
            assertTrue(warning.startsWith("This does not cancel the payment."))
            assertTrue(warning.contains("additional debit"))
        }
    }

    @Test
    fun `should name a record's identity from the wallet's rows or a short id`() {
        val id = ByteArray(32) { 0x0a }
        val alice = IdentityEntity(identityId = id, networkRaw = 1, mainDpnsName = "alice.dash")
        assertEquals("alice.dash", IdentityShieldRecoveryDisplay.identityLabel(id, listOf(alice)))
        assertEquals("0a0a0a0a0a0a…", IdentityShieldRecoveryDisplay.identityLabel(id, emptyList()))
        assertEquals("Unknown identity", IdentityShieldRecoveryDisplay.identityLabel(null, listOf(alice)))
    }

    @Test
    fun `should keep the last records when a read fails`() {
        val loaded = RecoveryReadState<String>().applySuccess(listOf("record"))
        assertFalse(loaded.isKnownEmpty)

        val failed = loaded.applyFailure("store busy")
        assertEquals(listOf("record"), failed.records)
        assertEquals("store busy", failed.failureMessage)

        assertFalse("never read is not known empty", RecoveryReadState<String>().isKnownEmpty)
        assertTrue(RecoveryReadState<String>().applySuccess(emptyList()).isKnownEmpty)
        assertNull(failed.applySuccess(emptyList()).failureMessage)
    }

    @Test
    fun `should report a returned shielded top-up as submitted with the requested amount`() {
        val outcome = ShieldedTopUpMessages.outcome(50_000_000_000, null).getOrThrow()
        assertEquals(ShieldedTopUpOutcome.Completed(50_000_000_000), outcome)
        assertEquals(
            "The spent notes were confirmed. Requested: 0.5 DASH.",
            ShieldedTopUpMessages.completedText(outcome.amount),
        )
    }

    @Test
    fun `should not offer a retry for a shielded top-up that may have gone through`() {
        val outcome = ShieldedTopUpMessages.outcome(
            50_000_000_000,
            DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed("not proved"),
        ).getOrThrow()
        assertEquals(ShieldedTopUpOutcome.MayHaveGoneThrough(50_000_000_000), outcome)

        val withPending = ShieldedTopUpMessages.unconfirmedText(outcome.amount, pendingListedBelow = true)
        assertTrue(withPending.contains("Do not retry"))
        assertTrue(withPending.endsWith("Pending top-ups for this identity are listed below."))
        assertFalse(
            ShieldedTopUpMessages.unconfirmedText(outcome.amount, pendingListedBelow = false)
                .contains("listed below"),
        )
    }

    @Test
    fun `should word a shielded top-up refused below its protocol version`() {
        val refused = ShieldedTopUpMessages.outcome(
            1,
            DashSdkError.PlatformWallet.ProtocolVersionNotActive("pv 13"),
        ).exceptionOrNull()
        assertTrue(refused!!.message!!.endsWith("Nothing was sent."))

        val other = ShieldedTopUpMessages.outcome(1, IllegalStateException("not enough notes"))
            .exceptionOrNull()
        assertEquals("not enough notes", other!!.message)
    }

    @Test
    fun `should say what a Check Now came to`() {
        assertNull(CheckNowStatus.Idle.text)
        assertNull(CheckNowStatus.Checking.text)
        assertFalse(CheckNowStatus.Finished.isFailure)
        assertEquals("Check failed: offline", CheckNowStatus.Failed("offline").text)
        assertTrue(CheckNowStatus.Failed("offline").isFailure)
    }
}
