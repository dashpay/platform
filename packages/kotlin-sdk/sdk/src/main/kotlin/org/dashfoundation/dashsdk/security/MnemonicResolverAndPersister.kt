package org.dashfoundation.dashsdk.security

import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.ffi.MnemonicNative
import org.dashfoundation.dashsdk.ffi.NativeMnemonicBridge

/**
 * Decrypt-on-demand mnemonic resolver — port of
 * `MnemonicResolverAndPersister.swift`.
 *
 * Rust calls [resolveMnemonicInto] synchronously whenever a derivation
 * needs the seed; per the CLAUDE.md doctrine the phrase crosses the
 * boundary exactly there and nowhere else, as raw UTF-8 bytes written
 * into the caller's buffer — never a JVM `String` (the iOS out-buffer
 * discipline; an immutable String can't be scrubbed and would sit in the
 * heap recoverable from a dump). Persisting new mnemonics goes through
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
     * safe: this never runs on the main thread. Every intermediate copy
     * of the phrase is zeroed before returning; [out] itself is zeroed by
     * the native trampoline after the copy into Rust-owned memory.
     */
    override fun resolveMnemonicInto(walletId: ByteArray, out: ByteArray): Int = try {
        val plain = runBlocking { storage.retrieveMnemonicUtf8(walletId) }
        when {
            plain == null -> RESOLVE_NOT_FOUND
            plain.size > out.size -> {
                plain.fill(0)
                RESOLVE_BUFFER_TOO_SMALL
            }
            else -> {
                plain.copyInto(out)
                val len = plain.size
                plain.fill(0)
                len
            }
        }
    } catch (_: Exception) {
        // Contract: never throw across JNI. A decrypt/Keystore failure on
        // a wallet that MAY have a stored seed is OTHER, not NOT_FOUND —
        // reporting it as "no seed (watch-only)" would misdirect recovery
        // (the iOS `.other` discipline).
        RESOLVE_OTHER
    }

    override fun close() {
        val h = handleRef.getAndSet(0)
        if (h != 0L) MnemonicNative.destroyResolver(h)
    }
}
