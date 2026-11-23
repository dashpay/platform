package org.dashfoundation.example.ui.contracts

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.fail
import org.junit.Test

/**
 * Pins [parseAverageResult] against the `dash_sdk_document_average` JSON:
 * the sum is a decimal string (from protocol version 15 it can pass 64
 * bits) and the average is the one Rust computed. Port of the Swift
 * `DocumentAggregateResultParsingTests`.
 */
class AverageResultParsingTest {
    @Test
    fun shouldShowAnUngroupedAverageWhoseSumPasses64Bits() {
        val result = parseAverageResult(
            """{"grouped":false,"averages":{"":{"count":2,"sum":"18446744073709551614","average":"9223372036854775807.0000"}}}""",
        )!!
        val total = result.total!!
        assertEquals("2", total.count)
        assertEquals("18446744073709551614", total.sum)
        assertEquals("9223372036854775807.0000", total.displayAverage())
        assertEquals(emptyMap<String, AverageEntry>(), result.groups)
    }

    @Test
    fun shouldKeepEveryGroupWhenOneSumPasses64Bits() {
        val result = parseAverageResult(
            """{"grouped":true,"averages":{
                "61":{"count":3,"sum":"30","average":"10.0000"},
                "62":{"count":3,"sum":"27670116110564327421","average":"9223372036854775807.0000"}
            }}""",
        )!!
        assertNull(result.total)
        assertEquals(setOf("61", "62"), result.groups.keys)
        assertEquals("10.0000", result.groups.getValue("61").displayAverage())
        assertEquals("27670116110564327421", result.groups.getValue("62").sum)
        assertEquals("9223372036854775807.0000", result.groups.getValue("62").displayAverage())
    }

    /**
     * Grouped by an optional range field, documents missing the value form
     * the `""` group. It is a group like any other, never the total.
     */
    @Test
    fun shouldKeepAnEmptyKeyGroupOfAGroupedAverageAmongTheGroups() {
        val result = parseAverageResult(
            """{"grouped":true,"averages":{
                "":{"count":1,"sum":"10","average":"10.0000"},
                "61":{"count":2,"sum":"18446744073709551614","average":"9223372036854775807.0000"}
            }}""",
        )!!
        assertNull(result.total)
        assertEquals(setOf("", "61"), result.groups.keys)
        assertEquals("10.0000", result.groups.getValue("").displayAverage())
    }

    @Test
    fun shouldKeepAnEmptyKeyGroupOfAGroupedSumAmongTheGroups() {
        val grouped = parseAggregateResult("""{"grouped":true,"sums":{"":"10","61":"18446744073709551614"}}""")!!
        assertNull(grouped.total)
        assertEquals(mapOf("" to "10", "61" to "18446744073709551614"), grouped.groups)

        val total = parseAggregateResult("""{"grouped":false,"sums":{"":"27670116110564327421"}}""")!!
        assertEquals("27670116110564327421", total.total)
        assertEquals(emptyMap<String, String>(), total.groups)
    }

    @Test
    fun shouldRefuseAGroupedFlagThatIsNotABoolean() {
        for (response in listOf(
            """{"grouped":"yes","sums":{"":"10"}}""",
            """{"grouped":1,"averages":{"":{"count":1,"sum":"10","average":"10.0000"}}}""",
        )) {
            try {
                if (response.contains("\"sums\"")) parseAggregateResult(response) else parseAverageResult(response)
                fail("expected $response to be refused")
            } catch (expected: IllegalArgumentException) {
                // Reported by the screen.
            }
        }
    }

    @Test
    fun shouldShowNoAverageWhenNothingMatched() {
        val result = parseAverageResult(
            """{"averages":{"":{"count":0,"sum":"0","average":null}}}""",
        )!!
        assertEquals("—", result.total!!.displayAverage())
    }

    @Test
    fun shouldRefuseAnEntryItCannotRead() {
        for (response in listOf(
            // A numeric sum is the shape before sums became strings.
            """{"averages":{"61":{"count":3,"sum":30,"average":"10.0000"}}}""",
            """{"averages":{"61":{"count":3,"sum":"x","average":"10.0000"}}}""",
            """{"averages":{"61":{"sum":"30","average":"10.0000"}}}""",
            """{"averages":{"61":{"count":3,"sum":"30","average":10}}}""",
            // Rust always writes `average`; only an explicit null means nothing matched.
            """{"averages":{"61":{"count":3,"sum":"30"}}}""",
            """{"averages":{"61":5}}""",
        )) {
            try {
                parseAverageResult(response)
                fail("expected $response to be refused")
            } catch (expected: IllegalArgumentException) {
                // The screen reports the message instead of dropping the row.
            }
        }
    }
}
