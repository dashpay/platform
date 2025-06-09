package com.dash.sdk.types

/**
 * Configuration for the Dash SDK
 */
data class SDKConfig(
    /**
     * Network to connect to
     */
    val network: Network,

    /**
     * Skip asset lock proof verification (useful for testing)
     */
    val skipAssetLockProofVerification: Boolean = false,

    /**
     * Number of times to retry failed requests
     */
    val requestRetryCount: Int = 3,

    /**
     * Request timeout in milliseconds
     */
    val requestTimeoutMs: Long = 30000,

    /**
     * Core IP address (for local networks)
     */
    val coreIpAddress: String? = null,

    /**
     * Platform port (for local networks)
     */
    val platformPort: Int = 0,

    /**
     * Whether to dump lookup sessions on drop (for debugging)
     */
    val dumpLookupSessionsOnDrop: Boolean = false
) {
    init {
        require(requestRetryCount >= 0) { "Request retry count must be non-negative" }
        require(requestTimeoutMs > 0) { "Request timeout must be positive" }
        require(platformPort >= 0) { "Platform port must be non-negative" }
    }
}