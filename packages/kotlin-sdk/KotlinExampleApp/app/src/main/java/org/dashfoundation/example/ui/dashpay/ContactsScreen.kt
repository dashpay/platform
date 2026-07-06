package org.dashfoundation.example.ui.dashpay

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.DashPayContactDetail
import org.dashfoundation.example.navigation.DashPayHidden
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * Established-contacts list — port of `ContactsView.swift`. A contact is
 * *established* when both direction rows exist for the same
 * `(owner, contact)` pair (the local projection of the Rust `established`
 * map); hidden contacts stay established but leave this list. Rows read
 * cached names/avatars from Room
 * ([org.dashfoundation.dashsdk.persistence.dao.DashpayDao.observeContactProfiles])
 * joined with the device-local alias/DPNS-hint meta store, searchable, with
 * a "Hidden contacts" recovery link and pull-to-refresh.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ContactsScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }

    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val metaStore = remember { DashPayContactMetaStore(context) }
    val metaVersion by metaStore.version.collectAsStateWithLifecycle()

    val rows by remember(idBytes) {
        container.database.dashpayDao().observeContactRequests(idBytes)
    }.collectAsStateWithLifecycle(emptyList())
    val contactProfiles by remember(idBytes) {
        container.database.dashpayDao().observeContactProfiles(idBytes)
    }.collectAsStateWithLifecycle(emptyList())

    var searchText by remember { mutableStateOf("") }
    var isRefreshing by remember { mutableStateOf(false) }

    val profilesByHex = remember(contactProfiles) {
        contactProfiles.associateBy { it.contactIdentityId.toHex() }
    }

    val established = remember(rows, profilesByHex, metaVersion, network) {
        rows.groupBy { it.contactIdentityId.toHex() }
            .mapNotNull { (hex, group) ->
                val hasOutgoing = group.any { it.isOutgoing }
                val hasIncoming = group.any { !it.isOutgoing }
                if (!hasOutgoing || !hasIncoming) return@mapNotNull null
                if (group.any { it.contactHidden }) return@mapNotNull null
                val contactId = group.first().contactIdentityId
                val profile = profilesByHex[hex]
                val dpnsHint = metaStore.dpnsHint(network, idBytes, contactId)
                EstablishedContact(
                    contactId = contactId,
                    displayName = dashPayContactDisplayName(
                        contactId = contactId,
                        alias = group.firstNotNullOfOrNull { it.contactAlias },
                        profileDisplayName = profile?.displayName,
                        dpnsLabel = dpnsHint,
                    ),
                    avatarUrl = profile?.avatarUrl,
                    dpnsName = dpnsHint,
                    paymentChannelBroken = group.any { it.paymentChannelBroken },
                )
            }
            .sortedBy { it.displayName.lowercase() }
    }

    val hasHiddenContacts = remember(rows) {
        rows.groupBy { it.contactIdentityId.toHex() }.any { (_, group) ->
            group.any { it.isOutgoing } && group.any { !it.isOutgoing } &&
                group.any { it.contactHidden }
        }
    }

    val filtered = remember(established, searchText) {
        val trimmed = searchText.trim()
        if (trimmed.isEmpty()) {
            established
        } else {
            established.filter { contact ->
                contact.displayName.contains(trimmed, ignoreCase = true) ||
                    contact.dpnsName?.contains(trimmed, ignoreCase = true) == true ||
                    contact.contactId.toHex().startsWith(trimmed.lowercase())
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Contacts") },
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
                if (filtered.isEmpty() && searchText.isEmpty() && !hasHiddenContacts) {
                    item {
                        DashPayEmptyRow(
                            title = "No contacts yet",
                            message = "Add your first contact to send Dash by username.",
                        )
                    }
                } else {
                    item {
                        SearchField(
                            value = searchText,
                            onValueChange = { searchText = it },
                            onClear = { searchText = "" },
                        )
                    }
                    items(filtered, key = { it.contactId.toHex() }) { contact ->
                        ContactRow(
                            contact = contact,
                            onClick = {
                                navController.navigate(
                                    DashPayContactDetail(
                                        identityIdHex = identityIdHex,
                                        contactIdHex = contact.contactId.toHex(),
                                    ),
                                )
                            },
                        )
                    }
                    if (hasHiddenContacts) {
                        item {
                            ListItem(
                                headlineContent = { Text("Hidden contacts") },
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clickable { navController.navigate(DashPayHidden(idBytes.toHex())) }
                                    .testTag("dashpay.openHidden"),
                            )
                        }
                    }
                }
            }
        }
    }
}

/** UI model for one established contact row. */
data class EstablishedContact(
    val contactId: ByteArray,
    val displayName: String,
    val avatarUrl: String?,
    val dpnsName: String?,
    val paymentChannelBroken: Boolean,
) {
    override fun equals(other: Any?): Boolean =
        other is EstablishedContact && contactId.contentEquals(other.contactId)

    override fun hashCode(): Int = contactId.contentHashCode()
}

@Composable
private fun ContactRow(contact: EstablishedContact, onClick: () -> Unit) {
    ListItem(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onClick() }
            .testTag("dashpay.contact.${Base58.encode(contact.contactId)}"),
        leadingContent = { DashPayAvatar(contact.avatarUrl, contact.displayName) },
        headlineContent = { Text(contact.displayName, style = MaterialTheme.typography.titleMedium) },
        supportingContent = {
            Text(
                contact.dpnsName ?: (contact.contactId.toHex().take(12) + "…"),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        },
        trailingContent = if (contact.paymentChannelBroken) {
            {
                Icon(
                    Icons.Default.Warning,
                    contentDescription = "Payment channel broken",
                    tint = MaterialTheme.colorScheme.error,
                )
            }
        } else {
            null
        },
    )
}

/** Shared inline search row (← Swift `searchField`). */
@Composable
internal fun SearchField(value: String, onValueChange: (String) -> Unit, onClear: () -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = Modifier.fillMaxWidth().testTag("dashpay.search"),
        placeholder = { Text("Search contacts") },
        singleLine = true,
        leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
        trailingIcon = {
            if (value.isNotEmpty()) {
                IconButton(onClick = onClear, modifier = Modifier.testTag("dashpay.search.clear")) {
                    Icon(Icons.Default.Clear, contentDescription = "Clear search")
                }
            }
        },
    )
}

/**
 * Inline empty state — the shared "list empty" row (← Swift
 * `DashPayListEmptyRow`), kept inside the scrollable so pull-to-refresh
 * still works on an empty list.
 */
@Composable
internal fun DashPayEmptyRow(title: String, message: String) {
    Column(
        modifier = Modifier.fillMaxWidth().padding(vertical = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text(title, style = MaterialTheme.typography.titleMedium)
        Text(
            message,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
    }
}
