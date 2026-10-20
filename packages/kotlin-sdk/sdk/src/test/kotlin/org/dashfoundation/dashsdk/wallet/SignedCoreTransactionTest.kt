package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.ffi.NativeCleaner
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Test
import java.nio.ByteBuffer
import java.util.concurrent.atomic.AtomicInteger

/**
 * Ownership contract for the deferred-payment token (blocker: "Kotlin
 * cancellation can orphan a token").
 *
 * These are pure-JVM tests — they never call the native release itself (that
 * needs the loaded cdylib on the emulator harness). They pin the two properties
 * the fix rests on: [ManagedPlatformWallet.SignedCoreTransaction] is an owning
 * [AutoCloseable], and the [NativeCleaner] backstop it registers runs its
 * release action exactly once (on the first clean / GC and never again), so a
 * token abandoned by a dropped object or an observed cancellation is released,
 * and a token already consumed by broadcast/release is not double-released.
 */
class SignedCoreTransactionTest {

    private fun registerBlob(
        token: Long,
        fee: Long,
        txid: String,
        txBytes: ByteArray,
        deliverable: Long = 0,
    ): ByteArray {
        val txidBytes = txid.toByteArray(Charsets.UTF_8)
        val buf = ByteBuffer.allocate(8 + 8 + 8 + 4 + txidBytes.size + 4 + txBytes.size)
        buf.putLong(token)
        buf.putLong(fee)
        buf.putLong(deliverable)
        buf.putInt(txidBytes.size)
        buf.put(txidBytes)
        buf.putInt(txBytes.size)
        buf.put(txBytes)
        return buf.array()
    }

    @Test
    fun fromRegisterBlobDecodesFieldsAndIsAnOwningCloseable() {
        val txBytes = byteArrayOf(1, 2, 3, 4, 5)
        val blob = registerBlob(token = 42L, fee = 7L, txid = "abcd", txBytes = txBytes)

        val signed = ManagedPlatformWallet.SignedCoreTransaction.fromRegisterBlob(blob)

        assertEquals(42L, signed.reservationToken)
        assertEquals(7L, signed.feeDuffs)
        assertEquals("abcd", signed.txidHex)
        assertArrayEquals(txBytes, signed.rawTxBytes)

        // Compile-time proof that the token is owned by a closeable: a dropped
        // object can be reclaimed via close() / GC rather than leaking the token.
        @Suppress("UNUSED_VARIABLE")
        val asCloseable: AutoCloseable = signed

        // Disarm the cleaner backstop before `signed` becomes unreachable: this
        // is a pure-JVM test with no cdylib loaded, so the registered native
        // release must never fire from the cleaner thread. (NativeCleaner already
        // contains the resulting UnsatisfiedLinkError in `runCatching`, so this is
        // determinism/hygiene rather than a crash fix.)
        signed.close()
    }

    @Test
    fun cleanerBackstopRunsTheReleaseActionExactlyOnce() {
        // The GC/close backstop SignedCoreTransaction relies on: the release
        // action runs once on the first clean() and never again — so releasing a
        // token that was already broadcast/consumed (or closing twice) cannot
        // fire a second native release.
        val runs = AtomicInteger(0)
        val owner = Any()
        val cleanable = NativeCleaner.register(owner) { runs.incrementAndGet() }

        cleanable.clean()
        cleanable.clean()

        assertEquals(1, runs.get())
    }

    // --- deliverableAmountDuffs -------------------------------------------
    //
    // Carried in the registration blob, computed Rust-side from the REGISTERED
    // transaction. It must NOT be re-derived from rawTxBytes: those are a
    // mutable copy the host owns, while the broadcast sends the registered
    // transaction referenced by the token.

    @Test
    fun deliverableAmountComesFromTheBlobNotTheBytes() {
        val signed = ManagedPlatformWallet.SignedCoreTransaction.fromRegisterBlob(
            registerBlob(token = 7L, fee = 432L, txid = "ab", txBytes = byteArrayOf(9, 9, 9),
                deliverable = 27_442_985L)
        )
        assertEquals(27_442_985L, signed.deliverableAmountDuffs)
    }

    @Test
    fun mutatingRawBytesCannotChangeTheDeliverableAmount() {
        // The guarantee the drain quote rests on: what was quoted is what the
        // registered transaction pays, whatever happens to the host's copy.
        val signed = ManagedPlatformWallet.SignedCoreTransaction.fromRegisterBlob(
            registerBlob(token = 1L, fee = 1L, txid = "cd", txBytes = byteArrayOf(1, 2, 3, 4),
                deliverable = 500_000L)
        )
        signed.rawTxBytes.fill(0xFF.toByte())
        assertEquals(500_000L, signed.deliverableAmountDuffs)
    }

    @Test
    fun deliverableAmountIsZeroWhenTheEngineReportsNoSingleDestination() {
        // Multi-recipient or OP_RETURN-only builds have no single deliverable
        // output; Rust reports 0 and the host reads that as "not applicable".
        val signed = ManagedPlatformWallet.SignedCoreTransaction.fromRegisterBlob(
            registerBlob(token = 2L, fee = 10L, txid = "ef", txBytes = ByteArray(0))
        )
        assertEquals(0L, signed.deliverableAmountDuffs)
    }
}
