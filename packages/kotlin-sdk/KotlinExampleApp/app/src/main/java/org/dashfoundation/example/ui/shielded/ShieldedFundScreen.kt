package org.dashfoundation.example.ui.shielded

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.dashsdk.funding.ShieldedProver
import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.ShieldedFundProgress
import org.dashfoundation.example.services.shielded.ShieldedFundFromAssetLockCoordinator.StartFundingResult
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.funding.canFundIdentity
import org.dashfoundation.example.ui.funding.parseOutPoint
import org.dashfoundation.example.ui.funding.shortOutPointDisplay
import org.dashfoundation.example.ui.funding.statusLabel
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * Shield funds from an asset lock — port of `ShieldedFundFromAssetLockView.swift`.
 * Gated on shielded support ([ShieldedGate]). Shows the Halo 2 prover
 * readiness (via [ShieldedProver.isReady], warming it on entry) and the
 * consensus-pinned shield fee ([ShieldedProver.estimateFee]).
 *
 * Submit is wired to the real shield FFI: the recipient defaults to the
 * wallet's own bound shielded address ("shield to self", via
 * [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.shieldedDefaultAddress]),
 * overridable with an 86-char hex (= 43 raw bytes). The funding runs through
 * the [org.dashfoundation.example.services.shielded.ShieldedFundFromAssetLockCoordinator]
 * (per-(wallet, recipient) slot + per-wallet serialization), calling
 * `platform_wallet_manager_shielded_fund_from_asset_lock`; the screen then
 * navigates to the dismissal-safe progress view. The Orchard note arrives on
 * the next shielded sync pass, not synchronously.
 *
 * RESUME mode (← the Swift view's `resumeFromLock` parameter): when
 * [resumeOutPointHex] is non-null the screen hides the Amount section (the
 * lock and its amount were fixed at original build time) and routes Submit to
 * [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.shieldedResumeFundFromAssetLock]
 * instead of `shieldedFundFromAssetLock`, seeded with the parsed outpoint.
 * The recipient is still chosen here — a shielded orphan lock carries no
 * recipient stamp, because the Orchard recipient is an external address
 * picked at ST-submit time, not allocated from the wallet.
 *
 * This is the resume path for `fundingTypeRaw == 5`
 * (AssetLockShieldedAddressTopUp) rows on the "Pending Platform Top Ups"
 * surface. Before it existed those rows had nowhere to go: the surface only
 * queried funding type 4, and its Resume opened the platform-ADDRESS screen,
 * whose submit calls the wrong FFI for a shielded lock.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ShieldedFundScreen(
    walletIdHex: String,
    navController: NavHostController,
    resumeOutPointHex: String? = null,
) {
    ShieldedGate(navController) {
        val container = LocalAppContainer.current
        val walletId = remember(walletIdHex) { walletIdHex.hexToBytes() }
        val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
        val isResume = resumeOutPointHex != null

        var amountText by rememberSaveable { mutableStateOf("") }
        var recipientHex by rememberSaveable { mutableStateOf("") }
        var error by remember { mutableStateOf<String?>(null) }
        var isSubmitting by remember { mutableStateOf(false) }

        // In resume mode, load the tracked lock so we can show its amount +
        // status. Keyed by `outPointHex` (primary key); a null result means
        // the lock was swept between opening the list and this screen.
        // ← FundFromAssetLockScreen's identical resume-mode load.
        val resumeLock by produceState<AssetLockEntity?>(
            initialValue = null,
            resumeOutPointHex,
        ) {
            value = resumeOutPointHex?.let {
                container.database.assetLockDao().getByOutPointHex(it)
            }
        }

        // Prover status + fee estimate (bridged, single-note-with-change = 2 actions).
        val proverReady by produceState(initialValue = false) {
            runCatching { ShieldedProver.warmUp() }
            value = runCatching { ShieldedProver.isReady() }.getOrDefault(false)
        }
        val feeEstimate by produceState<Long?>(initialValue = null) {
            value = runCatching {
                ShieldedProver.estimateFee(ShieldedProver.FeeKind.TransferOrShield, 2)
            }.getOrNull()
        }

        // Default "shield to self" recipient — the wallet's bound shielded
        // address (null until the wallet is bound), mirroring the Swift view.
        val defaultRecipient by produceState<ByteArray?>(initialValue = null, manager) {
            value = manager?.let { m ->
                runCatching { m.shieldedDefaultAddress(walletId) }.getOrNull()
            }
        }

        // The effective recipient: user hex override (86 hex chars = 43 bytes)
        // when valid, else the wallet's default.
        val trimmedHex = recipientHex.trim()
        val overrideRecipient = remember(trimmedHex) {
            if (trimmedHex.length == 86) runCatching { trimmedHex.hexToBytes() }
                .getOrNull()?.takeIf { it.size == 43 }
            else null
        }
        val recipient = overrideRecipient ?: defaultRecipient

        val amount = amountText.toLongOrNull()
        // Resume only needs a recipient (+ the loaded lock): the shield value
        // is derived Rust-side from the existing lock. Fresh needs an amount
        // too. ← FundFromAssetLockScreen's `canSubmit`.
        val canSubmit = manager != null && recipient != null && !isSubmitting &&
            if (isResume) resumeLock != null else (amount != null && amount > 0)

        Scaffold(
            topBar = {
                TopAppBar(
                    title = {
                        Text(if (isResume) "Resume Shield" else "Shield from Asset Lock")
                    },
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
                    .imePadding()
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                FormSection(title = "Prover") {
                    LabeledContent(
                        "Halo 2 prover",
                        if (proverReady) "Ready" else "Preparing…",
                    )
                    LabeledContent(
                        "Estimated fee",
                        feeEstimate?.let { "$it credits" } ?: "—",
                    )
                }

                if (isResume) {
                    // Read-only summary of the lock being resumed — replaces
                    // the Amount section (the locked amount is fixed by the
                    // original build). ← Swift `resumeFromAssetLockSection`.
                    FormSection(title = "Resuming") {
                        val lock = resumeLock
                        if (lock == null) {
                            Text(
                                "This asset lock is no longer tracked. Return to the " +
                                    "Pending Platform Top Ups list.",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.error,
                                modifier = Modifier.testTag("shieldedFund.resume.missing"),
                            )
                        } else {
                            Text(
                                "Asset Lock ${lock.shortOutPointDisplay}",
                                style = MaterialTheme.typography.bodyMedium,
                                modifier = Modifier.testTag("shieldedFund.resume.outpoint"),
                            )
                            Text(
                                "${lock.amountDuffs} duffs · ${lock.statusLabel}",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            Text(
                                if (lock.canFundIdentity) {
                                    "The asset lock already reached a usable proof state. " +
                                        "Pick a recipient to complete the shield."
                                } else {
                                    "The asset lock is broadcast and still awaiting " +
                                        "InstantSend / ChainLock finality. Resuming will wait " +
                                        "for finality, then shield into the pool."
                                },
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }

                FormSection(title = "Recipient") {
                    OutlinedTextField(
                        value = recipientHex,
                        onValueChange = { recipientHex = it },
                        label = { Text("Orchard address (hex, blank = shield to self)") },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth().testTag("shieldedFund.recipient"),
                    )
                    Text(
                        when {
                            overrideRecipient != null -> "Using pasted recipient."
                            defaultRecipient != null -> "Defaulting to this wallet's shielded address."
                            else -> "This wallet has no bound shielded address yet — paste a recipient."
                        },
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }

                if (!isResume) {
                    FormSection(title = "Amount") {
                        OutlinedTextField(
                            value = amountText,
                            onValueChange = { amountText = it.filter(Char::isDigit) },
                            label = { Text("Amount (duffs)") },
                            singleLine = true,
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                            modifier = Modifier.fillMaxWidth().testTag("shieldedFund.amount"),
                        )
                    }
                }

                SubmitButton(
                    text = if (isResume) "Resume Shield" else "Shield",
                    isLoading = isSubmitting,
                    enabled = canSubmit,
                    modifier = Modifier.fillMaxWidth().testTag("shieldedFund.submit"),
                ) {
                    val m = manager ?: return@SubmitButton
                    val recipientBytes = recipient ?: return@SubmitButton

                    // Resume mode dispatches to a different FFI than a fresh
                    // shield, so resolve the whole body up front — including
                    // the outpoint parse, which must not fail after the
                    // coordinator has already claimed the slot. The operation
                    // id carries the resumed lock's outpoint: resumable locks
                    // default to the same wallet-owned shielded recipient, so
                    // the coordinator's slot key alone cannot tell two locks
                    // apart and would silently reuse the first controller.
                    val (operationId, submitBody) = if (isResume) {
                        val lock = resumeLock ?: return@SubmitButton
                        val parsed = parseOutPoint(lock.outPointHex)
                        if (parsed == null) {
                            error = "Could not parse asset lock outpoint: ${lock.outPointHex}"
                            return@SubmitButton
                        }
                        val (txid, vout) = parsed
                        val body: suspend () -> Unit = {
                            m.shieldedResumeFundFromAssetLock(
                                walletId = walletId,
                                outPointTxid = txid,
                                outPointVout = vout,
                                recipientRaw43 = recipientBytes,
                            )
                        }
                        "resume:${lock.outPointHex}" to body
                    } else {
                        val amountDuffs = amount ?: return@SubmitButton
                        val body: suspend () -> Unit = {
                            m.shieldedFundFromAssetLock(
                                walletId = walletId,
                                recipientRaw43 = recipientBytes,
                                amountDuffs = amountDuffs,
                            )
                        }
                        "shield" to body
                    }

                    isSubmitting = true
                    // Start the funding through the coordinator (dismissal-safe,
                    // per-wallet serialized). The body performs the shield FFI.
                    // Resume shares the coordinator with fresh shields on
                    // purpose: both consume the same per-wallet shield_guard
                    // Rust-side, so a resume racing a fresh shield on one
                    // wallet has to be blocked by the same gate.
                    val result = container.shieldedFundCoordinator.startFunding(
                        walletId = walletId,
                        recipientRaw43 = recipientBytes,
                        operationId = operationId,
                        body = submitBody,
                    )
                    when (result) {
                        is StartFundingResult.Started -> {
                            navController.navigate(
                                ShieldedFundProgress(walletIdHex, recipientBytes.toHex()),
                            )
                        }
                        is StartFundingResult.BlockedByOtherWalletFunding -> {
                            error = "Another shielded funding is already in flight on this " +
                                "wallet (recipient ${result.blocker.recipientRaw43.toHex().take(16)}…). " +
                                "Shield-class operations are serialized per wallet — wait for it to finish."
                        }
                    }
                    isSubmitting = false
                }
            }
        }

        ErrorAlertDialog(message = error, onDismiss = { error = null })
    }
}
