package org.dashfoundation.example.ui.contracts

import org.dashfoundation.dashsdk.documents.DocumentEraseOutcome
import org.dashfoundation.dashsdk.documents.DocumentEraseNotSubmittedReason
import org.dashfoundation.dashsdk.queries.DocumentLifecycle
import org.dashfoundation.dashsdk.queries.DocumentLifecycleState
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** Presentation tests consume Rust decisions; they never simulate submission policy. */
class EraseAndDescribeTest {
    @Test
    fun unavailablePreflightDoesNotClaimSubmission() {
        val message = describeEraseOutcome(DocumentEraseOutcome.NotSubmitted(
            DocumentEraseNotSubmittedReason.PREFLIGHT_UNAVAILABLE, null, null,
        ))
        assertTrue(message, message.startsWith("Erase not submitted"))
        assertTrue(message, message.contains("read history before retrying"))
    }

    @Test
    fun closingReadFailureDoesNotReportSubmissionFailure() {
        val message = describeEraseOutcome(DocumentEraseOutcome.Submitted(null, null))
        assertTrue(message, message.startsWith("Erase submitted"))
        assertTrue(message, message.contains("Lifecycle not read"))
    }

    @Test
    fun absentSkipIsNotReportedAsCompletedErasure() {
        val message = describeEraseOutcome(DocumentEraseOutcome.NotSubmitted(
            DocumentEraseNotSubmittedReason.ABSENT, null, null,
        ))
        assertTrue(message, message.contains("No retained revisions exist"))
        assertFalse(message, message.contains("Erasure complete"))
    }

    @Test
    fun ownerRefusalNamesTheIdentityRustReported() {
        val message = describeEraseOutcome(DocumentEraseOutcome.NotSubmitted(
            DocumentEraseNotSubmittedReason.NOT_NEWEST_OWNER, "owner", null,
        ))
        assertTrue(message, message.contains("belongs to owner"))
    }

    @Test
    fun submittedProgressUsesTheClosingObservation() {
        val after = DocumentLifecycle(DocumentLifecycleState.ERASING, 50uL, 1uL, 2uL, 3uL, 4uL)
        val message = describeEraseOutcome(DocumentEraseOutcome.Submitted(null, after))
        assertTrue(message, message.contains("Erasure in progress: 50"))
    }
}
