package org.dashfoundation.dashsdk.identity

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.nio.ByteBuffer

/**
 * The usage-limits section of the add/register public-key blob (protocol version
 * 14): every row ends with a flags byte, followed only by the limits that are set,
 * each a big-endian u64. The Rust parser (`rs-unified-sdk-jni::pubkey_rows`) reads
 * exactly this layout.
 */
class IdentityPubkeyCodecTest {

    private fun key(
        keyId: Int,
        totalBudget: Long? = null,
        expiresAt: Long? = null,
    ) = IdentityPubkey(
        keyId = keyId,
        keyType = KeyType.ECDSA_SECP256K1,
        purpose = KeyPurpose.AUTHENTICATION,
        securityLevel = SecurityLevel.CRITICAL,
        pubkeyBytes = ByteArray(33) { keyId.toByte() },
        totalBudget = totalBudget,
        expiresAt = expiresAt,
    )

    /** The fixed header, the pubkey and the flags byte of a row without bounds. */
    private fun rowWithoutLimitsSize() = 4 + 1 + 1 + 1 + 1 + 1 + 2 + 33 + 1

    @Test
    fun `a key without limits ends with a zero flags byte`() {
        val encoded = IdentityPubkeyCodec.encode(listOf(key(1)))

        assertEquals(4 + rowWithoutLimitsSize(), encoded.size)
        assertEquals(0, encoded.last().toInt())
    }

    @Test
    fun `a budget and an expiry follow the flags byte as big-endian u64s`() {
        val encoded = IdentityPubkeyCodec.encode(
            listOf(key(1, totalBudget = 500_000_000L, expiresAt = 1_800_000_000_000L)),
        )

        assertEquals(4 + rowWithoutLimitsSize() + 16, encoded.size)
        val tail = ByteBuffer.wrap(encoded, 4 + rowWithoutLimitsSize() - 1, 17)
        assertEquals(0b11, tail.get().toInt())
        assertEquals(500_000_000L, tail.getLong())
        assertEquals(1_800_000_000_000L, tail.getLong())
    }

    @Test
    fun `one limit sets one flag bit and writes one value`() {
        val budgetOnly = IdentityPubkeyCodec.encode(listOf(key(1, totalBudget = 7L)))
        val expiryOnly = IdentityPubkeyCodec.encode(listOf(key(1, expiresAt = 9L)))

        assertEquals(4 + rowWithoutLimitsSize() + 8, budgetOnly.size)
        assertEquals(4 + rowWithoutLimitsSize() + 8, expiryOnly.size)
        assertEquals(0b01, budgetOnly[4 + rowWithoutLimitsSize() - 1].toInt())
        assertEquals(0b10, expiryOnly[4 + rowWithoutLimitsSize() - 1].toInt())
        assertArrayEquals(
            ByteBuffer.allocate(8).putLong(7L).array(),
            budgetOnly.copyOfRange(4 + rowWithoutLimitsSize(), budgetOnly.size),
        )
        assertArrayEquals(
            ByteBuffer.allocate(8).putLong(9L).array(),
            expiryOnly.copyOfRange(4 + rowWithoutLimitsSize(), expiryOnly.size),
        )
    }

    @Test
    fun `hasLimits reports either limit`() {
        assertFalse(key(1).hasLimits)
        assertTrue(key(1, totalBudget = 1L).hasLimits)
        assertTrue(key(1, expiresAt = 1L).hasLimits)
    }
}
