package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
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
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.RecipientPicker
import org.dashfoundation.example.ui.components.RecipientSelection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.credits.rememberManagedWalletFor
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * DashPay contacts for an identity — port of `FriendsView.swift`.
 *
 * Hydration mirrors the Swift `loadFriends()` pipeline: sync incoming
 * requests from the network (`ManagedPlatformWallet.dashpay.syncContactRequests`)
 * + fetch sent, then read the three contact-id lists off a fresh managed-
 * identity snapshot (`.contacts(identityId)`), falling back to the Room
 * [org.dashfoundation.dashsdk.persistence.dao.DashpayDao] rows when the wallet
 * isn't loaded or the network read fails. Accept is wired via
 * `.acceptIncomingRequest` (the already-bridged accept over the incoming
 * request handle); Send and Reject were already wired.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FriendsScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }
    val scope = rememberCoroutineScope()

    val identity by container.database.identityDao()
        .observeByIdentityId(idBytes)
        .collectAsStateWithLifecycle(initialValue = null)
    val wallet = rememberManagedWalletFor(identity?.walletId)

    // Room fallback (populated by the platform-wallet sync persister).
    val roomIncoming by container.database.dashpayDao()
        .observeContactRequests(idBytes, isOutgoing = false)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val roomOutgoing by container.database.dashpayDao()
        .observeContactRequests(idBytes, isOutgoing = true)
        .collectAsStateWithLifecycle(initialValue = emptyList())

    // Live hydration from the managed-identity enumeration (preferred when a
    // wallet is loaded). Null until the first hydrate finishes.
    var incomingIds by remember { mutableStateOf<List<ByteArray>?>(null) }
    var outgoingIds by remember { mutableStateOf<List<ByteArray>?>(null) }
    var establishedIds by remember { mutableStateOf<List<ByteArray>?>(null) }

    var recipient by remember { mutableStateOf<RecipientSelection?>(null) }
    var isSending by remember { mutableStateOf(false) }
    var acceptingHex by remember { mutableStateOf<String?>(null) }
    var error by remember { mutableStateOf<String?>(null) }

    // Hydrate from the network + managed identity when the wallet resolves.
    suspend fun hydrate() {
        val w = wallet ?: return
        // Sync from the network (best-effort — fall back to whatever local
        // state exists, matching Swift which reads local state regardless).
        runCatching { w.dashpay.syncContactRequests() }
        runCatching { w.dashpay.fetchSentContactRequests(idBytes) }
        runCatching { w.dashpay.contacts(idBytes) }.getOrNull()?.let { c ->
            incomingIds = c.incoming
            outgoingIds = c.outgoing
            establishedIds = c.established
        }
    }

    LaunchedEffect(wallet) {
        if (wallet != null) hydrate()
    }

    // Effective lists: prefer the live hydration; fall back to Room rows.
    val incoming = incomingIds
        ?: roomIncoming.map { it.contactIdentityId }
    val outgoing = outgoingIds
        ?: roomOutgoing.map { it.contactIdentityId }
    val established = establishedIds ?: emptyList()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Friends") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            // ── Send a contact request (bridged) ──────────────────────────
            FormSection(title = "Send Contact Request") {
                val networkRaw = identity?.networkRaw
                if (networkRaw == null || wallet == null) {
                    Text(
                        "Load this identity's wallet to send requests.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    RecipientPicker(
                        selection = recipient,
                        onSelectionChange = { recipient = it },
                        networkRaw = networkRaw,
                        excludeIdentityIdHex = identityIdHex,
                    )
                    SubmitButton(
                        text = "Send Request",
                        isLoading = isSending,
                        enabled = recipient != null && !isSending,
                        modifier = Modifier.fillMaxWidth().testTag("friends.send"),
                    ) {
                        val recipientId = recipient?.identityIdHex?.hexToBytes()
                            ?: return@SubmitButton
                        isSending = true
                        scope.launch {
                            try {
                                wallet.dashpay.sendContactRequest(
                                    senderIdentityId = idBytes,
                                    recipientIdentityId = recipientId,
                                    signerHandle = requireNotNull(
                                        container.walletManagerStore.activeManager.value,
                                    ).signerHandle,
                                ).close()
                                recipient = null
                                hydrate()
                            } catch (e: Exception) {
                                error = e.message ?: "Send failed"
                            } finally {
                                isSending = false
                            }
                        }
                    }
                }
            }

            // ── Incoming requests (accept + reject bridged) ───────────────
            FormSection(title = "Incoming") {
                if (incoming.isEmpty()) {
                    EmptyRow("No incoming requests.")
                } else {
                    incoming.forEach { senderId ->
                        val senderHex = senderId.toHex()
                        ListItem(
                            headlineContent = { Text(senderHex.take(16) + "…") },
                            supportingContent = { Text("Wants to connect") },
                            trailingContent = {
                                Column(horizontalAlignment = androidx.compose.ui.Alignment.End) {
                                    TextButton(
                                        onClick = {
                                            val w = wallet ?: return@TextButton
                                            acceptingHex = senderHex
                                            scope.launch {
                                                try {
                                                    val ok = w.dashpay.acceptIncomingRequest(
                                                        ourIdentityId = idBytes,
                                                        senderId = senderId,
                                                        signerHandle = requireNotNull(
                                                            container.walletManagerStore
                                                                .activeManager.value,
                                                        ).signerHandle,
                                                    )
                                                    if (!ok) {
                                                        error = "Request from ${senderHex.take(12)}… " +
                                                            "is not in local state — sync first."
                                                    }
                                                    hydrate()
                                                } catch (e: Exception) {
                                                    error = e.message ?: "Accept failed"
                                                } finally {
                                                    acceptingHex = null
                                                }
                                            }
                                        },
                                        enabled = wallet != null && acceptingHex == null,
                                        modifier = Modifier.testTag("friends.accept.$senderHex"),
                                    ) { Text("Accept") }
                                    TextButton(
                                        onClick = {
                                            val w = wallet ?: return@TextButton
                                            scope.launch {
                                                try {
                                                    w.dashpay.rejectContactRequest(
                                                        ourIdentityId = idBytes,
                                                        contactIdentityId = senderId,
                                                    )
                                                    hydrate()
                                                } catch (e: Exception) {
                                                    error = e.message ?: "Reject failed"
                                                }
                                            }
                                        },
                                        modifier = Modifier.testTag("friends.reject.$senderHex"),
                                    ) { Text("Reject") }
                                }
                            },
                        )
                    }
                }
            }

            // ── Outgoing requests ─────────────────────────────────────────
            FormSection(title = "Outgoing") {
                if (outgoing.isEmpty()) {
                    EmptyRow("No outgoing requests.")
                } else {
                    outgoing.forEach { recipientId ->
                        ListItem(
                            headlineContent = { Text(recipientId.toHex().take(16) + "…") },
                            supportingContent = { Text("Request sent") },
                        )
                    }
                }
            }

            // ── Established contacts ──────────────────────────────────────
            FormSection(title = "Contacts") {
                if (established.isEmpty()) {
                    EmptyRow("No established contacts.")
                } else {
                    established.forEach { contactId ->
                        ListItem(
                            headlineContent = { Text(contactId.toHex().take(16) + "…") },
                            supportingContent = { Text("Connected") },
                            modifier = Modifier.testTag("friends.contact.${contactId.toHex()}"),
                        )
                    }
                }
            }
        }
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

@Composable
private fun EmptyRow(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}
