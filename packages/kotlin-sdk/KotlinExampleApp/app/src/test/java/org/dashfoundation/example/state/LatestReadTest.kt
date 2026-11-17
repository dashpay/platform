package org.dashfoundation.example.state

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class LatestReadTest {

    /**
     * The quorum source indicator is refreshed on SPV progress and after a
     * mode change, on independent coroutines whose reads can finish out of
     * order. An older read that finishes last must not overwrite the newer
     * result, or the indicator would show a source the SDK no longer asks
     * first.
     */
    @Test
    fun anOlderReadThatFinishesLastIsDropped() = runTest {
        val reads = LatestRead()
        val published = mutableListOf<String>()
        val olderResult = CompletableDeferred<String>()

        val older = launch { reads.publishLatest(read = { olderResult.await() }) { published += it } }
        runCurrent()
        reads.publishLatest(read = { "trusted" }) { published += it }
        olderResult.complete("spv")
        older.join()

        assertEquals(listOf("trusted"), published)
    }
}
