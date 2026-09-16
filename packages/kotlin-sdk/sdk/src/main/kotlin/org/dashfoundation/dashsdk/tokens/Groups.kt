package org.dashfoundation.dashsdk.tokens

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.TokensNative

/**
 * Group-action discovery on token contracts — the Android analog of the
 * Swift `TokenGroupActionQueries`
 * (`packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/Tokens`), driving
 * the `PendingGroupActionsView` / `CoSignProposalView` flows.
 *
 * Both entry points are read-only network queries returning raw JSON strings
 * (decoded by the caller). They are `suspend` on [Dispatchers.IO] since the
 * native call blocks a worker thread on the Rust runtime.
 *
 * [walletHandle] is a live `PlatformWallet` handle from
 * `WalletManagerNative.getWallet`.
 */
class Groups internal constructor(private val walletHandle: Long) {

    /** Group-action proposal status filter — mirrors the FFI discriminant. */
    enum class Status(val value: Int) {
        /** Active / pending (awaiting co-signatures). */
        ACTIVE(0),

        /** Closed (fully signed and executed, or otherwise finalized). */
        CLOSED(1),
    }

    /**
     * List group-action proposals on `(tokenContractId, groupContractPosition)`
     * filtered by [status]. [startAtActionId] paginates from a 32-byte action
     * id; [limit] == 0 uses the native default. Returns the JSON array string
     * described in `platform_wallet_token_pending_group_actions`, or null.
     */
    suspend fun pendingActions(
        tokenContractId: ByteArray,
        groupContractPosition: Int,
        status: Status = Status.ACTIVE,
        startAtActionId: ByteArray? = null,
        limit: Int = 0,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            TokensNative.pendingGroupActions(
                walletHandle, tokenContractId, groupContractPosition,
                status.value, startAtActionId, limit,
            )
        }
    }

    /**
     * List the signers of a specific proposal ([actionId], 32 bytes). Returns
     * the JSON array `[{"identityId": "...", "power": <u32>}, ...]`, or null.
     */
    suspend fun actionSigners(
        tokenContractId: ByteArray,
        groupContractPosition: Int,
        status: Status,
        actionId: ByteArray,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            TokensNative.groupActionSigners(
                walletHandle, tokenContractId, groupContractPosition,
                status.value, actionId,
            )
        }
    }
}
