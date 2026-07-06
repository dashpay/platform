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
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material3.AlertDialog
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
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.QrScanner
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

private enum class AddMode { DPNS, IDENTITY_ID }

private sealed interface SearchState {
    data object Idle : SearchState
    data object Searching : SearchState
    data object NotFound : SearchState
    data class Found(val results: List<DpnsSearchResult>) : SearchState
}

/**
 * Add-a-contact form — port of `AddContactView.swift`. Two entry modes:
 * DPNS username (300 ms-debounced live prefix search) and a pasted base58
 * identity id (inline 32-byte validation). A resolved target renders a
 * preview card that gates Send; sending a request the target already sent
 * *you* surfaces a collision dialog (Accept vs Continue anyway). A QR entry
 * scans a DIP-15 auto-accept code and sends the request it describes.
 *
 * Deviation from Swift: the DPNS hint recorded on success is written to the
 * device-local meta store directly here (Swift funnels it through the tab's
 * `onSent`); the cross-screen optimistic-send overlay is not reproduced.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AddContactScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }

    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val metaStore = container.dashPayContactMetaStore

    val identity by remember(idBytes) {
        container.database.identityDao().observeByIdentityId(idBytes)
    }.collectAsStateWithLifecycle(initialValue = null)
    val walletId = identity?.walletId
    val wallet = remember(manager, walletId) {
        walletId?.let { manager?.wallet(forWalletId = it) }
    }

    var mode by remember { mutableStateOf(AddMode.DPNS) }
    var searchText by remember { mutableStateOf("") }
    var searchState by remember { mutableStateOf<SearchState>(SearchState.Idle) }
    var selectedResult by remember { mutableStateOf<DpnsSearchResult?>(null) }
    var idText by remember { mutableStateOf("") }
    var accountLabel by remember { mutableStateOf("") }
    var isSending by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var collisionRecipient by remember { mutableStateOf<ByteArray?>(null) }
    var previewProfile by remember { mutableStateOf<DashPayProfile?>(null) }

    val parsedIdentityId = remember(idText) { Base58.decodeIdentifier(idText) }
    val resolvedRecipient: ByteArray? = when (mode) {
        AddMode.DPNS -> selectedResult?.identityId
        AddMode.IDENTITY_ID -> parsedIdentityId
    }
    val recipientIsSelf = resolvedRecipient?.contentEquals(idBytes) == true
    val canSend = resolvedRecipient != null && !recipientIsSelf && !isSending

    // Scan-to-send: consume the QR string handed back by the scanner screen
    // (the house pattern — observe the saved-state flow, then null it out).
    val savedStateHandle = navController.currentBackStackEntry?.savedStateHandle
    LaunchedEffect(savedStateHandle, wallet, manager) {
        savedStateHandle
            ?.getStateFlow<String?>(QrScanner.RESULT_KEY, null)
            ?.collect { raw ->
                if (raw == null) return@collect
                savedStateHandle[QrScanner.RESULT_KEY] = null
                val w = wallet ?: return@collect
                val m = manager ?: return@collect
                isSending = true
                errorMessage = null
                try {
                    w.dashpay.sendContactRequestFromQr(
                        senderIdentityId = idBytes,
                        uri = raw.trim(),
                        signerHandle = m.signerHandle,
                        coreSignerHandle = m.mnemonicResolverHandle,
                    ).close()
                    kickDashPaySync(scope, m)
                    navController.popBackStack()
                } catch (e: Exception) {
                    errorMessage = "QR request failed: ${e.message ?: "unknown error"}"
                } finally {
                    isSending = false
                }
            }
    }

    // Debounced (~300 ms) DPNS prefix search. Re-keying on searchText cancels
    // the prior lookup for free; min 2 chars.
    LaunchedEffect(searchText) {
        selectedResult = null
        val trimmed = searchText.trim()
        if (trimmed.length < 2) {
            searchState = SearchState.Idle
            return@LaunchedEffect
        }
        delay(300)
        val w = wallet
        if (w == null) {
            searchState = SearchState.Idle
            errorMessage = "No wallet available for this identity"
            return@LaunchedEffect
        }
        searchState = SearchState.Searching
        try {
            val results = parseDpnsSearchResults(w.dashpay.searchDpnsNames(trimmed, 10))
            searchState = if (results.isEmpty()) SearchState.NotFound else SearchState.Found(results)
        } catch (e: Exception) {
            searchState = SearchState.Idle
            errorMessage = "Search failed: ${e.message ?: "unknown error"}"
        }
    }

    // Cache-only profile for the preview card (local read; most unknown
    // identities won't have one).
    val recipientHex = resolvedRecipient?.toHex()
    LaunchedEffect(recipientHex) {
        val recipient = resolvedRecipient
        val w = wallet
        previewProfile = if (recipient != null && w != null) {
            parseDashPayProfile(
                w.dashpay.getContactProfile(idBytes, recipient)
                    ?: w.dashpay.getProfile(recipient),
            )
        } else {
            null
        }
    }

    fun send(recipient: ByteArray) {
        val w = wallet ?: return
        val m = manager ?: return
        isSending = true
        errorMessage = null
        scope.launch {
            try {
                val label = accountLabel.trim().ifEmpty { null }
                w.dashpay.sendContactRequest(
                    senderIdentityId = idBytes,
                    recipientIdentityId = recipient,
                    signerHandle = m.signerHandle,
                    coreSignerHandle = m.mnemonicResolverHandle,
                    accountLabel = label,
                ).close()
                if (mode == AddMode.DPNS) {
                    selectedResult?.label?.let {
                        metaStore.setDpnsHint(it, network, idBytes, recipient)
                    }
                }
                kickDashPaySync(scope, m)
                navController.popBackStack()
            } catch (e: Exception) {
                errorMessage = e.message ?: "Send failed"
            } finally {
                isSending = false
            }
        }
    }

    fun acceptIncoming(recipient: ByteArray) {
        val w = wallet ?: return
        val m = manager ?: return
        isSending = true
        errorMessage = null
        scope.launch {
            try {
                val ok = w.dashpay.acceptIncomingRequest(
                    ourIdentityId = idBytes,
                    senderId = recipient,
                    signerHandle = m.signerHandle,
                    coreSignerHandle = m.mnemonicResolverHandle,
                )
                if (ok) {
                    kickDashPaySync(scope, m)
                    navController.popBackStack()
                } else {
                    errorMessage = "Their request isn't in local state — pull to refresh and " +
                        "accept it from Requests."
                }
            } catch (e: Exception) {
                errorMessage = "Accept failed: ${e.message ?: "unknown error"}"
            } finally {
                isSending = false
            }
        }
    }

    fun attemptSend() {
        val recipient = resolvedRecipient ?: return
        errorMessage = null
        scope.launch {
            val pendingIncoming = runCatching {
                container.database.dashpayDao().getContactRequestsByOwner(idBytes)
                    .filter { it.contactIdentityId.contentEquals(recipient) }
                    .let { pair -> pair.any { !it.isOutgoing } && pair.none { it.isOutgoing } }
            }.getOrDefault(false)
            if (pendingIncoming) {
                collisionRecipient = recipient
            } else {
                send(recipient)
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Add Contact") },
                navigationIcon = {
                    TextButton(
                        onClick = { navController.popBackStack() },
                        enabled = !isSending,
                        modifier = Modifier.testTag("dashpay.addContact.cancel"),
                    ) { Text("Cancel") }
                },
                actions = {
                    IconButton(
                        onClick = { navController.navigate(QrScanner) },
                        enabled = !isSending,
                        modifier = Modifier.testTag("dashpay.addViaQR"),
                    ) {
                        Icon(Icons.Default.QrCodeScanner, contentDescription = "Scan QR")
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
            org.dashfoundation.example.ui.components.AccessiblePicker(
                label = "Search by",
                options = AddMode.entries,
                selected = mode,
                optionLabel = { if (it == AddMode.DPNS) "Username (DPNS)" else "Identity ID" },
                testTag = "dashpay.addContact.mode",
            ) { mode = it }

            when (mode) {
                AddMode.DPNS -> DpnsSection(
                    searchText = searchText,
                    onSearchTextChange = { searchText = it },
                    onClear = { searchText = "" },
                    state = searchState,
                    selectedResult = selectedResult,
                    onSelect = { selectedResult = it; errorMessage = null },
                )
                AddMode.IDENTITY_ID -> IdSection(
                    idText = idText,
                    onIdTextChange = { idText = it },
                    isInvalid = idText.trim().isNotEmpty() && parsedIdentityId == null,
                )
            }

            val recipient = resolvedRecipient
            if (recipient != null) {
                PreviewSection(
                    recipient = recipient,
                    profile = previewProfile,
                    dpnsLabel = selectedResult?.label,
                    isSelf = recipientIsSelf,
                )
                FormSection(title = "Account label (optional)") {
                    OutlinedTextField(
                        value = accountLabel,
                        onValueChange = { accountLabel = it },
                        modifier = Modifier.fillMaxWidth().testTag("dashpay.addContact.accountLabel"),
                        placeholder = { Text("e.g. Main wallet") },
                        singleLine = true,
                    )
                }
                SubmitButton(
                    text = "Send Request",
                    isLoading = isSending,
                    enabled = canSend,
                    modifier = Modifier.fillMaxWidth().testTag("dashpay.addContact.send"),
                ) { attemptSend() }
            }

            if (errorMessage != null) {
                Text(
                    errorMessage!!,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }
        }
    }

    val collision = collisionRecipient
    if (collision != null) {
        AlertDialog(
            onDismissRequest = { collisionRecipient = null },
            title = { Text("Request already received") },
            text = { Text("This person already sent you a request — accept it instead?") },
            confirmButton = {
                TextButton(onClick = {
                    collisionRecipient = null
                    acceptIncoming(collision)
                }) { Text("Accept") }
            },
            dismissButton = {
                TextButton(onClick = {
                    collisionRecipient = null
                    send(collision)
                }) { Text("Continue anyway") }
            },
        )
    }
}

@Composable
private fun DpnsSection(
    searchText: String,
    onSearchTextChange: (String) -> Unit,
    onClear: () -> Unit,
    state: SearchState,
    selectedResult: DpnsSearchResult?,
    onSelect: (DpnsSearchResult) -> Unit,
) {
    FormSection(title = "Username") {
        OutlinedTextField(
            value = searchText,
            onValueChange = onSearchTextChange,
            modifier = Modifier.fillMaxWidth().testTag("dashpay.addContact.input"),
            placeholder = { Text("Search usernames") },
            singleLine = true,
            trailingIcon = {
                if (searchText.isNotEmpty()) {
                    IconButton(onClick = onClear, modifier = Modifier.testTag("dashpay.addContact.clear")) {
                        Icon(Icons.Default.Clear, contentDescription = "Clear")
                    }
                }
            },
        )
        when (state) {
            SearchState.Idle -> {
                if (searchText.trim().length < 2) {
                    HintText("Type at least 2 characters to search.")
                }
            }
            SearchState.Searching -> {
                Row(
                    modifier = Modifier.padding(vertical = 8.dp),
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    CircularProgressIndicator(modifier = Modifier.size(18.dp), strokeWidth = 2.dp)
                    HintText("Searching…")
                }
            }
            SearchState.NotFound -> {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    HintText("No usernames match \"${searchText.trim()}\".")
                    TextButton(
                        onClick = onClear,
                        modifier = Modifier.testTag("dashpay.addContact.retry"),
                    ) { Text("Clear and try again") }
                }
            }
            is SearchState.Found -> {
                state.results.forEach { result ->
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onSelect(result) }
                            .testTag("dashpay.addContact.result.${result.label}")
                            .padding(vertical = 6.dp),
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        DashPayAvatar(avatarUrl = null, displayName = result.label, size = 32.dp)
                        Column(Modifier.weight(1f)) {
                            Text(result.label, style = MaterialTheme.typography.bodyLarge)
                            Text(
                                Base58.encode(result.identityId).take(16) + "…",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        if (selectedResult == result) {
                            Icon(
                                Icons.Default.Check,
                                contentDescription = "Selected",
                                tint = MaterialTheme.colorScheme.primary,
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun IdSection(idText: String, onIdTextChange: (String) -> Unit, isInvalid: Boolean) {
    FormSection(title = "Identity ID") {
        OutlinedTextField(
            value = idText,
            onValueChange = onIdTextChange,
            modifier = Modifier.fillMaxWidth().testTag("dashpay.addContact.idInput"),
            placeholder = { Text("Paste identity ID (base58)") },
            singleLine = true,
        )
        if (isInvalid) {
            Text(
                "Not a valid identity id (expected base58)",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
            )
        }
    }
}

@Composable
private fun PreviewSection(
    recipient: ByteArray,
    profile: DashPayProfile?,
    dpnsLabel: String?,
    isSelf: Boolean,
) {
    val name = dashPayContactDisplayName(
        contactId = recipient,
        alias = null,
        profileDisplayName = profile?.displayName,
        dpnsLabel = dpnsLabel,
    )
    FormSection(title = "Send to") {
        Row(
            modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            DashPayAvatar(profile?.avatarUrl, name)
            Column(Modifier.weight(1f)) {
                Text(name, style = MaterialTheme.typography.titleMedium)
                Text(
                    Base58.encode(recipient).take(20) + "…",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                val message = profile?.publicMessage?.trim()
                if (!message.isNullOrEmpty()) {
                    Text(
                        message,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 2,
                    )
                }
            }
        }
        if (isSelf) {
            Text(
                "That's this identity — pick someone else.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
            )
        }
    }
}

@Composable
private fun HintText(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}
