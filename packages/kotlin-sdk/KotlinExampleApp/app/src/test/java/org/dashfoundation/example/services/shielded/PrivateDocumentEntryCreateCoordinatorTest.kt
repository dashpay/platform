package org.dashfoundation.example.services.shielded

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.wallet.PrivateDocumentSenderAuth
import org.dashfoundation.example.services.shielded.PrivateDocumentEntryCreateCoordinator.Refusal
import org.dashfoundation.example.services.shielded.PrivateDocumentEntryCreateCoordinator.Request
import org.dashfoundation.example.services.shielded.PrivateDocumentEntryCreateCoordinator.State
import org.dashfoundation.example.ui.contracts.PrivateEntryCreateOutcome
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The coordinator owns a private entry create past the screen that started it (TEST_PLAN
 * DOC-19): a screen recreated while the create proves sees it running and then its outcome, and a
 * request whose create may have gone through is never sent again in this process, whatever was
 * created or refused in between.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class PrivateDocumentEntryCreateCoordinatorTest {

    private fun request(note: String = "{\"note\":\"a\"}") = Request(
        walletIdHex = "aa",
        senderIdentityIdHex = "bb",
        contractIdHex = "cc",
        documentTypeName = "note",
        recipientIdentityIdHex = "dd",
        recipientKeyId = 1,
        auth = PrivateDocumentSenderAuth.ANONYMOUS,
        propertiesJson = note,
    )

    @Test
    fun `should keep a running create past the screen and refuse a second one from the wallet`() = runTest {
        val coordinator = PrivateDocumentEntryCreateCoordinator(TestScope(UnconfinedTestDispatcher(testScheduler)))
        val proof = CompletableDeferred<Unit>()
        var calls = 0

        assertNull(coordinator.start(request()) { calls++; proof.await(); throw unconfirmed() })
        assertEquals(State.InFlight(request()), coordinator.states.value["aa"])
        // The recreated screen taps again, even for another entry: nothing new is paid
        assertEquals(Refusal.BUSY, coordinator.start(request("{}")) { calls++; throw unconfirmed() })
        assertEquals(1, calls)

        proof.complete(Unit)
        val done = coordinator.states.value["aa"] as State.Done
        assertTrue(done.outcome is PrivateEntryCreateOutcome.Unconfirmed)
    }

    @Test
    fun `should never send again a request that may have gone through, even after a refused one`() = runTest {
        val coordinator = PrivateDocumentEntryCreateCoordinator(TestScope(UnconfinedTestDispatcher(testScheduler)))
        var calls = 0

        // A may have gone through
        assertNull(coordinator.start(request()) { calls++; throw unconfirmed() })
        assertTrue(request() in coordinator.held.value)
        // B is refused by Rust before anything is paid
        assertNull(coordinator.start(request("{}")) { calls++; throw DashSdkError.InvalidParameter("refused") })
        assertEquals(
            PrivateEntryCreateOutcome.Failed("refused"),
            (coordinator.states.value["aa"] as State.Done).outcome,
        )
        // Back to A: still held
        assertEquals(Refusal.HELD, coordinator.start(request()) { calls++; throw unconfirmed() })
        assertEquals(2, calls)
    }

    @Test
    fun `should not hold a request that was refused`() = runTest {
        val coordinator = PrivateDocumentEntryCreateCoordinator(TestScope(UnconfinedTestDispatcher(testScheduler)))
        assertNull(coordinator.start(request()) { throw DashSdkError.InvalidParameter("refused") })
        assertTrue(coordinator.held.value.isEmpty())
        // Fixed and sent again
        assertNull(coordinator.start(request()) { throw DashSdkError.InvalidParameter("refused again") })
    }

    private fun unconfirmed() = DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed("not proved")
}
