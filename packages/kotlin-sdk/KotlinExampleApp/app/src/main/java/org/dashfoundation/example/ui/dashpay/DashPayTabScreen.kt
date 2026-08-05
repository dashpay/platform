package org.dashfoundation.example.ui.dashpay

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Block
import androidx.compose.material.icons.filled.Group
import androidx.compose.material.icons.filled.Inbox
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.PersonAdd
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.Redeem
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.VisibilityOff
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.wallet.DashPayUnlockStatus
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.DashPayAddContact
import org.dashfoundation.example.navigation.DashPayContacts
import org.dashfoundation.example.navigation.DashPayHidden
import org.dashfoundation.example.navigation.DashPayIgnored
import org.dashfoundation.example.navigation.DashPayInvitations
import org.dashfoundation.example.navigation.DashPayProfile
import org.dashfoundation.example.navigation.DashPayRequests
import org.dashfoundation.example.navigation.IdentitiesHome
import org.dashfoundation.example.navigation.WalletsHome
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.EntityRow
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.formatDuffs
import org.dashfoundation.example.util.toHex

/**
 * DashPay tab root — port of `DashPayTabView.swift`. Picks the active
 * wallet-backed identity, surfaces the received-from-contacts balance and the
 * seedless-unlock banner, and hosts the DashPay sections (Contacts, Requests,
 * Add Contact, Your Profile, Ignored, Hidden). Pull-to-refresh and the
 * toolbar refresh both drive one `dashPaySyncNow()` sweep.
 *
 * Structural note: iOS embeds Contacts/Requests as a segmented control inside
 * this view; the Kotlin hub instead navigates to per-section routes (a
 * cleaner Compose fit), so `dashpay.segment` / `dashpay.profileHeader` /
 * `dashpay.usernamePrompt` are not reproduced here.
 *
 * The active identity is persisted per network and restored only after the
 * network manager has finished loading its wallets.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DashPayTabScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()

    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by remember(manager) {
        manager?.wallets ?: MutableStateFlow(emptyMap<String, ManagedPlatformWallet>())
    }.collectAsStateWithLifecycle()
    val restorationState by container.dashPayActiveIdentityRestorationCoordinator.state
        .collectAsStateWithLifecycle()

    val allWalletOwned by remember(network) {
        container.database.identityDao().observeWalletOwnedByNetwork(network.ffiValue)
    }.collectAsStateWithLifecycle(emptyList())

    val managerMatchesNetwork = manager?.network == network
    val eligible = remember(allWalletOwned, walletsMap, managerMatchesNetwork) {
        if (managerMatchesNetwork) {
            eligibleDashPayIdentities(allWalletOwned, walletsMap.keys)
        } else {
            emptyList()
        }
    }

    var preferenceRetryKey by remember(network) { mutableIntStateOf(0) }
    val selection = rememberDashPayActiveIdentitySelection(
        network = network,
        store = container.dashPayActiveIdentityStore,
        eligible = eligible,
        retryKey = preferenceRetryKey,
    )
    val restorationScreenState = dashPayRestorationScreenState(
        network = network,
        managerMatchesNetwork = managerMatchesNetwork,
        restorationState = restorationState,
    )
    val restorationReady =
        restorationScreenState == DashPayActiveIdentityRestorationState.Ready(network)
    val restorationFailure =
        restorationScreenState as? DashPayActiveIdentityRestorationState.Failed
    val selectionReady = selection as? DashPayActiveIdentitySelection.Ready
    val selectionFailure = selection as? DashPayActiveIdentitySelection.Failed
    val contentReady = managerMatchesNetwork && restorationReady && selectionReady != null

    var pendingSelectionId by remember(network) { mutableStateOf<String?>(null) }
    var selectionWriteError by remember(network) { mutableStateOf<String?>(null) }
    LaunchedEffect(selectionReady?.selectedIdentityIdBase58, pendingSelectionId) {
        if (
            pendingSelectionId != null &&
            selectionReady?.selectedIdentityIdBase58 == pendingSelectionId
        ) {
            pendingSelectionId = null
        }
    }

    var isRefreshing by remember(network) { mutableStateOf(false) }
    var unlockError by remember { mutableStateOf<String?>(null) }

    // Claim-invitation sheet, seeded either by the toolbar action or a
    // parked deep link. The pending URI is consumed (cleared) only when the
    // sheet is actually seeded — a walletless tap keeps it parked (deviation
    // from iOS, which drops the link; see AppUiState.pendingInviteUri).
    val appUiState = container.appUiState
    var claimSheetUri by remember { mutableStateOf<String?>(null) }
    var showClaimSheet by remember { mutableStateOf(false) }
    val pendingInvite by appUiState.pendingInviteUri.collectAsStateWithLifecycle()
    val claimInFlight by appUiState.invitationClaimInFlight.collectAsStateWithLifecycle()
    LaunchedEffect(pendingInvite, walletsMap, claimInFlight, showClaimSheet) {
        val uri = pendingInvite
        if (uri != null && walletsMap.isNotEmpty() && !claimInFlight && !showClaimSheet) {
            appUiState.pendingInviteUri.value = null
            claimSheetUri = uri
            showClaimSheet = true
        }
    }
    // A QR scan launched from the claim sheet returns its raw string here
    // (the shared scanner's savedStateHandle contract); park it through the
    // same pending-invite path so the sheet reopens seeded with it.
    val savedStateHandle = navController.currentBackStackEntry?.savedStateHandle
    LaunchedEffect(savedStateHandle) {
        savedStateHandle
            ?.getStateFlow<String?>(org.dashfoundation.example.navigation.QrScanner.RESULT_KEY, null)
            ?.collect { scanned ->
                if (!scanned.isNullOrBlank()) {
                    savedStateHandle[org.dashfoundation.example.navigation.QrScanner.RESULT_KEY] = null
                    appUiState.pendingInviteUri.value = scanned
                }
            }
    }

    fun refresh() {
        if (!contentReady) return
        val activeManager = manager ?: return
        scope.launch {
            isRefreshing = true
            runCatching { activeManager.dashPaySyncNow() }
            isRefreshing = false
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("DashPay") },
                actions = {
                    IconButton(
                        onClick = {
                            claimSheetUri = null
                            showClaimSheet = true
                        },
                        enabled = walletsMap.isNotEmpty(),
                        modifier = Modifier.testTag("dashpay.claimInvitation"),
                    ) {
                        Icon(Icons.Default.Redeem, contentDescription = "Claim invitation")
                    }
                    IconButton(
                        onClick = {
                            navController.navigate(
                                DashPayInvitations(
                                    selectionReady?.activeIdentity?.identityId?.toHex(),
                                ),
                            )
                        },
                        modifier = Modifier.testTag("dashpay.openSentInvitations"),
                    ) {
                        Icon(Icons.AutoMirrored.Filled.Send, contentDescription = "Sent invitations")
                    }
                    IconButton(
                        onClick = { refresh() },
                        enabled = contentReady,
                        modifier = Modifier.testTag("dashpay.refresh"),
                    ) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                },
            )
        },
    ) { padding ->
        PullToRefreshBox(
            isRefreshing = isRefreshing,
            onRefresh = { refresh() },
            modifier = Modifier.fillMaxSize().padding(padding),
        ) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(16.dp)
                    .testTag("dashpay.tab"),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                when {
                    restorationFailure != null -> BlockingState(
                        message = "DashPay identity restoration failed.",
                        detail = restorationFailure.error.message,
                        onRetry = {
                            scope.launch {
                                try {
                                    container.activateManager()
                                } catch (error: CancellationException) {
                                    throw error
                                } catch (_: Exception) {
                                    // The coordinator publishes the retryable failure state.
                                }
                            }
                        },
                    )
                    !restorationReady -> BlockingState(
                        message = "Restoring DashPay identity…",
                    )
                    selectionFailure != null -> BlockingState(
                        message = "Saved DashPay identity could not be loaded.",
                        detail = selectionFailure.error.message,
                        onRetry = { preferenceRetryKey++ },
                    )
                    selectionReady == null -> BlockingState(
                        message = "Loading saved DashPay identity…",
                    )
                    walletsMap.isEmpty() -> EmptyState(
                        title = "No wallet loaded",
                        message = "Load or create a wallet to use DashPay.",
                        buttonTitle = "Open Wallets",
                        buttonTestTag = "dashpay.openWallets",
                        onClick = { navController.navigate(WalletsHome) },
                    )
                    eligible.isEmpty() || selectionReady.activeIdentity == null -> EmptyState(
                        title = "No identities yet",
                        message = "Register an identity to start using DashPay.",
                        buttonTitle = "Open Identities",
                        buttonTestTag = "dashpay.openIdentities",
                        onClick = { navController.navigate(IdentitiesHome) },
                    )
                    else -> {
                        val identity = selectionReady.activeIdentity
                        val identityHex = identity.identityId.toHex()
                        val managed = identity.walletId?.let { manager?.wallet(forWalletId = it) }

                        if (eligible.size > 1) {
                            DashPayActiveIdentityPicker(
                                eligible = eligible,
                                selected = identity,
                                enabled = pendingSelectionId == null,
                            ) { selectedIdentity ->
                                if (pendingSelectionId != null) return@DashPayActiveIdentityPicker
                                val selectedId = Base58.encode(selectedIdentity.identityId)
                                pendingSelectionId = selectedId
                                selectionWriteError = null
                                scope.launch {
                                    try {
                                        container.dashPayActiveIdentityStore.select(
                                            network,
                                            selectedId,
                                        )
                                    } catch (error: CancellationException) {
                                        throw error
                                    } catch (error: Exception) {
                                        pendingSelectionId = null
                                        selectionWriteError =
                                            error.message ?: "Failed to save active identity"
                                    }
                                }
                            }
                        }

                        BalanceRow(
                            manager = manager,
                            walletId = identity.walletId,
                            identityId = identity.identityId,
                        )

                        UnlockBanner(
                            manager = manager,
                            walletIdHex = identity.walletId?.toHex(),
                            managed = managed,
                            onError = { unlockError = it },
                        )

                        FormSection(title = "DashPay") {
                            EntityRow(
                                icon = Icons.Default.Group,
                                title = "Contacts",
                                onClick = { navController.navigate(DashPayContacts(identityHex)) },
                                modifier = Modifier.testTag("dashpay.openContacts"),
                            )
                            EntityRow(
                                icon = Icons.Default.Inbox,
                                title = "Requests",
                                onClick = { navController.navigate(DashPayRequests(identityHex)) },
                                modifier = Modifier.testTag("dashpay.openRequests"),
                            )
                            EntityRow(
                                icon = Icons.Default.PersonAdd,
                                title = "Add Contact",
                                onClick = { navController.navigate(DashPayAddContact(identityHex)) },
                                modifier = Modifier.testTag("dashpay.addContact"),
                            )
                            EntityRow(
                                icon = Icons.Default.Person,
                                title = "Your Profile",
                                onClick = { navController.navigate(DashPayProfile(identityHex)) },
                                modifier = Modifier.testTag("dashpay.openProfile"),
                            )
                            EntityRow(
                                icon = Icons.Default.Block,
                                title = "Ignored",
                                onClick = { navController.navigate(DashPayIgnored(identityHex)) },
                                modifier = Modifier.testTag("dashpay.openIgnored"),
                            )
                            EntityRow(
                                icon = Icons.Default.VisibilityOff,
                                title = "Hidden",
                                onClick = { navController.navigate(DashPayHidden(identityHex)) },
                                modifier = Modifier.testTag("dashpay.openHidden"),
                            )
                        }
                    }
                }
            }
        }
    }

    val displayedError = selectionWriteError ?: unlockError
    ErrorAlertDialog(
        message = displayedError,
        onDismiss = {
            if (selectionWriteError != null) {
                selectionWriteError = null
            } else {
                unlockError = null
            }
        },
    )

    if (showClaimSheet) {
        ModalBottomSheet(onDismissRequest = { if (!claimInFlight) showClaimSheet = false }) {
            ClaimInvitationSheet(
                initialUri = claimSheetUri,
                preferredWalletIdHex = selectionReady?.activeIdentity?.walletId?.toHex(),
                onScanRequest = {
                    showClaimSheet = false
                    navController.navigate(org.dashfoundation.example.navigation.QrScanner)
                },
                onClose = { showClaimSheet = false },
            )
        }
    }
}

sealed interface DashPayActiveIdentitySelection {
    data object Loading : DashPayActiveIdentitySelection

    data class Ready(
        val selectedIdentityIdBase58: String?,
        val activeIdentity: IdentityEntity?,
    ) : DashPayActiveIdentitySelection

    data class Failed(
        val error: Throwable,
    ) : DashPayActiveIdentitySelection
}

internal fun dashPayRestorationScreenState(
    network: Network,
    managerMatchesNetwork: Boolean,
    restorationState: DashPayActiveIdentityRestorationState,
): DashPayActiveIdentityRestorationState =
    when {
        restorationState is DashPayActiveIdentityRestorationState.Failed &&
            restorationState.network == network ->
            restorationState
        !managerMatchesNetwork -> DashPayActiveIdentityRestorationState.Loading(network)
        restorationState == DashPayActiveIdentityRestorationState.Ready(network) ->
            restorationState
        else -> DashPayActiveIdentityRestorationState.Loading(network)
    }

@Composable
fun rememberDashPayActiveIdentitySelection(
    network: Network,
    store: DashPayActiveIdentityStore,
    eligible: List<IdentityEntity>,
    retryKey: Int = 0,
): DashPayActiveIdentitySelection {
    val preferenceFlow = remember(network, store, retryKey) {
        store.observe(network)
    }
    return rememberDashPayActiveIdentitySelection(
        network = network,
        retryKey = retryKey,
        preferenceFlow = preferenceFlow,
        eligible = eligible,
    )
}

@Composable
internal fun rememberDashPayActiveIdentitySelection(
    network: Network,
    retryKey: Int,
    preferenceFlow: Flow<DashPayActiveIdentityPreference>,
    eligible: List<IdentityEntity>,
): DashPayActiveIdentitySelection =
    key(network, retryKey, preferenceFlow) {
        val preference by preferenceFlow.collectAsStateWithLifecycle(
            initialValue = DashPayActiveIdentityPreference.Loading,
        )
        remember(preference, eligible) {
            when (val currentPreference = preference) {
                DashPayActiveIdentityPreference.Loading ->
                    DashPayActiveIdentitySelection.Loading
                is DashPayActiveIdentityPreference.Failed ->
                    DashPayActiveIdentitySelection.Failed(currentPreference.error)
                is DashPayActiveIdentityPreference.Ready ->
                    DashPayActiveIdentitySelection.Ready(
                        selectedIdentityIdBase58 = currentPreference.identityIdBase58,
                        activeIdentity = resolveActiveDashPayIdentity(
                            eligible,
                            currentPreference.identityIdBase58,
                        ),
                    )
            }
        }
    }

@Composable
fun DashPayActiveIdentityPicker(
    eligible: List<IdentityEntity>,
    selected: IdentityEntity,
    enabled: Boolean,
    onSelected: (IdentityEntity) -> Unit,
) {
    AccessiblePicker(
        label = "Identity",
        options = eligible,
        selected = selected,
        optionLabel = { identity ->
            pickerLabel(
                identity.identityId,
                identity.mainDpnsName ?: identity.dpnsName,
            )
        },
        testTag = "dashpay.identityPicker",
        enabled = enabled,
        onSelected = onSelected,
    )
}

@Composable
private fun BalanceRow(
    manager: org.dashfoundation.dashsdk.wallet.PlatformWalletManager?,
    walletId: ByteArray?,
    identityId: ByteArray,
) {
    var receivedDuffs by remember(walletId, identityId) { mutableStateOf(0L) }
    // Re-read after each completed sweep so received funds appear without a
    // screen reopen (the balance is an in-memory Rust snapshot pull-refresh
    // advances).
    val syncingFlow = remember(manager) { manager?.dashPaySyncIsSyncing ?: MutableStateFlow(false) }
    val isSyncing by syncingFlow.collectAsStateWithLifecycle(false)
    LaunchedEffect(manager, walletId, identityId, isSyncing) {
        val m = manager
        receivedDuffs = if (m != null && walletId != null) {
            parseAccountBalances(m.accountBalances(walletId))
                .filter { it.typeTag == 12 && it.userIdentityId.contentEquals(identityId) }
                .sumOf { it.confirmed + it.unconfirmed }
        } else {
            0L
        }
    }
    FormSection {
        LabeledContent(
            label = "Received from contacts",
            value = formatDuffs(receivedDuffs),
            modifier = Modifier.testTag("dashpay.receivedBalance"),
        )
    }
}

@Composable
private fun UnlockBanner(
    manager: org.dashfoundation.dashsdk.wallet.PlatformWalletManager?,
    walletIdHex: String?,
    managed: ManagedPlatformWallet?,
    onError: (String) -> Unit,
) {
    // Keep every composable call unconditional (collect / scope / state), then
    // render conditionally on the resolved status — avoids the fragile
    // early-return-before-composable-calls pattern.
    val statusFlow = remember(manager) {
        manager?.dashPayUnlockStatus ?: MutableStateFlow(emptyMap<String, DashPayUnlockStatus>())
    }
    val statusMap by statusFlow.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    var isUnlocking by remember { mutableStateOf(false) }

    val status = walletIdHex?.let { statusMap[it] }
    if (manager == null || status == null) return
    val hasSignal = status.draining || status.seedMismatch || status.pendingAccountBuilds > 0
    if (!hasSignal) return

    when {
        status.seedMismatch -> BannerRow(
            icon = Icons.Default.Warning,
            tint = MaterialTheme.colorScheme.error,
            text = "Seed verification failed — this wallet's Keystore seed doesn't match. " +
                "DashPay signing is disabled.",
            action = null,
        )
        status.draining -> BannerRow(
            icon = Icons.Default.Lock,
            tint = MaterialTheme.colorScheme.tertiary,
            text = "Finishing contact setup…",
            action = null,
        )
        status.pendingAccountBuilds > 0 -> {
            val n = status.pendingAccountBuilds
            BannerRow(
                icon = Icons.Default.Lock,
                tint = MaterialTheme.colorScheme.tertiary,
                text = "$n contact${if (n == 1) "" else "s"} waiting to finish setup",
                // Guard against a double-tap stacking concurrent unlocks: the
                // SDK's `draining` flag only flips true after the verify, so a
                // second tap in that window would launch a second drain.
                actionEnabled = !isUnlocking,
                action = if (managed != null) {
                    {
                        isUnlocking = true
                        scope.launch {
                            try {
                                val unlocked = manager.unlockWalletFromKeystore(managed)
                                if (!unlocked) {
                                    onError(
                                        "This wallet is watch-only on this device (no mnemonic in " +
                                            "the Keystore), so contact setup can't be finished here.",
                                    )
                                }
                            } catch (e: Exception) {
                                onError(e.message ?: "Unlock failed")
                            } finally {
                                isUnlocking = false
                            }
                        }
                    }
                } else {
                    null
                },
            )
        }
    }
}

@Composable
private fun BannerRow(
    icon: ImageVector,
    tint: androidx.compose.ui.graphics.Color,
    text: String,
    action: (() -> Unit)?,
    actionEnabled: Boolean = true,
) {
    Row(
        modifier = Modifier.fillMaxWidth().testTag("dashpay.unlockBanner"),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(icon, contentDescription = null, tint = tint)
        Text(text, style = MaterialTheme.typography.bodySmall, modifier = Modifier.weight(1f))
        if (action != null) {
            Button(onClick = action, enabled = actionEnabled) { Text("Unlock") }
        }
    }
}

@Composable
private fun BlockingState(
    message: String,
    detail: String? = null,
    onRetry: (() -> Unit)? = null,
) {
    Column(
        modifier = Modifier.fillMaxWidth().padding(vertical = 32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        if (onRetry == null) {
            CircularProgressIndicator()
        }
        Text(message, style = MaterialTheme.typography.titleMedium)
        detail?.takeIf { it.isNotBlank() }?.let { errorDetail ->
            Text(
                errorDetail,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.error,
            )
        }
        onRetry?.let { retry ->
            Button(onClick = retry) {
                Text("Retry")
            }
        }
    }
}

@Composable
private fun EmptyState(
    title: String,
    message: String,
    buttonTitle: String,
    buttonTestTag: String,
    onClick: () -> Unit,
) {
    Column(
        modifier = Modifier.fillMaxWidth().padding(vertical = 32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(title, style = MaterialTheme.typography.titleMedium)
        Text(
            message,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Button(onClick = onClick, modifier = Modifier.testTag(buttonTestTag)) { Text(buttonTitle) }
    }
}

/** Picker label: DPNS name when known, else a truncated base58 id. */
private fun pickerLabel(identityId: ByteArray, dpnsName: String?): String =
    dpnsName?.takeIf { it.isNotBlank() } ?: (Base58.encode(identityId).take(12) + "…")
