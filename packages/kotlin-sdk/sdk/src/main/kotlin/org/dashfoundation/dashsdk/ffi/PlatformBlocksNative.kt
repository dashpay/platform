package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for the block and state transition status reads (protocol version 15):
 * mirrors `rs-unified-sdk-jni/src/platform_blocks.rs`, one export per `rs-sdk-ffi` function.
 * Each returns the JSON the C function writes; [org.dashfoundation.dashsdk.queries.PlatformBlocks]
 * decodes it.
 *
 * Internal: the public API is [org.dashfoundation.dashsdk.queries.PlatformBlocks]. Errors throw
 * [DashSDKException]: a negative height or count, or a null string, is refused before the C
 * call with code 1 (InvalidParameter), as are the C functions' own argument refusals.
 */
internal object PlatformBlocksNative {

    /** Bridges `dash_sdk_system_get_blocks`. [startHeight] and [count] must not be negative. */
    external fun getBlocks(sdk: Long, startHeight: Long, count: Int): String?

    /**
     * Bridges `dash_sdk_system_get_owner_transactions_in_block`. [ownerId] is base58; [height]
     * must not be negative.
     */
    external fun getOwnerTransactionsInBlock(sdk: Long, height: Long, ownerId: String): String?

    /** Bridges `dash_sdk_system_get_recent_state_transition_status`. [transitionHash] is hex. */
    external fun getRecentStateTransitionStatus(sdk: Long, transitionHash: String): String?

    /** Bridges `dash_sdk_system_get_state_transition_status_by_hash`. [transitionHash] is hex. */
    external fun getStateTransitionStatusByHash(sdk: Long, transitionHash: String): String?
}
