package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext

/**
 * Suspend-friendly teardown fence for operations that borrow a manager's
 * raw native handles (`signerHandle` / `mnemonicResolverHandle`).
 *
 * Those handles are freed by `PlatformWalletManager.closeSuspending()`
 * with an unconditional, non-refcounted `Box::from_raw` on the Rust side
 * (`dash_sdk_signer_destroy` and the resolver equivalent), so an
 * operation still holding one across a blocking JNI call when the owning
 * manager is torn down would dereference freed memory. Callers run on
 * their OWN coroutine scopes (`withContext(Dispatchers.IO)`), so the
 * manager-scope join in teardown cannot cover them — this gate does:
 *
 * - [withOp] brackets one borrowing operation. It rejects new operations
 *   once teardown has begun (the manager is being replaced; the op is
 *   doomed anyway) and runs the body on [Dispatchers.IO], replacing the
 *   facades' previous `withContext(Dispatchers.IO)` wrapping.
 * - [closeAndAwait] flips the gate closed and suspends until every
 *   in-flight operation has returned — after which the native boxes are
 *   safe to free.
 *
 * The Swift host doesn't need this: its manager children are freed via
 * ARC after the last strong reference (including any in-flight call's)
 * drops. Kotlin hands out raw `Long`s, so the lifetime must be fenced
 * explicitly.
 */
class TeardownGate {
    private val lock = Mutex()
    private var closing = false
    private var active = 0
    private var idle: CompletableDeferred<Unit>? = null

    /**
     * Run one borrowing operation on [Dispatchers.IO], counted against
     * teardown. Throws [IllegalStateException] once [closeAndAwait] has
     * begun — matching the "closed wrapper fails fast" convention of
     * `ManagedPlatformWallet.handle`.
     */
    suspend fun <T> withOp(block: suspend () -> T): T {
        lock.withLock {
            check(!closing) { "PlatformWalletManager is closed" }
            active++
        }
        try {
            return withContext(Dispatchers.IO) { block() }
        } finally {
            // The decrement MUST run even when the operation's coroutine is
            // cancelled: Mutex.lock is a cancellable suspension point when
            // contended, and a skipped decrement over-counts `active`
            // forever — closeAndAwait() would then hang teardown.
            withContext(NonCancellable) {
                lock.withLock {
                    active--
                    if (active == 0) idle?.complete(Unit)
                }
            }
        }
    }

    /**
     * Reject new operations and suspend until all in-flight ones finish.
     * Idempotent; safe to call from any dispatcher (suspends, never
     * blocks a thread).
     */
    suspend fun closeAndAwait() {
        val wait = lock.withLock {
            closing = true
            if (active == 0) {
                null
            } else {
                idle ?: CompletableDeferred<Unit>().also { idle = it }
            }
        }
        wait?.await()
    }
}

/**
 * Gate-or-fallback bracket for SDK facades: facades constructed by a
 * manager (or by a wallet the manager built) carry its gate; facades
 * constructed bare — unit tests, ad-hoc use — fall back to plain
 * [Dispatchers.IO] with no fence, preserving their old behavior.
 */
internal suspend fun <T> TeardownGate?.op(block: suspend () -> T): T =
    if (this != null) withOp(block) else withContext(Dispatchers.IO) { block() }

/**
 * Run [block] like [op], but invoke [cleanup] if a completed result is discarded
 * by coroutine cancellation while [withContext] dispatches back to its caller.
 *
 * This matters for blocking JNI calls that return private material: they cannot
 * observe cancellation while native code is running, and `withContext` provides
 * prompt cancellation when it resumes the original dispatcher. Without this
 * handoff guard, the completed secret-bearing result can be dropped before the
 * caller ever receives a reference it can scrub.
 */
internal suspend fun <T> TeardownGate?.opWithCleanupOnCancellation(
    cleanup: (T) -> Unit,
    block: suspend () -> T,
): T {
    class CompletedResult<T>(val value: T)

    var completed: CompletedResult<T>? = null
    try {
        val value = op {
            block().also { completed = CompletedResult(it) }
        }
        // No suspension point exists between the successful handoff and this
        // clear, so ownership has moved to the caller from here onward.
        completed = null
        return value
    } finally {
        completed?.let { cleanup(it.value) }
    }
}

/**
 * Run [block] like [op], but if the block completed and only the handoff back
 * to a cancelled caller failed, return its result instead of throwing.
 *
 * For blocking JNI calls that move funds. Native code cannot observe coroutine
 * cancellation, so a payment can finish after the caller was cancelled, and
 * `withContext`'s prompt cancellation would then drop the result: the caller
 * could not tell the payment happened and might retry it. Returning the result
 * keeps it visible. Swift behaves the same way: its credit-movement calls run in
 * `Task.detached`, whose value comes back even when the caller is cancelled.
 *
 * Cancellation before [block] completes (gate entry, dispatch, a native error)
 * still throws. The coroutine stays cancelled, so its next suspension point
 * throws: the caller must handle the result before suspending again.
 */
internal suspend fun <T> TeardownGate?.opReturningCompletedResult(
    block: suspend () -> T,
): T {
    class CompletedResult<T>(val value: T)

    var completed: CompletedResult<T>? = null
    try {
        return op {
            block().also { completed = CompletedResult(it) }
        }
    } catch (e: CancellationException) {
        completed?.let { return it.value }
        throw e
    }
}
