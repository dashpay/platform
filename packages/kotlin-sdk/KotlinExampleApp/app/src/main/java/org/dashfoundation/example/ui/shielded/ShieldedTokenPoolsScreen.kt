package org.dashfoundation.example.ui.shielded

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberModalBottomSheetState
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
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.LifecycleResumeEffect
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.dashsdk.wallet.ShieldedTokenPoolBalance
import org.dashfoundation.dashsdk.wallet.WalletSyncEvent
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.ShieldedTokenSend
import org.dashfoundation.example.navigation.ShieldedTokenUnshield
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.collectEachAsStateWithLifecycle
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * The tokens a wallet holds in the token shielded pools it watches (SH-25).
 * Port of `ShieldedTokenPoolsView` in
 * `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Core/Views/ShieldedTokenPoolsView.swift`;
 * its accessibility identifiers are reused verbatim as test tags.
 *
 * Every value comes from Rust ([org.dashfoundation.dashsdk.wallet.PlatformWalletManager.shieldedTokenPoolBalances]):
 * one row per watched pool and bound account, as the local scan found it.
 * Rust keeps this state in its shielded store file, not in Room. The screen
 * only formats it: the token's name and decimals when the app has the token
 * locally (display only), otherwise the Base58 token id and the amount in the
 * token's smallest unit. Notes a submitted spend still holds are not in the
 * balance; Rust reports them apart and the row shows them as "Pending".
 *
 * The watched pools are scanned by the ordinary shielded sync, so Refresh
 * runs `syncShieldedNow` and then reads the balances again; a finished sync
 * pass for this wallet, and every return to the screen (after a spend form,
 * or after a Clear in Settings), reads them again too. A type 28 purchase
 * watches the bought token's pool by itself; Watch a Token Pool adds one of
 * the app's known tokens by hand, and a row's menu stops watching that token.
 * **Send** opens [ShieldedTokenSendScreen] (type 26, SH-26) and **Unshield**
 * opens [ShieldedTokenUnshieldScreen] (type 27, SH-27) for the row's token
 * and account.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ShieldedTokenPoolsScreen(walletIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val walletId = remember(walletIdHex) { walletIdHex.hexToBytes() }
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    // The known tokens (names, decimals, the watch picker) are those of the
    // network the listing manager is locked to, not the app's selection,
    // which can differ during a network switch.
    val network = manager?.network
    val syncState by container.shieldedService.state.collectAsStateWithLifecycle()
    val canResume by container.shieldedService.canResume.collectAsStateWithLifecycle()
    val hasShielded = remember { Sdk.hasShielded() }
    val tokens by remember(network) {
        network?.let { container.database.tokenDao().observeTokensByNetwork(it.ffiValue) }
            ?: flowOf(emptyList())
    }.collectEachAsStateWithLifecycle(initialValue = emptyList())
    val scope = rememberCoroutineScope()

    // The balances Rust last returned, in its order; null before the first
    // read finished. Keyed by manager and wallet so another one's rows never
    // show here.
    var balances by remember(manager, walletIdHex) { mutableStateOf<List<ShieldedTokenPoolBalance>?>(null) }
    var loadError by remember(manager, walletIdHex) { mutableStateOf<String?>(null) }
    // A newer read cancels the one in flight (the effect's key changes), so
    // only the newest read applies its result.
    var readEpoch by remember { mutableIntStateOf(0) }
    var checkStatus by remember { mutableStateOf<CheckNowStatus>(CheckNowStatus.Idle) }
    var unwatchError by remember { mutableStateOf<String?>(null) }
    // The watch call and the picker live in the back-stack ViewModel, so
    // neither closing the picker nor an activity recreation (rotation) drops
    // the reread that shows a watch Rust saved.
    val watchModel: ShieldedTokenPoolWatchViewModel = viewModel()

    LifecycleResumeEffect(Unit) {
        readEpoch++
        onPauseOrDispose {}
    }
    LaunchedEffect(manager, walletIdHex, readEpoch, watchModel.watchRevision) {
        val activeManager = manager ?: return@LaunchedEffect
        if (!hasShielded) {
            loadError = "This build does not support shielded token pools."
            return@LaunchedEffect
        }
        try {
            balances = activeManager.shieldedTokenPoolBalances(walletId)
            loadError = null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            loadError = "Could not read the token pool balances: ${e.message ?: "unknown error"}"
        }
    }
    // A finished sync pass may have scanned the watched pools: read again.
    LaunchedEffect(manager, walletIdHex) {
        manager?.syncEvents?.collect { event ->
            if (event is WalletSyncEvent.ShieldedResult && event.walletId.contentEquals(walletId)) {
                readEpoch++
            }
        }
    }

    val isChecking = checkStatus == CheckNowStatus.Checking
    val canRefresh = hasShielded && canResume && manager != null && !isChecking
    val refresh: () -> Unit = refresh@{
        val activeManager = manager ?: return@refresh
        checkStatus = CheckNowStatus.Checking
        scope.launch {
            checkStatus = try {
                activeManager.syncShieldedNow()
                CheckNowStatus.Finished
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                CheckNowStatus.Failed(e.message ?: "unknown error")
            }
            readEpoch++
        }
    }
    val groups = remember(balances) { groupShieldedTokenBalances(balances.orEmpty()) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Shielded Tokens") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = refresh,
                        enabled = canRefresh,
                        modifier = Modifier.testTag("shieldedTokens.toolbarRefreshButton"),
                    ) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                },
            )
        },
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding).testTag("shieldedTokens.list"),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            item {
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedButton(
                            onClick = refresh,
                            enabled = canRefresh,
                            modifier = Modifier.testTag("shieldedTokens.refreshButton"),
                        ) {
                            Icon(Icons.Default.Refresh, contentDescription = null)
                            Spacer(Modifier.width(6.dp))
                            Text("Refresh")
                            if (isChecking || syncState.isSyncing) {
                                Spacer(Modifier.width(8.dp))
                                CircularProgressIndicator(modifier = Modifier.size(14.dp), strokeWidth = 2.dp)
                            }
                        }
                        OutlinedButton(
                            onClick = { watchModel.showSheet = true },
                            enabled = hasShielded && manager != null,
                            modifier = Modifier.testTag("shieldedTokens.watchButton"),
                        ) {
                            Icon(Icons.Default.Add, contentDescription = null)
                            Spacer(Modifier.width(6.dp))
                            Text("Watch a Token Pool")
                        }
                    }
                    shieldedTokensRefreshText(checkStatus)?.let { text ->
                        Text(
                            text,
                            style = MaterialTheme.typography.bodySmall,
                            color = if (checkStatus.isFailure) {
                                MaterialTheme.colorScheme.error
                            } else {
                                MaterialTheme.colorScheme.onSurfaceVariant
                            },
                            modifier = Modifier.testTag("shieldedTokens.refreshStatus"),
                        )
                    }
                    Text(
                        "Refresh runs a shielded sync, which scans every watched token pool, " +
                            "then reads the balances again.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            loadError?.let { error ->
                item {
                    Text(
                        error,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("shieldedTokens.loadError"),
                    )
                }
            }
            if (balances == null && loadError == null) {
                item {
                    Box(Modifier.fillMaxWidth().testTag("shieldedTokens.loading"), contentAlignment = Alignment.Center) {
                        CircularProgressIndicator()
                    }
                }
            } else if (groups.isEmpty() && loadError == null) {
                item {
                    Column(
                        modifier = Modifier.fillMaxWidth().padding(vertical = 24.dp).testTag("shieldedTokens.empty"),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Text("No Watched Token Pools", style = MaterialTheme.typography.titleMedium)
                        Text(
                            "Buying a token from the shielded pool watches its pool. You can also " +
                                "watch a token's pool by hand.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
            items(groups, key = { it.tokenIdHex }) { group ->
                val known = knownShieldedToken(group.accounts.first(), tokens)
                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    TokenHeader(group, known)
                    group.accounts.forEach { balance ->
                        BalanceRow(
                            balance = balance,
                            token = known,
                            showsAccount = group.accounts.size > 1,
                            onSend = {
                                navController.navigate(
                                    ShieldedTokenSend(walletIdHex, balance.tokenId.toHex(), balance.accountIndex),
                                )
                            },
                            onUnshield = {
                                navController.navigate(
                                    ShieldedTokenUnshield(walletIdHex, balance.tokenId.toHex(), balance.accountIndex),
                                )
                            },
                            onUnwatch = {
                                val activeManager = manager ?: return@BalanceRow
                                scope.launch {
                                    try {
                                        activeManager.shieldedTokenPoolUnwatch(walletId, balance.tokenId)
                                    } catch (e: CancellationException) {
                                        throw e
                                    } catch (e: Exception) {
                                        unwatchError = e.message ?: "unknown error"
                                    }
                                    readEpoch++
                                }
                            },
                        )
                    }
                }
            }
        }
    }

    if (watchModel.showSheet) {
        ShieldedTokenPoolWatchSheet(
            tokens = tokens,
            watched = balances.orEmpty(),
            watchingTokenId = watchModel.watchingTokenId,
            watchError = watchModel.watchError,
            onWatch = onWatch@{ token ->
                val activeManager = manager ?: return@onWatch
                watchModel.watch(activeManager, walletId, token)
            },
            onDismiss = watchModel::closeSheet,
        )
    }

    ErrorAlertDialog(
        message = unwatchError,
        title = "Could not stop watching",
        onDismiss = { unwatchError = null },
    )
}

/**
 * The Watch a Token Pool picker and its call, held for the list's back-stack
 * entry. The watch is a blocking verified read that can outlive the sheet and
 * the activity (a rotation recreates the screen while it runs); its
 * completion bumps [watchRevision], which the list's read is keyed by, so the
 * list that is on screen when the call ends reads Rust's rows again, whether
 * the watch was saved or refused.
 */
class ShieldedTokenPoolWatchViewModel : ViewModel() {

    var showSheet by mutableStateOf(false)

    /** The local token id (hex) of the watch in flight, if any. */
    var watchingTokenId by mutableStateOf<String?>(null)
        private set

    /** Rust's refusal of the last watch, shown in the picker. */
    var watchError by mutableStateOf<String?>(null)
        private set

    /** Bumped when a watch call ends. */
    var watchRevision by mutableIntStateOf(0)
        private set

    /** Watch [token]'s pool for [walletId]; ignored while one runs. */
    fun watch(manager: PlatformWalletManager, walletId: ByteArray, token: TokenEntity) {
        if (watchingTokenId != null) return
        watchingTokenId = token.id.toHex()
        watchError = null
        viewModelScope.launch {
            try {
                manager.shieldedTokenPoolWatch(walletId, token.contractId, token.position)
                showSheet = false
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                watchError = e.message ?: "unknown error"
            } finally {
                watchingTokenId = null
                watchRevision++
            }
        }
    }

    fun closeSheet() {
        showSheet = false
        watchError = null
    }
}

@Composable
private fun TokenHeader(group: ShieldedTokenGroup, known: TokenEntity?) {
    val tokenIdBase58 = Base58.encode(group.tokenId)
    Column(modifier = Modifier.testTag("shieldedTokens.tokenHeader.$tokenIdBase58")) {
        Text(
            known?.name ?: tokenIdBase58,
            style = MaterialTheme.typography.labelLarge,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
        if (known != null) {
            Text(
                tokenIdBase58,
                style = MaterialTheme.typography.labelSmall,
                fontFamily = FontFamily.Monospace,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

@Composable
private fun BalanceRow(
    balance: ShieldedTokenPoolBalance,
    token: TokenEntity?,
    showsAccount: Boolean,
    onSend: () -> Unit,
    onUnshield: () -> Unit,
    onUnwatch: () -> Unit,
) {
    var menuOpen by remember { mutableStateOf(false) }
    val tokenIdBase58 = Base58.encode(balance.tokenId)
    Card(
        modifier = Modifier.fillMaxWidth()
            .testTag("shieldedTokens.row.$tokenIdBase58.${balance.accountIndex}"),
    ) {
        Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            if (showsAccount) {
                Text(
                    "Account ${balance.accountIndex}",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Balance", modifier = Modifier.weight(1f))
                Text(
                    shieldedTokenAmountText(balance.balance, token),
                    fontWeight = FontWeight.SemiBold,
                    modifier = Modifier.testTag("shieldedTokens.balance"),
                )
                Box {
                    IconButton(
                        onClick = { menuOpen = true },
                        modifier = Modifier.size(32.dp).testTag("shieldedTokens.rowMenu"),
                    ) {
                        Icon(Icons.Default.MoreVert, contentDescription = "Token pool actions")
                    }
                    DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                        DropdownMenuItem(
                            text = { Text("Unwatch") },
                            onClick = {
                                menuOpen = false
                                onUnwatch()
                            },
                            modifier = Modifier.testTag("shieldedTokens.unwatchButton"),
                        )
                    }
                }
            }
            Text(
                shieldedTokenNoteCountText(balance.unspentNoteCount),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.testTag("shieldedTokens.noteCount"),
            )
            if (balance.reservedBalance > 0uL) {
                Text(
                    "Pending: ${shieldedTokenAmountText(balance.reservedBalance, token)}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.tertiary,
                    modifier = Modifier.testTag("shieldedTokens.pendingBalance"),
                )
            }
            Text(
                shieldedTokenScanText(balance),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.testTag("shieldedTokens.scanState"),
            )
            balance.lastError?.let { error ->
                Text(
                    "Last scan stopped: $error",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.testTag("shieldedTokens.lastError"),
                )
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                TextButton(
                    onClick = onSend,
                    enabled = balance.balance > 0uL,
                    modifier = Modifier.testTag("shieldedTokens.sendButton"),
                ) { Text("Send") }
                TextButton(
                    onClick = onUnshield,
                    enabled = balance.balance > 0uL,
                    modifier = Modifier.testTag("shieldedTokens.unshieldButton"),
                ) { Text("Unshield") }
            }
        }
    }
}

/**
 * Pick one of the app's known tokens on the current network and ask Rust to
 * watch its shielded pool. Rust checks that the token has one (a verified
 * read of its configuration) and refuses it otherwise; the sheet shows Rust's
 * reason. ← Swift `ShieldedTokenPoolWatchSheet`.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ShieldedTokenPoolWatchSheet(
    tokens: List<TokenEntity>,
    watched: List<ShieldedTokenPoolBalance>,
    watchingTokenId: String?,
    watchError: String?,
    onWatch: (TokenEntity) -> Unit,
    onDismiss: () -> Unit,
) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val sortedTokens = remember(tokens) { tokens.sortedBy { it.name } }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        modifier = Modifier.testTag("shieldedTokens.watchSheet"),
    ) {
        LazyColumn(
            contentPadding = PaddingValues(horizontal = 16.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            item {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        "Watch a Token Pool",
                        style = MaterialTheme.typography.titleMedium,
                        modifier = Modifier.weight(1f),
                    )
                    TextButton(
                        onClick = onDismiss,
                        modifier = Modifier.testTag("shieldedTokens.watchSheet.close"),
                    ) { Text("Close") }
                }
            }
            watchError?.let { error ->
                item {
                    Text(
                        error,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("shieldedTokens.watchSheet.error"),
                    )
                }
            }
            if (sortedTokens.isEmpty()) {
                item {
                    Column(
                        modifier = Modifier.fillMaxWidth().padding(vertical = 24.dp)
                            .testTag("shieldedTokens.watchSheet.empty"),
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        Text("No Known Tokens", style = MaterialTheme.typography.titleMedium)
                        Text(
                            "Load a data contract that defines tokens to pick one here.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
            items(sortedTokens, key = { it.id.toHex() }) { token ->
                val tokenKey = token.id.toHex()
                val contractBase58 = Base58.encode(token.contractId)
                val isWatched = watched.any {
                    it.dataContractId.contentEquals(token.contractId) &&
                        it.tokenContractPosition == token.position
                }
                Row(
                    modifier = Modifier.fillMaxWidth()
                        .clickable(enabled = watchingTokenId == null) { onWatch(token) }
                        .padding(vertical = 8.dp)
                        .testTag("shieldedTokens.watchSheet.token.$contractBase58.${token.position}"),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text(token.name)
                        Text(
                            "$contractBase58 #${token.position}",
                            style = MaterialTheme.typography.labelSmall,
                            fontFamily = FontFamily.Monospace,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                    when {
                        watchingTokenId == tokenKey -> CircularProgressIndicator(modifier = Modifier.size(18.dp), strokeWidth = 2.dp)
                        isWatched -> Text(
                            "Watching",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
            item {
                if (sortedTokens.isNotEmpty()) {
                    Text(
                        "Only a token whose configuration has a shielded pool can be watched.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}
