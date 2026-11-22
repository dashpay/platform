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
            """{"averages":{"":{"count":2,"sum":"18446744073709551614","average":"9223372036854775807.0000"}}}""",
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
            """{"averages":{
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
