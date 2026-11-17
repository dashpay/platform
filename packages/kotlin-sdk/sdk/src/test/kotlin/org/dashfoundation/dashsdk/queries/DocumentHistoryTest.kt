package org.dashfoundation.dashsdk.queries

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the lifecycle parsing behind `Documents.lifecycle` and the wording
 * `describeEraseProgress` produces for the DocumentActionsScreen erase
 * section. The erase transition's own result only observes that the document
 * is absent from ordinary reads, which it already was before the erase, so a
 * lifecycle that is still DELETED with the same retained count must never be
 * presented as an accepted or completed erase.
 */
class DocumentHistoryTest {

    private fun page(state: String, remaining: Long, erasingStarted: Long = 0) =
        """{"entries":[],"lifecycle":{"state":"$state","remaining_revisions":$remaining,""" +
            """"deleted_at_ms":1700000000001,"erasing_started_at_ms":$erasingStarted,""" +
            """"erasing_from_time_ms":1700000000000,"erasing_from_revision":7}}"""

    @Test
    fun parsesEveryLifecycleFieldExactly() {
        val lifecycle = DocumentLifecycle.fromHistoryJson(page("ERASING", 37, 1700000000002))!!
        assertEquals(DocumentLifecycleState.ERASING, lifecycle.state)
        assertEquals(37uL, lifecycle.remainingRevisions)
        assertEquals(1700000000001uL, lifecycle.deletedAtMs)
        assertEquals(1700000000002uL, lifecycle.erasingStartedAtMs)
        assertEquals(1700000000000uL, lifecycle.erasingFromTimeMs)
        assertEquals(7uL, lifecycle.erasingFromRevision)
    }

    @Test
    fun partialOrUnknownLifecycleIsNullNotZeros() {
        assertNull(DocumentLifecycle.fromHistoryJson("""{"entries":[]}"""))
        assertNull(DocumentLifecycle.fromHistoryJson("""{"lifecycle":{"state":"DELETED"}}"""))
        assertNull(DocumentLifecycle.fromHistoryJson("""{"lifecycle":{"state":"GONE","remaining_revisions":0,"deleted_at_ms":0,"erasing_started_at_ms":0,"erasing_from_time_ms":0,"erasing_from_revision":0}}"""))
    }

    @Test
    fun negativeOrStringTypedNumbersAreMalformedNotValues() {
        // Rust serializes every lifecycle number as an unsigned integer, so a
        // string or a negative number is a malformed block, never a count.
        val stringTyped = page("DELETED", 5).replace("\"remaining_revisions\":5", "\"remaining_revisions\":\"5\"")
        assertNull(DocumentLifecycle.fromHistoryJson(stringTyped))
        assertNull(DocumentLifecycle.fromHistoryJson(page("DELETED", -1)))
        val negativeTime = page("DELETED", 5).replace("\"deleted_at_ms\":1700000000001", "\"deleted_at_ms\":-1")
        assertNull(DocumentLifecycle.fromHistoryJson(negativeTime))
        assertEquals(0uL, DocumentLifecycle.fromHistoryJson(page("DELETED", 0))!!.remainingRevisions)
    }

    @Test
    fun preservesEveryUnsigned64BitLifecycleNumber() {
        for (value in listOf("9007199254740993", "9223372036854775808", "18446744073709551615")) {
            val fields = listOf("remaining_revisions", "deleted_at_ms", "erasing_started_at_ms",
                "erasing_from_time_ms", "erasing_from_revision")
            val numbers = fields.joinToString(",") { "\"$it\":$value" }
            val lifecycle = DocumentLifecycle.fromHistoryJson("""{"lifecycle":{"state":"ERASING",$numbers}}""")!!
            for (actual in listOf(lifecycle.remainingRevisions, lifecycle.deletedAtMs,
                lifecycle.erasingStartedAtMs, lifecycle.erasingFromTimeMs, lifecycle.erasingFromRevision)) {
                assertEquals(value, actual.toString())
            }
        }
    }

    @Test
    fun rejectsMalformedOrOutOfRangeUnsignedNumbers() {
        for (value in listOf("18446744073709551616", "1.5", "true", "false")) {
            assertNull(DocumentLifecycle.fromHistoryJson(page("ERASING", 5)
                .replace("\"remaining_revisions\":5", "\"remaining_revisions\":$value")))
        }
        assertNull(DocumentLifecycle.fromHistoryJson(page("ERASING", 5)
            .replace("\"state\":\"ERASING\"", "\"state\":ERASING")))
    }

    @Test
    fun unchangedDeletedLifecycleIsNotPresentedAsAConfirmedErase() {
        val before = DocumentLifecycle.fromHistoryJson(page("DELETED", 5))!!
        val after = DocumentLifecycle.fromHistoryJson(page("DELETED", 5))!!
        val text = describeEraseProgress(before, after)
        assertTrue(text, text.startsWith("Not established"))
        assertFalse(text, text.contains("accepted", ignoreCase = true))
        assertFalse(text, text.contains("complete", ignoreCase = true))
    }

    @Test
    fun progressAndCompletionComeOnlyFromTheLifecycle() {
        val before = DocumentLifecycle.fromHistoryJson(page("DELETED", 150))!!
        val erasing = DocumentLifecycle.fromHistoryJson(page("ERASING", 50, 1))!!
        assertTrue(describeEraseProgress(before, erasing).startsWith("Erasure in progress: 50"))
        val absent = DocumentLifecycle.fromHistoryJson(page("ABSENT", 0, 1))!!
        assertTrue(describeEraseProgress(erasing, absent).startsWith("Erasure complete"))
    }
}
