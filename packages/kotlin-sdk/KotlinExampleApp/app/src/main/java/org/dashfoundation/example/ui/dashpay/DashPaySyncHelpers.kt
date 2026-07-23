package org.dashfoundation.example.ui.dashpay

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
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
