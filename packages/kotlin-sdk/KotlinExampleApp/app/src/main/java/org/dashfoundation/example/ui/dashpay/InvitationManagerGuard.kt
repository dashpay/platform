package org.dashfoundation.example.ui.dashpay

import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.example.di.AppContainer

/**
 * Verifies that an invitation operation still owns the manager and SDK for
 * the network it captured before entering application scope.
 */
internal fun AppContainer.isCurrentInvitationManager(
    expectedNetworkRaw: Int,
    capturedManager: PlatformWalletManager,
): Boolean =
    !appState.isLoading.value &&
        appState.currentNetwork.value.ffiValue == expectedNetworkRaw &&
        capturedManager.network.ffiValue == expectedNetworkRaw &&
        appState.sdk.value === capturedManager.sdk &&
        walletManagerStore.activeManager.value === capturedManager
