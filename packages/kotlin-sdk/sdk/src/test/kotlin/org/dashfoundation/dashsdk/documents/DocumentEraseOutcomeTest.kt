package org.dashfoundation.dashsdk.documents

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test

class DocumentEraseOutcomeTest {
    private val lifecycle = """{"state":"ERASING","remaining_revisions":18446744073709551615,"deleted_at_ms":1,"erasing_started_at_ms":2,"erasing_from_time_ms":3,"erasing_from_revision":4}"""

    @Test
    fun submittedWithUnavailableClosingReadRemainsSubmitted() {
        val outcome = DocumentEraseOutcome.fromJson("""{"kind":"SUBMITTED","reason":null,"owner":null,"before":$lifecycle,"after":null}""")
            as DocumentEraseOutcome.Submitted
        assertEquals(ULong.MAX_VALUE, outcome.before!!.remainingRevisions)
        assertNull(outcome.after)
    }

    @Test
    fun unavailablePreflightIsAnExplicitNotSubmittedReason() {
        val outcome = DocumentEraseOutcome.fromJson("""{"kind":"NOT_SUBMITTED","reason":"PREFLIGHT_UNAVAILABLE","owner":null,"before":null,"after":null}""")
            as DocumentEraseOutcome.NotSubmitted
        assertEquals(DocumentEraseNotSubmittedReason.PREFLIGHT_UNAVAILABLE, outcome.reason)
        assertNull(outcome.before)
    }

    @Test
    fun malformedObservationIsNeverTreatedAsUnavailable() {
        for (json in listOf(
            """{"kind":"SUBMITTED","reason":null,"owner":null,"before":{},"after":null}""",
            """{"kind":"SUBMITTED","reason":null,"owner":null,"before":null}""",
            """{"kind":"UNKNOWN","reason":null,"owner":null,"before":null,"after":null}""",
            """{"kind":"NOT_SUBMITTED","reason":"UNKNOWN","owner":null,"before":null,"after":null}""",
            """{"kind":"NOT_SUBMITTED","reason":"NOT_NEWEST_OWNER","owner":null,"before":null,"after":null}""",
        )) {
            val error = assertThrows(DocumentEraseResultUnreadableException::class.java) {
                DocumentEraseOutcome.fromJson(json)
            }
            assertEquals("Erase result unreadable; read history before retrying.", error.message)
        }
    }

    @Test
    fun submittedStillRejectsUnknownOrMissingTransportFields() {
        for (fields in listOf(
            "\"reason\":\"UNKNOWN\",\"owner\":null",
            "\"reason\":null",
            "\"owner\":null",
        )) {
            assertThrows(DocumentEraseResultUnreadableException::class.java) {
                DocumentEraseOutcome.fromJson("{\"kind\":\"SUBMITTED\",$fields,\"before\":null,\"after\":null}")
            }
        }
    }

    @Test
    fun decoderDoesNotReimplementTheSubmissionDecision() {
        for (reason in DocumentEraseNotSubmittedReason.entries) {
            val outcome = DocumentEraseOutcome.fromJson("""{"kind":"NOT_SUBMITTED","reason":"${reason.name}","owner":"11111111111111111111111111111111","before":$lifecycle,"after":null}""")
                as DocumentEraseOutcome.NotSubmitted
            assertEquals(reason, outcome.reason)
        }
    }
    @Test
    fun nativeLifecycleFixtureDecodesThroughTheSharedParser() {
        val json = requireNotNull(javaClass.getResource("/document_erase_outcome_from_native.json")).readText()
        val result = DocumentEraseOutcome.fromJson(json) as DocumentEraseOutcome.Submitted
        assertEquals(ULong.MAX_VALUE, result.before!!.remainingRevisions)
        assertEquals(ULong.MAX_VALUE, result.before!!.erasingFromRevision)
        assertNull(result.after)
    }

}
