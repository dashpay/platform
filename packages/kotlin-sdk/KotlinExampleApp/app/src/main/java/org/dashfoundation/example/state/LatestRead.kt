package org.dashfoundation.example.state

import java.util.concurrent.atomic.AtomicLong

/**
 * Publishes the result of the latest of overlapping reads: a read that
 * finishes after a later one started is dropped. For a value read on
 * independent coroutines whose reads can finish out of order, such as the
 * SDK's quorum source.
 */
internal class LatestRead {
    private val started = AtomicLong()

    /** Run [read] and hand its result to [publish], unless a later read started meanwhile. */
    suspend fun <T> publishLatest(read: suspend () -> T, publish: (T) -> Unit) {
        val generation = started.incrementAndGet()
        val value = read()
        if (started.get() == generation) publish(value)
    }
}
