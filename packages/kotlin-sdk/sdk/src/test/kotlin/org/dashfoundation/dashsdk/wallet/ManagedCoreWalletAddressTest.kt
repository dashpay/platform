package org.dashfoundation.dashsdk.wallet

import java.io.File
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test

/**
 * Host-JVM coverage for [ManagedCoreWallet.nextReceiveAddress] /
 * [ManagedCoreWallet.nextChangeAddress] — everything on the Kotlin side of
 * the JNI boundary. The native library cannot load on the host, so these
 * tests exercise exactly the paths that never reach an `external fun`:
 * argument validation, the closed-handle guard, the
 * [mapNativeErrors] error contract, and (as a source-scanning guard in the
 * [GateCoverageLintTest] tradition) the requirement that both public
 * methods actually WRAP their native call in [mapNativeErrors]. Real JNI
 * linkage, derivation, and used-set semantics belong to the instrumented
 * follow-up (`connectedDebugAndroidTest`).
 */
class ManagedCoreWalletAddressTest {

    // ── Argument validation: rejected BEFORE any native call ─────────

    @Test
    fun negativeAccountIndexIsRejectedBeforeAnyNativeCall() {
        // A live (non-zero) handle: if validation did NOT fire first, the
        // call would hit the unloadable native lib and throw
        // UnsatisfiedLinkError instead of IllegalArgumentException.
        val wallet = ManagedCoreWallet(handle = 1L)
        for (call in listOf<(Int) -> String>(
            { wallet.nextReceiveAddress(it) },
            { wallet.nextChangeAddress(it) },
        )) {
            try {
                call(-1)
                fail("expected IllegalArgumentException for accountIndex -1")
            } catch (e: IllegalArgumentException) {
                assertTrue(
                    "message should name the bad index: ${e.message}",
                    e.message!!.contains("-1")
                )
            }
        }
    }

    // ── Lifecycle: the closed-handle guard fires before any native call ──

    @Test
    fun closedWalletThrowsIllegalStateBeforeAnyNativeCall() {
        // A zero handle is the closed state; close() on it never reaches
        // the native destroy (HandleCleanup skips handle 0), so the whole
        // test stays on the host side of the boundary.
        val wallet = ManagedCoreWallet(handle = 0L)
        wallet.close()
        for (call in listOf(
            { wallet.nextReceiveAddress() },
            { wallet.nextChangeAddress() },
        )) {
            try {
                call()
                fail("expected IllegalStateException on a closed wallet")
            } catch (e: IllegalStateException) {
                assertTrue(
                    "message should say the wallet is closed: ${e.message}",
                    e.message!!.contains("closed")
                )
            }
        }
    }

    // ── Error contract: the exact mapping the address accessors rely on ──

    @Test
    fun nativeNotFoundMapsToTypedDashSdkError() {
        // Code 7 (NotFound) is what a nonexistent account surfaces as —
        // the failure mode the accessors' public contract cares about.
        try {
            mapNativeErrors { throw DashSDKException(7, "BIP-44 account 9 not found") }
            fail("expected DashSdkError.NotFound")
        } catch (e: DashSdkError.NotFound) {
            assertEquals("BIP-44 account 9 not found", e.message)
            assertTrue(e.cause is DashSDKException)
        }
    }

    // ── Structural guard (the review blocker, made permanent) ─────────

    /**
     * Both public address accessors MUST wrap their native call in
     * [mapNativeErrors] — the `DashSdkError.kt` contract for public SDK
     * entry points. This was a review blocker on the PR that introduced
     * them; enforce it structurally so it cannot regress silently.
     */
    @Test
    fun addressAccessorsWrapTheirNativeCallInMapNativeErrors() {
        val src = File(findSdkMainSources(), "wallet/ManagedCoreWallet.kt").readText()
        for (method in listOf("nextReceiveAddress", "nextChangeAddress")) {
            val header = "fun $method("
            val start = src.indexOf(header)
            assertTrue("ManagedCoreWallet.$method not found", start >= 0)
            // The method body runs to the next fun declaration (or EOF).
            val end = src.indexOf("fun ", start + header.length)
                .let { if (it < 0) src.length else it }
            val body = src.substring(start, end)
            val native = body.indexOf("WalletManagerNative.")
            val wrapper = body.indexOf("mapNativeErrors")
            assertTrue("$method makes no native call?", native >= 0)
            assertTrue(
                "$method must wrap its WalletManagerNative call in " +
                    "mapNativeErrors (public-boundary error contract)",
                wrapper in 0 until native
            )
        }
    }

    /** Same walk-up lookup as [GateCoverageLintTest]. */
    private fun findSdkMainSources(): File {
        var dir: File? = File(System.getProperty("user.dir") ?: ".")
        while (dir != null) {
            for (candidate in listOf(
                File(dir, "src/main/kotlin/org/dashfoundation/dashsdk"),
                File(dir, "sdk/src/main/kotlin/org/dashfoundation/dashsdk"),
                File(dir, "packages/kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk"),
            )) {
                if (candidate.isDirectory) return candidate
            }
            dir = dir.parentFile
        }
        error("could not locate the SDK main source set from ${System.getProperty("user.dir")}")
    }
}
