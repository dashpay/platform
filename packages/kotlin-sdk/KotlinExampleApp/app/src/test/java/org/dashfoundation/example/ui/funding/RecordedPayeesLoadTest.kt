package org.dashfoundation.example.ui.funding

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The resume form of an external asset lock enables "Complete External
 * Payment" only once the payees the lock recorded are loaded, and tells
 * waiting for the wallet, a lock that records none and a failed read apart:
 * a failed read offers a retry instead of reading as "no recipients" or as a
 * load that never ends. ← Swift `RecordedPayeesLoadTests`.
 */
class RecordedPayeesLoadTest {

    private val outPointHex = "ab".repeat(32) + ":1"
    private val payee = ManagedPlatformWallet.FundRecipient(0, ByteArray(20) { 0x11 }, 5_000L)

    @Test
    fun `the payees load only from a successful read that finds them`() = runTest {
        var readAt: Pair<ByteArray, Int>? = null
        val load = readRecordedPayees(outPointHex) { txid, vout ->
            readAt = txid to vout
            listOf(payee)
        }
        assertEquals(RecordedPayeesLoad.Loaded(listOf(payee)), load)
        assertEquals(listOf(payee), load.loadedPayees)
        // The display-order txid is read back in wire order.
        assertTrue(readAt!!.first.all { it == 0xab.toByte() })
        assertEquals(1, readAt!!.second)
    }

    @Test
    fun `a lock without recorded payees is not found and keeps submit disabled`() = runTest {
        for (found in listOf(null, emptyList<ManagedPlatformWallet.FundRecipient>())) {
            val load = readRecordedPayees(outPointHex) { _, _ -> found }
            assertEquals(RecordedPayeesLoad.NotFound, load)
            assertNull(load.loadedPayees)
        }
    }

    @Test
    fun `a failed read is a failure with a retry, not not-found or loading`() = runTest {
        val load = readRecordedPayees(outPointHex) { _, _ -> throw IllegalStateException("store unavailable") }
        assertEquals(RecordedPayeesLoad.Failed("store unavailable"), load)
        assertNull(load.loadedPayees)

        val unparsable = readRecordedPayees("not-an-outpoint") { _, _ -> listOf(payee) }
        assertTrue(unparsable is RecordedPayeesLoad.Failed)
    }

    @Test
    fun `no wallet yet waits for it, and waiting or loading keeps submit disabled`() = runTest {
        assertEquals(RecordedPayeesLoad.WaitingForWallet, readRecordedPayees(outPointHex, null))
        assertNull(RecordedPayeesLoad.WaitingForWallet.loadedPayees)
        assertNull(RecordedPayeesLoad.Loading.loadedPayees)
    }

    @Test
    fun `a cancelled read is not reported as a failure`() = runTest {
        assertThrows(CancellationException::class.java) {
            kotlinx.coroutines.runBlocking {
                readRecordedPayees(outPointHex) { _, _ -> throw CancellationException("left the screen") }
            }
        }
    }

    @Test
    fun `each recorded payee is shown with its amount or as the remainder`() {
        val remainder = ManagedPlatformWallet.FundRecipient(0, ByteArray(20) { 0x22 }, null)
        assertEquals(
            listOf(
                "1111111111111111… — 5000 credits",
                "2222222222222222… — remainder (this wallet)",
            ),
            recordedPayeeLines(listOf(payee, remainder)),
        )
    }
}
