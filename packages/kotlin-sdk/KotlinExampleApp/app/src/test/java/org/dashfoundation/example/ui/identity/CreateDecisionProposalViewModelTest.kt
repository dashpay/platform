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
 * Pins the fix for the "a paid create is cancelled when the screen goes away" review
 * finding: [CreateDecisionProposalViewModel] runs the create in its own scope, which lives
 * as long as the screen's back-stack entry rather than its composition, submits it at most
 * once, and reports a failure without a cancellation passing for one.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class CreateDecisionProposalViewModelTest {

    private val dispatcher = StandardTestDispatcher()

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
        val submission = CreateDecisionProposalViewModel()
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

        confirmed.complete("8vLzWy3oPYAwJ9W8TjWZ3vSJMy7bDqyuvbMvzQMxVjPN")
        advanceUntilIdle()
        assertFalse(submission.isSubmitting)
        assertEquals("8vLzWy3oPYAwJ9W8TjWZ3vSJMy7bDqyuvbMvzQMxVjPN", submission.createdProposalId)
        assertNull(submission.error)

        // So is one after the create succeeded.
        submission.submit {
            creates++
            "third"
        }
        advanceUntilIdle()
        assertEquals(1, creates)
        assertEquals("8vLzWy3oPYAwJ9W8TjWZ3vSJMy7bDqyuvbMvzQMxVjPN", submission.createdProposalId)
    }

    @Test
    fun shouldRecordAFailureAndAllowAnotherSubmit() = runTest(dispatcher) {
        val submission = CreateDecisionProposalViewModel()

        submission.submit { throw IllegalStateException("Platform refused the end date") }
        advanceUntilIdle()
        assertFalse(submission.isSubmitting)
        assertEquals("Platform refused the end date", submission.error)
        assertNull(submission.createdProposalId)

        submission.submit { "8vLzWy3oPYAwJ9W8TjWZ3vSJMy7bDqyuvbMvzQMxVjPN" }
        advanceUntilIdle()
        assertNull(submission.error)
        assertEquals("8vLzWy3oPYAwJ9W8TjWZ3vSJMy7bDqyuvbMvzQMxVjPN", submission.createdProposalId)
    }

    /**
     * Only clearing the back-stack entry's store stops a running create, and the
     * cancellation is not reported as a failure.
     */
    @Test
    fun shouldStopOnlyWhenTheEntryIsClearedWithoutReportingAFailure() = runTest(dispatcher) {
        val store = ViewModelStore()
        val submission = CreateDecisionProposalViewModel()
        store.put("createDecisionProposal", submission)
        val confirmed = CompletableDeferred<String>()

        submission.submit { confirmed.await() }
        advanceUntilIdle()
        assertTrue(submission.isSubmitting)

        store.clear()
        advanceUntilIdle()
        assertFalse(submission.isSubmitting)
        assertNull(submission.error)
        assertNull(submission.createdProposalId)
    }
}
