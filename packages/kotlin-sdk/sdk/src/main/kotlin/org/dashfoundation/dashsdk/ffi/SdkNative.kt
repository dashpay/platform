package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for SDK lifecycle — mirrors `rs-unified-sdk-jni/src/sdk.rs`.
 *
 * Internal: the public API is [org.dashfoundation.dashsdk.Sdk]. Handles are
 * raw Rust pointers as [Long]; passing a stale or foreign value is undefined
 * behavior, so ownership is confined to the SDK wrapper classes.
 */
internal object SdkNative {

    /** One-time Rust library init. Called by [NativeLoader], not directly. */
    external fun nativeInit()

    /** Enable console logging. Level: 0=Error, 1=Warn, 2=Info, 3=Debug, 4=Trace. */
    external fun enableLogging(level: Int)

    /**
     * Route tracing output to per-bucket files under [sessionRoot]
     * (platform-wallet-ffi). False if a subscriber was already installed
     * or the path is unwritable.
     */
    external fun enableFileLogging(level: Int, sessionRoot: String): Boolean

    /** Native SDK version string. */
    external fun version(): String

    /**
     * Create a trusted-context SDK instance.
     *
     * @param network FFINetwork ordinal (0=Mainnet, 1=Testnet, 2=Devnet, 3=Regtest)
     * @param dapiAddresses comma-separated DAPI URLs, or null for network defaults
     * @param quorumUrl quorum lookup base URL override (required for devnet), or null
     * @param platformVersion 0 = SDK default (auto-detect), non-zero pins a
     *   protocol version (rejected if unknown)
     * @return non-zero SDK handle
     * @throws DashSDKException on failure
     */
    external fun createTrusted(
        network: Int,
        dapiAddresses: String?,
        quorumUrl: String?,
        skipAssetLockProofVerification: Boolean,
        requestRetryCount: Int,
        requestTimeoutMs: Long,
        platformVersion: Int,
    ): Long

    /** Destroy a handle returned by [createTrusted]. Safe on 0. */
    external fun destroy(handle: Long)

    /** FFINetwork ordinal of a live SDK handle. */
    external fun getNetwork(handle: Long): Int

    /** Whether the native library was built with shielded (Orchard) support. */
    external fun hasShielded(): Boolean
}
