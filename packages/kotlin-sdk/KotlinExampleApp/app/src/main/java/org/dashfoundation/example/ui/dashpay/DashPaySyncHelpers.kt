package org.dashfoundation.example.ui.dashpay

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager

/**
 * Shared DashPay sync helpers — port of the free functions in
 * `ContactsView.swift` (`attachOrStartSync` / `kickDashPaySync`), used by
 * the DashPay hub, Contacts, and Requests screens.
 */

/**
 * Pull-to-refresh sync: if a sweep is already in flight, attach to it (wait
 * for [PlatformWalletManager.dashPaySyncIsSyncing] to clear) instead of
 * double-firing; otherwise run one pass. ← Swift `attachOrStartSync`.
 */
suspend fun attachOrStartSync(manager: PlatformWalletManager) {
    if (manager.dashPaySyncIsSyncing.value) {
        manager.dashPaySyncIsSyncing.first { !it }
    } else {
        runCatching { manager.dashPaySyncNow() }
    }
}

/**
 * Fire-and-forget kick of a sweep pass after a local mutation (send /
 * accept / pay) so the user isn't left on a stale list. The Rust manager
 * folds an in-flight pass into a no-op. ← Swift `kickDashPaySync`.
 */
fun kickDashPaySync(scope: CoroutineScope, manager: PlatformWalletManager) {
    scope.launch { runCatching { manager.dashPaySyncNow() } }
}

/**
 * Read DashPay's private contact request store (protocol version 15) for
 * [wallet]. Best effort: a network before protocol version 15 has no such
 * store. Private requests reach the wallet only through this read, so it
 * runs with every pull-to-refresh of the request list.
 * ← Swift `syncPrivateContactRequests` (`ContactsView.swift`).
 */
suspend fun syncPrivateContactRequests(
    wallet: ManagedPlatformWallet,
    manager: PlatformWalletManager,
) {
    runCatching { wallet.dashpay.syncPrivateContactRequests(manager.mnemonicResolverHandle) }
}

/**
 * Accept the incoming request from [senderId] to [ourIdentityId] with the
 * wallet's one accept, which replies the way the request came: privately
 * (paid from the wallet's shielded balance) or with a document, signed by the
 * manager's identity signer either way. The wallet decides; the app makes one
 * call. Returns false when the request is not in local state.
 * ← Swift `PlatformWalletManager.acceptContactRequestReplyingInKind`, as
 * `ContactRequestsView.swift` and `AddContactView.swift` call it.
 */
suspend fun acceptIncomingContactRequest(
    wallet: ManagedPlatformWallet,
    manager: PlatformWalletManager,
    ourIdentityId: ByteArray,
    senderId: ByteArray,
): Boolean {
    val request = wallet.dashpay.incomingContactRequest(ourIdentityId, senderId) ?: return false
    request.use { manager.acceptContactRequestReplyingInKind(wallet.walletId, it).close() }
    return true
}
