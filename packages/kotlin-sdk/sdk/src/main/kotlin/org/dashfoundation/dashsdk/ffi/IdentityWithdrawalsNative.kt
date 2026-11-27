package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for reading where an identity's withdrawals to Core stand: mirrors
 * `rs-unified-sdk-jni/src/identity_withdrawals.rs`, one export over
 * `platform_wallet_identity_withdrawals`.
 *
 * Internal: the public API is
 * [org.dashfoundation.dashsdk.identity.identityWithdrawals]. Errors throw [DashSDKException]
 * with the platform-wallet code shifted by `PLATFORM_WALLET_CODE_OFFSET`.
 */
internal object IdentityWithdrawalsNative {

    /**
     * Every withdrawal of [identityId] (32 bytes) to Core, read with proofs through the
     * wallet at [walletHandle], as the BLOB `identity_withdrawals.rs` documents, in Rust's
     * order (most recently updated first). Decoded by
     * [org.dashfoundation.dashsdk.identity.decodeIdentityWithdrawals]. Blocks for the
     * network round trips; call it off the main thread.
     */
    external fun identityWithdrawals(walletHandle: Long, identityId: ByteArray): ByteArray
}
