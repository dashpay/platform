package org.dashfoundation.dashsdk.wallet

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

/**
 * Pins the host side of [PlatformWalletManager.registerIdentityForTokenSync]:
 * the token ids reach the FFI back to back in the given order, and a
 * malformed id is refused before the native call, as Swift's wrapper does.
 */
class IdentityTokenSyncMarshalTest {

    private fun id(tag: Int) = ByteArray(32) { tag.toByte() }

    @Test
    fun shouldLayTokenIdsBackToBackInOrder() {
        val flat = flattenTokenSyncIds(id(9), listOf(id(1), id(2), id(3)))
        assertEquals(96, flat.size)
        assertArrayEquals(id(1), flat.copyOfRange(0, 32))
        assertArrayEquals(id(2), flat.copyOfRange(32, 64))
        assertArrayEquals(id(3), flat.copyOfRange(64, 96))
    }

    @Test
    fun shouldPassAnEmptyBufferForNoTokens() {
        assertEquals(0, flattenTokenSyncIds(id(9), emptyList()).size)
    }

    @Test
    fun shouldRefuseAnIdentityIdThatIsNot32Bytes() {
        val e = assertThrows(IllegalArgumentException::class.java) {
            flattenTokenSyncIds(ByteArray(31), listOf(id(1)))
        }
        assertEquals("identityId must be 32 bytes, got 31", e.message)
    }

    @Test
    fun shouldRefuseATokenIdThatIsNot32Bytes() {
        val e = assertThrows(IllegalArgumentException::class.java) {
            flattenTokenSyncIds(id(9), listOf(id(1), ByteArray(33)))
        }
        assertEquals("token id must be 32 bytes, got 33", e.message)
    }
}
