package org.dashfoundation.example.ui.contracts

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.queries.DocumentLifecycle
import org.dashfoundation.dashsdk.queries.DocumentLifecycleState
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the failure split in [eraseAndDescribe]: the erase decides the
 * submission, the lifecycle reads only describe it.
 *
 * A failed lifecycle observation must not report a broadcast as failed or
 * invite another fee-bearing submission. An ACTIVE or ABSENT observation
 * before the broadcast must instead explain why no submission is needed.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class EraseAndDescribeTest {

    private fun lifecycle(state: DocumentLifecycleState, remaining: Long) = DocumentLifecycle(
        state = state,
        remainingRevisions = remaining.toULong(),
        deletedAtMs = if (state == DocumentLifecycleState.DELETED || state == DocumentLifecycleState.ERASING) {
            1_700_000_000_001uL
        } else {
            0uL
        },
        erasingStartedAtMs = if (state == DocumentLifecycleState.ERASING) 1_700_000_000_002uL else 0uL,
        erasingFromTimeMs = if (state == DocumentLifecycleState.ERASING) 1_700_000_000_000uL else 0uL,
        erasingFromRevision = if (state == DocumentLifecycleState.ERASING) 7uL else 0uL,
    )

    @Test
    fun activeOrAbsentLifecycleSkipsThePaidErase() = runTest {
        for (state in listOf(DocumentLifecycleState.ACTIVE, DocumentLifecycleState.ABSENT)) {
            var erases = 0
            var reads = 0
            val message = eraseAndDescribe(
                readLifecycle = { reads++; lifecycle(state, 0) },
                erase = { erases++ },
            )
            assertEquals("no erase is needed for $state", 0, erases)
            assertEquals("no post-broadcast read is needed", 1, reads)
            assertTrue(message, message.startsWith("Erase not submitted"))
            assertTrue(message, message.contains(state.name))
            if (state == DocumentLifecycleState.ABSENT) {
                assertTrue(message, message.contains("No retained revisions exist"))
                assertFalse(message, message.contains("Erasure complete"))
            }
        }
    }

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
