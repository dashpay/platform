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

    private fun registerBlob(token: Long, fee: Long, txid: String, txBytes: ByteArray): ByteArray {
        val txidBytes = txid.toByteArray(Charsets.UTF_8)
        val buf = ByteBuffer.allocate(8 + 8 + 4 + txidBytes.size + 4 + txBytes.size)
        buf.putLong(token)
        buf.putLong(fee)
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
    // A DRAIN (SelectionStrategy.ALL) has the ENGINE compute the deliverable
    // amount (total inputs − fee, no change), so the caller never supplied it.
    // A swap deposit must quote from this exact figure and then broadcast this
    // same transaction, so the two cannot disagree. These pin the parse against
    // hand-built consensus bytes.

    /** Little-endian compact size, matching the parser. */
    private fun varInt(value: Long): ByteArray = when {
        value < 0xfd -> byteArrayOf(value.toByte())
        value <= 0xffff -> ByteBuffer.allocate(3).order(java.nio.ByteOrder.LITTLE_ENDIAN)
            .put(0xfd.toByte()).putShort(value.toShort()).array()
        else -> ByteBuffer.allocate(5).order(java.nio.ByteOrder.LITTLE_ENDIAN)
            .put(0xfe.toByte()).putInt(value.toInt()).array()
    }

    /** One input (32B txid + vout + empty scriptSig + sequence) and [outputs]. */
    private fun tx(outputs: List<Pair<Long, ByteArray>>, inputs: Int = 1): ByteArray {
        val out = java.io.ByteArrayOutputStream()
        out.write(ByteBuffer.allocate(4).order(java.nio.ByteOrder.LITTLE_ENDIAN).putInt(2).array())
        out.write(varInt(inputs.toLong()))
        repeat(inputs) {
            out.write(ByteArray(32) { 0x11 })
            out.write(ByteBuffer.allocate(4).order(java.nio.ByteOrder.LITTLE_ENDIAN).putInt(0).array())
            out.write(varInt(0))
            out.write(ByteBuffer.allocate(4).order(java.nio.ByteOrder.LITTLE_ENDIAN).putInt(-1).array())
        }
        out.write(varInt(outputs.size.toLong()))
        for ((value, script) in outputs) {
            out.write(ByteBuffer.allocate(8).order(java.nio.ByteOrder.LITTLE_ENDIAN).putLong(value).array())
            out.write(varInt(script.size.toLong()))
            out.write(script)
        }
        out.write(ByteBuffer.allocate(4).order(java.nio.ByteOrder.LITTLE_ENDIAN).putInt(0).array())
        return out.toByteArray()
    }

    private fun p2pkh(): ByteArray = byteArrayOf(0x76, 0xa9.toByte(), 0x14) + ByteArray(20) +
        byteArrayOf(0x88.toByte(), 0xac.toByte())

    private fun opReturn(payload: ByteArray): ByteArray =
        byteArrayOf(0x6a, payload.size.toByte()) + payload

    private fun signedWith(txBytes: ByteArray) =
        ManagedPlatformWallet.SignedCoreTransaction.fromRegisterBlob(
            registerBlob(token = 1L, fee = 226L, txid = "aa", txBytes = txBytes)
        )

    @Test
    fun deliverableAmountReadsTheVaultOutputBesideAMemo() {
        // The MAYACHAIN drain shape: vault = VOUT0, zero-value memo = VOUT1.
        val signed = signedWith(
            tx(listOf(1_790_000L to p2pkh(), 0L to opReturn("=:MAYA.CACAO:addr".toByteArray())))
        )
        assertEquals(1_790_000L, signed.deliverableAmountDuffs)
    }

    @Test
    fun deliverableAmountIgnoresOutputOrder() {
        // preserveOutputOrder is optional, so the value carrier is not always
        // VOUT0 — the parser must walk the outputs, not index into them.
        val signed = signedWith(
            tx(listOf(0L to opReturn(byteArrayOf(1, 2, 3)), 42_000L to p2pkh()))
        )
        assertEquals(42_000L, signed.deliverableAmountDuffs)
    }

    @Test
    fun deliverableAmountSurvivesMultipleInputsAndLongScripts() {
        // A drain selects EVERY spendable UTXO, so many inputs is the norm; the
        // input walk must skip variable-length scriptSigs correctly.
        val out = java.io.ByteArrayOutputStream()
        out.write(ByteBuffer.allocate(4).order(java.nio.ByteOrder.LITTLE_ENDIAN).putInt(2).array())
        out.write(varInt(3))
        repeat(3) {
            out.write(ByteArray(32) { 0x22 })
            out.write(ByteBuffer.allocate(4).order(java.nio.ByteOrder.LITTLE_ENDIAN).putInt(1).array())
            val scriptSig = ByteArray(107) { 0x33 } // realistic P2PKH signature script
            out.write(varInt(scriptSig.size.toLong()))
            out.write(scriptSig)
            out.write(ByteBuffer.allocate(4).order(java.nio.ByteOrder.LITTLE_ENDIAN).putInt(-1).array())
        }
        out.write(varInt(1))
        out.write(ByteBuffer.allocate(8).order(java.nio.ByteOrder.LITTLE_ENDIAN).putLong(999_777L).array())
        out.write(varInt(p2pkh().size.toLong()))
        out.write(p2pkh())
        out.write(ByteBuffer.allocate(4).order(java.nio.ByteOrder.LITTLE_ENDIAN).putInt(0).array())

        assertEquals(999_777L, signedWith(out.toByteArray()).deliverableAmountDuffs)
    }

    @Test(expected = IllegalStateException::class)
    fun deliverableAmountRefusesTwoSpendableOutputs() {
        // No single "deliverable" amount exists for a multi-recipient payment,
        // and a drain never builds one — refuse rather than pick arbitrarily.
        signedWith(tx(listOf(10L to p2pkh(), 20L to p2pkh()))).deliverableAmountDuffs
    }

    @Test(expected = IllegalStateException::class)
    fun deliverableAmountRefusesAnOpReturnOnlyTransaction() {
        signedWith(tx(listOf(0L to opReturn(byteArrayOf(9))))).deliverableAmountDuffs
    }

    @Test(expected = IllegalStateException::class)
    fun deliverableAmountRefusesMalformedBytes() {
        signedWith(byteArrayOf(1, 2, 3)).deliverableAmountDuffs
    }
}
