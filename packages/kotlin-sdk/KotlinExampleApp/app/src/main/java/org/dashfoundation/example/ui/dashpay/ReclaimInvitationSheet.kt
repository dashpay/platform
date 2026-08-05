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
    val isSdkLoading by appState.isLoading.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val appUiState = container.appUiState
    val reclaimState by appUiState.reclaimInvitation.collectAsStateWithLifecycle()

    // Identities on THIS wallet (the reclaim targets for top-up, and the
    // used-slot set for the register arm's next-unused index).
    val walletOwned by remember(network) {
        container.database.identityDao().observeWalletOwnedByNetwork(network.ffiValue)
    }.collectAsStateWithLifecycle(emptyList())
    val walletIdentities = remember(walletOwned) {
        walletOwned.filter { it.walletId?.contentEquals(invitation.walletId) == true }
    }

    val retainedSnapshot = when (val state = reclaimState) {
        is org.dashfoundation.example.state.AppUiState.ReclaimInvitationState.InFlight ->
            state.snapshot
        is org.dashfoundation.example.state.AppUiState.ReclaimInvitationState.Failed ->
            state.snapshot
        else -> null
    }?.takeIf { it.outPointHex == invitation.outPointHex }
    var targetTopUp by remember(invitation.outPointHex) {
        mutableStateOf(
            retainedSnapshot == null ||
                retainedSnapshot.target is org.dashfoundation.example.state.AppUiState
                    .ReclaimTarget.TopUp,
        )
    }
    var selectedIdentityB58 by remember(invitation.outPointHex) {
        mutableStateOf(
            (retainedSnapshot?.target as? org.dashfoundation.example.state.AppUiState
                .ReclaimTarget.TopUp)?.identityId?.let(Base58::encode),
        )
    }
    val selectedIdentity = remember(walletIdentities, selectedIdentityB58) {
        walletIdentities.firstOrNull { Base58.encode(it.identityId) == selectedIdentityB58 }
            ?: walletIdentities.firstOrNull()
    }
    var validationError by remember { mutableStateOf<String?>(null) }
    val isReclaiming = reclaimState is org.dashfoundation.example.state.AppUiState
        .ReclaimInvitationState.InFlight
    val stateForRow = when (val state = reclaimState) {
        is org.dashfoundation.example.state.AppUiState.ReclaimInvitationState.Completed ->
            state.takeIf { it.outPointHex == invitation.outPointHex }
        is org.dashfoundation.example.state.AppUiState.ReclaimInvitationState.Failed ->
            state.takeIf { it.snapshot.outPointHex == invitation.outPointHex }
        else -> null
    }
    val errorMessage =
        (stateForRow as? org.dashfoundation.example.state.AppUiState.ReclaimInvitationState.Failed)
            ?.safeError ?: validationError
    val infoMessage =
        (stateForRow as? org.dashfoundation.example.state.AppUiState.ReclaimInvitationState.Completed)
            ?.message

    val managerReady = !isSdkLoading && manager?.network == network
    val canReclaim = managerReady && canSubmitInvitationReclaim(
        isReclaiming = isReclaiming,
        statusRaw = invitation.statusRaw,
        targetTopUp = targetTopUp,
        hasSelectedIdentity = selectedIdentity != null,
        completedForRow = stateForRow is org.dashfoundation.example.state.AppUiState
            .ReclaimInvitationState.Completed,
    )

    fun reclaim() {
        if (!canReclaim || isReclaiming) return
        val mgr = manager ?: run {
            validationError = "No wallet loaded."
            return
        }
        val dao = container.database.invitationDao()
        val hex = invitation.outPointHex
        val reclaimAsTopUp = targetTopUp
        val targetIdentity = selectedIdentity
        val snapshot = org.dashfoundation.example.state.AppUiState.ReclaimSnapshot(
            networkRaw = network.ffiValue,
            outPointHex = hex,
            target = if (reclaimAsTopUp) {
                org.dashfoundation.example.state.AppUiState.ReclaimTarget.TopUp(
                    checkNotNull(targetIdentity).identityId.copyOf(),
                )
            } else {
                org.dashfoundation.example.state.AppUiState.ReclaimTarget.Register
            },
        )
        val operationId = appUiState.beginInvitationReclaim(snapshot) ?: return
        validationError = null
        onBusyChange(true)
        container.applicationScope.launch {
            var hadPriorReclaimInFlight = false
            try {
                withContext(NonCancellable) {
                    val permit = if (reclaimAsTopUp) {
                        container.registrationCoordinator.beginInvitationOperation(wallet.walletId)
                    } else {
                        container.registrationCoordinator.beginInvitationRegistration(wallet.walletId)
                    }
                    try {
                        check(container.isCurrentInvitationManager(snapshot.networkRaw, mgr)) {
                            "The active invitation manager changed."
                        }
                        val (txid, vout) =
                            InvitationReclaimLogic.outPointParts(invitation.rawOutPoint)

                        suspend fun markInFlight() {
                            hadPriorReclaimInFlight =
                                dao.getByOutPointHex(hex)?.reclaimInFlight ?: false
                            val updated =
                                dao.setReclaimInFlight(hex, true, System.currentTimeMillis())
                            check(updated == 1) {
                                "invitation row vanished before the reclaim marker landed"
                            }
                        }

                        if (reclaimAsTopUp) {
                            val identity = targetIdentity
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
                            val identityIndex = checkNotNull(permit.identityIndex)
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

                        check(
                            dao.setStatusAndMarker(
                                hex,
                                2,
                                false,
                                System.currentTimeMillis(),
                            ) == 1,
                        ) { "invitation row vanished before reclaim completion was saved" }
                    } finally {
                        container.registrationCoordinator.endInvitationOperation(permit)
                    }
                }
                appUiState.completeInvitationReclaim(
                    operationId,
                    "The invitation was reclaimed into identity credits.",
                )
            } catch (t: Throwable) {
                when (InvitationReclaimLogic.classifyReclaimFailure(t, hadPriorReclaimInFlight)) {
                    ReclaimOutcome.RECLAIMED -> {
                        val updated =
                            dao.setStatusAndMarker(hex, 2, false, System.currentTimeMillis())
                        if (updated == 1) {
                            appUiState.completeInvitationReclaim(
                                operationId,
                                "This invitation was already reclaimed by this wallet. " +
                                    "The credits were delivered to the target selected " +
                                    "for that reclaim.",
                            )
                        } else {
                            appUiState.failInvitationReclaim(
                                operationId,
                                "The invitation row vanished while saving reclaimed status.",
                            )
                        }
                    }
                    ReclaimOutcome.CLAIMED -> {
                        // Neutral copy — the claimant is intentionally not named.
                        val updated =
                            dao.setStatusAndMarker(hex, 1, false, System.currentTimeMillis())
                        if (updated == 1) {
                            appUiState.completeInvitationReclaim(
                                operationId,
                                "This invitation was already claimed.",
                            )
                        } else {
                            appUiState.failInvitationReclaim(
                                operationId,
                                "The invitation row vanished while saving claimed status.",
                            )
                        }
                    }
                    ReclaimOutcome.CONSUMED_AMBIGUOUS -> {
                        // Provably consumed, but attribution is unknowable with
                        // our own attempt in flight — conservative terminal
                        // Claimed, never an inferred Reclaimed.
                        val updated =
                            dao.setStatusAndMarker(hex, 1, false, System.currentTimeMillis())
                        if (updated == 1) {
                            appUiState.completeInvitationReclaim(
                                operationId,
                                "This invitation was already consumed — by the " +
                                    "invitee's claim, or possibly by your own earlier " +
                                    "interrupted reclaim. If that reclaim went through, " +
                                    "the credits were delivered to the target selected then.",
                            )
                        } else {
                            appUiState.failInvitationReclaim(
                                operationId,
                                "The invitation row vanished while saving consumed status.",
                            )
                        }
                    }
                    ReclaimOutcome.UNTRACKED_AFTER_OWN_ATTEMPT -> {
                        // No on-chain proof of consumption at all — status and
                        // marker stay untouched; surface the ambiguity.
                        appUiState.failInvitationReclaim(
                            operationId,
                            "This voucher is no longer tracked by the wallet after an " +
                                "earlier interrupted reclaim attempt. It may already have " +
                                "been consumed — check the target identity balance before retrying.",
                        )
                    }
                    ReclaimOutcome.ERROR -> {
                        if (InvitationReclaimLogic.shouldClearInFlightMarker(
                                t, hadPriorReclaimInFlight,
                            )
                        ) {
                            // This attempt set the marker itself and then failed
                            // the LOCAL resume guard — the consume never started,
                            // so the freshly-set marker is demonstrably stale.
                            val updated = dao.setReclaimInFlight(
                                hex,
                                false,
                                System.currentTimeMillis(),
                            )
                            if (updated != 1) {
                                appUiState.failInvitationReclaim(
                                    operationId,
                                    "The invitation row vanished while clearing reclaim state.",
                                )
                                return@launch
                            }
                        }
                        appUiState.failInvitationReclaim(
                            operationId,
                            "Reclaiming the invitation failed.",
                        )
                    }
                }
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
            onClick = {
                if (!isReclaiming) {
                    appUiState.clearInvitationReclaim()
                    onClose()
                }
            },
            enabled = !isReclaiming,
            modifier = Modifier.fillMaxWidth(),
        ) { Text("Cancel") }
    }
}

internal fun canSubmitInvitationReclaim(
    isReclaiming: Boolean,
    statusRaw: Int,
    targetTopUp: Boolean,
    hasSelectedIdentity: Boolean,
    completedForRow: Boolean,
): Boolean =
    !isReclaiming &&
        !completedForRow &&
        statusRaw == 0 &&
        (!targetTopUp || hasSelectedIdentity)
