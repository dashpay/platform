package org.dashfoundation.example.ui.dashpay

import android.text.format.DateUtils
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
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
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.SectionHeader
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * Incoming + outgoing contact requests — port of `ContactRequestsView.swift`.
 * Incoming rows carry Accept / Ignore with per-row in-flight state + inline
 * error; the Outgoing section shows pending sent requests. Rows are derived
 * from the Room contact-request rows: a `(owner, contact)` pair with only an
 * incoming row is a pending incoming request, only an outgoing row a pending
 * sent one, and both directions an established contact (shown in Contacts).
 *
 * Deviation from Swift: the cross-screen optimistic *sent* overlay is dropped
 * (AddContact is a separate route here, not a child sharing a `@Binding`); a
 * sent request appears once the post-send sync persists its outgoing row.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ContactRequestsScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }

    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val metaStore = container.dashPayContactMetaStore
    val metaVersion by metaStore.version.collectAsStateWithLifecycle()

    val identity by remember(idBytes) {
        container.database.identityDao().observeByIdentityId(idBytes)
    }.collectAsStateWithLifecycle(initialValue = null)
    val walletId = identity?.walletId
    val wallet = remember(manager, walletId) {
        walletId?.let { manager?.wallet(forWalletId = it) }
    }

    val rows by remember(idBytes) {
        container.database.dashpayDao().observeContactRequests(idBytes)
    }.collectAsStateWithLifecycle(emptyList())
    val contactProfiles by remember(idBytes) {
        container.database.dashpayDao().observeContactProfiles(idBytes)
    }.collectAsStateWithLifecycle(emptyList())
    val profilesByHex = remember(contactProfiles) {
        contactProfiles.associateBy { it.contactIdentityId.toHex() }
    }

    var inFlightIds by remember { mutableStateOf(emptySet<String>()) }
    var removedOverlayIds by remember { mutableStateOf(emptySet<String>()) }
    var rowErrors by remember { mutableStateOf(emptyMap<String, String>()) }
    var isRefreshing by remember { mutableStateOf(false) }

    // Prune the optimistic-removal overlay against the backing query: keep a
    // hex only while its pair is STILL incoming-only, so an entry is dropped
    // exactly when Room reflects the action's own change (accept promotes the
    // pair to established → it gains an outgoing row; ignore deletes the row →
    // the group disappears). This is scoped to the row change itself, so an
    // unrelated sweep completing can no longer flash the row back.
    LaunchedEffect(rows) {
        val byContact = rows.groupBy { it.contactIdentityId.toHex() }
        removedOverlayIds = removedOverlayIds.filter { hex ->
            val group = byContact[hex] ?: return@filter false
            group.any { !it.isOutgoing } && group.none { it.isOutgoing }
        }.toSet()
    }

    fun displayNameFor(contactId: ByteArray): String = dashPayContactDisplayName(
        contactId = contactId,
        alias = metaStore.alias(network, idBytes, contactId),
        profileDisplayName = profilesByHex[contactId.toHex()]?.displayName,
        dpnsLabel = metaStore.dpnsHint(network, idBytes, contactId),
    )

    val incomingPending = remember(rows, removedOverlayIds, profilesByHex, metaVersion, network) {
        rows.groupBy { it.contactIdentityId.toHex() }
            .mapNotNull { (hex, group) ->
                if (removedOverlayIds.contains(hex)) return@mapNotNull null
                if (group.any { it.isOutgoing }) return@mapNotNull null
                val incoming = group.firstOrNull { !it.isOutgoing } ?: return@mapNotNull null
                RequestRowItem(
                    contactId = incoming.contactIdentityId,
                    displayName = displayNameFor(incoming.contactIdentityId),
                    // Privacy: never load an unsolicited sender's avatar before
                    // the user accepts (an image GET leaks the recipient's IP).
                    avatarUrl = null,
                    createdAtMillis = incoming.createdAtMillis,
                )
            }
            .sortedByDescending { it.createdAtMillis }
    }

    val outgoingPending = remember(rows, profilesByHex, metaVersion, network) {
        rows.groupBy { it.contactIdentityId.toHex() }
            .mapNotNull { (hex, group) ->
                if (group.any { !it.isOutgoing }) return@mapNotNull null
                val outgoing = group.firstOrNull { it.isOutgoing } ?: return@mapNotNull null
                RequestRowItem(
                    contactId = outgoing.contactIdentityId,
                    displayName = displayNameFor(outgoing.contactIdentityId),
                    avatarUrl = profilesByHex[hex]?.avatarUrl,
                    createdAtMillis = outgoing.createdAtMillis,
                )
            }
            .sortedByDescending { it.createdAtMillis }
    }

    fun accept(contactId: ByteArray) {
        val w = wallet ?: return
        val m = manager ?: return
        val hex = contactId.toHex()
        rowErrors = rowErrors - hex
        inFlightIds = inFlightIds + hex
        scope.launch {
            try {
                val ok = w.dashpay.acceptIncomingRequest(
                    ourIdentityId = idBytes,
                    senderId = contactId,
                    signerHandle = m.signerHandle,
                    coreSignerHandle = m.mnemonicResolverHandle,
                )
                if (ok) {
                    removedOverlayIds = removedOverlayIds + hex
                    kickDashPaySync(scope, m)
                } else {
                    rowErrors = rowErrors + (hex to "Request not in local state — pull to refresh")
                }
            } catch (e: Exception) {
                rowErrors = rowErrors + (hex to "Accept failed: ${e.message ?: "unknown error"}")
            } finally {
                inFlightIds = inFlightIds - hex
            }
        }
    }

    fun ignore(contactId: ByteArray) {
        val w = wallet ?: return
        val hex = contactId.toHex()
        rowErrors = rowErrors - hex
        inFlightIds = inFlightIds + hex
        scope.launch {
            try {
                w.dashpay.ignoreContactSender(ourIdentityId = idBytes, contactIdentityId = contactId)
                removedOverlayIds = removedOverlayIds + hex
            } catch (e: Exception) {
                rowErrors = rowErrors + (hex to "Ignore failed: ${e.message ?: "unknown error"}")
            } finally {
                inFlightIds = inFlightIds - hex
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Requests") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        PullToRefreshBox(
            isRefreshing = isRefreshing,
            onRefresh = {
                scope.launch {
                    isRefreshing = true
                    manager?.let { attachOrStartSync(it) }
                    isRefreshing = false
                }
            },
            modifier = Modifier.fillMaxSize().padding(padding),
        ) {
            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                if (incomingPending.isEmpty() && outgoingPending.isEmpty()) {
                    item {
                        DashPayEmptyRow(
                            title = "No pending requests",
                            message = "Incoming contact requests and your pending sent requests " +
                                "show up here.",
                        )
                    }
                } else {
                    if (incomingPending.isNotEmpty()) {
                        item { SectionHeader("Incoming (${incomingPending.size})") }
                        items(incomingPending, key = { "in:${it.contactId.toHex()}" }) { row ->
                            val hex = row.contactId.toHex()
                            IncomingRequestRow(
                                item = row,
                                isInFlight = inFlightIds.contains(hex),
                                errorMessage = rowErrors[hex],
                                onAccept = { accept(row.contactId) },
                                onIgnore = { ignore(row.contactId) },
                            )
                        }
                    }
                    if (outgoingPending.isNotEmpty()) {
                        item { SectionHeader("Outgoing (${outgoingPending.size})") }
                        items(outgoingPending, key = { "out:${it.contactId.toHex()}" }) { row ->
                            OutgoingRequestRow(row)
                        }
                    }
                }
            }
        }
    }
}

/** UI model for one request row (incoming or outgoing). */
private data class RequestRowItem(
    val contactId: ByteArray,
    val displayName: String,
    val avatarUrl: String?,
    val createdAtMillis: Long,
)

@Composable
private fun IncomingRequestRow(
    item: RequestRowItem,
    isInFlight: Boolean,
    errorMessage: String?,
    onAccept: () -> Unit,
    onIgnore: () -> Unit,
) {
    Column(
        modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            DashPayAvatar(item.avatarUrl, item.displayName)
            Column(Modifier.weight(1f)) {
                Text(item.displayName, style = MaterialTheme.typography.titleMedium)
                Text(
                    relativeTimestamp(item.createdAtMillis),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        if (isInFlight) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.Center) {
                CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
            }
        } else {
            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Button(onClick = onAccept, modifier = Modifier.testTag("dashpay.request.accept")) {
                    Text("Accept")
                }
                OutlinedButton(
                    onClick = onIgnore,
                    modifier = Modifier.testTag("dashpay.request.ignore"),
                ) {
                    Text("Ignore")
                }
            }
        }
        if (errorMessage != null) {
            Text(
                errorMessage,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
            )
        }
    }
}

@Composable
private fun OutgoingRequestRow(item: RequestRowItem) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        DashPayAvatar(item.avatarUrl, item.displayName)
        Column(Modifier.weight(1f)) {
            Text(item.displayName, style = MaterialTheme.typography.titleMedium)
            Text(
                relativeTimestamp(item.createdAtMillis),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Text(
            "Pending",
            style = MaterialTheme.typography.labelMedium,
            fontWeight = FontWeight.Medium,
            color = MaterialTheme.colorScheme.tertiary,
        )
    }
}

/** "3 min. ago"-style relative time from Unix millis; "—" for the zero sentinel. */
private fun relativeTimestamp(millis: Long): String {
    if (millis <= 0) return "—"
    return DateUtils.getRelativeTimeSpanString(
        millis,
        System.currentTimeMillis(),
        DateUtils.MINUTE_IN_MILLIS,
    ).toString()
}
