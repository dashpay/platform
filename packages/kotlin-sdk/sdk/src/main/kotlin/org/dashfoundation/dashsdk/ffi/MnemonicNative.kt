package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for the mnemonic resolver — mirrors
 * `rs-unified-sdk-jni/src/mnemonic.rs`.
 */
internal object MnemonicNative {

    /**
     * Create a native `MnemonicResolverHandle` backed by [bridge].
     * The bridge is held as a JNI GlobalRef until [destroyResolver].
     */
    external fun createResolver(bridge: NativeMnemonicBridge): Long

    /** Destroy a handle from [createResolver]; drops the bridge ref. Safe on 0. */
    external fun destroyResolver(handle: Long)

    /**
     * Generate a BIP-39 mnemonic (key-wallet-ffi). wordCount ∈ {12,15,18,21,24};
     * language is the FFILanguage ordinal (0 = English).
     * @throws DashSDKException on failure
     */
    external fun generateMnemonic(wordCount: Int, language: Int): String
}

/**
 * Kotlin side of the synchronous mnemonic-resolve callback. Called from
 * Rust (possibly on Tokio worker threads) — implementations must be
 * thread-safe and fast, and MUST NOT throw.
 *
 * Method name and `([B[B)I` signature are looked up by the native
 * trampoline — keep in sync with `mnemonic.rs`.
 */
abstract class NativeMnemonicBridge {

    companion object {
        /** No mnemonic stored for the wallet (watch-only / external-signable). */
        const val RESOLVE_NOT_FOUND: Int = -1

        /** The phrase does not fit in the caller-supplied buffer. */
        const val RESOLVE_BUFFER_TOO_SMALL: Int = -2

        /**
         * A mnemonic IS stored but could not be produced — Keystore
         * locked, decrypt failure, etc. Distinct from [RESOLVE_NOT_FOUND]
         * so a transient failure is never reported as "this wallet has no
         * seed" (the iOS `.other` channel; collapsing them points the
         * user toward the wrong remediation).
         */
        const val RESOLVE_OTHER: Int = -3
    }

    /**
     * Write the UTF-8 bytes of the BIP-39 phrase for the 32-byte
     * [walletId] into the caller-supplied [out] buffer and return the
     * byte count, or [RESOLVE_NOT_FOUND] / [RESOLVE_BUFFER_TOO_SMALL].
     *
     * Out-buffer discipline (mirrors iOS `MaskedMnemonicUTF8` /
     * `retrieveMnemonicUTF8Bytes`): the phrase must never be materialized
     * as a JVM `String` — an immutable String cannot be scrubbed and
     * lives on the heap until (if ever) collected, recoverable from a
     * heap dump. Implementations copy raw bytes into [out] and zero every
     * intermediate buffer of their own before returning; the native
     * trampoline zeroes [out] after copying it into Rust-owned memory.
     */
    abstract fun resolveMnemonicInto(walletId: ByteArray, out: ByteArray): Int
}
