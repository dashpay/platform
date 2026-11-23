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
import androidx.compose.foundation.selection.toggleable
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
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
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
 * "Send privately" sends a private request paid from the wallet's shielded
 * balance (`sendPrivateContactRequest`) and makes the collision accept reply
 * privately too ([CollisionAccept]).
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
    // Send a private contact request (protocol version 15): paid from the
    // wallet's shielded balance, so nothing on chain names either side.
    var sendPrivately by remember { mutableStateOf(false) }
    var isSending by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    // Set when a private send or reply may have gone through (its result
    // could not be confirmed): the screen only offers to close, never to
    // submit again, as the send screen does for an unconfirmed spend.
    var unconfirmedNotice by remember { mutableStateOf<String?>(null) }
    // Whether the wallet holds the pair with the resolved recipient for a
    // private request, as a send decides it (in flight, paid and not yet
    // read back, or kept by the shielded store's journal until the payment is
    // proven never to land), or for a public request awaiting its outcome
    // (`contactRequestHolds`). While it does, Send and the collision accept
    // are off and the screen offers the sync that settles it: the private
    // one, which reads a landed request back, or the DashPay one, which
    // settles a public request. Read from the wallet, so reopening this
    // screen keeps it. A failed read keeps them off until a read succeeds.
    var recipientPending by remember { mutableStateOf(PrivateRepliesPendingSnapshot()) }
    // Why the shielded store could not be opened, as last read with the
    // pending state; null when it is open.
    var shieldedStoreFailure by remember { mutableStateOf<String?>(null) }
    var isSyncingPrivate by remember { mutableStateOf(false) }
    // A DashPay sync the screen started is running.
    var isCheckingPublic by remember { mutableStateOf(false) }
    var pendingReadVersion by remember { mutableStateOf(0) }
    // The automatic refreshes made since the wallet's private payments were
    // last read restored (`PrivatePaymentsRecovery.recheckDelayMs`).
    var readinessRechecks by remember { mutableStateOf(0) }
    // The submission the collision dialog asks about, as fixed at the tap.
    var collisionSubmission by remember { mutableStateOf<ContactRequestSubmission?>(null) }
    var previewProfile by remember { mutableStateOf<DashPayProfile?>(null) }

    val parsedIdentityId = remember(idText) { Base58.decodeIdentifier(idText) }
    val resolvedRecipient: ByteArray? = when (mode) {
        AddMode.DPNS -> selectedResult?.identityId
        AddMode.IDENTITY_ID -> parsedIdentityId
    }
    val recipientIsSelf = resolvedRecipient?.contentEquals(idBytes) == true
    val canSend = resolvedRecipient != null && !recipientIsSelf && !isSending &&
        unconfirmedNotice == null && recipientPending.allowsSubmission(resolvedRecipient.toHex())
    val recipientBlocked = resolvedRecipient != null &&
        (
            recipientPending.isRestoringPrivatePayments ||
                recipientPending.isAwaitingOutcome(resolvedRecipient.toHex()) ||
                recipientPending.isPending(resolvedRecipient.toHex()) ||
                recipientPending.isUnavailable
            )

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
                // The QR send is public: never with "Send privately" on (the
                // scan action is off then; this covers a result that arrives
                // after the switch was turned on).
                if (sendPrivately) {
                    errorMessage = "A scanned QR sends a public request. Turn off Send privately to use it."
                    return@collect
                }
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
    LaunchedEffect(recipientHex, wallet, pendingReadVersion) {
        val contactIds = listOfNotNull(resolvedRecipient)
        // Recovery first: the readiness shown is read after the wallet was
        // asked to retry a restoration its payments still wait for.
        val refreshed = PrivatePaymentsRecovery.refresh(
            lastReadiness = recipientPending.readiness,
            storeFailure = container.shieldedStoreFailure.value,
            readReadiness = { privateContactReadiness(wallet, idBytes) },
            configureAgain = { container.configureShieldedStoreAgain() },
        )
        shieldedStoreFailure = refreshed.storeFailure
        val pendingRead = contactRequestHolds(wallet, idBytes, contactIds)
        recipientPending = recipientPending
            .withReadiness(refreshed.read?.readiness, refreshed.read?.restoreFailure)
            .recorded(pendingRead, contactIds.map { it.toHex() })
        // While the payments stay unrestored, refresh again after a pause, for
        // as long as the screen is shown: a restoration the wallet deferred
        // behind a busy store completes later and changes no key of this
        // effect (neither a bind nor a DashPay sync ends for it).
        if (PrivatePaymentsRecovery.waitForRecheck(recipientPending.readiness, readinessRechecks)) {
            readinessRechecks++
            pendingReadVersion++
        } else if (recipientPending.readiness?.privatePaymentsRestored == true) {
            readinessRechecks = 0
        }
    }
    // Binding the shielded account restores the wallet's private payments:
    // read the readiness again then.
    LaunchedEffect(container.shieldedService) {
        container.shieldedService.isBound.collect { pendingReadVersion++ }
    }
    // A deferred restoration runs once the shielded scan holding the store
    // ends: refresh then too, ahead of the timer.
    LaunchedEffect(container.shieldedService) {
        container.shieldedService.state
            .map { it.isSyncing }
            .distinctUntilChanged()
            .collect { syncing -> if (!syncing) pendingReadVersion++ }
    }
    // A DashPay sync settles a public request awaiting its outcome: read the
    // recipient's holds again when one ends, the background pass included.
    LaunchedEffect(manager) {
        manager?.dashPaySyncIsSyncing?.collect { syncing -> if (!syncing) pendingReadVersion++ }
    }
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

    /** Send [submission]'s request, as it was chosen at the tap. */
    fun send(submission: ContactRequestSubmission) {
        val w = wallet ?: return
        val m = manager ?: return
        val recipient = submission.recipient
        val label = submission.accountLabel
        isSending = true
        errorMessage = null
        scope.launch {
            // Records the DPNS name the recipient was found by, once a request
            // to them went (or may have gone) out.
            fun recordDpnsHint() {
                submission.dpnsLabel?.let { metaStore.setDpnsHint(it, network, idBytes, recipient) }
            }
            try {
                // Both sends sign with the manager's identity signer.
                if (submission.sendPrivately) {
                    m.sendPrivateContactRequest(
                        walletId = w.walletId,
                        senderIdentityId = idBytes,
                        recipientIdentityId = recipient,
                        accountLabel = label,
                    ).close()
                } else {
                    w.dashpay.sendContactRequest(
                        senderIdentityId = idBytes,
                        recipientIdentityId = recipient,
                        signerHandle = m.signerHandle,
                        coreSignerHandle = m.mnemonicResolverHandle,
                        accountLabel = label,
                    ).close()
                }
                recordDpnsHint()
                kickDashPaySync(scope, m)
                navController.popBackStack()
            } catch (e: Exception) {
                val outcome = ContactRequestSubmissionOutcome.of(e)
                recipientPending = recipientPending.withOutcome(outcome, recipient.toHex())
                when (outcome) {
                    // Nothing was sent: the wallet holds the pair. The screen
                    // shows it as pending, with the private sync, until the
                    // per-contact read says it is free.
                    ContactRequestSubmissionOutcome.PrivateRequestPending -> Unit
                    // Nothing was sent: the screen shows the wallet setting up
                    // its private payments until its readiness says restored.
                    ContactRequestSubmissionOutcome.PrivatePaymentsNotRestored -> Unit
                    // Nothing was signed: an earlier public request to the
                    // recipient awaits its outcome. The screen shows it being
                    // checked, with the DashPay sync, never a retry.
                    ContactRequestSubmissionOutcome.PublicRequestUnconfirmed -> kickDashPaySync(scope, m)
                    // Nothing was sent and nothing is pending: Send stays on,
                    // and sending again continues the read.
                    ContactRequestSubmissionOutcome.PrivateStoreNotRead ->
                        errorMessage = ContactRequestSubmissionOutcome.PRIVATE_STORE_NOT_READ_MESSAGE
                    // Pending, not failed: the next sync settles it.
                    ContactRequestSubmissionOutcome.MayHaveGoneThrough -> {
                        recordDpnsHint()
                        kickDashPaySync(scope, m)
                        unconfirmedNotice = submission.mayHaveGoneThroughMessage
                    }
                    is ContactRequestSubmissionOutcome.Failed -> errorMessage = outcome.message
                }
            } finally {
                isSending = false
                pendingReadVersion++
            }
        }
    }

    /**
     * Read the private store, which settles a pending private request, then
     * re-read the recipient's pending state: Send comes back once nothing is
     * pending, for the user to try again.
     */
    fun syncPrivateRequests() {
        val w = wallet ?: return
        val m = manager ?: return
        isSyncingPrivate = true
        scope.launch {
            try {
                syncPrivateContactRequests(w, m)?.let { errorMessage = it }
            } finally {
                isSyncingPrivate = false
                pendingReadVersion++
            }
        }
    }

    /**
     * Run a DashPay sync, which settles a public request awaiting its
     * outcome first (attaching to one already running), then re-read the
     * recipient's holds: Send comes back once the wallet frees the pair, and
     * a request found is recorded as sent.
     */
    fun checkPublicRequest() {
        val m = manager ?: return
        isCheckingPublic = true
        scope.launch {
            try {
                attachOrStartSync(m)
            } finally {
                isCheckingPublic = false
                pendingReadVersion++
            }
        }
    }

    /** The collision dialog's accept of the request [submission]'s recipient already sent. */
    fun acceptIncoming(submission: ContactRequestSubmission) {
        val w = wallet ?: return
        val m = manager ?: return
        val recipient = submission.recipient
        // A private request to this contact may be pending in the wallet.
        if (!recipientPending.allowsSubmission(recipient.toHex())) return
        // "Send privately" at the tap is the user's choice of a private
        // request, so it holds for the accept too ([CollisionAccept]).
        val accept = submission.collisionAccept
        isSending = true
        errorMessage = null
        scope.launch {
            try {
                val ok = acceptIncomingContactRequest(
                    wallet = w,
                    manager = m,
                    ourIdentityId = idBytes,
                    senderId = recipient,
                    accept = accept,
                )
                if (ok) {
                    kickDashPaySync(scope, m)
                    navController.popBackStack()
                } else {
                    errorMessage = "Their request isn't in local state — pull to refresh and " +
                        "accept it from Requests."
                }
            } catch (e: Exception) {
                val outcome = ContactRequestSubmissionOutcome.of(e)
                recipientPending = recipientPending.withOutcome(outcome, recipient.toHex())
                when (outcome) {
                    ContactRequestSubmissionOutcome.MayHaveGoneThrough -> {
                        // Pending, not failed: the next sync settles it.
                        kickDashPaySync(scope, m)
                        unconfirmedNotice = submission.mayHaveGoneThroughMessage
                    }
                    // Nothing was signed: the screen shows the pair being
                    // checked, with the DashPay sync, never a retry.
                    ContactRequestSubmissionOutcome.PublicRequestUnconfirmed -> kickDashPaySync(scope, m)
                    // Nothing was sent: the screen offers the private sync.
                    ContactRequestSubmissionOutcome.PrivateRequestPending -> Unit
                    ContactRequestSubmissionOutcome.PrivatePaymentsNotRestored -> Unit
                    ContactRequestSubmissionOutcome.PrivateStoreNotRead ->
                        errorMessage = ContactRequestSubmissionOutcome.PRIVATE_STORE_NOT_READ_MESSAGE
                    is ContactRequestSubmissionOutcome.Failed ->
                        errorMessage = "Accept failed: ${outcome.message}"
                }
            } finally {
                isSending = false
                pendingReadVersion++
            }
        }
    }

    fun attemptSend() {
        val recipient = resolvedRecipient ?: return
        if (isSending) return
        // Fix the submission at the tap: the lookup below is asynchronous, and
        // what the user changes on the form meanwhile must not change what
        // goes out. Send and the switch are off until it returns.
        val submission = ContactRequestSubmission.of(
            recipient = recipient,
            sendPrivately = sendPrivately,
            accountLabel = accountLabel,
            dpnsLabel = if (mode == AddMode.DPNS) selectedResult?.label else null,
        )
        errorMessage = null
        isSending = true
        scope.launch {
            val pendingIncoming = try {
                runCatching {
                    container.database.dashpayDao().getContactRequestsByOwner(idBytes)
                        .filter { it.contactIdentityId.contentEquals(recipient) }
                        .let { pair -> pair.any { !it.isOutgoing } && pair.none { it.isOutgoing } }
                }.getOrDefault(false)
            } finally {
                isSending = false
            }
            if (pendingIncoming) {
                collisionSubmission = submission
            } else {
                send(submission)
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
                    // A scanned QR sends a public request at once (Rust's QR
                    // send has no private form), so it is off while "Send
                    // privately" is on.
                    IconButton(
                        onClick = { navController.navigate(QrScanner) },
                        enabled = !isSending && !sendPrivately,
                        modifier = Modifier.testTag("dashpay.addViaQR"),
                    ) {
                        Icon(
                            Icons.Default.QrCodeScanner,
                            contentDescription = if (sendPrivately) {
                                "Scan QR (sends a public request; turn off Send privately to use it)"
                            } else {
                                "Scan QR (sends a public request)"
                            },
                        )
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
                FormSection {
                    // One toggleable row, so TalkBack reads the label with the
                    // switch's role and state.
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier
                            .fillMaxWidth()
                            .toggleable(
                                value = sendPrivately,
                                enabled = !isSending,
                                role = Role.Switch,
                                onValueChange = { sendPrivately = it },
                            )
                            .testTag("dashpay.addContact.sendPrivately"),
                    ) {
                        Text("Send privately", modifier = Modifier.weight(1f))
                        Switch(checked = sendPrivately, onCheckedChange = null, enabled = !isSending)
                    }
                    Text(
                        "Paid from this wallet's shielded balance, so nothing on chain names either of you.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                SubmitButton(
                    text = "Send Request",
                    isLoading = isSending,
                    enabled = canSend,
                    modifier = Modifier.fillMaxWidth().testTag("dashpay.addContact.send"),
                ) { attemptSend() }
                if (recipientBlocked) {
                    Text(
                        ContactRequestSubmissionOutcome.rowMessage(
                            pending = recipientPending.isPending(recipient.toHex()),
                            awaitingOutcome = recipientPending.isAwaitingOutcome(recipient.toHex()),
                            restoring = recipientPending.isRestoringPrivatePayments,
                            restoreFailure = recipientPending.restoreFailure,
                            shieldedStoreFailure = shieldedStoreFailure,
                            unavailable = recipientPending.isUnavailable,
                            error = null,
                        ).orEmpty(),
                        style = MaterialTheme.typography.bodySmall,
                        modifier = Modifier.testTag("dashpay.addContact.pending"),
                    )
                    // A DashPay sync settles a public request awaiting its
                    // outcome, and a private sync a pending private request;
                    // neither restores private payments, so neither is offered
                    // for that.
                    when {
                        recipientPending.isRestoringPrivatePayments -> Unit
                        recipientPending.isAwaitingOutcome(recipient.toHex()) -> TextButton(
                            onClick = { checkPublicRequest() },
                            enabled = !isCheckingPublic,
                            modifier = Modifier.testTag("dashpay.addContact.checkPublic"),
                        ) { Text(if (isCheckingPublic) "Checking…" else "Check again") }
                        else -> TextButton(
                            onClick = { syncPrivateRequests() },
                            enabled = !isSyncingPrivate,
                            modifier = Modifier.testTag("dashpay.addContact.syncPrivate"),
                        ) { Text(if (isSyncingPrivate) "Syncing private requests…" else "Sync private requests") }
                    }
                }
            }

            if (errorMessage != null) {
                Text(
                    errorMessage!!,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.testTag("dashpay.addContact.error"),
                )
            }
        }
    }

    val collision = collisionSubmission
    if (collision != null) {
        val accept = collision.collisionAccept
        AlertDialog(
            onDismissRequest = { collisionSubmission = null },
            title = { Text("Request already received") },
            text = { Text(accept.message) },
            confirmButton = {
                TextButton(
                    onClick = {
                        collisionSubmission = null
                        acceptIncoming(collision)
                    },
                    modifier = Modifier.testTag("dashpay.addContact.collision.accept"),
                ) { Text(accept.buttonTitle) }
            },
            dismissButton = {
                TextButton(
                    onClick = {
                        collisionSubmission = null
                        send(collision)
                    },
                    modifier = Modifier.testTag("dashpay.addContact.collision.continue"),
                ) { Text("Continue anyway") }
            },
        )
    }

    unconfirmedNotice?.let { notice ->
        AlertDialog(
            onDismissRequest = { navController.popBackStack() },
            title = { Text("Submitted") },
            text = { Text(notice) },
            confirmButton = {
                TextButton(
                    onClick = { navController.popBackStack() },
                    modifier = Modifier.testTag("dashpay.addContact.submitted.done"),
                ) { Text("Done") }
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
