package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for a wallet's Platform address balance change history and the
 * address-sync queries: mirrors `rs-unified-sdk-jni/src/address_history.rs`, one export per
 * C function. Each returns the big-endian BLOB that file documents (or an `Int` for
 * [branchQueryDepth]); the public API and the decoders are in
 * [org.dashfoundation.dashsdk.wallet.platformAddressBalanceHistory] and
 * [org.dashfoundation.dashsdk.queries.AddressSyncQueries]. Every call but [branchQueryDepth]
 * blocks for network round trips; call them off the main thread.
 *
 * Errors throw [DashSDKException]: code 1 for a negative height or count, a wrong-length
 * hash or a null array, the rs-sdk-ffi code for a failed query, and for [balanceHistory] the
 * platform-wallet code shifted by `PLATFORM_WALLET_CODE_OFFSET`.
 */
internal object AddressHistoryNative {

    /** `platform_address_wallet_balance_history` on the wallet at [walletHandle]. */
    external fun balanceHistory(
        walletHandle: Long,
        includeCompacted: Boolean,
        minProvedHeight: Long,
    ): ByteArray

    /** `dash_sdk_address_fetch_recent_balance_changes`. */
    external fun recentBalanceChanges(sdk: Long, startHeight: Long): ByteArray

    /** `dash_sdk_address_fetch_compacted_balance_changes`. */
    external fun compactedBalanceChanges(sdk: Long, startBlockHeight: Long): ByteArray

    /** `dash_sdk_address_fetch_trunk_state`. */
    external fun trunkState(sdk: Long): ByteArray

    /** `dash_sdk_address_fetch_branch_state`; [expectedHash] is 32 bytes. */
    external fun branchState(
        sdk: Long,
        key: ByteArray,
        depth: Int,
        expectedHash: ByteArray,
        checkpointHeight: Long,
    ): ByteArray

    /** `dash_sdk_address_branch_query_depth`; 0 for a null SDK handle. */
    external fun branchQueryDepth(sdk: Long, estimatedCount: Long): Int
}
