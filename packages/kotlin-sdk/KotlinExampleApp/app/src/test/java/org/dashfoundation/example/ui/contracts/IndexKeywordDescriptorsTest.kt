package org.dashfoundation.example.ui.contracts

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Pins the protocol-v14 index keyword rendering helpers
 * ([indexAxisDescriptors], [indexTerminal]) the DocumentTypeDetailsScreen
 * index rows use: the boolean-or-string `countable` spellings, the
 * `averageable` / `rangeAverageable` sugar desugar (which must match DPP's),
 * and the indexOnly `$ownerId` terminal default.
 */
class IndexKeywordDescriptorsTest {

    @Test
    fun preV14IndexHasNoDescriptors() {
        val index = buildJsonObject {
            put("name", "byOwner")
            put("unique", true)
        }
        assertEquals(emptyList<String>(), indexAxisDescriptors(index))
    }

    @Test
    fun countableParsesBooleanAndStringSpellings() {
        assertEquals(
            listOf("Countable"),
            indexAxisDescriptors(buildJsonObject { put("countable", true) }),
        )
        assertEquals(
            listOf("Countable"),
            indexAxisDescriptors(buildJsonObject { put("countable", "countable") }),
        )
        assertEquals(
            listOf("Countable (offsets)"),
            indexAxisDescriptors(buildJsonObject { put("countable", "countableAllowingOffset") }),
        )
        assertEquals(
            emptyList<String>(),
            indexAxisDescriptors(buildJsonObject { put("countable", "notCountable") }),
        )
    }

    @Test
    fun fullRankedCountIndexListsEveryAxis() {
        val index = buildJsonObject {
            put("countable", "countable")
            put("rangeCountable", true)
            put("rankedCountable", true)
        }
        assertEquals(
            listOf("Countable", "Range Count", "Ranked by Count"),
            indexAxisDescriptors(index),
        )
    }

    @Test
    fun averageableSugarDesugarsToCountableAndSummable() {
        val index = buildJsonObject {
            put("averageable", "amount")
            put("rangeAverageable", true)
        }
        assertEquals(
            listOf("Countable", "Range Count", "Summable (amount)", "Range Sum"),
            indexAxisDescriptors(index),
        )
    }

    @Test
    fun summableLonghandLists() {
        val index = buildJsonObject {
            put("summable", "amount")
            put("rangeSummable", true)
            put("rankedSummable", true)
        }
        assertEquals(
            listOf("Summable (amount)", "Range Sum", "Ranked by Sum"),
            indexAxisDescriptors(index),
        )
    }

    @Test
    fun terminalDefaultsToOwnerIdOnlyOnIndexOnlyTypes() {
        val bare = JsonObject(emptyMap())
        assertNull(indexTerminal(bare, indexOnly = false))
        assertEquals("\$ownerId", indexTerminal(bare, indexOnly = true))

        val declared = buildJsonObject { put("terminal", "postId") }
        assertEquals("postId", indexTerminal(declared, indexOnly = true))
        assertEquals("postId", indexTerminal(declared, indexOnly = false))
    }
}
