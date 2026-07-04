package org.dashfoundation.example.ui.credits

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.flow.MutableStateFlow
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer

/**
 * Resolve the live [ManagedPlatformWallet] that owns [walletId] from the
 * active [org.dashfoundation.dashsdk.wallet.PlatformWalletManager], or null
 * if the wallet isn't loaded (watch-only / loaded identity, or the manager
 * hasn't activated). The credits screens need the wallet handle + the
 * manager's signer to sign the state transition.
 *
 * Shared by the Top Up / Transfer / Withdraw credits screens, which are all
 * keyed on an identity id and reach the wallet through
 * [org.dashfoundation.dashsdk.persistence.entities.IdentityEntity.walletId].
 */
@Composable
fun rememberManagedWalletFor(walletId: ByteArray?): ManagedPlatformWallet? {
    val container = LocalAppContainer.current
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by remember(manager) {
        manager?.wallets ?: MutableStateFlow(emptyMap<String, ManagedPlatformWallet>())
    }.collectAsStateWithLifecycle()
    if (walletId == null) return null
    val hex = remember(walletId) { walletId.joinToString("") { "%02x".format(it) } }
    return walletsMap[hex]
}
