package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.dpns.DpnsMarketplaceName
import org.dashfoundation.dashsdk.dpns.DpnsNameHistoryEvent
import org.dashfoundation.dashsdk.persistence.entities.DpnsNameEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.hexToBytes
import java.text.DateFormat
import java.util.Date

private sealed interface MarketplaceAction {
    val name: String
    data class SetPrice(override val name: String) : MarketplaceAction
    data class Delist(override val name: String) : MarketplaceAction
    data class Transfer(override val name: String) : MarketplaceAction
    data class Purchase(
        override val name: String,
        val expectedPriceCredits: ULong,
    ) : MarketplaceAction
}

/** End-to-end DPNS marketplace example using only wallet-level SDK APIs. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DpnsMarketplaceScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val identityId = remember(identityIdHex) { identityIdHex.hexToBytes() }
    val identity by container.database.identityDao().observeByIdentityId(identityId)
        .collectAsStateWithLifecycle(initialValue = null)
    val durableNames by container.database.dpnsNameDao().observeMarketplaceByIdentity(identityId)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val wallet = identity?.walletId?.let { manager?.wallet(it) }
    val scope = rememberCoroutineScope()

    var prefix by remember { mutableStateOf("") }
    var results by remember { mutableStateOf<List<DpnsMarketplaceName>>(emptyList()) }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var lastSyncMs by remember { mutableStateOf(0L) }
    var syncMessage by remember { mutableStateOf<String?>(null) }
    var pendingAction by remember { mutableStateOf<MarketplaceAction?>(null) }
    var actionPrice by remember { mutableStateOf("") }
    var actionRecipient by remember { mutableStateOf("") }
    val histories = remember { mutableStateMapOf<String, List<DpnsNameHistoryEvent>>() }

    LaunchedEffect(manager) {
        lastSyncMs = (manager?.dpnsLastSyncUnixSeconds() ?: 0L) * 1_000L
    }

    fun launch(block: suspend () -> Unit) {
        if (busy) return
        busy = true
        scope.launch {
            try {
                block()
            } catch (t: Throwable) {
                error = t.message ?: "DPNS marketplace operation failed"
            } finally {
                busy = false
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("DPNS Marketplace") },
                navigationIcon = {
                    IconButton(onClick = navController::popBackStack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        enabled = wallet != null && !busy,
                        modifier = Modifier.testTag("dpnsMarketplace.refresh"),
                        onClick = {
                            val activeManager = manager ?: return@IconButton
                            val activeWallet = wallet ?: return@IconButton
                            launch {
                                val summary = activeManager.dpnsMarketplace.sync(activeWallet.handle)
                                lastSyncMs = summary.syncUnixMs
                                syncMessage = "${summary.tracked} tracked, ${summary.added.size} added, " +
                                    "${summary.departed.size} departed, ${summary.pricesChanged.size} repriced"
                            }
                        },
                    ) {
                        if (busy) CircularProgressIndicator() else {
                            Icon(Icons.Filled.Refresh, contentDescription = "Sync marketplace")
                        }
                    }
                },
            )
        },
    ) { padding ->
        Column(
            Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Browse") {
                OutlinedTextField(
                    value = prefix,
                    onValueChange = { prefix = it },
                    label = { Text("Name prefix (empty browses all)") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().testTag("dpnsMarketplace.searchField"),
                )
                Button(
                    enabled = wallet != null && !busy,
                    modifier = Modifier.testTag("dpnsMarketplace.search"),
                    onClick = {
                        val activeManager = manager ?: return@Button
                        val activeWallet = wallet ?: return@Button
                        launch {
                            results = activeManager.dpnsMarketplace.search(
                                activeWallet.handle,
                                prefix.trim(),
                                50,
                            )
                        }
                    },
                ) { Text("Search") }
                if (wallet == null) {
                    Text("This identity does not have an active local wallet.", color = MaterialTheme.colorScheme.error)
                }
                results.forEach { row ->
                    MarketplaceResultCard(
                        row = row,
                        isMine = row.ownerId.contentEquals(identityId),
                        onPurchase = row.priceCredits?.let { price ->
                            { pendingAction = MarketplaceAction.Purchase(row.label, price) }
                        },
                    )
                }
                if (results.isEmpty()) {
                    Text("Search by prefix or browse all names.", color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }

            FormSection(title = "My Names") {
                if (durableNames.isEmpty()) {
                    Text("No marketplace state yet. Tap refresh to sync.")
                }
                durableNames.forEach { row ->
                    OwnedNameCard(
                        row = row,
                        history = histories[row.normalizedLabel],
                        onSetPrice = { pendingAction = MarketplaceAction.SetPrice(row.label) },
                        onDelist = { pendingAction = MarketplaceAction.Delist(row.label) },
                        onTransfer = { pendingAction = MarketplaceAction.Transfer(row.label) },
                        onHistory = history@{
                            val activeManager = manager ?: return@history
                            val activeWallet = wallet ?: return@history
                            launch {
                                histories[row.normalizedLabel] =
                                    activeManager.dpnsMarketplace.history(activeWallet.handle, row.label)
                            }
                        },
                    )
                }
            }

            FormSection(title = "Sync") {
                Text(
                    if (lastSyncMs == 0L) "Not synced yet"
                    else "Last synced ${DateFormat.getDateTimeInstance().format(Date(lastSyncMs))}",
                    modifier = Modifier.testTag("dpnsMarketplace.lastSync"),
                )
                syncMessage?.let { Text(it, style = MaterialTheme.typography.bodySmall) }
            }
        }
    }

    pendingAction?.let { action ->
        MarketplaceConfirmationDialog(
            action = action,
            price = actionPrice,
            recipient = actionRecipient,
            onPriceChange = { actionPrice = it.filter(Char::isDigit) },
            onRecipientChange = { actionRecipient = it },
            onDismiss = { pendingAction = null },
            onConfirm = {
                val activeManager = manager
                val activeWallet = wallet
                if (activeManager == null || activeWallet == null) {
                    error = "The wallet manager is unavailable"
                    pendingAction = null
                    return@MarketplaceConfirmationDialog
                }
                launch {
                    when (action) {
                        is MarketplaceAction.SetPrice -> {
                            val credits = actionPrice.toULongOrNull()
                                ?: throw IllegalArgumentException("Enter a price in credits")
                            activeManager.dpnsMarketplace.setPrice(
                                activeWallet.handle, identityId, action.name, credits,
                                activeManager.signerHandle,
                            )
                        }
                        is MarketplaceAction.Delist -> activeManager.dpnsMarketplace.delist(
                            activeWallet.handle, identityId, action.name, activeManager.signerHandle,
                        )
                        is MarketplaceAction.Transfer -> {
                            val recipient = Base58.decodeIdentifier(actionRecipient)
                                ?: throw IllegalArgumentException("Enter a valid recipient identity")
                            activeManager.dpnsMarketplace.transfer(
                                activeWallet.handle, identityId, action.name, recipient,
                                activeManager.signerHandle,
                            )
                        }
                        is MarketplaceAction.Purchase -> activeManager.dpnsMarketplace.purchase(
                            activeWallet.handle, identityId, action.name, action.expectedPriceCredits,
                            activeManager.signerHandle,
                        )
                    }
                    val summary = activeManager.dpnsMarketplace.sync(activeWallet.handle)
                    lastSyncMs = summary.syncUnixMs
                    if (results.isNotEmpty()) {
                        results = activeManager.dpnsMarketplace.search(activeWallet.handle, prefix.trim(), 50)
                    }
                }
                pendingAction = null
                actionPrice = ""
                actionRecipient = ""
            },
        )
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

@Composable
private fun MarketplaceResultCard(
    row: DpnsMarketplaceName,
    isMine: Boolean,
    onPurchase: (() -> Unit)?,
) {
    Card(Modifier.fillMaxWidth().testTag("dpnsMarketplace.search.${row.normalizedLabel}")) {
        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text("${row.label}.dash", style = MaterialTheme.typography.titleMedium)
            Text(if (isMine) "Owned by this identity" else "Owner ${Base58.encode(row.ownerId)}")
            Text(row.priceCredits?.let(::formatCredits) ?: "Not for sale")
            if (!isMine && onPurchase != null) {
                TextButton(
                    onClick = onPurchase,
                    modifier = Modifier.testTag("dpnsMarketplace.buy.${row.normalizedLabel}"),
                ) { Text("Buy") }
            }
        }
    }
}

@Composable
private fun OwnedNameCard(
    row: DpnsNameEntity,
    history: List<DpnsNameHistoryEvent>?,
    onSetPrice: () -> Unit,
    onDelist: () -> Unit,
    onTransfer: () -> Unit,
    onHistory: () -> Unit,
) {
    Card(Modifier.fillMaxWidth().testTag("dpnsMarketplace.owned.${row.normalizedLabel}")) {
        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text("${row.label}.${row.parentDomainName}", style = MaterialTheme.typography.titleMedium)
            Text(
                when (row.saleStatusRaw) {
                    0 -> row.priceCredits
                        ?.let { "Owned · ${formatCredits(it.toULong())}" }
                        ?: "Owned · not listed"
                    1 -> "Sold"
                    2 -> "Transferred"
                    else -> "Unknown status"
                },
            )
            row.counterpartyIdentityId?.let { Text("Counterparty ${Base58.encode(it)}") }
            Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                if (row.isOwned) {
                    TextButton(onClick = onSetPrice) { Text(if (row.priceCredits == null) "List" else "Reprice") }
                    if (row.priceCredits != null) TextButton(onClick = onDelist) { Text("Delist") }
                    TextButton(onClick = onTransfer) { Text("Transfer") }
                }
                TextButton(onClick = onHistory) { Text("History") }
            }
            history?.forEach { event ->
                Text(
                    "${event.kind.name.lowercase().replace('_', ' ')} · " +
                        DateFormat.getDateTimeInstance().format(Date(event.atMs)) +
                        (event.priceCredits?.let { " · ${formatCredits(it)}" } ?: ""),
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }
    }
}

@Composable
private fun MarketplaceConfirmationDialog(
    action: MarketplaceAction,
    price: String,
    recipient: String,
    onPriceChange: (String) -> Unit,
    onRecipientChange: (String) -> Unit,
    onDismiss: () -> Unit,
    onConfirm: () -> Unit,
) {
    val title = when (action) {
        is MarketplaceAction.SetPrice -> "List or reprice ${action.name}.dash"
        is MarketplaceAction.Delist -> "Delist ${action.name}.dash"
        is MarketplaceAction.Transfer -> "Transfer ${action.name}.dash"
        is MarketplaceAction.Purchase -> "Purchase ${action.name}.dash"
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                when (action) {
                    is MarketplaceAction.SetPrice -> {
                        OutlinedTextField(
                            value = price,
                            onValueChange = onPriceChange,
                            label = { Text("Price in credits") },
                            modifier = Modifier.testTag("dpnsMarketplace.price"),
                        )
                        price.toULongOrNull()?.let { Text(formatCredits(it)) }
                    }
                    is MarketplaceAction.Transfer -> OutlinedTextField(
                        value = recipient,
                        onValueChange = onRecipientChange,
                        label = { Text("Recipient identity (Base58 or hex)") },
                        modifier = Modifier.testTag("dpnsMarketplace.recipient"),
                    )
                    is MarketplaceAction.Purchase -> Text(
                        "Confirm the exact listed price: ${formatCredits(action.expectedPriceCredits)}. " +
                            "If the seller changes it, the purchase fails without executing.",
                    )
                    is MarketplaceAction.Delist -> Text("This removes the sale price but keeps ownership.")
                }
                Text("Your configured authentication may be requested to sign.")
            }
        },
        confirmButton = {
            TextButton(onClick = onConfirm, modifier = Modifier.testTag("dpnsMarketplace.confirm")) {
                Text("Confirm")
            }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

private fun formatCredits(credits: ULong): String {
    val whole = credits / 1_000u
    val fraction = (credits % 1_000u).toString().padStart(3, '0')
    return "$credits credits ($whole.$fraction duffs)"
}
