package org.dashfoundation.example.ui.shielded

import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.wallet.ShieldedIdentityDebitRecoveryStatus
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import org.dashfoundation.example.testing.spendUnconfirmed
import org.dashfoundation.example.testing.broadcastUnconfirmed

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
        assertEquals(ShieldedPaymentOutcome.Submitted, IdentityShieldMessages.outcome(null))
        assertTrue(IdentityShieldMessages.SUBMITTED.contains("submitted"))
        assertTrue(IdentityShieldMessages.SUBMITTED.contains("pending"))
    }

    @Test
    fun `should not offer a retry for an identity shield that may have gone through`() {
        for (error in listOf(
            spendUnconfirmed("not proved"),
            broadcastUnconfirmed("not confirmed"),
        )) {
            assertEquals(ShieldedPaymentOutcome.MayHaveGoneThrough, IdentityShieldMessages.outcome(error))
        }
        assertTrue(IdentityShieldMessages.MAY_HAVE_GONE_THROUGH.contains("Do not retry"))
        assertTrue(IdentityShieldMessages.MAY_HAVE_GONE_THROUGH.contains("In-Progress Identity Shields"))
    }

    @Test
    fun `should word the wallet's refusals made before anything was sent`() {
        val pending = IdentityShieldMessages.outcome(
            DashSdkError.PlatformWallet.ShieldedIdentityDebitPending("unresolved"),
        )
        assertEquals(ShieldedPaymentOutcome.Failed(IdentityShieldMessages.DEBIT_PENDING), pending)

        val tooLow = IdentityShieldMessages.outcome(
            DashSdkError.PlatformWallet.InsufficientIdentityCredits(
                identityId = "7y3Q",
                requiredCredits = 150_000_000_000uL,
                availableCredits = 100_000_000_000uL,
            ),
        ) as ShieldedPaymentOutcome.Failed
        assertTrue(tooLow.message.contains("too low"))
        assertTrue(tooLow.message, tooLow.message.contains("needs 1.5 DASH"))
        assertTrue(tooLow.message, tooLow.message.contains("has 1 DASH"))
        assertTrue(tooLow.message.endsWith("Nothing was sent."))

        for (error in listOf(
            DashSdkError.PlatformWallet.IdentityBalanceUnavailable("no proof"),
            DashSdkError.PlatformWallet.ProtocolVersionNotActive("pv 13"),
            DashSdkError.PlatformWallet.SdkPinnedBelowProtocolVersion("pinned"),
        )) {
            val outcome = IdentityShieldMessages.outcome(error) as ShieldedPaymentOutcome.Failed
            assertTrue(outcome.message, outcome.message.endsWith("Nothing was sent."))
        }
    }

    @Test
    fun `should show any other failure's own message`() {
        val outcome = IdentityShieldMessages.outcome(IllegalStateException("signer missing"))
        assertEquals(ShieldedPaymentOutcome.Failed("signer missing"), outcome)
    }

    @Test
    fun `should rethrow a cancellation instead of reporting it`() {
        try {
            IdentityShieldMessages.outcome(CancellationException("left"))
            fail("expected the cancellation to be rethrown")
        } catch (_: CancellationException) {
        }
        try {
            ShieldedTopUpMessages.outcome(CancellationException("left"))
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
        val labels = mapOf("0a".repeat(32) to "alice.dash")
        assertEquals("alice.dash", IdentityShieldRecoveryDisplay.identityLabel(id, labels))
        assertEquals("0a0a0a0a0a0a…", IdentityShieldRecoveryDisplay.identityLabel(id, emptyMap()))
        assertEquals("Unknown identity", IdentityShieldRecoveryDisplay.identityLabel(null, labels))
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
        assertEquals(ShieldedPaymentOutcome.Submitted, ShieldedTopUpMessages.outcome(null))
        assertEquals(
            "The spent notes were confirmed. Requested: 0.5 DASH.",
            ShieldedTopUpMessages.completedText(50_000_000_000),
        )
    }

    @Test
    fun `should show a one-credit amount exactly, never as zero`() {
        assertEquals(
            "The spent notes were confirmed. Requested: 0.00000000001 DASH.",
            ShieldedTopUpMessages.completedText(1),
        )
        assertTrue(
            ShieldedTopUpMessages.unconfirmedText(1, pendingListedBelow = false)
                .startsWith("The top-up of 0.00000000001 DASH could not be confirmed."),
        )
        assertEquals("0.00000000001 DASH", formatCreditsExact(1uL))
        val tooLow = IdentityShieldMessages.outcome(
            DashSdkError.PlatformWallet.InsufficientIdentityCredits("7y3Q", 100_000_000_001uL, 1uL),
        ) as ShieldedPaymentOutcome.Failed
        assertTrue(tooLow.message, tooLow.message.contains("needs 1.00000000001 DASH"))
        assertTrue(tooLow.message, tooLow.message.contains("has 0.00000000001 DASH"))
    }

    @Test
    fun `should not offer a retry for a shielded top-up that may have gone through`() {
        val outcome = ShieldedTopUpMessages.outcome(
            spendUnconfirmed("not proved"),
        )
        assertEquals(ShieldedPaymentOutcome.MayHaveGoneThrough, outcome)

        val withPending = ShieldedTopUpMessages.unconfirmedText(50_000_000_000, pendingListedBelow = true)
        assertTrue(withPending.contains("Do not retry"))
        assertTrue(withPending.endsWith("Pending top-ups for this identity are listed below."))
        assertFalse(
            ShieldedTopUpMessages.unconfirmedText(50_000_000_000, pendingListedBelow = false)
                .contains("listed below"),
        )
    }

    @Test
    fun `should word a shielded top-up refused below its protocol version`() {
        val refused = ShieldedTopUpMessages.outcome(
            DashSdkError.PlatformWallet.ProtocolVersionNotActive("pv 13"),
        ) as ShieldedPaymentOutcome.Failed
        assertTrue(refused.message.endsWith("Nothing was sent."))

        assertEquals(
            ShieldedPaymentOutcome.Failed("not enough notes"),
            ShieldedTopUpMessages.outcome(IllegalStateException("not enough notes")),
        )
    }

    @Test
    fun `should say what a Check Now came to`() {
        assertNull(CheckNowStatus.Idle.text)
        assertNull(CheckNowStatus.Checking.text)
        assertFalse(CheckNowStatus.Finished.isFailure)
        // The sync call cannot say this wallet was scanned, so the line never
        // claims the records are current.
        assertTrue(CheckNowStatus.Finished.text!!.startsWith("Sync requested."))
        assertTrue(CheckNowStatus.Finished.text!!.contains("cannot tell whether the scan"))
        assertEquals("Check failed: offline", CheckNowStatus.Failed("offline").text)
        assertTrue(CheckNowStatus.Failed("offline").isFailure)
    }
}
