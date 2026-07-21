package org.dashfoundation.dashsdk.config

import org.dashfoundation.dashsdk.Network

/**
 * Caller-supplied connection overrides for [org.dashfoundation.dashsdk.Sdk].
 *
 * On iOS these live in `UserDefaults` and are read inside `SDK.init`
 * (`SwiftDashSDK/SDK.swift`); a library reading app preferences directly is
 * un-idiomatic on Android, so the example app resolves its DataStore
 * preferences into this object and the gating policy stays in [applyTo]-time
 * logic inside the SDK — identical rules, explicit inputs.
 */
data class SdkConfig(
    val network: Network,
    /**
     * Comma-separated DAPI URL list override. Applied for regtest and
     * devnet unconditionally, and for mainnet/testnet only when
     * [useDockerSetup] is set (dashmate-on-localhost flow). Default for the
     * docker flow is [DEFAULT_LOCAL_DAPI].
     */
    val dapiAddresses: String? = null,
    /**
     * Trusted-context-provider quorum lookup base URL. Required for devnet
     * (no built-in default exists on the Rust side); honored for regtest
     * and docker setups; never forwarded for plain mainnet/testnet.
     */
    val quorumUrl: String? = null,
    /** Route mainnet/testnet at a local dashmate stack (localhost DAPI). */
    val useDockerSetup: Boolean = false,
    val skipAssetLockProofVerification: Boolean = false,
    val requestRetryCount: Int = 1,
    val requestTimeoutMs: Long = 8_000,
    /**
     * 0 = let the Rust SDK seed at the per-network minimum protocol version
     * with auto-detect on; non-zero pins the exact platform version.
     */
    val platformVersion: Int = 0,
) {
    companion object {
        /** Default local dashmate Platform DAPI address (iOS parity). */
        const val DEFAULT_LOCAL_DAPI: String = "http://127.0.0.1:2443"
    }
}
