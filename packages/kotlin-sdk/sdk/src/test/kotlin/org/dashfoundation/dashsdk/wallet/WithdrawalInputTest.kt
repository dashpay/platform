package org.dashfoundation.dashsdk.wallet

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertThrows
import org.junit.Test

/**
 * [ManagedPlatformWallet.WithdrawalInput] is a plain carrier and Rust
 * validates it. These tests cover its own behaviour (content equality, a
 * `toString` that never prints the hash or amount) and the blob encoding
 * the JNI decodes.
 */
class WithdrawalInputTest {

    private val hash = ByteArray(20) { it.toByte() }

    @Test
    fun `should compare inputs by hash content`() {
        val a = ManagedPlatformWallet.WithdrawalInput(0, hash.copyOf(), 5L)
        val b = ManagedPlatformWallet.WithdrawalInput(0, hash.copyOf(), 5L)
        assertEquals(a, b)
        assertEquals(a.hashCode(), b.hashCode())
        assertNotEquals(a, ManagedPlatformWallet.WithdrawalInput(0, hash.copyOf(), 6L))
    }

    @Test
    fun `should not print the hash or amount`() {
        val text = ManagedPlatformWallet.WithdrawalInput(0, hash, 123_456_789L).toString()
        assertFalse(text.contains("123456789"))
        assertFalse(text.contains("hash"))
    }

    @Test
    fun `should encode inputs as big-endian 29-byte rows in list order`() {
        val other = ByteArray(20) { 0x7F }
        val blob = encodeWithdrawalInputs(
            listOf(
                ManagedPlatformWallet.WithdrawalInput(0, hash, 0x0102L),
                ManagedPlatformWallet.WithdrawalInput(1, other, -1L),
            ),
        )
        assertEquals(4 + 2 * 29, blob.size)
        assertArrayEquals(byteArrayOf(0, 0, 0, 2), blob.copyOfRange(0, 4))
        assertEquals(0.toByte(), blob[4])
        assertArrayEquals(hash, blob.copyOfRange(5, 25))
        assertArrayEquals(byteArrayOf(0, 0, 0, 0, 0, 0, 1, 2), blob.copyOfRange(25, 33))
        // Values are passed through for Rust to judge: P2SH and a negative
        // amount are encoded as given.
        assertEquals(1.toByte(), blob[33])
        assertArrayEquals(other, blob.copyOfRange(34, 54))
        assertArrayEquals(ByteArray(8) { -1 }, blob.copyOfRange(54, 62))
    }

    @Test
    fun `should encode an empty list for Rust to reject`() {
        assertArrayEquals(byteArrayOf(0, 0, 0, 0), encodeWithdrawalInputs(emptyList()))
    }

    @Test
    fun `should refuse inputs that cannot fit the fixed-width row`() {
        assertThrows(IllegalArgumentException::class.java) {
            encodeWithdrawalInputs(listOf(ManagedPlatformWallet.WithdrawalInput(0, ByteArray(19), 1L)))
        }
        assertThrows(IllegalArgumentException::class.java) {
            encodeWithdrawalInputs(listOf(ManagedPlatformWallet.WithdrawalInput(256, hash, 1L)))
        }
    }
}
