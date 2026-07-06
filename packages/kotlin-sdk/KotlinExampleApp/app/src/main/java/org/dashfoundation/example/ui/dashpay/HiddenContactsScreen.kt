package org.dashfoundation.example.ui.dashpay

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * Hidden established-contacts list — port of `HiddenContactsView.swift`. The
 * exact complement of the Contacts list: established pairs with any row
 * `contactHidden`. Unhide republishes `contactInfo` with `displayHidden =
 * false` (preserving alias/note) then kicks a sync. Optimistic removal +
 * per-row in-flight/error.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HiddenContactsScreen(ownerIdentityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val ownerBytes = remember(ownerIdentityIdHex) { ownerIdentityIdHex.hexToBytes() }

    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val metaStore = container.dashPayContactMetaStore
    val metaVersion by metaStore.version.collectAsStateWithLifecycle()

    val identity by remember(ownerBytes) {
        container.database.identityDao().observeByIdentityId(ownerBytes)
    }.collectAsStateWithLifecycle(initialValue = null)
    val walletId = identity?.walletId
    val wallet = remember(manager, walletId) { walletId?.let { manager?.wallet(forWalletId = it) } }

    val rows by remember(ownerBytes) {
        container.database.dashpayDao().observeContactRequests(ownerBytes)
    }.collectAsStateWithLifecycle(emptyList())
    val contactProfiles by remember(ownerBytes) {
        container.database.dashpayDao().observeContactProfiles(ownerBytes)
    }.collectAsStateWithLifecycle(emptyList())
    val profilesByHex = remember(contactProfiles) { contactProfiles.associateBy { it.contactIdentityId.toHex() } }

    var inFlightIds by remember { mutableStateOf(emptySet<String>()) }
    var removedOverlayIds by remember { mutableStateOf(emptySet<String>()) }
    var rowErrors by remember { mutableStateOf(emptyMap<String, String>()) }

    // Prune the overlay once Room reflects the unhide (the rows lose their
    // hidden flag): keep a hex only while its contact is still hidden, so a
    // later re-hide within this screen session isn't masked.
    LaunchedEffect(rows) {
        val stillHidden = rows.groupBy { it.contactIdentityId.toHex() }
            .filterValues { group -> group.any { it.contactHidden } }.keys
        removedOverlayIds = removedOverlayIds.filter { it in stillHidden }.toSet()
    }

    val hiddenContacts = remember(rows, profilesByHex, removedOverlayIds, metaVersion, network) {
        rows.groupBy { it.contactIdentityId.toHex() }
            .mapNotNull { (hex, group) ->
                val hasOut = group.any { it.isOutgoing }
                val hasIn = group.any { !it.isOutgoing }
                if (!hasOut || !hasIn || group.none { it.contactHidden }) return@mapNotNull null
                if (removedOverlayIds.contains(hex)) return@mapNotNull null
                val contactId = group.first().contactIdentityId
                val profile = profilesByHex[hex]
                val alias = group.firstNotNullOfOrNull { it.contactAlias }
                HiddenContactItem(
                    contactId = contactId,
                    displayName = dashPayContactDisplayName(
                        contactId = contactId,
                        alias = alias,
                        profileDisplayName = profile?.displayName,
                        dpnsLabel = metaStore.dpnsHint(network, ownerBytes, contactId),
                    ),
                    avatarUrl = profile?.avatarUrl,
                    alias = alias,
                    note = group.firstNotNullOfOrNull { it.contactNote },
                )
            }
            .sortedBy { it.displayName.lowercase() }
    }

    fun unhide(contact: HiddenContactItem) {
        val m = manager ?: return
        val w = wallet ?: return
        val hex = contact.contactId.toHex()
        rowErrors = rowErrors - hex
        inFlightIds = inFlightIds + hex
        scope.launch {
            try {
                w.dashpay.setContactInfo(
                    identityId = ownerBytes,
                    contactId = contact.contactId,
                    alias = contact.alias,
                    note = contact.note,
                    displayHidden = false,
                    signerHandle = m.signerHandle,
                    coreSignerHandle = m.mnemonicResolverHandle,
                )
                removedOverlayIds = removedOverlayIds + hex
                kickDashPaySync(scope, m)
            } catch (e: Exception) {
                rowErrors = rowErrors + (hex to "Unhide failed: ${e.message ?: "unknown error"}")
            } finally {
                inFlightIds = inFlightIds - hex
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Hidden") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            if (hiddenContacts.isEmpty()) {
                item {
                    DashPayEmptyRow(
                        title = "No hidden contacts",
                        message = "Contacts you hide stay payable but leave your Contacts list, " +
                            "and are listed here so you can unhide them.",
                    )
                }
            } else {
                items(hiddenContacts, key = { it.contactId.toHex() }) { contact ->
                    val hex = contact.contactId.toHex()
                    ReversibleContactRow(
                        displayName = contact.displayName,
                        avatarUrl = contact.avatarUrl,
                        isInFlight = inFlightIds.contains(hex),
                        errorMessage = rowErrors[hex],
                        actionLabel = "Unhide",
                        actionTestTag = "dashpay.hidden.unhide",
                        onAction = { unhide(contact) },
                    )
                }
            }
        }
    }
}

/** UI model for one hidden contact — carries alias/note so unhide can republish them. */
private data class HiddenContactItem(
    val contactId: ByteArray,
    val displayName: String,
    val avatarUrl: String?,
    val alias: String?,
    val note: String?,
)
