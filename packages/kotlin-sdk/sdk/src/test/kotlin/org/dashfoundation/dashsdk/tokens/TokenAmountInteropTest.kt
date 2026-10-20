package org.dashfoundation.dashsdk.tokens

import java.math.BigInteger
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class TokenAmountInteropTest {

    @Test
    fun `java adapter round trips full u64 domain through raw jlong`() {
        val max = BigInteger("18446744073709551615")
        assertEquals(-1L, TokenAmountInterop.toRawLongBits(max))
        assertEquals(max, TokenAmountInterop.fromRawLongBits(-1L))
        assertEquals(
            BigInteger("9223372036854775808"),
            TokenAmountInterop.fromRawLongBits(Long.MIN_VALUE),
        )
    }

    @Test
    fun `java adapter rejects values outside u64`() {
        assertThrows(IllegalArgumentException::class.java) {
            TokenAmountInterop.toRawLongBits(BigInteger.valueOf(-1))
        }
        assertThrows(IllegalArgumentException::class.java) {
            TokenAmountInterop.toRawLongBits(BigInteger.ONE.shiftLeft(64))
        }
    }
}
