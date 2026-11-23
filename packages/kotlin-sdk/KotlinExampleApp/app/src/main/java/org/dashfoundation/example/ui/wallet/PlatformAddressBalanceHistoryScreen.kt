package org.dashfoundation.example.ui.wallet

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
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
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.wallet.BalanceHistoryEntry
import org.dashfoundation.dashsdk.wallet.CompactedBalanceHistoryEntry
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.dashsdk.wallet.PlatformAddressBalanceHistory
import org.dashfoundation.dashsdk.wallet.platformAddressBalanceHistory
import org.dashfoundation.example.di.LocalAppContainer
import java.math.BigDecimal

private val ADDED_COLOR = Color(0xFF2E9D4E)
private val SET_COLOR = Color(0xFFE08600)

/**
 * Balance change history of a wallet's DIP-17 Platform addresses (ADDR-10) — port of
 * `PlatformAddressBalanceHistoryView.swift` (SwiftExampleApp/Views), opened from the wallet's
 * Platform Balance menu.
 *
 * Both sections come from one [platformAddressBalanceHistory] call: the first load asks for
 * recent changes only; "Load older history" asks for both and replaces both sections from
 * that one result, so a block compacted in between never shows twice. Every load after the
 * first passes the `provedHeight` Rust returned for the history on screen, so Rust never
 * replaces it with an older read. Paging, the filter to this wallet's addresses (every
 * platform payment account), the height floor and its retries, the reconciliation of the two
 * zones, the newest-first order and each address's bech32m form all happen in Rust; this
 * screen renders what it gets and keeps that height. The wallet is resolved from the active
 * manager at each load, never held in composition.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PlatformAddressBalanceHistoryScreen(walletIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by remember(manager) {
        manager?.wallets ?: MutableStateFlow(emptyMap<String, ManagedPlatformWallet>())
    }.collectAsStateWithLifecycle()
    val walletLoaded = walletsMap.containsKey(walletIdHex)

    var recent by remember { mutableStateOf<List<BalanceHistoryEntry>>(emptyList()) }
    var hasLoadedRecent by remember { mutableStateOf(false) }
    var isLoadingRecent by remember { mutableStateOf(false) }
    var recentError by remember { mutableStateOf<String?>(null) }
    // Whether a load has succeeded, so `recent` holds rows Rust returned. A failed refresh
    // then keeps showing them under its error; only a first load that fails shows it alone.
    var hasShownRecent by remember { mutableStateOf(false) }
    // Null until older history is asked for.
    var compacted by remember { mutableStateOf<List<CompactedBalanceHistoryEntry>?>(null) }
    var isLoadingCompacted by remember { mutableStateOf(false) }
    var compactedError by remember { mutableStateOf<String?>(null) }
    // The height Rust reported for the history on screen (0 before the first load).
    var shownProvedHeight by remember { mutableLongStateOf(0L) }
    val scope = rememberCoroutineScope()

    fun isBusy() = isLoadingRecent || isLoadingCompacted

    suspend fun read(includeCompacted: Boolean): PlatformAddressBalanceHistory {
        val wallet = manager?.wallets?.value?.get(walletIdHex)
            ?: throw IllegalStateException("Wallet handle not found in the wallet manager.")
        return wallet.platformAddressBalanceHistory(includeCompacted, shownProvedHeight)
    }

    fun message(e: Exception): String = (e as? DashSdkError)?.userMessage ?: e.message ?: "Could not load the history"

    // First load and a refresh without older history: the recent zone only.
    suspend fun loadRecent() {
        if (isBusy()) return
        isLoadingRecent = true
        try {
            val history = read(includeCompacted = false)
            recent = history.recent
            shownProvedHeight = history.provedHeight
            hasShownRecent = true
            recentError = null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            recentError = message(e)
        } finally {
            isLoadingRecent = false
            hasLoadedRecent = true
        }
    }

    // "Load older history": both zones from one call, replacing both sections.
    suspend fun loadOlder() {
        if (isBusy()) return
        isLoadingCompacted = true
        try {
            val history = read(includeCompacted = true)
            recent = history.recent
            compacted = history.compacted
            shownProvedHeight = history.provedHeight
            hasShownRecent = true
            hasLoadedRecent = true
            recentError = null
            compactedError = null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            compactedError = message(e)
        } finally {
            isLoadingCompacted = false
        }
    }

    LaunchedEffect(walletLoaded) {
        if (walletLoaded && !hasLoadedRecent) loadRecent()
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Balance History") },
                navigationIcon = {
                    IconButton(
                        onClick = { navController.popBackStack() },
                        modifier = Modifier.testTag("balanceHistory.done"),
                    ) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        PullToRefreshBox(
            isRefreshing = isBusy() && hasLoadedRecent,
            // Reload what is shown: both sections from one call when older history is shown.
            onRefresh = { scope.launch { if (compacted != null) loadOlder() else loadRecent() } },
            modifier = Modifier.fillMaxSize().padding(padding),
        ) {
            LazyColumn(
                modifier = Modifier.fillMaxSize().testTag("balanceHistory.list"),
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                item { SectionTitle("Recent") }
                val shownRecentError = recentError
                when {
                    !walletLoaded -> item {
                        ErrorRow("The wallet isn't loaded.", "balanceHistory.recent.error")
                    }
                    shownRecentError != null && !hasShownRecent -> item {
                        ErrorRow(shownRecentError, "balanceHistory.recent.error")
                    }
                    !hasLoadedRecent || (isLoadingRecent && recent.isEmpty()) -> item {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            modifier = Modifier.padding(horizontal = 16.dp)
                                .testTag("balanceHistory.recent.loading"),
                        ) {
                            CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                            Text("Loading recent changes…", color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                    else -> {
                        if (recent.isEmpty()) {
                            item {
                                Text(
                                    "No balance changes in recent blocks.",
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    modifier = Modifier.padding(horizontal = 16.dp)
                                        .testTag("balanceHistory.recent.empty"),
                                )
                            }
                        } else {
                            itemsIndexed(recent) { index, entry ->
                                RecentRow(entry, Modifier.testTag("balanceHistory.recent.row.$index"))
                            }
                        }
                        // A failed refresh keeps the last good rows, as Older does.
                        if (shownRecentError != null) {
                            item { ErrorRow(shownRecentError, "balanceHistory.recent.error") }
                        }
                    }
                }
                item {
                    Footer("Changes in blocks Platform has not compacted yet. Pull to refresh.")
                    HorizontalDivider(Modifier.padding(vertical = 8.dp))
                    SectionTitle("Older (compacted)")
                }
                compacted?.let { rows ->
                    if (rows.isEmpty()) {
                        item {
                            Text(
                                "No older balance changes.",
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.padding(horizontal = 16.dp)
                                    .testTag("balanceHistory.compacted.empty"),
                            )
                        }
                    } else {
                        itemsIndexed(rows) { index, entry ->
                            CompactedRow(entry, Modifier.testTag("balanceHistory.compacted.row.$index"))
                        }
                    }
                }
                compactedError?.let { error ->
                    item { ErrorRow(error, "balanceHistory.compacted.error") }
                }
                if (compacted == null || compactedError != null) {
                    item {
                        TextButton(
                            onClick = { scope.launch { loadOlder() } },
                            enabled = !isBusy() && walletLoaded,
                            modifier = Modifier.padding(horizontal = 8.dp).testTag("balanceHistory.loadOlder"),
                        ) {
                            if (isLoadingCompacted) {
                                CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                                Text("  Loading older history…")
                            } else {
                                Text("Load older history")
                            }
                        }
                    }
                }
                item { Footer("Platform merges older blocks into ranges and keeps them for about a week.") }
            }
        }
    }
}

@Composable
private fun SectionTitle(title: String) {
    Text(
        title,
        style = MaterialTheme.typography.titleSmall,
        modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
    )
}

@Composable
private fun Footer(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
    )
}

@Composable
private fun ErrorRow(message: String, tag: String) {
    Text(
        message,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.error,
        modifier = Modifier.padding(horizontal = 16.dp).testTag(tag),
    )
}

@Composable
private fun RecentRow(entry: BalanceHistoryEntry, modifier: Modifier) {
    Column(modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                "Block ${entry.blockHeight}",
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.Medium,
                modifier = Modifier.weight(1f),
            )
            when (val change = entry.change) {
                is BalanceHistoryEntry.Change.Added ->
                    Text("+${formatCreditsExact(change.credits)}", color = ADDED_COLOR)
                is BalanceHistoryEntry.Change.Set ->
                    Text("set to ${formatCreditsExact(change.credits)}", color = SET_COLOR)
            }
        }
        AddressText(entry.address)
    }
}

@Composable
private fun CompactedRow(entry: CompactedBalanceHistoryEntry, modifier: Modifier) {
    Column(modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                "Blocks ${entry.startBlockHeight} to ${entry.endBlockHeight}",
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.Medium,
                modifier = Modifier.weight(1f),
            )
            when (val change = entry.change) {
                is CompactedBalanceHistoryEntry.Change.SetFinal ->
                    Text("set to ${formatCreditsExact(change.credits)}", color = SET_COLOR)
                is CompactedBalanceHistoryEntry.Change.Additions ->
                    Text("+${formatCreditsExact(change.total)}", color = ADDED_COLOR)
            }
        }
        AddressText(entry.address)
        (entry.change as? CompactedBalanceHistoryEntry.Change.Additions)?.entries?.forEach { addition ->
            Row(Modifier.padding(start = 12.dp)) {
                Text(
                    "Block ${addition.blockHeight}",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.weight(1f),
                )
                Text(
                    "+${formatCreditsExact(addition.credits)}",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun AddressText(address: String) {
    Text(
        address,
        style = MaterialTheme.typography.bodySmall,
        fontFamily = FontFamily.Monospace,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
    )
}

/**
 * Credits as DASH at full credit precision (exact decimal arithmetic, 1 DASH = 1e11 credits),
 * as Swift's `KeyLimitsFormatting.dash`: 400 credits is "0.000000004 DASH", never "0 DASH".
 */
internal fun formatCreditsExact(credits: Long): String =
    BigDecimal.valueOf(credits).movePointLeft(11).stripTrailingZeros().toPlainString() + " DASH"
