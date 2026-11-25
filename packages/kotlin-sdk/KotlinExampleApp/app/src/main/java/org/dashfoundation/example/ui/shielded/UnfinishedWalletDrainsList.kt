package org.dashfoundation.example.ui.shielded

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.dashsdk.wallet.WalletDrainOutpoint
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.ShieldedFundProgress
import org.dashfoundation.example.services.shielded.ShieldedFundFromAssetLockController
import org.dashfoundation.example.services.shielded.ShieldedFundFromAssetLockCoordinator.StartFundingResult
import org.dashfoundation.example.services.shielded.WalletDrainCoordinator
import org.dashfoundation.example.services.shielded.WalletDrainCoordinator.RowAction
import org.dashfoundation.example.services.shielded.WalletDrainCoordinator.RowStatus
import org.dashfoundation.example.services.shielded.asWalletDrainWallet
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.util.toHex

/**
 * What the saved-transfers heading claims. It counts saved records, not
 * unfinished transfers: each row says what Rust reported for it. While the
 * stored ids cannot be read the heading gives no count, because the list is
 * then whatever an earlier read left. Port of `UnfinishedWalletDrainsHeading`
 * in `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Views/UnfinishedWalletDrainsList.swift`.
 */
data class UnfinishedWalletDrainsHeading(
    val title: String,
    /** Says that the rows listed come from an earlier successful read. */
    val lastKnownNotice: String?,
    /** A section-level "Try again", which works without any row. */
    val showsRetry: Boolean,
) {
    companion object {
        /** Null when there is nothing to show: no rows and the read succeeded. */
        fun make(count: Int, storeErrorPresent: Boolean): UnfinishedWalletDrainsHeading? {
            if (storeErrorPresent) {
                return UnfinishedWalletDrainsHeading(
                    title = "Saved Max transfers",
                    lastKnownNotice = if (count > 0) {
                        "Last known from an earlier read. This list may be out of date."
                    } else {
                        null
                    },
                    showsRetry = true,
                )
            }
            if (count == 0) return null
            return UnfinishedWalletDrainsHeading(
                title = "Saved Max transfers ($count)",
                lastKnownNotice = null,
                showsRetry = false,
            )
        }
    }
}

/**
 * Wallet Detail section listing this wallet's saved Max transfers, one per
 * stored operation id, each with the status the [WalletDrainCoordinator]
 * last read from Rust. Port of `UnfinishedWalletDrainsList` (same Swift
 * file); the accessibility identifiers are reused verbatim as test tags.
 *
 * A row whose status named an asset lock resumes that exact outpoint, which
 * can never build a second lock. Any other row only offers "Check again",
 * which reads the status (and retries clearing a finished one's record).
 * Which of the two a row offers is Rust's rule, asked through the
 * coordinator.
 */
@Composable
fun UnfinishedWalletDrainsList(
    walletIdHex: String,
    walletId: ByteArray,
    network: Network,
    manager: PlatformWalletManager?,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val coordinator = container.walletDrainCoordinator
    val scope = rememberCoroutineScope()

    val allEntries by coordinator.entries.collectAsStateWithLifecycle()
    val storeError by coordinator.storeError.collectAsStateWithLifecycle()
    // Observed so a row follows its controller as one is added or removed.
    val controllers by container.shieldedFundCoordinator.controllers.collectAsStateWithLifecycle()

    var checking by remember { mutableStateOf(emptySet<java.util.UUID>()) }
    var retrying by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    // Read every stored id of this wallet when the screen opens and whenever
    // the manager changes: a relaunch lands here with unread rows.
    LaunchedEffect(manager, walletIdHex, network) {
        coordinator.refreshFromStore()
        manager?.let { coordinator.reconcile(network, listOf(walletId), it.asWalletDrainWallet()) }
    }

    // `entries` holds every network's stored ids, and reconcile reads only
    // the current network's, so another network's would stay unread here.
    val entries = allEntries.filter {
        it.record.belongsTo(walletId) && it.record.networkRaw == network.ffiValue
    }
    val heading = UnfinishedWalletDrainsHeading.make(entries.size, storeError != null)
    if (heading != null) {
        FormSection(
            title = heading.title,
            modifier = Modifier.testTag("walletDetail.unfinishedMaxTransfers"),
        ) {
            if (heading.showsRetry) {
                TextButton(
                    onClick = {
                        val m = manager ?: return@TextButton
                        if (retrying) return@TextButton
                        retrying = true
                        scope.launch {
                            coordinator.refreshFromStore()
                            coordinator.reconcile(network, listOf(walletId), m.asWalletDrainWallet())
                            retrying = false
                        }
                    },
                    enabled = !retrying && manager != null,
                    modifier = Modifier.testTag("walletDetail.unfinishedMaxTransfers.retryButton"),
                ) { Text("Try again") }
            }
            storeError?.let {
                Text(
                    it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.testTag("walletDetail.unfinishedMaxTransfers.storeError"),
                )
            }
            heading.lastKnownNotice?.let {
                Text(
                    it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            entries.forEachIndexed { index, entry ->
                // This operation's controller in any phase, matched by its id.
                val controller = controllers.values.firstOrNull {
                    it.operationId == coordinator.controllerOperationId(entry.operationId)
                }
                DrainRow(
                    entry = entry,
                    controller = controller,
                    coordinator = coordinator,
                    isChecking = entry.operationId in checking,
                    canAct = manager != null,
                    onOpenProgress = {
                        navController.navigate(
                            ShieldedFundProgress(walletIdHex, entry.record.recipientRaw43.toHex()),
                        )
                    },
                    onResume = { outpoint ->
                        val m = manager ?: return@DrainRow
                        scope.launch {
                            when (
                                val result = coordinator.resume(
                                    entry.record, outpoint, network, m.asWalletDrainWallet(),
                                )
                            ) {
                                is StartFundingResult.Started -> navController.navigate(
                                    ShieldedFundProgress(
                                        walletIdHex,
                                        entry.record.recipientRaw43.toHex(),
                                    ),
                                )
                                is StartFundingResult.BlockedByOtherWalletFunding -> error =
                                    "Another shielded funding is in progress on this wallet " +
                                        "(recipient ${hexShort(result.blocker.recipientRaw43)}). " +
                                        "Resume this transfer after that one finishes."
                            }
                        }
                    },
                    onCheckAgain = {
                        val m = manager ?: return@DrainRow
                        val id = entry.operationId
                        if (id in checking) return@DrainRow
                        checking = checking + id
                        scope.launch {
                            coordinator.checkAgain(entry.record, network, m.asWalletDrainWallet())
                            checking = checking - id
                        }
                    },
                )
                if (index < entries.size - 1) HorizontalDivider()
            }
        }
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

/** One saved transfer. Follows its controller's live phase when it has one. */
@Composable
private fun DrainRow(
    entry: WalletDrainCoordinator.Entry,
    controller: ShieldedFundFromAssetLockController?,
    coordinator: WalletDrainCoordinator,
    isChecking: Boolean,
    canAct: Boolean,
    onOpenProgress: () -> Unit,
    onResume: (WalletDrainOutpoint) -> Unit,
    onCheckAgain: () -> Unit,
) {
    val phase by (controller?.phase ?: remember { MutableStateFlow(null) })
        .collectAsStateWithLifecycle()
    val inFlight = phase?.isActive == true
    val action = remember(entry, inFlight) { coordinator.rowAction(inFlight, entry) }
    val status = coordinator.rowStatus(inFlight, entry)

    Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
        verticalAlignment = Alignment.Top,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(
                "Max to ${hexShort(entry.record.recipientRaw43)}",
                style = MaterialTheme.typography.bodyLarge,
                maxLines = 1,
            )
            entry.outpoint?.let {
                Text(
                    "Asset lock ${outpointLabel(it)}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.MiddleEllipsis,
                )
            }
            Text(
                status.text,
                style = MaterialTheme.typography.bodySmall,
                color = when (status) {
                    is RowStatus.ReadFailed,
                    is RowStatus.FinishedNotCleared,
                    is RowStatus.NotSentNotCleared,
                    -> MaterialTheme.colorScheme.error
                    RowStatus.Unfinished, RowStatus.UnfinishedLockUnknown -> WARNING
                    else -> MaterialTheme.colorScheme.onSurfaceVariant
                },
                // Before the first read the row offers no Resume. Check again
                // stays: if no reconcile reaches this id, it is the only way
                // to read it.
                modifier = Modifier.testTag(
                    if (status == RowStatus.Checking) {
                        "walletDetail.unfinishedMaxTransfer.checkingStatus"
                    } else {
                        "walletDetail.unfinishedMaxTransfer.status"
                    },
                ),
            )
        }
        when (action) {
            RowAction.InProgress -> TextButton(
                onClick = onOpenProgress,
                enabled = controller != null,
                modifier = Modifier.testTag("walletDetail.unfinishedMaxTransfer.inProgress"),
            ) {
                CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                Text("In progress", modifier = Modifier.padding(start = 6.dp))
            }
            is RowAction.Resume -> Button(
                onClick = { onResume(action.outpoint) },
                enabled = canAct,
                modifier = Modifier.testTag("walletDetail.unfinishedMaxTransfer.resumeButton"),
            ) { Text("Resume") }
            RowAction.CheckAgain -> OutlinedButton(
                onClick = onCheckAgain,
                enabled = canAct && !isChecking,
                modifier = Modifier.testTag("walletDetail.unfinishedMaxTransfer.checkAgainButton"),
            ) {
                if (isChecking) {
                    CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                } else {
                    Text("Check again")
                }
            }
        }
    }
}

private val WARNING = Color(0xFFB26A00)

/** Display-order txid (reversed wire bytes), as asset-lock rows show it. */
private fun outpointLabel(outpoint: WalletDrainOutpoint): String {
    val txid = outpoint.txidWire.reversedArray().toHex()
    return "${txid.take(8)}…:${outpoint.vout}"
}

private fun hexShort(bytes: ByteArray): String {
    val hex = bytes.toHex()
    return if (hex.length <= 16) hex else "${hex.take(8)}…${hex.takeLast(8)}"
}
