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
 * Method name and `([B)Ljava/lang/String;` signature are looked up by the
 * native trampoline — keep in sync with `mnemonic.rs`.
 */
abstract class NativeMnemonicBridge {

    /**
     * Return the BIP-39 phrase for the 32-byte [walletId], or null when no
     * mnemonic is stored (watch-only / external-signable wallets).
     */
    abstract fun resolveMnemonic(walletId: ByteArray): String?
}
