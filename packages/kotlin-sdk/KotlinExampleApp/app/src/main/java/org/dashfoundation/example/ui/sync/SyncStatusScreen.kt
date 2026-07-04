package org.dashfoundation.example.ui.sync

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.services.PlatformBalanceSyncService.PlatformSyncState
import org.dashfoundation.dashsdk.wallet.SpvSubProgress
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.formatCredits
import org.dashfoundation.example.util.formatRelative
import java.io.File
import java.util.Date

/**
 * Sync tab root — full port of `CoreContentView.swift`'s three sections:
 *
 * 1. **Core Sync Status** — per-phase SPV progress rows (headers, filter
 *    headers, masternodes, filters) off the manager's 1 Hz
 *    [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.spvProgress]
 *    poll, the header-tip block time, and Start/Pause + Clear controls.
 *    `startSpv` uses `filesDir/spv/<network>` (← the iOS
 *    `Documents/SPV/<network>` dataDir) and the DataStore-backed peer
 *    override (`useLocalhostCore` / `localCorePeers` — the Settings tab's
 *    SPV Peers section, ← `spvPeerOverride()`).
 * 2. **Platform Sync Status** — the BLAST address-sync state reduced by
 *    `PlatformBalanceSyncService`, balance/active-address aggregates, and
 *    the Sync Now request.
 * 3. **Shielded Sync Status** — gated on [Sdk.hasShielded]; live scan /
 *    tree-progress counters from `ShieldedService`.
 */
@Composable
fun SyncStatusScreen() {
    val appState = LocalAppState.current
    val container = LocalAppContainer.current
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    var error by remember { mutableStateOf<String?>(null) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Text("Sync Status", style = MaterialTheme.typography.headlineSmall)

        FormSection(title = "Platform") {
            LabeledContent("Network", network.displayName)
            LabeledContent("SDK", if (sdk != null) "Connected" else "Not connected")
        }

        // ── Section 1: Core (SPV) ──────────────────────────────────────
        FormSection(title = "Core Sync Status") {
            val currentManager = manager
            if (currentManager == null) {
                LabeledContent("State", "Wallet manager not active")
            } else {
                val progress by currentManager.spvProgress.collectAsStateWithLifecycle()
                val tipUnixSeconds by currentManager.spvTipUnixSecondsFlow
                    .collectAsStateWithLifecycle()

                // Liveness poll — `spvProgress` alone can't distinguish
                // "synced and running" from "stopped".
                var isRunning by remember { mutableStateOf(false) }
                LaunchedEffect(currentManager) {
                    while (true) {
                        isRunning = runCatching { currentManager.isSpvRunning() }
                            .getOrDefault(false)
                        delay(1_000)
                    }
                }

                CompactSyncRow("Headers", progress.headers)
                CompactSyncRow("Filter Headers", progress.filterHeaders)
                CompactSyncRow("Masternodes", progress.masternodes)
                CompactSyncRow("Filters", progress.filters)

                if (tipUnixSeconds > 0) {
                    LabeledContent(
                        "Block Time",
                        formatRelative(Date(tipUnixSeconds * 1000)),
                    )
                }
                LabeledContent("State", if (isRunning) "Running" else "Stopped")

                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 8.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
                ) {
                    Button(
                        onClick = {
                            scope.launch {
                                try {
                                    if (isRunning) {
                                        currentManager.stopSpv()
                                    } else {
                                        // ← CoreContentView.startSync():
                                        // Documents/SPV/<network> + peer override.
                                        val dataDir = File(
                                            context.filesDir,
                                            "spv/${network.networkName}",
                                        ).apply { mkdirs() }
                                        val peers = appState.spvPeerOverride()
                                        currentManager.startSpv(
                                            dataDir = dataDir.absolutePath,
                                            peers = peers,
                                            restrictToConfiguredPeers = peers.isNotEmpty(),
                                        )
                                    }
                                } catch (e: Exception) {
                                    error = e.message ?: "SPV operation failed"
                                }
                            }
                        },
                        modifier = Modifier.testTag("sync.spvStartStop"),
                    ) { Text(if (isRunning) "Pause" else "Start") }

                    OutlinedButton(
                        onClick = {
                            scope.launch {
                                try {
                                    currentManager.clearSpvStorage()
                                } catch (e: Exception) {
                                    error = e.message ?: "Failed to clear SPV storage"
                                }
                            }
                        },
                        enabled = !isRunning,
                        modifier = Modifier.testTag("sync.spvClear"),
                    ) { Text("Clear") }
                }
            }
        }

        // ── Section 2: Platform (BLAST address sync) ───────────────────
        FormSection(title = "Platform Sync Status") {
            val state by container.platformBalanceSyncService.state
                .collectAsStateWithLifecycle()
            val totalBalance by container.platformBalanceSyncService.totalPlatformBalance
                .collectAsStateWithLifecycle(initialValue = 0L)
            val activeAddresses by container.platformBalanceSyncService.activeAddressCount
                .collectAsStateWithLifecycle(initialValue = 0)

            when (val s = state) {
                is PlatformSyncState.Idle -> LabeledContent("State", "Not synced yet")
                is PlatformSyncState.Syncing -> LabeledContent("State", "Syncing…")
                is PlatformSyncState.Synced -> {
                    LabeledContent("State", "Synced")
                    if (s.syncHeight > 0) {
                        LabeledContent("Sync Height", s.syncHeight.toString())
                    }
                    if (s.syncTimestamp > 0) {
                        LabeledContent(
                            "Last sync",
                            formatRelative(Date(s.syncTimestamp * 1000)),
                        )
                    }
                }

                is PlatformSyncState.Error -> LabeledContent("State", "Error: ${s.message}")
            }

            LabeledContent("Platform Balance", formatCredits(totalBalance))
            LabeledContent("Active Addresses", activeAddresses.toString())

            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
            ) {
                Button(
                    onClick = {
                        scope.launch { container.platformBalanceSyncService.manualSync() }
                    },
                    enabled = state !is PlatformSyncState.Syncing,
                    modifier = Modifier.testTag("sync.platformSyncNow"),
                ) { Text("Sync Now") }
            }
        }

        // ── Section 3: Shielded ────────────────────────────────────────
        FormSection(title = "Shielded Sync Status") {
            if (!Sdk.hasShielded()) {
                LabeledContent("State", "Not available in this build")
            } else {
                val shielded by container.shieldedService.state.collectAsStateWithLifecycle()
                val balanceFlow by container.shieldedService.shieldedBalance
                    .collectAsStateWithLifecycle()
                val balance by remember(balanceFlow) { balanceFlow }
                    .collectAsStateWithLifecycle(initialValue = 0L)

                LabeledContent(
                    "State",
                    when {
                        shielded.isSyncing -> "Syncing…"
                        !container.shieldedService.isBound -> "Not bound"
                        shielded.syncCountSinceLaunch == 0 -> "Not synced yet"
                        else -> "Idle"
                    },
                )
                LabeledContent("Shielded Balance", formatCredits(balance))
                LabeledContent("Notes Scanned", shielded.totalScanned.toString())
                LabeledContent("New Notes", shielded.totalNewNotes.toString())
                LabeledContent("Syncs Since Launch", shielded.syncCountSinceLaunch.toString())

                if (shielded.treeTotalTarget > 0) {
                    LinearProgressIndicator(
                        progress = {
                            (shielded.treeLeavesCommitted.toFloat() /
                                shielded.treeTotalTarget.toFloat()).coerceIn(0f, 1f)
                        },
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 8.dp),
                    )
                    LabeledContent(
                        "Tree",
                        "${shielded.treeLeavesCommitted} / ${shielded.treeTotalTarget} leaves",
                    )
                }
            }
        }
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

/**
 * One compact SPV phase row (← `CompactSyncRow` in CoreContentView.swift):
 * title, progress bar, `current/target` heights. Skipped when the phase
 * hasn't reported yet.
 */
@Composable
private fun CompactSyncRow(title: String, sub: SpvSubProgress?) {
    if (sub == null) return
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text(
            title,
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.width(96.dp),
        )
        LinearProgressIndicator(
            // Fractional 0..1, same scale Swift feeds ProgressView(value:).
            progress = { sub.percentage.toFloat().coerceIn(0f, 1f) },
            modifier = Modifier.weight(1f),
        )
        Text(
            "%,d/%,d".format(sub.currentHeight, sub.targetHeight),
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}
