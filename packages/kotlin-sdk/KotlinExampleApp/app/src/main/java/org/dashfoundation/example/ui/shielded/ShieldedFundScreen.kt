package org.dashfoundation.example.ui.shielded

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
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
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.LifecycleResumeEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.funding.ShieldedProver
import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.AssetLockBuiltOutPoint
import org.dashfoundation.dashsdk.wallet.WalletDrainRules
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.ShieldedFundProgress
import org.dashfoundation.example.services.shielded.ShieldedFundFromAssetLockCoordinator.StartFundingResult
import org.dashfoundation.example.services.shielded.WalletDrainCoordinator
import org.dashfoundation.example.services.shielded.asWalletDrainWallet
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.funding.canFundIdentity
import org.dashfoundation.example.ui.funding.parseOutPoint
import org.dashfoundation.example.ui.funding.shortOutPointDisplay
import org.dashfoundation.example.ui.funding.statusLabel
import org.dashfoundation.example.util.PaymentFailure
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex
import java.util.UUID

/**
 * Shield funds from an asset lock — port of `ShieldedFundFromAssetLockView.swift`.
 * Gated on shielded support ([ShieldedGate]). Shows the Halo 2 prover
 * readiness (via [ShieldedProver.isReady], warming it on entry) and the
 * consensus-pinned shield fee
 * ([org.dashfoundation.dashsdk.wallet.PlatformWalletManager.estimateShieldedFee],
 * computed at the manager's network-tracked platform version).
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
 * A lock that recorded its recipient when it was built
 * ([AssetLockEntity.shieldedRecipientRaw43]) pays that recipient: Rust
 * refuses any other, so the screen shows it read-only. Only an older lock
 * without one lets the recipient be chosen here (← the Swift view's
 * `lockBoundRecipient`).
 *
 * MAX mode (← the Swift view's `drainMax` toggle, fresh-shield flow only):
 * when the "Max" switch is on, the Amount and Recipient fields are replaced
 * by [WalletDrainPreviewSection], Rust's estimate of what a drain of Core
 * account index 0 would sweep, and Submit starts a wallet drain through the
 * [WalletDrainCoordinator], which stores the operation id before anything
 * is called. Max pays this wallet's own shielded address, read at submit:
 * the drain sweeps whatever is eligible when it runs, which can exceed the
 * estimate, so no other recipient can be named. Whether a new Max is
 * offered on the current network, and whether a preview allows a start,
 * are Rust's rules ([WalletDrainRules]). An unfinished Max is recovered
 * from the wallet screen ([UnfinishedWalletDrainsList]), not from here.
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
        val appState = LocalAppState.current
        val walletId = remember(walletIdHex) { walletIdHex.hexToBytes() }
        val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
        val isResume = resumeOutPointHex != null

        var amountText by rememberSaveable { mutableStateOf("") }
        var recipientHex by rememberSaveable { mutableStateOf("") }
        var error by remember { mutableStateOf<String?>(null) }
        var isSubmitting by remember { mutableStateOf(false) }
        val scope = rememberCoroutineScope()

        // Max: sweep everything at Core account index 0 through a wallet
        // drain to the wallet's own shielded address. Rust picks the inputs
        // and the amount; the screen only shows its estimate.
        var drainMax by rememberSaveable { mutableStateOf(false) }
        // The estimate with the manager, wallet and account index it was
        // read for.
        var drainQuote by remember { mutableStateOf<WalletDrainQuote?>(null) }
        val network by appState.currentNetwork.collectAsStateWithLifecycle()
        // Rust's rule for whether a NEW Max is offered on this network. A
        // rule that cannot be asked offers nothing.
        val maxOffered = remember(network) {
            runCatching { WalletDrainRules.startOffered(network) }.getOrDefault(false)
        }
        val isMax = !isResume && drainMax && maxOffered

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
        // Re-keyed on the published protocol version: the SDK learns the
        // network's version on a background refresh after the manager
        // exists, so an estimate produced before the ratchet completed was
        // computed at the seed version (← iOS SendTransactionView
        // re-resolves on `platformState.platformProtocolVersion`).
        val protocolVersion by appState.platformProtocolVersion.collectAsStateWithLifecycle()
        // ...and re-read on every resume, because the flow alone can go
        // stale: that refresh republishes the unchanged seed when its
        // proven fetch fails, and the SDK independently ratchets its
        // version from ANY proof-verified query's response metadata
        // without publishing at all. The estimate is a pure handle
        // lookup, so re-reading is free.
        var feeReadEpoch by remember { mutableIntStateOf(0) }
        LifecycleResumeEffect(Unit) {
            feeReadEpoch++
            onPauseOrDispose {}
        }
        val feeEstimate by produceState<Long?>(
            initialValue = null,
            manager,
            protocolVersion,
            feeReadEpoch,
        ) {
            value = manager?.let { m ->
                runCatching {
                    m.estimateShieldedFee(ShieldedProver.FeeKind.TransferOrShield, 2)
                }.getOrNull()
            }
        }

        // Default "shield to self" recipient — the wallet's bound shielded
        // address (null until the wallet is bound), mirroring the Swift view.
        val defaultRecipient by produceState<ByteArray?>(initialValue = null, manager) {
            value = manager?.let { m ->
                runCatching { m.shieldedDefaultAddress(walletId) }.getOrNull()
            }
        }

        // The recipient a resumed lock recorded when it was built. Rust pays
        // it and refuses any other, so the screen offers no choice.
        val lockBoundRecipient = if (isResume) lockBoundRecipient(resumeLock) else null

        // The effective recipient: the lock's recorded one, else the user
        // hex override (86 hex chars = 43 bytes) when valid, else the
        // wallet's default.
        val trimmedHex = recipientHex.trim()
        val overrideRecipient = remember(trimmedHex) {
            if (trimmedHex.length == 86) runCatching { trimmedHex.hexToBytes() }
                .getOrNull()?.takeIf { it.size == 43 }
            else null
        }
        val recipient = effectiveShieldRecipient(lockBoundRecipient, overrideRecipient, defaultRecipient)

        val amount = amountText.toLongOrNull()
        // Resume only needs a recipient (+ the loaded lock): the shield value
        // is derived Rust-side from the existing lock. Fresh needs an amount
        // too. ← FundFromAssetLockScreen's `canSubmit`.
        val canSubmit = manager != null && recipient != null && !isSubmitting &&
            if (isResume) resumeLock != null else (amount != null && amount > 0)

        // Max needs an estimate read for exactly these inputs, and Rust's
        // rule that it allows a start (only "nothing to shield" refuses).
        val currentQuote = manager?.let { m ->
            drainQuote?.takeIf { it.matches(m, walletId, MAX_FUNDING_ACCOUNT_INDEX) }
        }
        val quoteAllowsStart = remember(currentQuote) {
            currentQuote != null && runCatching {
                WalletDrainRules.previewAllowsStart(currentQuote.preview.status)
            }.getOrDefault(false)
        }
        val canSubmitMax = manager != null && !isSubmitting && quoteAllowsStart

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

                if (!isResume) {
                    FormSection(title = "Core Source") {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(
                                "Max (everything at this account index)",
                                style = MaterialTheme.typography.bodyMedium,
                                modifier = Modifier.weight(1f),
                            )
                            Switch(
                                checked = isMax,
                                onCheckedChange = {
                                    drainMax = it
                                    drainQuote = null
                                },
                                enabled = maxOffered && !isSubmitting,
                                modifier = Modifier.testTag("shieldFromCore.maxToggle"),
                            )
                        }
                        Text(
                            when {
                                !maxOffered -> CORE_SOURCE_TEXT + " " + MAX_UNAVAILABLE_TEXT
                                isMax ->
                                    "Max sweeps account index $MAX_FUNDING_ACCOUNT_INDEX: the BIP44 " +
                                        "and BIP32 accounts at that index together with every " +
                                        "DashPay receiving account. Standard funds at other " +
                                        "account indices are not swept. The amount is set by " +
                                        "the wallet, not typed here."
                                else -> CORE_SOURCE_TEXT
                            },
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }

                if (isMax) {
                    FormSection(title = "Recipient") {
                        Text(
                            defaultRecipient?.toHex() ?: "This wallet's shielded address is not available yet.",
                            style = MaterialTheme.typography.bodySmall,
                            modifier = Modifier.testTag("shieldFromCore.fixedRecipient"),
                        )
                        Text(
                            "Max pays this wallet's own shielded address. The swept amount " +
                                "can differ from the estimate, so no other recipient can be named.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    manager?.let { m ->
                        WalletDrainPreviewSection(
                            manager = m,
                            walletId = walletId,
                            fundingAccountIndex = MAX_FUNDING_ACCOUNT_INDEX,
                            quote = drainQuote,
                            onQuote = { drainQuote = it },
                        )
                    }
                }

                if (!isMax && lockBoundRecipient != null) FormSection(title = "Recipient") {
                    Text(
                        lockBoundRecipient.toHex(),
                        style = MaterialTheme.typography.bodySmall,
                        modifier = Modifier.testTag("shieldFromCore.fixedRecipient"),
                    )
                    Text(
                        "This asset lock is bound to this recipient: it was recorded when the " +
                            "lock was built, and resuming can only pay it.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }

                if (!isMax && lockBoundRecipient == null) FormSection(title = "Recipient") {
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

                if (!isResume && !isMax) {
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

                if (isMax) SubmitButton(
                    text = "Shield Max",
                    isLoading = isSubmitting,
                    enabled = canSubmitMax,
                    modifier = Modifier.fillMaxWidth().testTag("shieldFromCore.submitButton"),
                ) {
                    val m = manager ?: return@SubmitButton
                    val quote = currentQuote ?: return@SubmitButton
                    // Second boundary behind the disabled switch: nothing is
                    // stored or called for a Max this network does not offer.
                    if (!maxOffered) {
                        error = "Nothing was sent. $MAX_UNAVAILABLE_TEXT"
                        return@SubmitButton
                    }
                    if (!quoteAllowsStart) return@SubmitButton
                    isSubmitting = true
                    scope.launch {
                        try {
                            // The own address is read here, at submit, so
                            // neither a cached recipient nor the override
                            // text can be paid.
                            val own = try {
                                m.shieldedDefaultAddress(walletId)
                            } catch (e: CancellationException) {
                                throw e
                            } catch (e: Exception) {
                                error = "Could not read this wallet's shielded address, so " +
                                    "nothing was sent: ${e.message ?: e}"
                                return@launch
                            }
                            if (own == null || own.size != 43) {
                                error = "This wallet's shielded address is not available yet, " +
                                    "so nothing was sent."
                                return@launch
                            }
                            when (
                                val result = container.walletDrainCoordinator.start(
                                    walletId = walletId,
                                    network = network,
                                    fundingAccountIndex = quote.fundingAccountIndex,
                                    recipientRaw43 = own,
                                    wallet = m.asWalletDrainWallet(),
                                )
                            ) {
                                is WalletDrainCoordinator.StartResult.Started ->
                                    navController.navigate(
                                        ShieldedFundProgress(walletIdHex, own.toHex()),
                                    )
                                is WalletDrainCoordinator.StartResult.BlockedByOtherWalletFunding ->
                                    error = "Another shielded funding is already in progress on " +
                                        "this wallet (recipient " +
                                        "${result.blocker.recipientRaw43.toHex().take(16)}…). " +
                                        "Shield-class operations are serialized per wallet, so " +
                                        "try again after that one finishes."
                                is WalletDrainCoordinator.StartResult.MarkerWriteFailed ->
                                    error = result.message
                            }
                        } finally {
                            isSubmitting = false
                        }
                    }
                }

                if (!isMax) SubmitButton(
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
                    // A fresh shield mints a NEW id per submission: the
                    // coordinator retains a completed controller for ~30s,
                    // and a fixed marker would match it and rebind — showing
                    // the old result instead of running this submission's
                    // body. Minted here, at submission time, so returning to
                    // this form and shielding again is always a new
                    // operation ("resume:<outpoint>" stays stable so a
                    // re-tap of the SAME lock still rebinds).
                    // The lock a resume names, so a slot whose failure holds
                    // it may restart through it (and only it).
                    var resuming: AssetLockBuiltOutPoint? = null
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
                        resuming = PaymentFailure.outPoint(txid, vout)
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
                        "shield:${UUID.randomUUID()}" to body
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
                        resuming = resuming,
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

/**
 * The Core account index a Max sweeps. This screen has no account picker,
 * so it is the SDK wrapper's default, as the Swift view uses when no
 * account is picked.
 */
private const val MAX_FUNDING_ACCOUNT_INDEX = 0

private const val CORE_SOURCE_TEXT =
    "The Core account's UTXOs are locked into an asset lock; the locked DASH becomes " +
        "shielded credits on the destination Orchard address."

/**
 * The recipient a resumed [lock] recorded when it was built, or null for a
 * lock tracked before recipients were recorded (and for a stored value that
 * is not 43 bytes, which the wallet load already refuses). A resume pays it
 * and Rust refuses any other (← the Swift view's `lockBoundRecipient`).
 */
internal fun lockBoundRecipient(lock: AssetLockEntity?): ByteArray? =
    lock?.shieldedRecipientRaw43?.takeIf { it.size == 43 }

/**
 * The recipient Submit pays: the resumed lock's recorded one when it has
 * one, else the pasted [overrideRecipient], else the wallet's
 * [defaultRecipient].
 */
internal fun effectiveShieldRecipient(
    lockBound: ByteArray?,
    overrideRecipient: ByteArray?,
    defaultRecipient: ByteArray?,
): ByteArray? = lockBound ?: overrideRecipient ?: defaultRecipient

/** Shown where Rust's rule does not offer a new Max on the current network. */
private const val MAX_UNAVAILABLE_TEXT =
    "Max is not available on mainnet until issue #5580 is fixed: the wallet could record " +
        "a Max transfer as finished from an unverified \"already consumed\" report. " +
        "Unfinished Max transfers can still be resumed from the wallet screen."
