package org.dashfoundation.dashsdk

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.WalletDrainNative
import org.dashfoundation.dashsdk.wallet.WalletDrainLockListing
import org.dashfoundation.dashsdk.wallet.WalletDrainOutcome
import org.dashfoundation.dashsdk.wallet.WalletDrainOutcome.State
import org.dashfoundation.dashsdk.wallet.WalletDrainPreview
import org.dashfoundation.dashsdk.wallet.WalletDrainRecoveryAction
import org.dashfoundation.dashsdk.wallet.WalletDrainRules
import org.dashfoundation.dashsdk.wallet.WalletDrainStatusRead
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * The `WalletDrainNative` JNI surface against the real library: every
 * export binds with the declared signature, the wallet-drain rules answer
 * as `platform_wallet::wallet::shielded::wallet_drain_recovery` defines
 * them, and a call that cannot reach a wallet fails the way its wrapper
 * expects (the reads throw; the execution returns a pending outcome with
 * the error beside it).
 *
 * The rules' tables are owned and tested in Rust. The expectations here are
 * the same tables, so this test is what catches a JNI argument landing in
 * the wrong slot.
 */
@RunWith(AndroidJUnit4::class)
class WalletDrainFfiTest {

    private val reads: List<WalletDrainStatusRead?> = listOf(
        null,
        WalletDrainStatusRead.Failed,
        WalletDrainStatusRead.Reported(State.NotSent),
        WalletDrainStatusRead.Reported(State.Pending),
        WalletDrainStatusRead.Reported(State.Completed),
    )

    @Before
    fun setUp() {
        NativeLoader.ensureLoaded()
        assumeTrue("the native library has no shielded support", Sdk.hasShielded())
    }

    @Test
    fun shouldKeepAStoredIdUnlessTheOperationNeedsNothingMore() {
        for (overlapped in listOf(false, true)) {
            assertFalse(
                "completed: forget",
                WalletDrainRules.keepsStoredId(WalletDrainStatusRead.Reported(State.Completed), overlapped),
            )
            assertTrue(
                "pending: keep",
                WalletDrainRules.keepsStoredId(WalletDrainStatusRead.Reported(State.Pending), overlapped),
            )
            assertTrue(
                "a failed read: keep",
                WalletDrainRules.keepsStoredId(WalletDrainStatusRead.Failed, overlapped),
            )
            assertEquals(
                "not sent: forget unless an attempt overlapped the read",
                overlapped,
                WalletDrainRules.keepsStoredId(WalletDrainStatusRead.Reported(State.NotSent), overlapped),
            )
        }
    }

    @Test
    fun shouldOfferAResumeOnlyForAPendingReportThatNamedAnAssetLock() {
        for (read in reads) {
            for (named in listOf(false, true)) {
                assertEquals(
                    "a running attempt wins: $read, named: $named",
                    WalletDrainRecoveryAction.InProgress,
                    WalletDrainRules.recoveryAction(true, read, named),
                )
                val expected =
                    if (named && read == WalletDrainStatusRead.Reported(State.Pending)) {
                        WalletDrainRecoveryAction.Resume
                    } else {
                        WalletDrainRecoveryAction.CheckAgain
                    }
                assertEquals(
                    "$read, named: $named",
                    expected,
                    WalletDrainRules.recoveryAction(false, read, named),
                )
            }
        }
    }

    @Test
    fun shouldCountOnlyACompletedAttemptAsFinished() {
        assertFalse(WalletDrainRules.attemptFinished(State.NotSent))
        assertFalse(WalletDrainRules.attemptFinished(State.Pending))
        assertTrue(WalletDrainRules.attemptFinished(State.Completed))
    }

    @Test
    fun shouldAllowAStartUnlessThePreviewHasNothingToShield() {
        assertTrue(WalletDrainRules.previewAllowsStart(WalletDrainPreview.Status.Ready))
        assertTrue(WalletDrainRules.previewAllowsStart(WalletDrainPreview.Status.InputLimitMayBeExceeded))
        assertFalse(WalletDrainRules.previewAllowsStart(WalletDrainPreview.Status.InsufficientFunds))
    }

    @Test
    fun shouldOfferANewDrainOnEveryNetworkButMainnet() {
        for (network in Network.entries) {
            assertEquals(network.name, network != Network.MAINNET, WalletDrainRules.startOffered(network))
        }
        // An ordinal this library does not know is refused, not read as a
        // test network.
        assertThrows(DashSDKException::class.java) { WalletDrainNative.startOffered(9) }
    }

    @Test
    fun shouldListADrainLockByItsStoredIdOrAsAResumableCheckpoint() {
        // (funding type, status, has operation id, host holds a stored id)
        val shielded = 5
        val address = 4
        for ((arguments, expected) in listOf(
            listOf(shielded, 0, false, false) to WalletDrainLockListing.Unaffected,
            listOf(shielded, 0, false, true) to WalletDrainLockListing.Unaffected,
            listOf(shielded, 0, true, true) to WalletDrainLockListing.Hidden,
            listOf(shielded, 3, true, true) to WalletDrainLockListing.Hidden,
            listOf(shielded, 6, true, true) to WalletDrainLockListing.Hidden,
            listOf(shielded, 0, true, false) to WalletDrainLockListing.Offered,
            listOf(shielded, 1, true, false) to WalletDrainLockListing.Unaffected,
            listOf(address, 0, true, false) to WalletDrainLockListing.Unaffected,
        )) {
            assertEquals(
                arguments.toString(),
                expected,
                WalletDrainRules.lockListing(
                    fundingTypeRaw = arguments[0] as Int,
                    statusRaw = arguments[1] as Int,
                    hasOperationId = arguments[2] as Boolean,
                    hostHoldsStoredId = arguments[3] as Boolean,
                ),
            )
        }
        // A status or funding type Rust does not know is not listed: the
        // native call refuses it, and the rule answers "unknown" for it.
        assertThrows(DashSDKException::class.java) {
            WalletDrainNative.lockListing(shielded, 99, hasOperationId = true, hostHoldsStoredId = false)
        }
        assertNull(WalletDrainRules.lockListingOrNull(shielded, 99, hasOperationId = true, hostHoldsStoredId = false))
        assertNull(WalletDrainRules.lockListingOrNull(99, 0, hasOperationId = true, hostHoldsStoredId = false))
        assertEquals(
            WalletDrainLockListing.Offered,
            WalletDrainRules.lockListingOrNull(shielded, 0, hasOperationId = true, hostHoldsStoredId = false),
        )
        assertEquals(
            WalletDrainLockListing.Unaffected,
            WalletDrainRules.lockListing(shielded, 99, hasOperationId = true, hostHoldsStoredId = false),
        )
    }

    @Test
    fun shouldReportNotSentAsPendingOnlyWhileAHostCallIsActive() {
        for (state in State.entries) {
            assertEquals("no call active: $state", state, WalletDrainRules.statusDuringHostCall(state, false))
        }
        assertEquals(State.Pending, WalletDrainRules.statusDuringHostCall(State.NotSent, true))
        // Once Rust has seen the operation, its answer stands.
        assertEquals(State.Pending, WalletDrainRules.statusDuringHostCall(State.Pending, true))
        assertEquals(State.Completed, WalletDrainRules.statusDuringHostCall(State.Completed, true))
        // A state Rust does not know is refused, not passed through.
        assertThrows(DashSDKException::class.java) { WalletDrainNative.statusDuringHostCall(3, true) }
        assertThrows(DashSDKException::class.java) { WalletDrainNative.statusDuringHostCall(300, false) }
    }

    @Test
    fun shouldRefuseAStateRustDoesNotKnow() {
        assertThrows(DashSDKException::class.java) { WalletDrainNative.attemptFinished(3) }
        assertThrows(DashSDKException::class.java) { WalletDrainNative.attemptFinished(300) }
        assertThrows(DashSDKException::class.java) { WalletDrainNative.previewAllowsStart(3) }
        assertThrows(DashSDKException::class.java) {
            WalletDrainNative.keepsStoredId(readSucceeded = true, reportedState = 3, attemptOverlappedRead = false)
        }
        // A failed read carries no state: the byte is ignored.
        assertTrue(
            WalletDrainNative.keepsStoredId(readSucceeded = false, reportedState = 300, attemptOverlappedRead = false),
        )
    }

    @Test
    fun shouldThrowFromTheReadsWhenTheManagerHandleIsInvalid() {
        val walletId = ByteArray(32) { 1 }
        val operationId = ByteArray(16) { 2 }
        assertThrows(DashSDKException::class.java) { WalletDrainNative.preview(0L, walletId, 0) }
        assertThrows(DashSDKException::class.java) { WalletDrainNative.status(0L, walletId, operationId) }
        // Arguments of the wrong size are refused before any wallet code runs.
        assertThrows(DashSDKException::class.java) { WalletDrainNative.preview(0L, ByteArray(31), 0) }
        assertThrows(DashSDKException::class.java) { WalletDrainNative.status(0L, walletId, ByteArray(15)) }
        assertThrows(DashSDKException::class.java) { WalletDrainNative.preview(0L, walletId, -1) }
    }

    @Test
    fun shouldReturnAPendingOutcomeWithItsErrorWhenTheDrainCannotReachAWallet() {
        val packed = WalletDrainNative.fundFromWalletDrain(
            managerHandle = 0L,
            walletId = ByteArray(32) { 1 },
            operationId = ByteArray(16) { 2 },
            accountIndex = 0,
            // Not a valid Orchard address either; whichever check fails first,
            // the failure is returned beside the outcome, never thrown.
            recipientRaw43 = ByteArray(43) { 3 },
            coreSignerHandle = 1L,
        )
        val outcome = WalletDrainOutcome.decode(packed)

        assertEquals("a call that did not run cannot say nothing was sent", State.Pending, outcome.state)
        assertNull(outcome.outpoint)
        assertNotNull(outcome.error)

        assertThrows(DashSDKException::class.java) {
            WalletDrainNative.fundFromWalletDrain(0L, ByteArray(32), ByteArray(15), 0, ByteArray(43), 1L)
        }
    }
}
