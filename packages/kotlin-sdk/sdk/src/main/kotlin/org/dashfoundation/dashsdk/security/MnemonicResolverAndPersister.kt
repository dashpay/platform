package org.dashfoundation.dashsdk.security

import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.ffi.MnemonicNative
import org.dashfoundation.dashsdk.ffi.NativeMnemonicBridge

/**
 * Decrypt-on-demand mnemonic resolver — port of
 * `MnemonicResolverAndPersister.swift`.
 *
 * Rust calls [resolveMnemonic] synchronously whenever a derivation needs
 * the seed; per the CLAUDE.md doctrine the phrase crosses the boundary
 * exactly there and nowhere else. Persisting new mnemonics goes through
 * [WalletStorage.storeMnemonic] (Keystore-wrapped AES-GCM in DataStore).
 *
 * [nativeHandle] is the `MnemonicResolverHandle` to pass into FFI entry
 * points that derive from a stored mnemonic; release with [close].
 */
class MnemonicResolverAndPersister(
    private val storage: WalletStorage,
) : NativeMnemonicBridge(), AutoCloseable {

    private val handleRef =
        java.util.concurrent.atomic.AtomicLong(MnemonicNative.createResolver(this))

    val nativeHandle: Long
        get() = handleRef.get().also {
            check(it != 0L) { "MnemonicResolverAndPersister has been closed" }
        }

    /**
     * Synchronous resolve on the calling (Tokio) thread. `runBlocking` is
     * required — the FFI contract is synchronous write-into-buffer — and
     * safe: this never runs on the main thread.
     */
    override fun resolveMnemonic(walletId: ByteArray): String? = try {
        runBlocking { storage.retrieveMnemonic(walletId) }
    } catch (_: Exception) {
        // Contract: never throw across JNI; null → NOT_FOUND on the Rust
        // side, which surfaces as a wallet-operation error.
        null
    }

    override fun close() {
        val h = handleRef.getAndSet(0)
        if (h != 0L) MnemonicNative.destroyResolver(h)
    }
}
