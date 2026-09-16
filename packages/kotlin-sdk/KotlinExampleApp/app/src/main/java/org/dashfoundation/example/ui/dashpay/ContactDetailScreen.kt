package org.dashfoundation.example.ui.dashpay

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.persistence.entities.DashpayPaymentEntity
import org.dashfoundation.dashsdk.tokens.ContactInfoPublishOutcome
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.theme.appStatusColors
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.formatDuffs
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * One contact's detail — port of `ContactDetailView.swift`: profile header,
 * Send Dash (via [SendDashPayPaymentSheet]), Room-driven payment history
 * (refreshed through the durable `refreshDashPayPayments` path), and the
 * device-synced alias / note / hide controls backed by
 * `dashpay.setContactInfo` — surfacing the DIP-15 deferred / watch-only
 * publish outcomes as notices.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ContactDetailScreen(
    identityIdHex: String,
    contactIdHex: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }
    val contactBytes = remember(contactIdHex) { contactIdHex.hexToBytes() }

    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val metaStore = container.dashPayContactMetaStore
    val metaVersion by metaStore.version.collectAsStateWithLifecycle()

    val identity by remember(idBytes) {
        container.database.identityDao().observeByIdentityId(idBytes)
    }.collectAsStateWithLifecycle(initialValue = null)
    val walletId = identity?.walletId

    val allRows by remember(idBytes) {
        container.database.dashpayDao().observeContactRequests(idBytes)
    }.collectAsStateWithLifecycle(emptyList())
    val pairRows = remember(allRows, contactBytes) {
        allRows.filter { it.contactIdentityId.contentEquals(contactBytes) }
    }
    val contactProfile by remember(idBytes, contactBytes) {
        container.database.dashpayDao().observeContactProfile(idBytes, contactBytes)
    }.collectAsStateWithLifecycle(initialValue = null)
    val payments by remember(idBytes, contactBytes) {
        container.database.dashpayDao().observePayments(idBytes, contactBytes)
    }.collectAsStateWithLifecycle(emptyList())
    val sortedPayments = remember(payments) { payments.sortedByDescending { it.createdAt.time } }

    val channelBroken = pairRows.any { it.paymentChannelBroken }
    val localAlias = pairRows.firstNotNullOfOrNull { it.contactAlias }
    val localNote = pairRows.firstNotNullOfOrNull { it.contactNote }
    val contactAccountLabel = pairRows.firstOrNull { !it.isOutgoing }?.contactAccountLabel
    val isHidden = pairRows.any { it.contactHidden }
    val dpnsHint = remember(metaVersion, network) { metaStore.dpnsHint(network, idBytes, contactBytes) }
    val displayName = dashPayContactDisplayName(
        contactId = contactBytes,
        alias = localAlias,
        profileDisplayName = contactProfile?.displayName,
        dpnsLabel = dpnsHint,
    )

    var showPaymentSheet by remember { mutableStateOf(false) }
    var aliasEditorOpen by remember { mutableStateOf(false) }
    var noteEditorOpen by remember { mutableStateOf(false) }
    var isRefreshingPayments by remember { mutableStateOf(false) }
    var paymentsError by remember { mutableStateOf<String?>(null) }
    var isSavingContactInfo by remember { mutableStateOf(false) }
    var contactInfoError by remember { mutableStateOf<String?>(null) }
    var publishNotice by remember { mutableStateOf<String?>(null) }

    fun refreshPayments() {
        val m = manager ?: return
        val wid = walletId ?: run { paymentsError = "Identity has no wallet association"; return }
        if (isRefreshingPayments) return
        isRefreshingPayments = true
        paymentsError = null
        scope.launch {
            try {
                m.refreshDashPayPayments(wid, idBytes)
            } catch (e: Exception) {
                paymentsError = "Payment refresh failed: ${e.message ?: "unknown error"}"
            } finally {
                isRefreshingPayments = false
            }
        }
    }

    fun saveContactInfo(alias: String?, note: String?, hidden: Boolean) {
        val m = manager ?: return
        val wid = walletId ?: run { contactInfoError = "No wallet available for this identity"; return }
        val wallet = m.wallet(forWalletId = wid) ?: run {
            contactInfoError = "No wallet available for this identity"
            return
        }
        isSavingContactInfo = true
        contactInfoError = null
        publishNotice = null
        scope.launch {
            try {
                val outcome = wallet.dashpay.setContactInfo(
                    identityId = idBytes,
                    contactId = contactBytes,
                    alias = alias?.ifBlank { null },
                    note = note?.ifBlank { null },
                    displayHidden = hidden,
                    signerHandle = m.signerHandle,
                    coreSignerHandle = m.mnemonicResolverHandle,
                )
                publishNotice = when (outcome) {
                    ContactInfoPublishOutcome.PUBLISHED -> null
                    ContactInfoPublishOutcome.DEFERRED_UNTIL_TWO_CONTACTS ->
                        "Saved on this device. It will sync to your other devices once this " +
                            "identity has two or more contacts."
                    ContactInfoPublishOutcome.SKIPPED_WATCH_ONLY ->
                        "Saved on this device only — this watch-only identity can't publish to Platform."
                }
            } catch (e: Exception) {
                contactInfoError = "Save failed: ${e.message ?: "unknown error"}"
            } finally {
                isSavingContactInfo = false
            }
        }
    }

    // Key on the manager instance + wallet-id availability, NOT Unit:
    // refreshPayments() no-ops until both `manager` and `walletId` are
    // non-null, and the Room identity emits null first (initialValue), so a
    // one-shot LaunchedEffect(Unit) runs the initial refresh too early, skips
    // the durable refreshDashPayPayments load, and never retries once the
    // wallet association arrives. Re-firing when they become available drives
    // the refresh at the right moment. (Keyed on `walletId != null` rather
    // than the ByteArray so an identity-row re-emit with the same wallet
    // doesn't trigger a redundant refresh.)
    LaunchedEffect(manager, walletId != null) {
        if (manager != null && walletId != null) refreshPayments()
    }
    val syncingFlow = remember(manager) { manager?.dashPaySyncIsSyncing ?: MutableStateFlow(false) }
    val isSyncing by syncingFlow.collectAsStateWithLifecycle(false)
    LaunchedEffect(isSyncing) { if (!isSyncing) refreshPayments() }

    var paymentSending by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(displayName) },
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
            // Header
            FormSection {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                    horizontalArrangement = Arrangement.spacedBy(14.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    DashPayAvatar(contactProfile?.avatarUrl, displayName, size = 56.dp)
                    Column(Modifier.weight(1f)) {
                        Text(displayName, style = MaterialTheme.typography.titleLarge)
                        dpnsHint?.let {
                            Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        Text(
                            Base58.encode(contactBytes).take(20) + "…",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        contactProfile?.publicMessage?.trim()?.takeIf { it.isNotEmpty() }?.let {
                            Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 2)
                        }
                    }
                }
                localNote?.let { LabeledContent("Note", it) }
                contactAccountLabel?.let { LabeledContent("Their account", it) }
            }

            // Send
            FormSection {
                ListItem(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable(enabled = !channelBroken) { showPaymentSheet = true }
                        .testTag("dashpay.detail.sendDash"),
                    leadingContent = { Icon(Icons.AutoMirrored.Filled.Send, contentDescription = null) },
                    headlineContent = {
                        Text(
                            "Send Dash",
                            color = if (channelBroken) MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.onSurface,
                        )
                    },
                )
                if (channelBroken) {
                    Text(
                        "Payment channel broken — ask the contact to send a new request",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.tertiary,
                    )
                }
            }

            // Payments
            FormSection(title = "Payments (${sortedPayments.size})") {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.End,
                ) {
                    IconButton(
                        onClick = { refreshPayments() },
                        enabled = !isRefreshingPayments,
                        modifier = Modifier.testTag("dashpay.detail.refreshPayments"),
                    ) {
                        if (isRefreshingPayments) {
                            CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
                        } else {
                            Icon(Icons.Default.Refresh, contentDescription = "Refresh payments")
                        }
                    }
                }
                if (sortedPayments.isEmpty()) {
                    Text(
                        if (isRefreshingPayments) "Loading payments…" else "No payments yet",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    sortedPayments.forEach { PaymentHistoryRow(it) }
                }
                paymentsError?.let {
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                }
            }

            // Local settings
            FormSection(title = "Contact settings") {
                ListItem(
                    modifier = Modifier.fillMaxWidth().clickable { aliasEditorOpen = true }.testTag("dashpay.detail.aliasEdit"),
                    headlineContent = { Text("Alias") },
                    trailingContent = { Text(localAlias ?: "None", color = MaterialTheme.colorScheme.onSurfaceVariant) },
                )
                ListItem(
                    modifier = Modifier.fillMaxWidth().clickable { noteEditorOpen = true }.testTag("dashpay.detail.noteEdit"),
                    headlineContent = { Text("Note") },
                    trailingContent = { Text(if (localNote == null) "None" else "Edit", color = MaterialTheme.colorScheme.onSurfaceVariant) },
                )
                Row(
                    modifier = Modifier.fillMaxWidth().padding(vertical = 6.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text("Hide contact")
                    Switch(
                        checked = isHidden,
                        onCheckedChange = { saveContactInfo(localAlias, localNote, it) },
                        enabled = !isSavingContactInfo,
                        modifier = Modifier.testTag("dashpay.detail.hideToggle"),
                    )
                }
                if (isSavingContactInfo) {
                    Text("Saving…", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                contactInfoError?.let {
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                }
                publishNotice?.let {
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.tertiary)
                }
                Text(
                    "Alias, note and hide are encrypted and synced to your other devices via " +
                        "Platform once this identity has two or more contacts.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }

    if (showPaymentSheet && manager != null && walletId != null) {
        // Block interactive dismissal (swipe / scrim / back) while a send is
        // in flight, so the sheet can't be torn down mid-broadcast (the send
        // itself is also NonCancellable — defense in depth).
        val sheetState = rememberModalBottomSheetState(confirmValueChange = { !paymentSending })
        ModalBottomSheet(
            onDismissRequest = { if (!paymentSending) showPaymentSheet = false },
            sheetState = sheetState,
        ) {
            SendDashPayPaymentSheet(
                manager = manager!!,
                walletId = walletId!!,
                senderIdentityId = idBytes,
                contactId = contactBytes,
                contactDisplayName = displayName,
                contactDpnsName = dpnsHint,
                onSendingChange = { paymentSending = it },
                onSent = { refreshPayments() },
                onClose = { showPaymentSheet = false },
            )
        }
    }

    if (aliasEditorOpen) {
        LocalFieldEditor(
            title = "Alias",
            prompt = "e.g. Mom",
            initialValue = localAlias ?: "",
            identifierPrefix = "dashpay.detail.alias",
            onDismiss = { aliasEditorOpen = false },
            onSave = { value ->
                aliasEditorOpen = false
                saveContactInfo(value, localNote, isHidden)
            },
        )
    }
    if (noteEditorOpen) {
        LocalFieldEditor(
            title = "Note",
            prompt = "Anything to remember about this contact",
            initialValue = localNote ?: "",
            identifierPrefix = "dashpay.detail.note",
            onDismiss = { noteEditorOpen = false },
            onSave = { value ->
                noteEditorOpen = false
                saveContactInfo(localAlias, value, isHidden)
            },
        )
    }
}

@Composable
private fun PaymentHistoryRow(payment: DashpayPaymentEntity) {
    val sent = payment.directionRaw == 0
    Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 2.dp),
        horizontalArrangement = Arrangement.spacedBy(10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(
                if (sent) "Sent" else "Received",
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.Medium,
            )
            Text(
                payment.txid.take(16) + "…",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            payment.memo?.takeIf { it.isNotEmpty() }?.let {
                Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1)
            }
        }
        Column(horizontalAlignment = Alignment.End) {
            Text(formatDuffs(payment.amountDuffs), style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.SemiBold)
            Text(
                statusLabel(payment.statusRaw),
                style = MaterialTheme.typography.bodySmall,
                color = statusColor(payment.statusRaw),
            )
        }
    }
}

@Composable
private fun statusColor(statusRaw: Int) = when (statusRaw) {
    1 -> appStatusColors.success
    2 -> MaterialTheme.colorScheme.error
    else -> MaterialTheme.colorScheme.tertiary
}

private fun statusLabel(statusRaw: Int) = when (statusRaw) {
    1 -> "Confirmed"
    2 -> "Failed"
    else -> "Pending"
}

@Composable
private fun LocalFieldEditor(
    title: String,
    prompt: String,
    initialValue: String,
    identifierPrefix: String,
    onDismiss: () -> Unit,
    onSave: (String?) -> Unit,
) {
    var value by remember { mutableStateOf(initialValue) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            OutlinedTextField(
                value = value,
                onValueChange = { value = it },
                modifier = Modifier.fillMaxWidth().testTag("$identifierPrefix.field"),
                placeholder = { Text(prompt) },
                singleLine = true,
            )
        },
        confirmButton = {
            TextButton(
                onClick = { onSave(value.trim().ifEmpty { null }) },
                modifier = Modifier.testTag("$identifierPrefix.save"),
            ) { Text("Save") }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, modifier = Modifier.testTag("$identifierPrefix.cancel")) {
                Text("Cancel")
            }
        },
    )
}
