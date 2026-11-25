package org.dashfoundation.dashsdk.tokens

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class TokenConfigUpdateOutcomeTest {

    @Test
    fun `should decode a pending group action`() {
        assertEquals(
            TokenConfigUpdateOutcome.Pending,
            TokenConfigUpdateOutcome.fromJson("""{"outcome":"pending"}"""),
        )
    }

    @Test
    fun `should decode an applied update with the contract it produced`() {
        val outcome = TokenConfigUpdateOutcome.fromJson(
            """{"outcome":"applied","contract":{"id":"abc","version":2},"serializationHex":"ab01"}""",
        )

        assertTrue(outcome is TokenConfigUpdateOutcome.Applied)
        val contract = (outcome as TokenConfigUpdateOutcome.Applied).contract
        assertEquals("""{"id":"abc","version":2}""", contract.json)
        assertArrayEquals(byteArrayOf(0xab.toByte(), 0x01), contract.binarySerialization)
    }

    @Test
    fun `should decode an applied update whose contract could not be fetched`() {
        assertEquals(
            TokenConfigUpdateOutcome.AppliedNotRefreshed(3, "the node returned version 2"),
            TokenConfigUpdateOutcome.fromJson(
                """{"outcome":"appliedNotRefreshed","minimumVersion":3,"reason":"the node returned version 2"}""",
            ),
        )
    }

    @Test
    fun `should refuse an outcome it does not know`() {
        assertThrows(IllegalArgumentException::class.java) {
            TokenConfigUpdateOutcome.fromJson("""{"outcome":"maybe"}""")
        }
    }
}
