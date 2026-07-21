package org.dashfoundation.dashsdk.ffi

import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The API-29-safe GC backstop must honor the `java.lang.ref.Cleaner`
 * contract: exactly-once on explicit clean, and eventually-once when the
 * owner is collected without an explicit close.
 */
class NativeCleanerTest {

    @Test
    fun explicitCleanRunsExactlyOnce() {
        val runs = AtomicInteger(0)
        val cleanable = NativeCleaner.register(Any()) { runs.incrementAndGet() }
        cleanable.clean()
        cleanable.clean()
        assertEquals(1, runs.get())
    }

    @Test
    fun gcBackstopFiresWhenOwnerIsDropped() {
        val runs = AtomicInteger(0)
        var owner: Any? = Any()
        NativeCleaner.register(owner!!) { runs.incrementAndGet() }
        owner = null
        // Lenient GC nudge loop — phantom enqueue timing is VM-dependent.
        val deadline = System.currentTimeMillis() + 10_000
        while (runs.get() == 0 && System.currentTimeMillis() < deadline) {
            System.gc()
            Thread.sleep(50)
        }
        assertTrue("backstop did not fire after owner became unreachable", runs.get() == 1)
    }
}
