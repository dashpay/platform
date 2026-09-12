package org.dashfoundation.example.ui.contracts

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.queries.DocumentLifecycle
import org.dashfoundation.dashsdk.queries.DocumentLifecycleState
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the failure split in [eraseAndDescribe]: the erase decides the
 * submission, the lifecycle reads only describe it.
 *
 * The hazard: the lifecycle read runs after the erase has been broadcast and
 * confirmed. A transient DAPI or JNI failure there used to escape the submit
 * helper's failure callback, so the UI showed a confirmed erase as failed and
 * invited the user to submit a second, fee-bearing one.
 *
 * Red→green: drop the `try`/`catch` around the reads in [eraseAndDescribe] and
 * [lifecycleReadFailureAfterBroadcast_stillReportsSuccess] fails — the read's
 * exception propagates out instead of producing a success message.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class EraseAndDescribeTest {

    private fun lifecycle(state: DocumentLifecycleState, remaining: Long) = DocumentLifecycle(
        state = state,
        remainingRevisions = remaining,
        deletedAtMs = 1_700_000_000_001,
        erasingStartedAtMs = if (state == DocumentLifecycleState.ERASING) 1_700_000_000_002 else 0,
        erasingFromTimeMs = 1_700_000_000_000,
        erasingFromRevision = 7,
    )

    @Test
    fun lifecycleReadFailureAfterBroadcast_stillReportsSuccess() = runTest {
        var eraseCount = 0
        var reads = 0

        val message = eraseAndDescribe(
            readLifecycle = {
                reads++
                if (reads == 1) {
                    lifecycle(DocumentLifecycleState.DELETED, 150)
                } else {
                    throw IllegalStateException("DAPI timeout") // after the broadcast
                }
            },
            erase = { eraseCount++ },
        )

        assertEquals("the erase must have been submitted exactly once", 1, eraseCount)
        assertTrue(message, message.startsWith("Erase submitted"))
        assertTrue(message, message.contains("Lifecycle not read"))
    }

    @Test
    fun lifecycleReadFailureBeforeBroadcast_stillSubmitsAndDescribes() = runTest {
        var eraseCount = 0
        var reads = 0

        val message = eraseAndDescribe(
            readLifecycle = {
                reads++
                if (reads == 1) throw IllegalStateException("DAPI timeout")
                lifecycle(DocumentLifecycleState.ERASING, 50)
            },
            erase = { eraseCount++ },
        )

        assertEquals(1, eraseCount)
        assertTrue(message, message.contains("Erasure in progress: 50"))
    }

    @Test
    fun eraseFailureIsNotDressedUpAsASubmission() = runTest {
        var reads = 0
        val thrown = runCatching {
            eraseAndDescribe(
                readLifecycle = { reads++; lifecycle(DocumentLifecycleState.DELETED, 150) },
                erase = { throw IllegalStateException("broadcast rejected") },
            )
        }.exceptionOrNull()

        assertTrue("the erase's own failure must reach the caller", thrown is IllegalStateException)
        assertEquals("the closing read must not run after a failed erase", 1, reads)
    }

    @Test
    fun cancellationIsNotSwallowedByTheReadGuard() = runTest {
        var eraseCount = 0
        val thrown = runCatching {
            eraseAndDescribe(
                readLifecycle = { throw CancellationException("screen disposed") },
                erase = { eraseCount++ },
            )
        }.exceptionOrNull()

        assertTrue(
            "cancellation must propagate so structured concurrency is intact",
            thrown is CancellationException,
        )
        assertEquals("a cancelled read must not go on to submit the erase", 0, eraseCount)
    }

    @Test
    fun completedErasureIsDescribedFromTheLifecycleReads() = runTest {
        var reads = 0
        val message = eraseAndDescribe(
            readLifecycle = {
                reads++
                if (reads == 1) {
                    lifecycle(DocumentLifecycleState.DELETED, 100)
                } else {
                    lifecycle(DocumentLifecycleState.ABSENT, 0)
                }
            },
            erase = {},
        )

        assertTrue(message, message.contains("Erasure complete"))
    }
}
