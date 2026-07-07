package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for the async signer — mirrors
 * `rs-unified-sdk-jni/src/signer.rs`.
 */
internal object SignerNative {

    /** Create a native `SignerHandle` backed by [bridge] (held as GlobalRef). */
    external fun createSigner(bridge: NativeSignerBridge): Long

    /** Destroy a handle from [createSigner]; drops the bridge ref. Safe on 0. */
    external fun destroySigner(handle: Long)

    /**
     * Complete an in-flight sign request. Exactly once per token; pass
     * either a signature or an error message.
     */
    external fun completeSign(token: Long, signature: ByteArray?, errorMessage: String?)

    /**
     * One-shot ECDSA sign: raw 32-byte private key + payload → signature,
     * entirely inside Rust (`dash_sdk_signer_create_from_private_key` +
     * `dash_sdk_signer_sign`). Caller zeroes [privateKey] after the call.
     * @throws DashSDKException on failure
     */
    external fun signWithPrivateKey(privateKey: ByteArray, network: Int, data: ByteArray): ByteArray?

    /**
     * One-shot derive-then-sign for platform-address keys (the
     * `keyType == 0xFF` branch). Derives an ECDSA secp256k1 key from
     * `(mnemonicUtf8, derivationPath)`, signs [data], returns the signature —
     * entirely inside Rust (`dash_sdk_sign_with_mnemonic_and_path`). The
     * derived key never crosses JNI; the seed + scalar are held in
     * Rust-owned zeroizing buffers. Mirrors
     * `KeychainSigner.signPlatformAddressOnDemand` on iOS.
     *
     * [mnemonicUtf8] is the raw UTF-8 phrase bytes (never a `java.lang.String`,
     * which cannot be scrubbed). The caller OWNS the array and MUST `fill(0)`
     * it after the call; Rust copies it into a scrubbed buffer and holds no
     * other copy.
     * @throws DashSDKException on any derivation/signing failure
     */
    external fun signWithMnemonicAndPathInto(
        mnemonicUtf8: ByteArray,
        derivationPath: String,
        network: Int,
        data: ByteArray,
    ): ByteArray?
}

/**
 * Kotlin side of the async signer vtable. Both methods are invoked from
 * Rust on Tokio worker threads; implementations must be thread-safe and
 * MUST NOT throw.
 *
 * Method names/signatures are looked up by the native trampolines
 * (`signAsync` `([BI[BJ)V`, `canSignWith` `([BI)Z`) — keep in sync with
 * `signer.rs`.
 */
abstract class NativeSignerBridge {

    /**
     * Asynchronously produce a signature for [data] with the private key
     * matching [pubkeyBytes]/[keyType] (KeyType discriminant, or 0xFF for
     * platform-address-hash lookups). MUST return promptly and arrange for
     * [SignerNative.completeSign] to be called with [completionToken]
     * exactly once — Rust bounds the wait at five minutes, after which the
     * request fails with a protocol error.
     */
    abstract fun signAsync(
        pubkeyBytes: ByteArray,
        keyType: Int,
        data: ByteArray,
        completionToken: Long,
    )

    /** Fast synchronous availability check — no I/O beyond a cache/DB hit. */
    abstract fun canSignWith(pubkeyBytes: ByteArray, keyType: Int): Boolean
}
