package org.dashfoundation.dashsdk.ffi

import java.lang.ref.PhantomReference
import java.lang.ref.ReferenceQueue
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.concurrent.thread

/**
 * GC backstop for native handles — the role `java.lang.ref.Cleaner` plays
 * on iOS-parity handle owners, reimplemented on [PhantomReference] +
 * [ReferenceQueue] because `java.lang.ref.Cleaner` only exists on
 * Android 13 (API 33) while this SDK supports minSdk 29, and core-library
 * desugaring does not cover `java.lang.ref`.
 *
 * Contract matches `Cleaner.register`: the action runs exactly once, on
 * whichever comes first of explicit [Cleanable.clean] (the owner's
 * `close()`) or the owner becoming phantom-reachable. Actions must not
 * reference the owner (they'd never fire) — handle owners pass a
 * standalone `HandleCleanup(handleRef)`.
 */
internal object NativeCleaner {

    /** Mirror of `java.lang.ref.Cleaner.Cleanable`. */
    internal fun interface Cleanable {
        fun clean()
    }

    private val queue = ReferenceQueue<Any>()

    // Keeps each registration's PhantomReference strongly reachable until
    // it is cleaned (explicitly or via GC) — a collected Ref never fires.
    private val pending = ConcurrentHashMap<Ref, Unit>()

    private class Ref(
        referent: Any,
        queue: ReferenceQueue<Any>,
        private val action: Runnable,
    ) : PhantomReference<Any>(referent, queue) {
        private val ran = AtomicBoolean(false)

        fun runOnce() {
            if (ran.compareAndSet(false, true)) {
                runCatching { action.run() }
            }
        }
    }

    init {
        thread(name = "dash-native-cleaner", isDaemon = true) {
            while (true) {
                runCatching {
                    val ref = queue.remove() as Ref
                    pending.remove(ref)
                    ref.runOnce()
                    ref.clear()
                }
            }
        }
    }

    fun register(owner: Any, action: Runnable): Cleanable {
        val ref = Ref(owner, queue, action)
        pending[ref] = Unit
        return Cleanable {
            pending.remove(ref)
            ref.runOnce()
            ref.clear()
        }
    }
}
