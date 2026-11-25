package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.repeatOnLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.identity.IdentityWithdrawal
import org.dashfoundation.dashsdk.identity.WithdrawalStatus
import org.dashfoundation.dashsdk.identity.identityWithdrawals
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.collectEachAsStateWithLifecycle
import org.dashfoundation.example.util.formatCredits
import org.dashfoundation.example.util.formatDate
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex
import java.text.DateFormat
import java.util.Date

/** How often the list refreshes itself while a withdrawal is not final (Swift: 15 s). */
private const val AUTO_REFRESH_INTERVAL_MS = 15_000L

/**
 * Where an identity's credit withdrawals to Core stand — port of
 * `IdentityWithdrawalsView.swift` (SwiftExampleApp/Views).
 *
 * Each row comes from [identityWithdrawals], which reads the identity's `withdrawal` documents
 * from the withdrawals system contract with proofs and returns the whole list, each withdrawal
 * once, most recently updated first. Rust decides the status, its label and whether the
 * withdrawal is final; this screen only presents them and refreshes: on open, on pull to
 * refresh, on the refresh button, and every 15 s while the screen is started and any row is
 * not final (paused while an error is shown).
 *
 * The Platform side of a withdrawal ends at "Complete". Platform state stores the asset unlock
 * index of the Core transaction (and the Core height it was requested at), not its txid, so a
 * row shows the index rather than a txid.
 *
 * The wallet is resolved from the active manager at each read and never held in composition.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun IdentityWithdrawalsScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }
    val identity by container.database.identityDao()
        .observeByIdentityId(idBytes)
        .collectEachAsStateWithLifecycle(initialValue = null)
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by remember(manager) {
        manager?.wallets ?: MutableStateFlow(emptyMap<String, ManagedPlatformWallet>())
    }.collectAsStateWithLifecycle()
    val walletIdHex = identity?.walletId?.toHex()
    val walletLoaded = walletIdHex != null && walletsMap.containsKey(walletIdHex)

    var withdrawals by remember { mutableStateOf<List<IdentityWithdrawal>>(emptyList()) }
    var hasLoaded by remember { mutableStateOf(false) }
    var isLoading by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var lastRefreshed by remember { mutableStateOf<Date?>(null) }
    // Bumped on every reload so a late answer from an older read cannot overwrite a newer one.
    var loadGeneration by remember { mutableIntStateOf(0) }
    val scope = rememberCoroutineScope()

    // Whether any listed withdrawal is still moving, as Rust decided it. A function so the
    // polling loop reads the current state, not the value of the composition that launched it.
    fun autoRefreshActive(): Boolean = withdrawals.any { !it.isFinal } && errorMessage == null

    suspend fun reload() {
        // Resolved at each read from the manager's map, which keeps the wrapper alive.
        val wallet = walletIdHex?.let { manager?.wallets?.value?.get(it) } ?: return
        val generation = ++loadGeneration
        isLoading = true
        try {
            val rows = wallet.identityWithdrawals(idBytes)
            if (generation == loadGeneration) {
                withdrawals = rows
                errorMessage = null
                lastRefreshed = Date()
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            if (generation == loadGeneration) {
                errorMessage = (e as? DashSdkError)?.userMessage ?: e.message ?: "Could not load withdrawals"
            }
        } finally {
            if (generation == loadGeneration) {
                isLoading = false
                hasLoaded = true
            }
        }
    }

    val lifecycleOwner = LocalLifecycleOwner.current
    LaunchedEffect(walletIdHex, walletLoaded) {
        if (!walletLoaded) return@LaunchedEffect
        // Polls only while the screen is started; leaving it cancels the loop.
        lifecycleOwner.repeatOnLifecycle(Lifecycle.State.STARTED) {
            reload()
            while (true) {
                delay(AUTO_REFRESH_INTERVAL_MS)
                if (autoRefreshActive() && !isLoading) reload()
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Withdrawals") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = { scope.launch { reload() } },
                        enabled = !isLoading && walletLoaded,
                        modifier = Modifier.testTag("identityWithdrawals.refresh"),
                    ) {
                        if (isLoading) {
                            CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
                        } else {
                            Icon(Icons.Filled.Refresh, contentDescription = "Refresh withdrawals")
                        }
                    }
                },
            )
        },
    ) { padding ->
        PullToRefreshBox(
            isRefreshing = isLoading && hasLoaded,
            onRefresh = { scope.launch { reload() } },
            modifier = Modifier.fillMaxSize().padding(padding),
        ) {
            LazyColumn(
                modifier = Modifier.fillMaxSize().testTag("identityWithdrawals.list"),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                item {
                    Column(Modifier.padding(horizontal = 16.dp)) {
                        LabeledContent(
                            "Identity",
                            identity?.mainDpnsName ?: (identityIdHex.take(16) + "…"),
                        )
                        LabeledContent(
                            "Last refreshed",
                            lastRefreshed?.let { DateFormat.getTimeInstance(DateFormat.MEDIUM).format(it) }
                                ?: "Not yet",
                            modifier = Modifier.testTag("identityWithdrawals.lastRefreshed"),
                        )
                        if (autoRefreshActive()) {
                            Text(
                                "Refreshing every 15 s while a withdrawal is in progress",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.testTag("identityWithdrawals.autoRefresh"),
                            )
                        }
                        Text(
                            STATUS_FOOTER,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(top = 8.dp),
                        )
                    }
                }

                errorMessage?.let { message ->
                    item {
                        Column(Modifier.padding(horizontal = 16.dp)) {
                            Text(
                                "Could not load withdrawals",
                                style = MaterialTheme.typography.titleSmall,
                            )
                            Text(
                                message,
                                color = MaterialTheme.colorScheme.error,
                                style = MaterialTheme.typography.bodyMedium,
                                modifier = Modifier.testTag("identityWithdrawals.error"),
                            )
                            OutlinedButton(
                                onClick = { scope.launch { reload() } },
                                enabled = !isLoading && walletLoaded,
                                modifier = Modifier.testTag("identityWithdrawals.retry"),
                            ) { Text("Try Again") }
                        }
                    }
                }

                item {
                    Text(
                        "Withdrawals",
                        style = MaterialTheme.typography.titleSmall,
                        modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
                    )
                }

                when {
                    identity != null && !walletLoaded -> item {
                        Text(
                            "The wallet that owns this identity isn't loaded.",
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.padding(horizontal = 16.dp)
                                .testTag("identityWithdrawals.walletMissing"),
                        )
                    }
                    withdrawals.isEmpty() && (!hasLoaded || isLoading) -> item {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            modifier = Modifier.padding(horizontal = 16.dp)
                                .testTag("identityWithdrawals.loading"),
                        ) {
                            CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                            Text("Loading withdrawals…", color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                    withdrawals.isEmpty() && errorMessage == null -> item {
                        Column(
                            Modifier.fillMaxWidth().padding(16.dp).testTag("identityWithdrawals.empty"),
                            horizontalAlignment = Alignment.CenterHorizontally,
                        ) {
                            Text("No withdrawals", style = MaterialTheme.typography.titleMedium)
                            Text(
                                "No withdrawals found. A withdrawal submitted moments ago can take " +
                                    "a block to appear; refresh to check again.",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                    else -> items(withdrawals, key = { it.idHex }) { withdrawal ->
                        WithdrawalRow(withdrawal)
                        HorizontalDivider()
                    }
                }
            }
        }
    }
}

/** One withdrawal (← `WithdrawalRow` in `IdentityWithdrawalsView.swift`). */
@Composable
private fun WithdrawalRow(withdrawal: IdentityWithdrawal) {
    // The first four bytes of the document id in hex, used in the test tags.
    val idPrefix = withdrawal.idHex.take(8)
    Column(
        verticalArrangement = Arrangement.spacedBy(4.dp),
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp)
            .testTag("identityWithdrawals.row.$idPrefix"),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                formatCredits(withdrawal.amountCredits),
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.weight(1f),
            )
            Text(
                withdrawal.statusLabel,
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.Medium,
                color = statusColor(withdrawal.status),
                modifier = Modifier.testTag("identityWithdrawals.status.$idPrefix"),
            )
        }
        SelectionContainer {
            Text(
                withdrawal.destination ?: "Script ${withdrawal.outputScriptHex}",
                style = MaterialTheme.typography.bodySmall,
                fontFamily = FontFamily.Monospace,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
        }
        withdrawal.transactionIndex?.let { index ->
            val signHeight = withdrawal.transactionSignHeight
            Text(
                if (signHeight != null) {
                    "Asset unlock #$index, requested at Core height $signHeight"
                } else {
                    "Asset unlock #$index"
                },
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.testTag("identityWithdrawals.coreTransaction.$idPrefix"),
            )
        }
        Text(
            "Submitted ${formatDate(Date(withdrawal.createdAtMs))}",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Updated ${formatDate(Date(withdrawal.updatedAtMs))}",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/** The tint for a status (← `statusColor` in `IdentityWithdrawalsView.swift`). */
@Composable
private fun statusColor(status: WithdrawalStatus): Color = when (status) {
    WithdrawalStatus.QUEUED -> MaterialTheme.colorScheme.onSurfaceVariant
    WithdrawalStatus.POOLED, WithdrawalStatus.EXPIRED -> Color(0xFFE08600)
    WithdrawalStatus.BROADCASTED -> Color(0xFF1E6FD9)
    WithdrawalStatus.COMPLETE -> Color(0xFF2E9D4E)
    WithdrawalStatus.FAILED -> MaterialTheme.colorScheme.error
}

private const val STATUS_FOOTER =
    "A withdrawal is queued, then pooled into an asset unlock transaction. " +
        "Broadcasted means Platform built that transaction and queued it for a " +
        "quorum to sign and for broadcast to Core; it does not show whether either " +
        "has happened. Complete means Platform saw it in a chain-locked Core block; that is " +
        "where Platform's side ends. Expired means Core did not mine it in time " +
        "and Platform signs it again. Failed means the payout is below Core's " +
        "dust threshold, so Core never mines it.\n\n" +
        "Platform state stores the asset unlock index of the Core transaction, " +
        "not its transaction id, so the transaction id is not shown. " +
        "Withdrawals from Platform addresses or the shielded pool are not listed."
