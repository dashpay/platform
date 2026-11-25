package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SwipeToDismissBox
import androidx.compose.material3.SwipeToDismissBoxValue
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberSwipeToDismissBoxState
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.wallet.IdentityDeviceRemovalResult
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.example.services.tokens.IdentityTokenSync

/**
 * An identity row that asks to be removed when swiped from the end: port of
 * the trailing `swipeActions` on the identities list in
 * `IdentitiesContentView.swift`. Like iOS (`allowsFullSwipe: false`), the
 * swipe never removes the row by itself: it calls [onRemoveRequested], which
 * opens [RemoveIdentityDialog], and the row springs back. The same request is
 * offered as an accessibility action named "Remove".
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun RemovableIdentityRow(
    onRemoveRequested: () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    val state = rememberSwipeToDismissBoxState(
        confirmValueChange = { value ->
            if (value == SwipeToDismissBoxValue.EndToStart) onRemoveRequested()
            false
        },
    )
    SwipeToDismissBox(
        state = state,
        enableDismissFromStartToEnd = false,
        modifier = modifier.semantics {
            customActions = listOf(
                CustomAccessibilityAction("Remove") {
                    onRemoveRequested()
                    true
                },
            )
        },
        backgroundContent = {
            Row(
                modifier = Modifier
                    .fillMaxSize()
                    .background(MaterialTheme.colorScheme.errorContainer)
                    .padding(horizontal = 20.dp),
                horizontalArrangement = Arrangement.End,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(
                    Icons.Filled.Delete,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onErrorContainer,
                )
                Text(
                    "Remove",
                    color = MaterialTheme.colorScheme.onErrorContainer,
                    modifier = Modifier.padding(start = 8.dp),
                )
            }
        },
    ) { content() }
}

/**
 * Confirmation for removing an identity from this device: port of the
 * `confirmationDialog` in `IdentitiesContentView.swift` (SwiftExampleApp),
 * with the same title, message and buttons. [onConfirm] runs the removal.
 */
@Composable
internal fun RemoveIdentityDialog(
    displayName: String,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Remove $displayName from this device?") },
        text = {
            Text(
                "This only deletes the local copy. The identity remains on the Dash Platform " +
                    "network. You can reload it later if you still have the keys.",
            )
        },
        confirmButton = {
            TextButton(
                onClick = onConfirm,
                modifier = Modifier.testTag("identities.remove.confirm"),
            ) { Text("Remove from Device", color = MaterialTheme.colorScheme.error) }
        },
        dismissButton = {
            TextButton(
                onClick = onDismiss,
                modifier = Modifier.testTag("identities.remove.cancel"),
            ) { Text("Keep on Device") }
        },
    )
}

/**
 * Remove [identityId] from this device through
 * [PlatformWalletManager.removeIdentityFromDevice] and return the message to
 * show, or null when everything was removed. Port of
 * `IdentitiesContentView.removeIdentityLocally` (SwiftExampleApp): the
 * wording of both failures follows it.
 */
internal suspend fun removeIdentityFromDeviceMessage(
    manager: PlatformWalletManager?,
    identityId: ByteArray,
): String? {
    if (manager == null) {
        return "The identity was not removed from this device: the wallet manager is not ready."
    }
    val result = try {
        // Under the token-watch lock, so a Tokens refresh queued for this
        // identity runs after the removal and sees it gone.
        IdentityTokenSync.withWatchLock { manager.removeIdentityFromDevice(identityId) }
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        // A refusal from the live wallet (a DashPay contact request in flight,
        // private payments not restored) lands here with Rust's reason, as
        // does a failed row delete; either way the identity is still listed,
        // and removing it again retries.
        return "The identity was not removed from this device: ${e.message ?: e.javaClass.simpleName}. " +
            "It is still listed; remove it again to retry."
    }
    return keysLeftMessage(result.keysLeft)
}

/** The message for secrets [IdentityDeviceRemovalResult.keysLeft] names; null for none. */
internal fun keysLeftMessage(keysLeft: List<String>): String? {
    if (keysLeft.isEmpty()) return null
    val names = keysLeft.joinToString(" and ")
    val (stays, them) = if (keysLeft.size == 1) "It stays" to "it" else "They stay" to "them"
    return "The identity was removed from this device, but secure storage refused to delete " +
        "its $names. $stays on the device for now: the next identity removal deletes " +
        "leftover identity keys, and a voting key goes when the identity is loaded and " +
        "removed again."
}
