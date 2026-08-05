package org.dashfoundation.example.ui.dashpay

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.identity.RegistrationKeys
import org.dashfoundation.dashsdk.tokens.InvitationPreview
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.services.DashpayKeyProvisioning
import org.dashfoundation.example.util.toHex

/**
 * Claim-invitation sheet — port of `ClaimInvitationSheet.swift`. Paste (or
 * deep-link-seed) a `dashpay://invite` link, preview it off-chain
 * ([InvitationPreview] — the only gate is structural validity; the amount
 * shows "—" because the legacy link doesn't carry it), then register a
 * brand-new identity for the invitee funded by the imported voucher — the
 * fresh-onboarding path: works with **no** pre-existing identity and no L1
 * Dash on this side.
 *
 * Claim wallet selection pins the iOS rule: the active identity's wallet,
 * else the first loaded wallet (a fresh invitee has no identity yet).
 * The claim registers the full 6-key set (base four + the DashPay
 * ENCRYPTION/DECRYPTION pair) so the new identity can send contact
 * requests immediately. On success, a link that carried a `du` username
 * prompts "Add <username>?" — confirm resolves the inviter via DPNS and
 * sends a normal contact request from the new identity.
 *
 * The URI is a bearer credential; it is never logged. The claim runs in
 * the application scope (dismissal-safe); back/dismiss is gated while
 * claiming, and the application-scoped claim state defers any second deep
 * link until this one resolves.
 */
@Composable
fun ClaimInvitationSheet(
    initialUri: String? = null,
    preferredWalletIdHex: String? = null,
    onScanRequest: (() -> Unit)? = null,
    onClaimStarted: () -> Unit = {},
    onClose: () -> Unit,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val appUiState = container.appUiState
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val isSdkLoading by appState.isLoading.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by remember(manager) {
        manager?.wallets ?: kotlinx.coroutines.flow.MutableStateFlow(
            emptyMap<String, org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet>(),
        )
    }.collectAsStateWithLifecycle()
    val walletOwned by remember(network) {
        container.database.identityDao().observeWalletOwnedByNetwork(network.ffiValue)
    }.collectAsStateWithLifecycle(emptyList())

    val claimState by appUiState.claimInvitation.collectAsStateWithLifecycle()
    val retainedRequest = when (val state = claimState) {
        is org.dashfoundation.example.state.AppUiState.ClaimInvitationState.InFlight -> state.request
        is org.dashfoundation.example.state.AppUiState.ClaimInvitationState.Failed -> state.request
        else -> null
    }
    var uriText by remember {
        mutableStateOf(retainedRequest?.uri?.reveal() ?: initialUri.orEmpty())
    }
    var preview by remember { mutableStateOf(InvitationPreview.INVALID) }
    // The exact URI the displayed preview was parsed from. The submit gate
    // and claim() both key off THIS, never off the live field text — edits
    // after a valid parse would otherwise leave a stale-valid preview
    // enabling a claim of the newly typed, unvalidated URI.
    var previewedUri by remember { mutableStateOf<String?>(null) }
    var validationError by remember { mutableStateOf<String?>(null) }
    val isClaiming = claimState is org.dashfoundation.example.state.AppUiState
        .ClaimInvitationState.InFlight ||
        claimState is org.dashfoundation.example.state.AppUiState
            .ClaimInvitationState.ContactSending
    val errorMessage = when (val state = claimState) {
        is org.dashfoundation.example.state.AppUiState.ClaimInvitationState.Failed ->
            state.safeError
        is org.dashfoundation.example.state.AppUiState.ClaimInvitationState.ContactFailed ->
            state.safeError
        else -> validationError
    }

    LaunchedEffect(claimState) {
        if (claimState is org.dashfoundation.example.state.AppUiState.ClaimInvitationState.Completed) {
            appUiState.clearInvitationClaim()
            onClose()
        }
    }

    val setSecureScreen = org.dashfoundation.example.LocalSecureScreen.current
    androidx.compose.runtime.DisposableEffect(uriText.isNotBlank()) {
        val containsBearer = uriText.isNotBlank()
        if (containsBearer) setSecureScreen(true)
        onDispose { if (containsBearer) setSecureScreen(false) }
    }

    // Claim wallet: the DashPay tab's ACTIVE identity's wallet (passed by
    // the host — the user's actual selection, matching iOS), else the first
    // wallet-owned identity's wallet, else the first loaded wallet — a
    // fresh invitee with no identity can still claim.
    val claimWallet = remember(walletOwned, walletsMap, preferredWalletIdHex) {
        preferredWalletIdHex?.let { walletsMap[it] }
            ?: walletOwned.firstOrNull {
                it.walletId != null && walletsMap.containsKey(it.walletId!!.toHex())
            }?.walletId?.let { walletsMap[it.toHex()] }
            ?: walletsMap.values.firstOrNull()
    }

    // Keyed on claimWallet too: a deep-link-seeded sheet can compose before
    // the wallet flows emit, and the preview must re-parse once one exists.
    LaunchedEffect(uriText, claimWallet) {
        val trimmed = uriText.trim()
        val parsed = if (trimmed.isEmpty()) {
            InvitationPreview.INVALID
        } else {
            claimWallet?.dashpay?.let { runCatching { it.parseInvitation(trimmed) }.getOrNull() }
                ?: InvitationPreview.INVALID
        }
        preview = parsed
        previewedUri = trimmed.takeIf { parsed.structurallyValid }
    }

    val managerReady = !isSdkLoading && manager?.network == network
    val canClaim = !isClaiming && managerReady && claimWallet != null &&
        previewedUri != null && previewedUri == uriText.trim()

    fun claim() {
        if (!canClaim || isClaiming) return
        val wallet = claimWallet ?: return
        val mgr = manager ?: return
        // Submit only the URI the displayed preview validated.
        val uri = previewedUri ?: return
        val operationNetworkRaw = network.ffiValue
        val operationId = appUiState.beginInvitationClaim(
            operationNetworkRaw,
            uri,
            wallet.walletId.toHex(),
        ) ?: return
        val inviterUsername = preview.inviterUsername
        // The claim owns the URI from here — release the parked copy.
        onClaimStarted()
        validationError = null
        container.applicationScope.launch {
            try {
                val newIdentityId = withContext(NonCancellable) {
                    val permit = container.registrationCoordinator
                        .beginInvitationRegistration(wallet.walletId)
                    try {
                        check(container.isCurrentInvitationManager(operationNetworkRaw, mgr)) {
                            "The active invitation manager changed."
                        }
                        val identityIndex = checkNotNull(permit.identityIndex)
                        val previews = mgr.identityRegistration.previewRegistrationKeySet(
                            walletHandle = wallet.handle,
                            mnemonicResolverHandle = mgr.mnemonicResolverHandle,
                            identityIndex = identityIndex,
                            count = RegistrationKeys.keyCount(includeDashPayKeys = true),
                        )
                        val keySet = DashpayKeyProvisioning.provision(
                            previews = previews,
                            includeDashPayKeys = true,
                            walletId = wallet.walletId,
                            persister = { keyHex, priv, owner ->
                                container.walletStorage.storePrivateKey(
                                    keyHex, priv, ownerWalletId = owner,
                                )
                            },
                        )
                        mgr.identityRegistration.claimInvitation(
                            walletHandle = wallet.handle,
                            uri = uri,
                            identityIndex = identityIndex,
                            keys = keySet,
                            signerHandle = mgr.signerHandle,
                        )
                    } finally {
                        container.registrationCoordinator.endInvitationOperation(permit)
                    }
                }
                appUiState.completeInvitationClaim(operationId, newIdentityId, inviterUsername)
            } catch (t: Throwable) {
                appUiState.failInvitationClaim(
                    operationId,
                    "Claiming the invitation failed. The link was not included in the error.",
                )
            }
        }
    }

    fun sendContact() {
        val sending = appUiState.beginInvitationContactSend() ?: return
        val wallet = walletsMap[sending.walletIdHex] ?: run {
            appUiState.failInvitationContactSend(
                sending.operationId,
                "Identity claimed, but its wallet is no longer loaded.",
            )
            return
        }
        val mgr = manager ?: run {
            appUiState.failInvitationContactSend(
                sending.operationId,
                "Identity claimed, but the wallet manager is no longer available.",
            )
            return
        }
        container.applicationScope.launch {
            var permit: org.dashfoundation.example.services.RegistrationCoordinator
                .InvitationOperationPermit? = null
            try {
                permit = container.registrationCoordinator.beginInvitationOperation(wallet.walletId)
                check(container.isCurrentInvitationManager(sending.networkRaw, mgr)) {
                    "The active invitation manager changed."
                }
                // Resolve the inviter's identity id from the link's username
                // via wallet-scoped DPNS search (exact-label match — the id
                // is not on the wire).
                val inviter = parseDpnsSearchResults(
                    wallet.dashpay.searchDpnsNames(sending.username, 10),
                ).firstOrNull { it.label.equals(sending.username, ignoreCase = true) }
                if (inviter == null) {
                    appUiState.failInvitationContactSend(
                        sending.operationId,
                        "Identity claimed, but the inviter couldn't be found to add.",
                    )
                    return@launch
                }
                wallet.dashpay.sendContactRequest(
                    senderIdentityId = sending.identityId,
                    recipientIdentityId = inviter.identityId,
                    signerHandle = mgr.signerHandle,
                    coreSignerHandle = mgr.mnemonicResolverHandle,
                ).close()
                appUiState.completeInvitationContactSend(sending.operationId)
            } catch (t: Throwable) {
                appUiState.failInvitationContactSend(
                    sending.operationId,
                    "Sending the contact request failed.",
                )
            } finally {
                permit?.let { container.registrationCoordinator.endInvitationOperation(it) }
            }
        }
    }

    Column(
        modifier = Modifier.fillMaxWidth().padding(16.dp).testTag("dashpay.invite.claim"),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("Claim Invitation", style = MaterialTheme.typography.titleLarge)
        Text("Paste an invitation link", style = MaterialTheme.typography.labelLarge)
        Text(
            "From a friend's “Invite a friend”. It funds a brand-new " +
                "identity for you.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        OutlinedTextField(
            value = uriText,
            onValueChange = { uriText = it },
            label = { Text("dashpay://invite…") },
            enabled = !isClaiming,
            singleLine = true,
            modifier = Modifier.fillMaxWidth().testTag("dashpay.invite.claim.uriField"),
        )
        if (onScanRequest != null) {
            TextButton(
                onClick = { if (!isClaiming) onScanRequest() },
                enabled = !isClaiming,
                modifier = Modifier.fillMaxWidth().testTag("dashpay.invite.claim.scan"),
            ) { Text("Scan QR code") }
        }
        if (uriText.isNotBlank()) {
            if (preview.structurallyValid) {
                // The legacy link carries neither amount nor expiry.
                Text("Amount: — (shown after the funding transaction is fetched)")
                preview.inviterUsername?.let { Text("From: $it") }
                if (!preview.isInstant) {
                    Text(
                        "ChainLock invitation (no InstantSend lock in the link).",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            } else {
                Text("Invalid invitation link.", color = MaterialTheme.colorScheme.error)
            }
        }
        if (claimWallet == null) {
            Text(
                "Create a wallet to claim this invitation.",
                color = MaterialTheme.colorScheme.error,
            )
        }
        errorMessage?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        Button(
            onClick = { claim() },
            enabled = canClaim,
            modifier = Modifier.fillMaxWidth().testTag("dashpay.invite.claim.submit"),
        ) {
            if (isClaiming) {
                CircularProgressIndicator(Modifier.size(18.dp))
                Text("  Claiming…")
            } else {
                Text("Claim")
            }
        }
        TextButton(
            onClick = {
                if (!isClaiming) {
                    appUiState.clearInvitationClaim()
                    onClose()
                }
            },
            enabled = !isClaiming,
            modifier = Modifier.fillMaxWidth(),
        ) { Text("Cancel") }
    }

    val contactState = claimState
    if (
        contactState is org.dashfoundation.example.state.AppUiState
            .ClaimInvitationState.ContactPrompt ||
        contactState is org.dashfoundation.example.state.AppUiState
            .ClaimInvitationState.ContactFailed
    ) {
        val username = when (contactState) {
            is org.dashfoundation.example.state.AppUiState.ClaimInvitationState.ContactPrompt ->
                contactState.username
            is org.dashfoundation.example.state.AppUiState.ClaimInvitationState.ContactFailed ->
                contactState.username
            else -> error("unreachable")
        }
        AlertDialog(
            onDismissRequest = { /* explicit choice required */ },
            title = { Text("Add $username?") },
            text = { Text("Send a contact request to the person who invited you.") },
            confirmButton = {
                TextButton(onClick = {
                    sendContact()
                }) { Text("Add") }
            },
            dismissButton = {
                TextButton(onClick = {
                    appUiState.clearInvitationClaim()
                    onClose()
                }) { Text("Not now") }
            },
        )
    }
}
