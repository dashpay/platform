package org.dashfoundation.example.ui.sync

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
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
import androidx.compose.ui.text.input.KeyboardType
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
import org.dashfoundation.example.ui.theme.appStatusColors
import org.dashfoundation.example.services.sync.CompactFilterRescan
import org.dashfoundation.example.services.sync.CompactFilterRescanResult
import org.dashfoundation.example.services.sync.CompactFilterRescanWallet
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
    val successColor = appStatusColors.success

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
                var rescanHeightText by remember { mutableStateOf("0") }
                var rescanInFlight by remember { mutableStateOf(false) }
                var rescanResult by remember { mutableStateOf<CompactFilterRescanResult?>(null) }
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
                LabeledContent(
                    "State",
                    if (isRunning) "Running" else "Stopped",
                    valueColor = if (isRunning) successColor else null,
                )

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

                OutlinedTextField(
                    value = rescanHeightText,
                    onValueChange = { rescanHeightText = it.filter(Char::isDigit) },
                    label = { Text("Compact-filter rescan height") },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().testTag("sync.spvRescanHeight"),
                )
                Button(
                    onClick = {
                        val height = rescanHeightText.toIntOrNull() ?: return@Button
                        rescanInFlight = true
                        scope.launch {
                            try {
                                val targets = currentManager.wallets.value.values.map { wallet ->
                                    CompactFilterRescanWallet(wallet.walletIdHex, wallet.walletId)
                                }
                                rescanResult = CompactFilterRescan.armAll(targets, height) { id, from ->
                                    currentManager.rescanSpvFilters(id, from)
                                }
                            } catch (e: Exception) {
                                error = e.message ?: "Failed to arm compact-filter rescan"
                            } finally {
                                rescanInFlight = false
                            }
                        }
                    },
                    enabled = !rescanInFlight && rescanHeightText.toIntOrNull() != null &&
                        currentManager.wallets.value.isNotEmpty(),
                    modifier = Modifier.testTag("sync.spvRescan"),
                ) { Text(if (rescanInFlight) "Arming…" else "Arm Rescan") }

                rescanResult?.let { result ->
                    Text(
                        "Accepted ${result.acceptedWalletIds.size} wallet rewind request(s) from height " +
                            "${result.fromHeight}. A running SPV loop acts next tick; a stopped " +
                            "loop acts on next start. Equal/forward heights are harmless no-ops. " +
                            "This request is in-memory and must be " +
                            "reissued after process death if not yet consumed.",
                        style = MaterialTheme.typography.bodySmall,
                        modifier = Modifier.testTag("sync.spvRescanState"),
                    )
                    result.failures.forEach { failure ->
                        Text(
                            "${failure.walletIdHex.take(12)}…: ${failure.error.message}",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.testTag("sync.spvRescanFailure.${failure.walletIdHex}"),
                        )
                    }
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
                    LabeledContent("State", "Synced", valueColor = successColor)
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

                is PlatformSyncState.Error -> LabeledContent(
                    "State",
                    "Error: ${s.message}",
                    valueColor = MaterialTheme.colorScheme.error,
                )
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

                // ← CoreContentView.swift "Clear": actually clears synced data
                // (#3959) — native reset + Room clear, fail-closed. Disabled
                // mid-pass so we never clear underneath a running sync.
                val platformManager = manager
                OutlinedButton(
                    onClick = {
                        val mgr = platformManager ?: return@OutlinedButton
                        scope.launch {
                            try {
                                container.platformBalanceSyncService.clearLocalState(
                                    network = network,
                                    walletIdsOnNetwork = mgr.wallets.value.values.map { it.walletId },
                                )
                            } catch (e: Exception) {
                                error = e.message ?: "Failed to clear platform sync data"
                            }
                        }
                    },
                    enabled = platformManager != null && state !is PlatformSyncState.Syncing,
                    modifier = Modifier.testTag("sync.platformClear"),
                ) { Text("Clear") }
            }
        }

        // ── Section 3: Shielded ────────────────────────────────────────
        FormSection(title = "Shielded Sync Status") {
            if (!Sdk.hasShielded()) {
                LabeledContent("State", "Not available in this build")
            } else {
                val shielded by container.shieldedService.state.collectAsStateWithLifecycle()
                val isBound by container.shieldedService.isBound
                    .collectAsStateWithLifecycle()
                val canResume by container.shieldedService.canResume
                    .collectAsStateWithLifecycle()
                val balanceFlow by container.shieldedService.shieldedBalance
                    .collectAsStateWithLifecycle()
                val balance by remember(balanceFlow) { balanceFlow }
                    .collectAsStateWithLifecycle(initialValue = 0L)

                LabeledContent(
                    "State",
                    when {
                        shielded.isSyncing -> "Syncing…"
                        !isBound -> "Not bound"
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

                // ← CoreContentView.swift "Clear" (SH-12): Rust-side cold
                // reset (`clearShielded`) + Room wipe, fail-closed. Resets the
                // shared commitment tree + every wallet's watermark so the
                // next bind + sync cold-rebuilds from index 0 — the fix for a
                // tree frozen at N/N with zero notes scanned after a
                // watermark/notes desync. Disabled mid-pass so we never clear
                // underneath a running sync.
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 8.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
                ) {
                    OutlinedButton(
                        onClick = {
                            scope.launch {
                                try {
                                    container.shieldedService.clearLocalState()
                                } catch (e: Exception) {
                                    error = e.message ?: "Failed to clear shielded data"
                                }
                            }
                        },
                        // Gate on `canResume` (stashed bind credentials), not
                        // the transient `isBound` (← CoreContentView.swift, which
                        // gates on `!isSyncing` alone with credentials kept by
                        // clearLocalState). A frozen tree leaves us unbound but
                        // resumable — that's exactly when Clear is needed — so
                        // gating on `isBound` pinned it disabled. `!isSyncing`
                        // uses the pass-in-flight flag now, so it's satisfiable
                        // between passes instead of stuck on the loop-alive flag.
                        enabled = canResume && !shielded.isSyncing,
                        modifier = Modifier.testTag("sync.shieldedClear"),
                    ) { Text("Clear") }
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
