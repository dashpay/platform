package org.dashfoundation.example.ui.dashpay

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.example.util.toHex

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
 * The hex ids of the contacts among [contactIds] that [ourIdentityId] has a
 * private request to in flight, or paid and not yet read back, as the wallet
 * records it (one snapshot). While one is, Rust refuses another send or reply
 * to it, so the screens show it as pending and offer neither; a private sync
 * settles it. Empty when the identity is not managed, as the wallet then
 * holds nothing for it. Null when the wallet could not be read (no wallet,
 * or a read failed), which the screens keep apart from "nothing pending"
 * ([PrivateRepliesPendingSnapshot]).
 * ← Swift `privateRepliesPending` (`ContactsView.swift`).
 */
suspend fun privateRepliesPending(
    wallet: ManagedPlatformWallet?,
    ourIdentityId: ByteArray,
    contactIds: List<ByteArray>,
): Set<String>? {
    if (contactIds.isEmpty()) return emptySet()
    if (wallet == null) return null
    return try {
        wallet.dashpay.privateRepliesPending(ourIdentityId, contactIds).map { it.toHex() }.toSet()
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        null
    }
}

/**
 * Read DashPay's private contact request store (protocol version 15) for
 * [wallet], returning what the refresh should tell the user, or null.
 * Private requests reach the wallet only through this read, so it runs with
 * every pull-to-refresh of the request list. A network before protocol
 * version 15 does not serve the store yet, which is expected and quiet; any
 * other failure, or requests read but not saved, is reported.
 * ← Swift `syncPrivateContactRequests` (`ContactsView.swift`).
 */
suspend fun syncPrivateContactRequests(
    wallet: ManagedPlatformWallet,
    manager: PlatformWalletManager,
): String? = try {
    val result = wallet.dashpay.syncPrivateContactRequests(manager.mnemonicResolverHandle)
    PrivateContactRefreshNotice.notice(result.unpersistedIdentityCount)
} catch (e: CancellationException) {
    throw e
} catch (e: Exception) {
    PrivateContactRefreshNotice.notice(e)
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
