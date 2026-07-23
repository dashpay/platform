package org.dashfoundation.example.ui.dashpay

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
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * Ignored-senders list — port of `IgnoredContactsView.swift`. Lists every
 * sender this identity has ignored (per-sender mute, reversible, local-only)
 * with an Un-ignore action; names/avatars resolve from the cached
 * contact-profile Room rows. Optimistic removal + per-row in-flight/error.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun IgnoredContactsScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }

    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val identity by remember(idBytes) {
        container.database.identityDao().observeByIdentityId(idBytes)
    }.collectAsStateWithLifecycle(initialValue = null)
    val walletId = identity?.walletId
    val wallet = remember(manager, walletId) { walletId?.let { manager?.wallet(forWalletId = it) } }

    val ignoredRows by remember(idBytes) {
        container.database.dashpayDao().observeIgnoredSenders(idBytes)
    }.collectAsStateWithLifecycle(emptyList())
    val contactProfiles by remember(idBytes) {
        container.database.dashpayDao().observeContactProfiles(idBytes)
    }.collectAsStateWithLifecycle(emptyList())
    val profilesByHex = remember(contactProfiles) { contactProfiles.associateBy { it.contactIdentityId.toHex() } }

    var inFlightIds by remember { mutableStateOf(emptySet<String>()) }
    var removedOverlayIds by remember { mutableStateOf(emptySet<String>()) }
    var rowErrors by remember { mutableStateOf(emptyMap<String, String>()) }

    val visibleRows = remember(ignoredRows, removedOverlayIds) {
        ignoredRows
            .filter { !removedOverlayIds.contains(it.ignoredSenderId.toHex()) }
            .sortedByDescending { it.ignoredAt.time }
    }

    // Prune the overlay once Room reflects the un-ignore (the row is deleted):
    // keep a hex only while its ignored row still exists, so a later re-ignore
    // of the same sender within this screen session isn't masked.
    LaunchedEffect(ignoredRows) {
        val present = ignoredRows.mapTo(HashSet()) { it.ignoredSenderId.toHex() }
        removedOverlayIds = removedOverlayIds.filter { it in present }.toSet()
    }

    fun unignore(senderId: ByteArray) {
        val w = wallet ?: return
        val hex = senderId.toHex()
        rowErrors = rowErrors - hex
        inFlightIds = inFlightIds + hex
        scope.launch {
            try {
                w.dashpay.unignoreContactSender(ourIdentityId = idBytes, contactIdentityId = senderId)
                removedOverlayIds = removedOverlayIds + hex
            } catch (e: Exception) {
                rowErrors = rowErrors + (hex to "Un-ignore failed: ${e.message ?: "unknown error"}")
            } finally {
                inFlightIds = inFlightIds - hex
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Ignored") },
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
            if (visibleRows.isEmpty()) {
                item {
                    DashPayEmptyRow(
                        title = "No ignored contacts",
                        message = "Senders you ignore are hidden from your pending requests and " +
                            "listed here so you can un-ignore them.",
                    )
                }
            } else {
                items(visibleRows, key = { it.ignoredSenderId.toHex() }) { row ->
                    val hex = row.ignoredSenderId.toHex()
                    val profile = profilesByHex[hex]
                    ReversibleContactRow(
                        displayName = dashPayContactDisplayName(row.ignoredSenderId, null, profile?.displayName, null),
                        avatarUrl = profile?.avatarUrl,
                        isInFlight = inFlightIds.contains(hex),
                        errorMessage = rowErrors[hex],
                        actionLabel = "Un-ignore",
                        actionTestTag = "dashpay.ignored.unignore",
                        onAction = { unignore(row.ignoredSenderId) },
                    )
                }
            }
        }
    }
}

/**
 * Shared reversible-mute row (avatar + name + spinner|action + inline error),
 * used by the Ignored and Hidden lists.
 */
@Composable
internal fun ReversibleContactRow(
    displayName: String,
    avatarUrl: String?,
    isInFlight: Boolean,
    errorMessage: String?,
    actionLabel: String,
    actionTestTag: String,
    onAction: () -> Unit,
) {
    Column(
        modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            DashPayAvatar(avatarUrl, displayName)
            Text(displayName, style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            if (isInFlight) {
                CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
            } else {
                OutlinedButton(onClick = onAction, modifier = Modifier.testTag(actionTestTag)) {
                    Text(actionLabel)
                }
            }
        }
        errorMessage?.let {
            Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
        }
    }
}
