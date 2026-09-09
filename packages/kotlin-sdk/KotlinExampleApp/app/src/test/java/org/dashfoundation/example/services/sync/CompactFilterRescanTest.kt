package org.dashfoundation.example.services.sync

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class CompactFilterRescanTest {

    @Test
    fun `per wallet typed failures are collected without aborting other rewinds`() = runTest {
        val wallets = listOf(
            CompactFilterRescanWallet("a", ByteArray(32) { 1 }),
            CompactFilterRescanWallet("missing", ByteArray(32) { 2 }),
            CompactFilterRescanWallet("c", ByteArray(32) { 3 }),
        )
        val calls = mutableListOf<Pair<Byte, Int>>()

        val result = CompactFilterRescan.armAll(wallets, 123) { id, height ->
            calls += id[0] to height
            if (id[0] == 2.toByte()) throw DashSdkError.NotFound("Wallet not found")
        }

        assertEquals(listOf("a", "c"), result.acceptedWalletIds)
        assertEquals(1, result.failures.size)
        assertEquals("missing", result.failures.single().walletIdHex)
        assertTrue(result.failures.single().error is DashSdkError.NotFound)
        assertEquals(listOf(1.toByte(), 2.toByte(), 3.toByte()), calls.map { it.first })
        assertTrue(calls.all { it.second == 123 })
    }
}
