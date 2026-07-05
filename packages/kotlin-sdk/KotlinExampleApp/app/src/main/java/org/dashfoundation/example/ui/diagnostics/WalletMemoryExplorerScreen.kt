package org.dashfoundation.example.ui.diagnostics

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.formatDuffs
import org.dashfoundation.example.util.truncateMiddle
import java.text.DateFormat
import java.util.Date

/**
 * Live wallet-manager memory state — port of
 * `WalletMemoryExplorerView.swift`. Renders what the Kotlin
 * `PlatformWalletManager` actually holds in memory right now: the wallets
 * map, per-wallet lock-free balances, SPV progress, the SPV chain tip,
 * and the Rust-side sync-loop liveness flags (`is*SyncRunning`). The iOS
 * view's deeper drill-downs (core wallet state snapshot, identity scan
 * state, provider state, per-account UTXO pools) wait on their
 * `platform_wallet_manager_*` snapshot exports.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WalletMemoryExplorerScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    var reloadKey by remember { mutableIntStateOf(0) }
    var spvRunning by remember { mutableStateOf<Boolean?>(null) }
    var addressSyncRunning by remember { mutableStateOf<Boolean?>(null) }
    var identitySyncRunning by remember { mutableStateOf<Boolean?>(null) }
    var shieldedSyncRunning by remember { mutableStateOf<Boolean?>(null) }
    var balances by remember {
        mutableStateOf<Map<String, ManagedPlatformWallet.Balance>>(emptyMap())
    }
    var summaries by remember {
        mutableStateOf<Map<String, ManagedPlatformWallet.InMemorySummary>>(emptyMap())
    }
    var identityRows by remember {
        mutableStateOf<Map<String, List<ManagedPlatformWallet.IdentityState>>>(emptyMap())
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Wallet Memory Explorer") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = { reloadKey++ },
                        modifier = Modifier.testTag("walletMemory.refresh"),
                    ) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                },
            )
        },
    ) { padding ->
        val currentManager = manager
        if (currentManager == null) {
            Text(
                "Wallet manager not active — activate a network first.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier
                    .padding(padding)
                    .padding(24.dp)
                    .testTag("walletMemory.noManager"),
            )
            return@Scaffold
        }

        val wallets by currentManager.wallets.collectAsStateWithLifecycle()
        val spvProgress by currentManager.spvProgress.collectAsStateWithLifecycle()
        val spvTipUnixSeconds by currentManager.spvTipUnixSecondsFlow
            .collectAsStateWithLifecycle()

        LaunchedEffect(currentManager, wallets.keys, reloadKey) {
            spvRunning = runCatching { currentManager.isSpvRunning() }.getOrNull()
            addressSyncRunning =
                runCatching { currentManager.isPlatformAddressSyncRunning() }.getOrNull()
            identitySyncRunning =
                runCatching { currentManager.isIdentitySyncRunning() }.getOrNull()
            shieldedSyncRunning = if (Sdk.hasShielded()) {
                runCatching { currentManager.isShieldedSyncRunning() }.getOrNull()
            } else {
                null
            }
            balances = wallets.mapNotNull { (idHex, wallet) ->
                runCatching { idHex to wallet.balance() }.getOrNull()
            }.toMap()
            summaries = wallets.mapNotNull { (idHex, wallet) ->
                runCatching { idHex to wallet.inMemorySummary() }.getOrNull()
            }.toMap()
            identityRows = wallets.mapValues { (_, wallet) ->
                runCatching { wallet.inMemoryIdentityStates() }.getOrDefault(emptyList())
            }
        }

        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Manager") {
                LabeledContent("Network", currentManager.network.displayName)
                LabeledContent("Wallets in Memory", "${wallets.size}")
            }

            FormSection(title = "SPV Sync") {
                LabeledContent("Loop", liveness(spvRunning))
                LabeledContent("State", spvProgress.overallState.toString())
                LabeledContent(
                    "Progress",
                    "%.1f%%".format(spvProgress.overallPercentage),
                )
                LabeledContent("Syncing", if (spvProgress.isSyncing) "Yes" else "No")
                LabeledContent(
                    "Chain Tip",
                    if (spvTipUnixSeconds > 0) {
                        DateFormat.getDateTimeInstance()
                            .format(Date(spvTipUnixSeconds * 1000))
                    } else {
                        "—"
                    },
                )
            }

            FormSection(title = "Platform Address Sync") {
                LabeledContent("Loop", liveness(addressSyncRunning))
            }

            FormSection(title = "Identity Sync") {
                LabeledContent("Loop", liveness(identitySyncRunning))
            }

            FormSection(title = "Shielded Sync") {
                LabeledContent(
                    "Loop",
                    if (Sdk.hasShielded()) liveness(shieldedSyncRunning) else "Unavailable",
                )
            }

            FormSection(title = "Wallets (${wallets.size})") {
                if (wallets.isEmpty()) {
                    Text(
                        "No wallets loaded into the manager.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                wallets.keys.sorted().forEachIndexed { index, idHex ->
                    if (index > 0) HorizontalDivider(Modifier.padding(vertical = 8.dp))
                    val wallet = wallets.getValue(idHex)
                    Text(
                        truncateMiddle(idHex, 12, 8),
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                        modifier = Modifier.testTag("walletMemory.wallet.$idHex"),
                    )
                    LabeledContent(
                        "Handle",
                        if (wallet.isClosed) "Closed" else "Live",
                    )
                    balances[idHex]?.let { balance ->
                        LabeledContent("Confirmed", formatDuffs(balance.confirmed))
                        LabeledContent("Unconfirmed", formatDuffs(balance.unconfirmed))
                        LabeledContent("Immature", formatDuffs(balance.immature))
                        LabeledContent("Locked", formatDuffs(balance.locked))
                    }
                    summaries[idHex]?.let { summary ->
                        LabeledContent("Identities", "${summary.identitiesCount}")
                        LabeledContent("Watched", "${summary.watchedCount}")
                        LabeledContent("Last Scanned Index", "${summary.lastScannedIndex}")
                        LabeledContent("Tracked Asset Locks", "${summary.trackedAssetLocksCount}")
                    }
                    val rows = identityRows[idHex].orEmpty()
                    if (rows.isNotEmpty()) {
                        Text(
                            "Identities (${rows.size})",
                            style = MaterialTheme.typography.labelMedium,
                            modifier = Modifier.padding(top = 8.dp),
                        )
                        rows.forEach { row ->
                            LabeledContent(
                                label = truncateMiddle(Base58.encode(row.identityId), 12, 8),
                                value = buildString {
                                    append(if (row.watched) "watched" else "index ${row.index}")
                                    append(" · ")
                                    append(identityStatusLabel(row.status))
                                },
                            )
                        }
                    }
                }
            }
        }
    }
}

/** Human label for a managed-identity lifecycle status. */
private fun identityStatusLabel(status: ManagedPlatformWallet.IdentityStatus): String =
    when (status) {
        ManagedPlatformWallet.IdentityStatus.PENDING_CREATION -> "Pending"
        ManagedPlatformWallet.IdentityStatus.ACTIVE -> "Active"
        ManagedPlatformWallet.IdentityStatus.FAILED_CREATION -> "Failed"
        ManagedPlatformWallet.IdentityStatus.NOT_FOUND -> "Not Found"
        ManagedPlatformWallet.IdentityStatus.UNKNOWN -> "Unknown"
    }

private fun liveness(running: Boolean?): String = when (running) {
    null -> "—"
    true -> "Running"
    false -> "Stopped"
}
