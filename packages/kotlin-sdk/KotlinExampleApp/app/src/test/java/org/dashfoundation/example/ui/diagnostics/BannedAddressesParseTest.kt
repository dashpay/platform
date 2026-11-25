package org.dashfoundation.example.ui.diagnostics

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * Ban info from the SPV peer list: a JSON `null` or a missing field shows as
 * no value, never as the text "null" that Android's `optString` returns.
 * Runs on Robolectric for Android's own org.json.
 */
@RunWith(RobolectricTestRunner::class)
class BannedAddressesParseTest {

    @Test
    fun shouldShowNoReasonWhenTheReasonIsJsonNull() {
        val entries = parseBanInfo(
            """[{"address": "34.1.2.3:20001", "banned": true, "banCount": 2, "bannedUntilMs": 5, "reason": null}]""",
        )
        assertEquals(1, entries.size)
        assertEquals("34.1.2.3:20001", entries[0].address)
        assertNull(entries[0].reason)
    }

    @Test
    fun shouldFallBackToADashForANullOrMissingAddress() {
        val entries = parseBanInfo("""[{"address": null, "reason": "  "}, {}]""")
        assertEquals(listOf("—", "—"), entries.map { it.address })
        assertEquals(listOf(null, null), entries.map { it.reason })
    }

    @Test
    fun shouldKeepAReasonThatIsSet() {
        val entries = parseBanInfo("""[{"address": "34.1.2.3:20001", "reason": "misbehaving"}]""")
        assertEquals("misbehaving", entries[0].reason)
    }
}
