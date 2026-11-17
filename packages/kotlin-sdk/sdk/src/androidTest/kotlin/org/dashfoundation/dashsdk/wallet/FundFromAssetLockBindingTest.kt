package org.dashfoundation.dashsdk.wallet

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.AssetLockBuiltBridge
import org.dashfoundation.dashsdk.ffi.AssetLockBuiltTrampolineDriver
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.WalletManagerNative
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import java.nio.ByteBuffer
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicReference

/**
 * Binding-level coverage for the four asset-lock funding JNI exports
 * (`walletFundFromAssetLock[External]`, `walletResumeFundFromAssetLock[External]`)
 * — the no-wallet discipline of [CoreWalletSetGapLimitBindingTest]: a
 * symbol or descriptor mismatch surfaces here as `UnsatisfiedLinkError`,
 * and each JNI validation branch is pinned to the message it throws, which
 * doubles as a parameter-order pin (the ints and longs share descriptor
 * slots, so swapped arguments would land in the wrong branch). No network,
 * no wallet, no funds: every call either stops at JNI validation (raw code
 * 1) or crosses into the FFI with the dead handle 0 and fails there (code in
 * the platform-wallet range).
 *
 * Which FFI each scope calls (owned vs external) is pinned Rust-side by
 * `wallet_manager::tests::should_route_each_*_scope_to_its_own_ffi`. The
 * asset-lock-built trampoline, which a dead handle never reaches, is driven
 * directly through the debug-only [AssetLockBuiltTrampolineDriver].
 */
@RunWith(AndroidJUnit4::class)
class FundFromAssetLockBindingTest {

    private val noopBridge = AssetLockBuiltBridge { }

    /** One explicit-amount payee and one remainder, P2PKH. */
    private val validBlob: ByteArray = ByteBuffer.allocate(4 + 2 * 30).apply {
        putInt(2)
        put(0); put(ByteArray(20) { 0x0B }); put(1); putLong(500L)
        put(0); put(ByteArray(20) { 0x0A }); put(0); putLong(0L)
    }.array()

    /** Claims two rows but carries only one. */
    private val truncatedBlob: ByteArray = validBlob.copyOfRange(0, 4 + 30)

    @Before
    fun loadNative() = NativeLoader.ensureLoaded()

    private fun fund(
        external: Boolean,
        amountDuffs: Long = 100_000L,
        accountIndex: Int = 0,
        platformAccountIndex: Int = 0,
        blob: ByteArray = validBlob,
    ): DashSDKException = assertThrows(DashSDKException::class.java) {
        if (external) {
            WalletManagerNative.walletFundFromAssetLockExternal(
                0L, amountDuffs, accountIndex, platformAccountIndex, blob, 0L, 0L, noopBridge,
            )
        } else {
            WalletManagerNative.walletFundFromAssetLock(
                0L, amountDuffs, accountIndex, platformAccountIndex, blob, 0L, 0L,
            )
        }
    }

    private fun resume(
        external: Boolean,
        txid: ByteArray = ByteArray(32),
        vout: Int = 0,
        platformAccountIndex: Int = 0,
        blob: ByteArray = validBlob,
    ): DashSDKException = assertThrows(DashSDKException::class.java) {
        if (external) {
            WalletManagerNative.walletResumeFundFromAssetLockExternal(
                0L, txid, vout, platformAccountIndex, blob, 0L, 0L,
            )
        } else {
            WalletManagerNative.walletResumeFundFromAssetLock(
                0L, txid, vout, platformAccountIndex, blob, 0L, 0L,
            )
        }
    }

    private fun assertJniRejection(e: DashSDKException, branch: String) {
        assertEquals("JNI-side rejection carries raw code 1: ${e.message}", 1, e.code)
        assertTrue(
            "the rejection must name the $branch branch, got: ${e.message}",
            e.message.orEmpty().startsWith(branch),
        )
    }

    private fun assertReachedFfi(e: DashSDKException) {
        assertTrue(
            "must fail inside the FFI (code >= ${DashSdkError.PLATFORM_WALLET_CODE_OFFSET}), " +
                "got ${e.code}: ${e.message}",
            e.code >= DashSdkError.PLATFORM_WALLET_CODE_OFFSET,
        )
    }

    @Test
    fun shouldRejectBadFundArgumentsBeforeTheFfi() {
        for (external in listOf(false, true)) {
            assertJniRejection(fund(external, amountDuffs = 0L), "amountDuffs")
            assertJniRejection(fund(external, amountDuffs = -1L), "amountDuffs")
            assertJniRejection(fund(external, accountIndex = -1), "accountIndex")
            assertJniRejection(fund(external, platformAccountIndex = -1), "platformAccountIndex")
            assertJniRejection(fund(external, blob = truncatedBlob), "recipients blob")
        }
    }

    @Test
    fun shouldRejectBadResumeArgumentsBeforeTheFfi() {
        for (external in listOf(false, true)) {
            assertJniRejection(resume(external, vout = -1), "outPointVout")
            assertJniRejection(resume(external, platformAccountIndex = -1), "platformAccountIndex")
            assertJniRejection(resume(external, txid = ByteArray(31)), "outPointTxid")
            assertJniRejection(resume(external, blob = truncatedBlob), "recipients blob")
        }
    }

    @Test
    fun shouldReachTheFfiWithWellFormedArguments() {
        for (external in listOf(false, true)) {
            assertReachedFfi(fund(external))
            assertReachedFfi(resume(external))
        }
    }

    @Test
    fun shouldDeliverTheOutpointFromANativeThreadBeforeReturning() {
        val raw = ByteArray(32) { (it + 1).toByte() } + byteArrayOf(0x02, 0x01, 0x00, 0x00)
        val received = CopyOnWriteArrayList<ByteArray>()
        val callbackThread = AtomicReference<Thread>()
        val recording = AssetLockBuiltBridge {
            callbackThread.set(Thread.currentThread())
            received += it
        }

        AssetLockBuiltTrampolineDriver.invokeOnNativeThread(arrayOf(recording), raw)

        assertEquals("delivered exactly once, before the call returned", 1, received.size)
        assertArrayEquals(raw, received.single())
        assertNotEquals(Thread.currentThread(), callbackThread.get())
        val outPoint = ManagedPlatformWallet.AssetLockBuiltOutPoint.fromRawBytes(received.single())
        assertEquals(258, outPoint.vout)
    }

    @Test
    fun shouldClearAThrowingCallbackSoTheNextCallbackOnTheWorkerRuns() {
        val raw = ByteArray(36) { it.toByte() }
        val received = CopyOnWriteArrayList<ByteArray>()
        val throwing = AssetLockBuiltBridge { throw IllegalStateException("host failed") }
        val recording = AssetLockBuiltBridge { received += it }

        // Returns normally: the throw is cleared on the worker, never rethrown here.
        AssetLockBuiltTrampolineDriver.invokeOnNativeThread(arrayOf(throwing, recording), raw)

        assertEquals(1, received.size)
        assertArrayEquals(raw, received.single())
    }

    @Test
    fun shouldAcceptANullBuiltBridge() {
        val e = assertThrows(DashSDKException::class.java) {
            WalletManagerNative.walletFundFromAssetLockExternal(
                0L, 100_000L, 0, 0, validBlob, 0L, 0L, null,
            )
        }
        assertReachedFfi(e)
    }
}
