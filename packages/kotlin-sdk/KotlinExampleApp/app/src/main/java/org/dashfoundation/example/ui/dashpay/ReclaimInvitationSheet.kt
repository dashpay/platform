package org.dashfoundation.example.ui.dashpay

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
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
import org.dashfoundation.dashsdk.persistence.entities.InvitationEntity
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.services.DashpayKeyProvisioning
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.dashpay.InvitationReclaimLogic.ReclaimOutcome
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.formatDuffs
import org.dashfoundation.example.util.toHex

/**
 * Reclaim-invitation sheet — port of `ReclaimInvitationSheet.swift`. The
 * inviter consumes a still-unclaimed voucher into a Platform identity of
 * their own; the DASH was OP_RETURN-burned at create, so the value is
 * **recovered as identity credits**, never L1 Dash. Two targets: top up an
 * existing identity, or register a new one (base 4-key set — a reclaim
 * sends no contact request).
 *
 * Crash-safety contract (verbatim from iOS):
 * - In-memory [isReclaiming] single-flights submit AND dismissal — the
 *   persisted `reclaimInFlight` marker is crash forensics, never the
 *   concurrency guard (a Room-Flow re-emit recomposes this sheet; an
 *   unguarded second consume would let the loser's classifier overwrite
 *   Reclaimed with Claimed).
 * - The marker is persisted (and the write MUST succeed) only immediately
 *   before the on-chain consume; the register arm's key pre-persist runs
 *   BEFORE the marker so a purely local failure never strands one.
 * - Terminal status + marker are saved in one statement; failures resolve
 *   through the pure [InvitationReclaimLogic] classifier.
 *
 * The consume runs in the application scope under [NonCancellable] for the
 * marker → consume → status sequence, so a sheet teardown can never leave a
 * consumed voucher with an unwritten terminal status.
 */
@Composable
fun ReclaimInvitationSheet(
    invitation: InvitationEntity,
    wallet: ManagedPlatformWallet,
    onBusyChange: (Boolean) -> Unit = {},
    onClose: () -> Unit,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    // Identities on THIS wallet (the reclaim targets for top-up, and the
    // used-slot set for the register arm's next-unused index).
    val walletOwned by remember(network) {
        container.database.identityDao().observeWalletOwnedByNetwork(network.ffiValue)
    }.collectAsStateWithLifecycle(emptyList())
    val walletIdentities = remember(walletOwned) {
        walletOwned.filter { it.walletId?.contentEquals(invitation.walletId) == true }
    }

    var targetTopUp by remember { mutableStateOf(true) }
    var selectedIdentityB58 by remember { mutableStateOf<String?>(null) }
    val selectedIdentity = remember(walletIdentities, selectedIdentityB58) {
        walletIdentities.firstOrNull { Base58.encode(it.identityId) == selectedIdentityB58 }
            ?: walletIdentities.firstOrNull()
    }
    var isReclaiming by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var infoMessage by remember { mutableStateOf<String?>(null) }

    val canReclaim = !isReclaiming && invitation.statusRaw == 0 &&
        (!targetTopUp || selectedIdentity != null)

    fun reclaim() {
        if (!canReclaim || isReclaiming) return
        isReclaiming = true
        onBusyChange(true)
        errorMessage = null
        infoMessage = null
        val mgr = manager ?: run {
            errorMessage = "No wallet loaded."
            isReclaiming = false
            onBusyChange(false)
            return
        }
        val dao = container.database.invitationDao()
        val hex = invitation.outPointHex
        container.applicationScope.launch {
            var hadPriorReclaimInFlight = false
            try {
                withContext(NonCancellable) {
                    val (txid, vout) = InvitationReclaimLogic.outPointParts(invitation.rawOutPoint)

                    // Persist the in-flight marker ONLY immediately before the
                    // on-chain consume — never before pre-broadcast local work.
                    // The write must SUCCEED before the consume may run (an
                    // unpersisted marker + consume + crash would strand the
                    // row). The persisted prior value is captured first: it is
                    // what downgrades a later "already consumed" from
                    // "provably a foreign claim" to "explicitly ambiguous".
                    suspend fun markInFlight() {
                        hadPriorReclaimInFlight =
                            dao.getByOutPointHex(hex)?.reclaimInFlight ?: false
                        val updated = dao.setReclaimInFlight(hex, true, System.currentTimeMillis())
                        check(updated == 1) {
                            "invitation row vanished before the reclaim marker landed"
                        }
                    }

                    if (targetTopUp) {
                        val identity = selectedIdentity
                            ?: error("Pick an identity to top up.")
                        markInFlight()
                        mgr.identityCredits.reclaimInvitationAsTopUp(
                            walletHandle = wallet.handle,
                            identityId = identity.identityId,
                            outPointTxid = txid,
                            outPointVout = vout,
                            coreSignerHandle = mgr.mnemonicResolverHandle,
                        )
                    } else {
                        // Spans committed identities AND slots held by
                        // in-flight registrations on the app-scoped
                        // coordinator (no Room row exists for those yet).
                        val heldByCoordinator = container.registrationCoordinator
                            .controllers.value.keys
                            .filter { it.walletIdHex == wallet.walletId.toHex() }
                            .map { it.identityIndex }
                        val identityIndex = InvitationReclaimLogic.nextUnusedIdentityIndex(
                            walletIdentities.map { it.identityIndex } + heldByCoordinator,
                        )
                        // Pre-broadcast local work — BEFORE the marker, so a
                        // failure here leaves no in-flight marker. Base 4-key
                        // set: a reclaim sends no contact request (iOS
                        // authKeyCount = 4).
                        val previews = mgr.identityRegistration.previewRegistrationKeySet(
                            walletHandle = wallet.handle,
                            mnemonicResolverHandle = mgr.mnemonicResolverHandle,
                            identityIndex = identityIndex,
                            count = org.dashfoundation.dashsdk.identity.RegistrationKeys
                                .keyCount(includeDashPayKeys = false),
                        )
                        val keySet = DashpayKeyProvisioning.provision(
                            previews = previews,
                            includeDashPayKeys = false,
                            walletId = wallet.walletId,
                            persister = { keyHex, priv, owner ->
                                container.walletStorage.storePrivateKey(
                                    keyHex, priv, ownerWalletId = owner,
                                )
                            },
                        )
                        markInFlight()
                        mgr.identityRegistration.reclaimInvitationAsNewIdentity(
                            walletHandle = wallet.handle,
                            outPointTxid = txid,
                            outPointVout = vout,
                            identityIndex = identityIndex,
                            keys = keySet,
                            signerHandle = mgr.signerHandle,
                            coreSignerHandle = mgr.mnemonicResolverHandle,
                        )
                    }

                    // Room is the UI source: flip the local row to Reclaimed
                    // and clear the marker in one statement.
                    dao.setStatusAndMarker(hex, 2, false, System.currentTimeMillis())
                }
                onClose()
            } catch (t: Throwable) {
                when (InvitationReclaimLogic.classifyReclaimFailure(t, hadPriorReclaimInFlight)) {
                    ReclaimOutcome.RECLAIMED -> {
                        dao.setStatusAndMarker(hex, 2, false, System.currentTimeMillis())
                        infoMessage = "This invitation was already reclaimed by this " +
                            "wallet. The credits were delivered to the target selected " +
                            "for that reclaim."
                    }
                    ReclaimOutcome.CLAIMED -> {
                        // Neutral copy — the claimant is intentionally not named.
                        dao.setStatusAndMarker(hex, 1, false, System.currentTimeMillis())
                        infoMessage = "This invitation was already claimed."
                    }
                    ReclaimOutcome.CONSUMED_AMBIGUOUS -> {
                        // Provably consumed, but attribution is unknowable with
                        // our own attempt in flight — conservative terminal
                        // Claimed, never an inferred Reclaimed.
                        dao.setStatusAndMarker(hex, 1, false, System.currentTimeMillis())
                        infoMessage = "This invitation was already consumed — by the " +
                            "invitee's claim, or possibly by your own earlier " +
                            "interrupted reclaim. If that reclaim went through, the " +
                            "credits were delivered to the target you selected then."
                    }
                    ReclaimOutcome.UNTRACKED_AFTER_OWN_ATTEMPT -> {
                        // No on-chain proof of consumption at all — status and
                        // marker stay untouched; surface the ambiguity.
                        errorMessage = "This voucher is no longer tracked by the wallet " +
                            "after an earlier interrupted reclaim attempt. It may " +
                            "already have been consumed by that attempt — check the " +
                            "balance of the identity you targeted then before retrying."
                    }
                    ReclaimOutcome.ERROR -> {
                        if (InvitationReclaimLogic.shouldClearInFlightMarker(
                                t, hadPriorReclaimInFlight,
                            )
                        ) {
                            // This attempt set the marker itself and then failed
                            // the LOCAL resume guard — the consume never started,
                            // so the freshly-set marker is demonstrably stale.
                            dao.setReclaimInFlight(hex, false, System.currentTimeMillis())
                        }
                        errorMessage = t.message ?: "Reclaiming the invitation failed."
                    }
                }
            } finally {
                isReclaiming = false
                onBusyChange(false)
            }
        }
    }

    Column(
        modifier = Modifier.fillMaxWidth().padding(16.dp).testTag("dashpay.invite.reclaim"),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("Reclaim Invitation", style = MaterialTheme.typography.titleLarge)
        Text(
            "${formatDuffs(invitation.amountDuffs)} voucher · " +
                invitation.outPointHex.take(12) + "…",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "The value is recovered as identity credits (the invitation burned " +
                "the DASH into Platform). The invitee can no longer claim it afterwards.",
            style = MaterialTheme.typography.bodyMedium,
        )

        Text("Recover into", style = MaterialTheme.typography.labelLarge)
        Row(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.testTag("dashpay.invite.reclaim.target"),
        ) {
            FilterChip(
                selected = targetTopUp,
                onClick = { if (!isReclaiming) targetTopUp = true },
                label = { Text("Existing identity") },
            )
            FilterChip(
                selected = !targetTopUp,
                onClick = { if (!isReclaiming) targetTopUp = false },
                label = { Text("New identity") },
            )
        }

        if (targetTopUp) {
            val pickerSelection = selectedIdentity
            if (walletIdentities.isEmpty() || pickerSelection == null) {
                Text(
                    "No identities on this wallet yet — register a new one instead.",
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            } else {
                AccessiblePicker(
                    label = "Identity",
                    options = walletIdentities,
                    selected = pickerSelection,
                    optionLabel = {
                        it.mainDpnsName ?: it.dpnsName ?: Base58.encode(it.identityId).take(12)
                    },
                    testTag = "dashpay.invite.reclaim.identityPicker",
                ) { selectedIdentityB58 = Base58.encode(it.identityId) }
            }
        } else {
            Text(
                "A brand-new identity funded by this voucher.",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }

        infoMessage?.let { Text(it, color = MaterialTheme.colorScheme.tertiary) }
        errorMessage?.let { Text(it, color = MaterialTheme.colorScheme.error) }

        Button(
            onClick = { reclaim() },
            enabled = canReclaim,
            modifier = Modifier.fillMaxWidth().testTag("dashpay.invite.reclaim.submit"),
        ) {
            if (isReclaiming) {
                CircularProgressIndicator(Modifier.size(18.dp))
                Text("  Reclaiming…")
            } else {
                Text("Reclaim")
            }
        }
        TextButton(
            onClick = { if (!isReclaiming) onClose() },
            enabled = !isReclaiming,
            modifier = Modifier.fillMaxWidth(),
        ) { Text("Cancel") }
    }
}
