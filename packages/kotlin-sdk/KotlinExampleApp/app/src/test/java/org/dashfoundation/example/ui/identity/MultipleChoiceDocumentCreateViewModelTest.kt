package org.dashfoundation.example.ui.identity

import androidx.lifecycle.ViewModelStore
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * [MultipleChoiceDocumentCreateViewModel] holds the two paid writes of the multiple choice
 * proposal screens (creating a proposal, entering a choice) as
 * [CreateDecisionProposalViewModel] holds the yes/no create: the write runs in the
 * ViewModel's own scope, which lives as long as the screen's back-stack entry rather than
 * its composition, is submitted at most once until its outcome was shown, and reports a
 * failure without a cancellation passing for one.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class MultipleChoiceDocumentCreateViewModelTest {

    private val dispatcher = StandardTestDispatcher()

    private val proposalId = "8vLzWy3oPYAwJ9W8TjWZ3vSJMy7bDqyuvbMvzQMxVjPN"
    private val choiceId = "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk"

    @Before
    fun setMain() {
        Dispatchers.setMain(dispatcher)
    }

    @After
    fun resetMain() {
        Dispatchers.resetMain()
    }

    @Test
    fun shouldRecordTheCreatedIdAndNeverSubmitTwice() = runTest(dispatcher) {
        val submission = MultipleChoiceDocumentCreateViewModel()
        val confirmed = CompletableDeferred<String>()
        var creates = 0

        submission.submit {
            creates++
            confirmed.await()
        }
        advanceUntilIdle()
        assertTrue(submission.isSubmitting)

        // A second tap while the first create runs is ignored.
        submission.submit {
            creates++
            "second"
        }
        advanceUntilIdle()
        assertEquals(1, creates)

        confirmed.complete(proposalId)
        advanceUntilIdle()
        assertFalse(submission.isSubmitting)
        assertEquals(proposalId, submission.createdId)
        assertNull(submission.error)

        // So is one after the create succeeded.
        submission.submit {
            creates++
            "third"
        }
        advanceUntilIdle()
        assertEquals(1, creates)
        assertEquals(proposalId, submission.createdId)
    }

    @Test
    fun shouldRecordAFailureAndAllowAnotherSubmit() = runTest(dispatcher) {
        val submission = MultipleChoiceDocumentCreateViewModel()

        submission.submit { throw IllegalStateException("Platform refused the proposal") }
        advanceUntilIdle()
        assertFalse(submission.isSubmitting)
        assertEquals("Platform refused the proposal", submission.error)
        assertNull(submission.createdId)

        submission.submit { proposalId }
        advanceUntilIdle()
        assertNull(submission.error)
        assertEquals(proposalId, submission.createdId)
    }

    /**
     * The detail screen enters one choice after another: once it showed a confirmed choice
     * it acknowledges it, and only then is another one submitted.
     */
    @Test
    fun shouldAllowAnotherChoiceOnlyOnceTheConfirmedOneWasAcknowledged() = runTest(dispatcher) {
        val entry = MultipleChoiceDocumentCreateViewModel()
        var entries = 0

        entry.submit {
            entries++
            choiceId
        }
        advanceUntilIdle()
        assertEquals(choiceId, entry.createdId)

        entry.acknowledge()
        assertNull(entry.createdId)

        entry.submit {
            entries++
            proposalId
        }
        advanceUntilIdle()
        assertEquals(2, entries)
        assertEquals(proposalId, entry.createdId)
    }

    /** Acknowledging while a write runs neither drops its outcome nor lets a second one in. */
    @Test
    fun shouldNotAcknowledgeWhileAWriteRuns() = runTest(dispatcher) {
        val entry = MultipleChoiceDocumentCreateViewModel()
        val confirmed = CompletableDeferred<String>()
        var entries = 0

        entry.submit {
            entries++
            confirmed.await()
        }
        advanceUntilIdle()
        entry.acknowledge()
        entry.submit {
            entries++
            "second"
        }
        advanceUntilIdle()
        assertTrue(entry.isSubmitting)
        assertEquals(1, entries)

        confirmed.complete(choiceId)
        advanceUntilIdle()
        assertEquals(choiceId, entry.createdId)
    }

    /**
     * Only clearing the back-stack entry's store stops a running write, and the
     * cancellation is not reported as a failure.
     */
    @Test
    fun shouldStopOnlyWhenTheEntryIsClearedWithoutReportingAFailure() = runTest(dispatcher) {
        val store = ViewModelStore()
        val submission = MultipleChoiceDocumentCreateViewModel()
        store.put("createMultipleChoiceProposal", submission)
        val confirmed = CompletableDeferred<String>()

        submission.submit { confirmed.await() }
        advanceUntilIdle()
        assertTrue(submission.isSubmitting)

        store.clear()
        advanceUntilIdle()
        assertFalse(submission.isSubmitting)
        assertNull(submission.error)
        assertNull(submission.createdId)
    }
}
