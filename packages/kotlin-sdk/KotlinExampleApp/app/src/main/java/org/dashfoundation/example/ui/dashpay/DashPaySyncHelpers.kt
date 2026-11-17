package org.dashfoundation.example.ui.dashpay

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.tokens.Dashpay
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
 * The wallet's two holds on [ourIdentityId]'s pairs with [contactIds], as
 * hex ids, one read of each per contact: held for a private request, as a
 * send decides it (`Dashpay.privateRepliesPending`: a private request in
 * flight, paid and not yet read back, or kept by the shielded store's
 * journal), and held for a public request whose broadcast may have been
 * delivered (`Dashpay.publicContactRequestsUnconfirmed`), which a DashPay
 * sync settles. While either holds a pair, Rust refuses another send or
 * reply to it, so the screens offer neither. Empty when the identity is not
 * managed, as the wallet then holds nothing for it. Null when the wallet
 * could not be read (no wallet, or a read failed), which the screens keep
 * apart from "nothing held" ([PrivateRepliesPendingSnapshot]).
 * ← Swift `contactRequestHolds(wallet:identityId:contactIds:)` (`ContactsView.swift`).
 */
suspend fun contactRequestHolds(
    wallet: ManagedPlatformWallet?,
    ourIdentityId: ByteArray,
    contactIds: List<ByteArray>,
): ContactRequestHolds? {
    if (contactIds.isEmpty()) return ContactRequestHolds()
    if (wallet == null) return null
    return contactRequestHolds(
        readPrivatelyHeld = {
            wallet.dashpay.privateRepliesPending(ourIdentityId, contactIds).map { it.toHex() }.toSet()
        },
        readPublicUnconfirmed = {
            wallet.dashpay.publicContactRequestsUnconfirmed(ourIdentityId, contactIds)
                .map { it.toHex() }.toSet()
        },
    )
}

/** The wallet's readiness for an identity's contact requests and, when it flags a failed restoration, why. */
data class PrivateContactReadinessRead(
    val readiness: Dashpay.PrivateContactReadiness,
    val restoreFailure: String?,
)

/**
 * The wallet's readiness for [ourIdentityId]'s contact requests and, when it
 * flags a failed restoration, why it failed; null when the readiness could
 * not be read ([PrivateRepliesPendingSnapshot.withReadiness] keeps the last
 * one then).
 * ← Swift `privateContactReadiness(wallet:identityId:)` (`ContactsView.swift`).
 */
suspend fun privateContactReadiness(
    wallet: ManagedPlatformWallet?,
    ourIdentityId: ByteArray,
): PrivateContactReadinessRead? {
    if (wallet == null) return null
    val readiness = try {
        wallet.dashpay.privateContactReadiness(ourIdentityId)
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        return null
    }
    if (!readiness.privatePaymentsRestoreFailed) return PrivateContactReadinessRead(readiness, null)
    val reason = try {
        wallet.dashpay.privatePaymentsRestoreFailure(ourIdentityId)
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        null
    }
    return PrivateContactReadinessRead(readiness, reason)
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
