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
        assertEquals(37L, lifecycle.remainingRevisions)
        assertEquals(1700000000001L, lifecycle.deletedAtMs)
        assertEquals(1700000000002L, lifecycle.erasingStartedAtMs)
        assertEquals(1700000000000L, lifecycle.erasingFromTimeMs)
        assertEquals(7L, lifecycle.erasingFromRevision)
    }

    @Test
    fun partialOrUnknownLifecycleIsNullNotZeros() {
        assertNull(DocumentLifecycle.fromHistoryJson("""{"entries":[]}"""))
        assertNull(DocumentLifecycle.fromHistoryJson("""{"lifecycle":{"state":"DELETED"}}"""))
        assertNull(DocumentLifecycle.fromHistoryJson("""{"lifecycle":{"state":"GONE","remaining_revisions":0,"deleted_at_ms":0,"erasing_started_at_ms":0,"erasing_from_time_ms":0,"erasing_from_revision":0}}"""))
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
