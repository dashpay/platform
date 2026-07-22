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
 * claiming, and [org.dashfoundation.example.state.AppUiState.invitationClaimInFlight]
 * defers any second deep link until this one resolves.
 */
@Composable
fun ClaimInvitationSheet(
    initialUri: String? = null,
    onClose: () -> Unit,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val appUiState = container.appUiState
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by remember(manager) {
        manager?.wallets ?: kotlinx.coroutines.flow.MutableStateFlow(
            emptyMap<String, org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet>(),
        )
    }.collectAsStateWithLifecycle()
    val walletOwned by remember(network) {
        container.database.identityDao().observeWalletOwnedByNetwork(network.ffiValue)
    }.collectAsStateWithLifecycle(emptyList())

    var uriText by remember { mutableStateOf(initialUri.orEmpty()) }
    var preview by remember { mutableStateOf(InvitationPreview.INVALID) }
    var isClaiming by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    // Post-claim "Add <username>?" prompt payload: (username, new identity id).
    var contactPrompt by remember { mutableStateOf<Pair<String, ByteArray>?>(null) }

    // Claim wallet: the active identity's wallet, else the first loaded
    // wallet — a fresh invitee with no identity can still claim.
    val claimWallet = remember(walletOwned, walletsMap) {
        walletOwned.firstOrNull {
            it.walletId != null && walletsMap.containsKey(it.walletId!!.toHex())
        }?.walletId?.let { walletsMap[it.toHex()] } ?: walletsMap.values.firstOrNull()
    }

    LaunchedEffect(uriText) {
        val trimmed = uriText.trim()
        preview = if (trimmed.isEmpty()) {
            InvitationPreview.INVALID
        } else {
            claimWallet?.dashpay?.let { runCatching { it.parseInvitation(trimmed) }.getOrNull() }
                ?: InvitationPreview.INVALID
        }
    }

    val canClaim = !isClaiming && preview.structurallyValid && claimWallet != null

    fun claim() {
        if (!canClaim || isClaiming) return
        val wallet = claimWallet ?: return
        val mgr = manager ?: return
        val uri = uriText.trim()
        isClaiming = true
        appUiState.invitationClaimInFlight.value = true
        errorMessage = null
        container.applicationScope.launch {
            try {
                val newIdentityId = withContext(NonCancellable) {
                    val identityIndex = InvitationReclaimLogic.nextUnusedIdentityIndex(
                        walletOwned
                            .filter { it.walletId?.contentEquals(wallet.walletId) == true }
                            .map { it.identityIndex },
                    )
                    // Full fresh-registration set: base four + the DashPay
                    // enc/dec pair, pre-persisted before the broadcast.
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
                }
                val username = preview.inviterUsername
                if (username != null) {
                    contactPrompt = username to newIdentityId
                } else {
                    onClose()
                }
            } catch (t: Throwable) {
                errorMessage = t.message ?: "Claiming the invitation failed."
            } finally {
                isClaiming = false
                appUiState.invitationClaimInFlight.value = false
            }
        }
    }

    fun sendContact(username: String, newIdentityId: ByteArray) {
        val wallet = claimWallet ?: return
        val mgr = manager ?: return
        container.applicationScope.launch {
            try {
                // Resolve the inviter's identity id from the link's username
                // via wallet-scoped DPNS search (exact-label match — the id
                // is not on the wire).
                val inviter = parseDpnsSearchResults(
                    wallet.dashpay.searchDpnsNames(username, 10),
                ).firstOrNull { it.label.equals(username, ignoreCase = true) }
                if (inviter == null) {
                    errorMessage = "Identity claimed, but $username couldn't be found to add."
                    return@launch
                }
                wallet.dashpay.sendContactRequest(
                    senderIdentityId = newIdentityId,
                    recipientIdentityId = inviter.identityId,
                    signerHandle = mgr.signerHandle,
                    coreSignerHandle = mgr.mnemonicResolverHandle,
                ).close()
                onClose()
            } catch (t: Throwable) {
                errorMessage = t.message ?: "Sending the contact request failed."
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
            onClick = { if (!isClaiming) onClose() },
            enabled = !isClaiming,
            modifier = Modifier.fillMaxWidth(),
        ) { Text("Cancel") }
    }

    contactPrompt?.let { (username, newIdentityId) ->
        AlertDialog(
            onDismissRequest = { /* explicit choice required */ },
            title = { Text("Add $username?") },
            text = { Text("Send a contact request to the person who invited you.") },
            confirmButton = {
                TextButton(onClick = {
                    contactPrompt = null
                    sendContact(username, newIdentityId)
                }) { Text("Add") }
            },
            dismissButton = {
                TextButton(onClick = {
                    contactPrompt = null
                    onClose()
                }) { Text("Not now") }
            },
        )
    }
}
