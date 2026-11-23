package org.dashfoundation.example.ui.credits

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.LifecycleResumeEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.credits.FundingInput
import org.dashfoundation.dashsdk.funding.ShieldedProver
import org.dashfoundation.dashsdk.persistence.entities.ShieldedActivityEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.shielded.CheckNowStatus
import org.dashfoundation.example.ui.shielded.CheckNowStatusLine
import org.dashfoundation.example.ui.shielded.ShieldedTopUpMessages
import org.dashfoundation.example.ui.shielded.ShieldedPaymentOutcome
import org.dashfoundation.example.ui.wallet.formatCreditsExact
import org.dashfoundation.example.services.shielded.ShieldedIdentityPaymentCoordinator
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.parseDashToCredits

/** Where a top-up is paid from. ← Swift `TopUpIdentityView.FundingSelection`. */
private enum class TopUpFunding(val label: String, val tag: String) {
    PLATFORM_ADDRESSES("Platform Addresses", "topup.fundingSource.platform"),
    SHIELDED("Shielded Balance", "topup.fundingSource.shielded"),
}

/**
 * The identity as this screen shows it, copied out of Room: entities compare
 * by id only, so a balance update would not recompose. A missing row has no
 * balance rather than a zero one.
 */
private data class TopUpIdentityInfo(val label: String?, val balance: Long, val walletIdHex: String?)

/** The shielded account a top-up spends from, as in Swift. */
private const val SHIELDED_TOP_UP_ACCOUNT = 0

/**
 * Top up an identity's credit balance: port of `TopUpIdentityView.swift`
 * (packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Views/). Two sources:
 *
 * - **Platform Addresses**: enumerates the wallet's balance-carrying
 *   Platform-payment addresses
 *   ([org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.addressesWithBalances]),
 *   greedily packs them up to the requested amount (largest-balance-first,
 *   matching Swift's `buildInputs`), then credits the identity through
 *   [org.dashfoundation.dashsdk.credits.IdentityCredits.topUpFromAddresses].
 * - **Shielded Balance** (Type 22, SH-22): spends notes of the identity's
 *   wallet's shielded account 0 through
 *   [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.shieldedIdentityTopUpFromPool].
 *   Offered when the account's spendable balance Rust announces is known and
 *   above zero (notes reserved for a pending spend do not count). The fee is
 *   spent from the notes on top of the amount; whether the notes cover both
 *   is Rust's refusal, made before anything is built. A call that may have
 *   gone through says so and is never offered again as a retry, and the
 *   identity's pending top-ups (kind 9 rows Rust records before broadcasting)
 *   are listed under Pending Shielded Top-Ups, also after a restart (SH-23).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TopUpIdentityScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()

    val identity by remember(identityIdHex) {
        container.database.identityDao().observeByIdentityId(idBytes).map { row ->
            row?.let { TopUpIdentityInfo(it.mainDpnsName, it.balance, it.walletId?.toHex()) }
        }
    }.collectAsStateWithLifecycle(initialValue = null)
    val walletIdKey = identity?.walletIdHex
    val identityWalletId = remember(walletIdKey) { walletIdKey?.hexToBytes() }
    val wallet = rememberManagedWalletFor(identityWalletId)

    var funding by rememberSaveable { mutableStateOf(TopUpFunding.PLATFORM_ADDRESSES) }
    var amountText by rememberSaveable { mutableStateOf("") }
    var shieldedAmountText by rememberSaveable { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var isSubmitting by remember { mutableStateOf(false) }
    var done by remember { mutableStateOf(false) }
    var checkStatus by remember { mutableStateOf<CheckNowStatus>(CheckNowStatus.Idle) }
    var isChecking by remember { mutableStateOf(false) }

    // Account 0's spendable shielded credits, as Rust announces them. Null
    // until known: an unknown balance never offers the shielded source.
    val hasShielded = remember { Sdk.hasShielded() }
    val shieldedSpendable by remember(walletIdKey, hasShielded) {
        if (identityWalletId != null && hasShielded) {
            container.shieldedService.accountBalance(identityWalletId, SHIELDED_TOP_UP_ACCOUNT)
        } else {
            flowOf(null)
        }
    }.collectAsStateWithLifecycle(initialValue = null)
    val offersShielded = (shieldedSpendable ?: 0L) > 0L

    // This identity's top-ups from the pool still pending in its wallet.
    val pendingShieldedTopUps by remember(walletIdKey, identityIdHex) {
        if (identityWalletId != null) {
            container.database.shieldedDao().observePendingIdentityTopUps(identityWalletId, idBytes)
        } else {
            emptyFlow()
        }
    }.collectAsStateWithLifecycle(initialValue = emptyList())

    // Minimum fee of a shielded top-up (two actions; the wallet prices the
    // notes it selects), re-read on resume and when the network's version is
    // published, as SendTransactionScreen does.
    val protocolVersion by appState.platformProtocolVersion.collectAsStateWithLifecycle()
    var feeReadEpoch by remember { mutableIntStateOf(0) }
    LifecycleResumeEffect(Unit) {
        feeReadEpoch++
        onPauseOrDispose {}
    }
    val shieldedTopUpFee by produceState<Long?>(null, manager, protocolVersion, feeReadEpoch) {
        value = manager?.let { m ->
            runCatching { m.estimateShieldedFee(ShieldedProver.FeeKind.IdentityTopUpFromPool, 2) }
                .getOrNull()
        }
    }

    // A shielded top-up runs on the application scope: a rotation or a tab
    // switch while it proves neither drops its outcome nor allows a second
    // one for this identity.
    val paymentCoordinator = container.shieldedIdentityPaymentCoordinator
    val paymentStates by paymentCoordinator.states.collectAsStateWithLifecycle()
    val paymentKey = walletIdKey?.let {
        ShieldedIdentityPaymentCoordinator.Key(
            ShieldedIdentityPaymentCoordinator.Kind.TOP_UP_FROM_POOL,
            it,
            identityIdHex.lowercase(),
        )
    }
    val paymentState = paymentKey?.let { paymentStates[it] }
    val topUpRunning = paymentState is ShieldedIdentityPaymentCoordinator.State.InFlight
    // A running top-up keeps its source selected whatever the balance says.
    LaunchedEffect(topUpRunning) {
        if (topUpRunning) funding = TopUpFunding.SHIELDED
    }
    val isBusy = isSubmitting || topUpRunning

    // A shielded selection the wallet cannot fund right now stays selected
    // with submission off; the user switches source explicitly, so an amount
    // meant for the pool never pays from Platform addresses.
    val shieldedUnavailableWhileSelected = funding == TopUpFunding.SHIELDED && !offersShielded

    val amount = amountText.toLongOrNull()
    val shieldedAmount = parseDashToCredits(shieldedAmountText)
    val canSubmit = when (funding) {
        TopUpFunding.PLATFORM_ADDRESSES ->
            wallet != null && manager != null && amount != null && amount > 0
        TopUpFunding.SHIELDED ->
            identityWalletId != null && manager != null && offersShielded &&
                shieldedAmount != null && shieldedTopUpFee != null
    }

    // The proof takes tens of seconds; leaving would drop its outcome and
    // allow a second top-up (← interactiveDismissDisabled).
    BackHandler(enabled = isBusy) {}

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Top Up Identity") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }, enabled = !isBusy) {
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
            FormSection(title = "Identity") {
                LabeledContent("Identity", identity?.label ?: identityIdHex.take(16) + "…")
                LabeledContent(
                    "Current balance",
                    identity?.let { formatCreditsExact(it.balance) } ?: "Unknown",
                )
            }

            val pendingSection: @Composable () -> Unit = {
                PendingShieldedTopUpsSection(
                    rows = pendingShieldedTopUps,
                    isChecking = isChecking,
                    checkStatus = checkStatus,
                    // A build without shielded support has no sync export.
                    canCheck = hasShielded && manager != null,
                    onCheckNow = {
                        val m = manager ?: return@PendingShieldedTopUpsSection
                        isChecking = true
                        checkStatus = CheckNowStatus.Checking
                        scope.launch {
                            checkStatus = try {
                                m.syncShieldedNow()
                                CheckNowStatus.Finished
                            } catch (e: CancellationException) {
                                throw e
                            } catch (e: Exception) {
                                CheckNowStatus.Failed(e.message ?: "unknown error")
                            } finally {
                                isChecking = false
                            }
                        }
                    },
                )
            }

            val finished = paymentState as? ShieldedIdentityPaymentCoordinator.State.Done
            if (finished != null && finished.outcome !is ShieldedPaymentOutcome.Failed && paymentKey != null) {
                // Replaces the form so the user cannot resubmit straight away.
                ShieldedOutcomeSection(
                    outcome = finished.outcome,
                    amount = finished.amount,
                    pendingListedBelow = pendingShieldedTopUps.isNotEmpty(),
                    identityBalance = identity?.balance,
                    onDone = {
                        paymentCoordinator.acknowledge(paymentKey)
                        navController.popBackStack()
                    },
                )
                if (finished.outcome == ShieldedPaymentOutcome.MayHaveGoneThrough) pendingSection()
                return@Column
            }

            pendingSection()

            FormSection(title = "Funding") {
                if (offersShielded || funding == TopUpFunding.SHIELDED) {
                    SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                        TopUpFunding.entries.forEachIndexed { index, source ->
                            SegmentedButton(
                                selected = funding == source,
                                onClick = { funding = source },
                                enabled = !isBusy,
                                shape = SegmentedButtonDefaults.itemShape(index, TopUpFunding.entries.size),
                                modifier = Modifier.testTag(source.tag),
                            ) { Text(source.label) }
                        }
                    }
                }
                if (shieldedUnavailableWhileSelected && !topUpRunning) {
                    Text(
                        "Shielded Balance is not available right now, so Top Up is off " +
                            "until it is, or pick another source.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
                if (hasShielded && shieldedSpendable == null) {
                    Text(
                        "Shielded Balance unavailable. The shielded balance is not known yet.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.testTag("topup.fundingSource.shieldedUnavailable"),
                    )
                }
                Text(
                    when (funding) {
                        TopUpFunding.PLATFORM_ADDRESSES ->
                            "Funded from the wallet's Platform-payment addresses."
                        TopUpFunding.SHIELDED ->
                            "Shielded Balance spends notes from this wallet's shielded " +
                                "pool instead; no identity key signs."
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                when (funding) {
                    TopUpFunding.PLATFORM_ADDRESSES -> OutlinedTextField(
                        value = amountText,
                        onValueChange = { amountText = it.filter(Char::isDigit) },
                        label = { Text("Amount (credits)") },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        modifier = Modifier.fillMaxWidth().testTag("topUpIdentity.amount"),
                    )
                    TopUpFunding.SHIELDED -> {
                        // Free-form, not prefilled with the whole balance: the
                        // fee is spent from the notes on top of it.
                        OutlinedTextField(
                            value = shieldedAmountText,
                            onValueChange = { shieldedAmountText = it },
                            label = { Text("Amount (DASH)") },
                            singleLine = true,
                            enabled = !isBusy,
                            isError = shieldedAmountText.isNotBlank() && shieldedAmount == null,
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                            modifier = Modifier.fillMaxWidth().testTag("topup.shielded.amount"),
                        )
                        if (shieldedAmountText.isNotBlank() && shieldedAmount == null) {
                            Text(
                                "Enter a positive amount with at most 11 decimal places: " +
                                    "the smallest amount is 0.00000000001 DASH.",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.error,
                                modifier = Modifier.testTag("topup.shielded.amountProblem"),
                            )
                        }
                        LabeledContent(
                            "Spendable shielded",
                            shieldedSpendable?.let { formatCreditsExact(it) } ?: "Unknown",
                        )
                        LabeledContent(
                            "Fee",
                            shieldedTopUpFee?.let { "at least " + formatCreditsExact(it) } ?: "Not available",
                            modifier = Modifier.testTag("topup.shielded.fee"),
                        )
                        Text(
                            ShieldedTopUpMessages.FEE_NOTE,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }

            SubmitButton(
                text = when {
                    done -> "Topped Up"
                    topUpRunning ->
                        "Building proof, this takes tens of seconds…"
                    else -> "Top Up Balance"
                },
                isLoading = isBusy,
                enabled = canSubmit && !done && !isBusy,
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag(
                        if (funding == TopUpFunding.SHIELDED) "topup.submit" else "topUpIdentity.submit",
                    ),
            ) {
                val mgr = manager ?: return@SubmitButton
                when (funding) {
                    TopUpFunding.PLATFORM_ADDRESSES -> {
                        val w = wallet ?: return@SubmitButton
                        val amt = amount ?: return@SubmitButton
                        isSubmitting = true
                        scope.launch {
                            try {
                                // Enumerate balance-carrying addresses, then greedily
                                // pack up to the requested amount (largest first),
                                // matching TopUpIdentityView.buildInputs.
                                val candidates = w.addressesWithBalances()
                                    .sortedByDescending { it.credits }
                                val inputs = packInputs(candidates, amt)
                                if (inputs.isEmpty()) {
                                    error = "Not enough Platform-address balance to fund " +
                                        "$amt credits."
                                    return@launch
                                }
                                mgr.identityCredits.topUpFromAddresses(
                                    walletHandle = w.handle,
                                    identityId = idBytes,
                                    inputs = inputs,
                                    signerHandle = mgr.signerHandle,
                                )
                                done = true
                            } catch (e: Exception) {
                                error = e.message ?: "Top up failed"
                            } finally {
                                isSubmitting = false
                            }
                        }
                    }
                    TopUpFunding.SHIELDED -> {
                        // Captured at the tap, so later edits cannot change
                        // what is sent (← ShieldedTopUpRequest.capture).
                        val walletId = identityWalletId ?: return@SubmitButton
                        val key = paymentKey ?: return@SubmitButton
                        val credits = shieldedAmount ?: return@SubmitButton
                        // Set before the async call so a second tap cannot
                        // start another top-up.
                        isSubmitting = true
                        scope.launch {
                            try {
                                // The balance read now, not the one this tap
                                // captured: an unknown balance never pays.
                                val spendableNow = container.shieldedService
                                    .accountBalance(walletId, SHIELDED_TOP_UP_ACCOUNT)
                                    .first()
                                if (spendableNow == null || spendableNow <= 0L) {
                                    error = "The shielded balance is not known right now, " +
                                        "so nothing was sent."
                                    return@launch
                                }
                                // The call and its outcome belong to the
                                // coordinator, not to this screen.
                                val started = paymentCoordinator.start(key, credits) {
                                    try {
                                        mgr.shieldedIdentityTopUpFromPool(
                                            walletId = walletId,
                                            identityId = idBytes,
                                            amount = credits,
                                            account = SHIELDED_TOP_UP_ACCOUNT,
                                        )
                                        ShieldedTopUpMessages.outcome(null)
                                    } catch (e: Exception) {
                                        ShieldedTopUpMessages.outcome(e)
                                    }
                                }
                                if (!started) {
                                    error = "A top-up of this identity from the shielded pool is " +
                                        "still running, or its outcome was not shown yet. " +
                                        "Nothing new was sent."
                                }
                            } finally {
                                isSubmitting = false
                            }
                        }
                    }
                }
            }
        }
    }

    ErrorAlertDialog(
        message = error,
        title = "Could not top up identity",
        onDismiss = { error = null },
    )
    // A refused or failed shielded top-up, held by the coordinator until shown.
    val failed = (paymentState as? ShieldedIdentityPaymentCoordinator.State.Done)
        ?.outcome as? ShieldedPaymentOutcome.Failed
    if (failed != null && paymentKey != null) {
        ErrorAlertDialog(
            message = failed.message,
            title = "Could not top up identity",
            onDismiss = { paymentCoordinator.acknowledge(paymentKey) },
        )
    }
}

/**
 * Result of a shielded top-up. The wallet's proof covers the spent notes and
 * the identity's balance at a block, not that this transition added the
 * amount, so the identity balance is shown on its own line.
 * ← Swift `TopUpIdentityView.shieldedOutcomeSection`.
 */
@Composable
private fun ShieldedOutcomeSection(
    outcome: ShieldedPaymentOutcome,
    amount: Long,
    pendingListedBelow: Boolean,
    identityBalance: Long?,
    onDone: () -> Unit,
) {
    FormSection(title = "Result") {
        Column(
            modifier = Modifier.testTag("topup.shielded.outcome"),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            when (outcome) {
                is ShieldedPaymentOutcome.Failed -> Unit
                ShieldedPaymentOutcome.Submitted -> {
                    Text(
                        "Top-up submitted",
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.primary,
                    )
                    Text(ShieldedTopUpMessages.completedText(amount))
                }
                ShieldedPaymentOutcome.MayHaveGoneThrough -> {
                    Text(
                        "May have gone through",
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.tertiary,
                    )
                    Text(ShieldedTopUpMessages.unconfirmedText(amount, pendingListedBelow))
                }
            }
            Row {
                Text(
                    "Last saved identity balance: ",
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    identityBalance?.let { formatCreditsExact(it) } ?: "Unknown",
                    fontWeight = FontWeight.Medium,
                )
            }
            OutlinedButton(onClick = onDone, modifier = Modifier.fillMaxWidth()) { Text("Done") }
        }
    }
}

/**
 * Pending top-ups of this identity from the shielded pool. Rust keeps their
 * notes reserved and checks them on each shielded sync.
 * ← Swift `TopUpIdentityView.pendingShieldedTopUpsSection`.
 */
@Composable
private fun PendingShieldedTopUpsSection(
    rows: List<ShieldedActivityEntity>,
    isChecking: Boolean,
    checkStatus: CheckNowStatus,
    canCheck: Boolean,
    onCheckNow: () -> Unit,
) {
    if (rows.isEmpty()) return
    FormSection(title = ShieldedTopUpMessages.PENDING_HEADER) {
        rows.forEach { row ->
            LabeledContent(
                "Top-up from shielded pool",
                formatCreditsExact(row.amount),
                modifier = Modifier.testTag("topup.shielded.pendingRow"),
            )
        }
        OutlinedButton(
            onClick = onCheckNow,
            enabled = canCheck && !isChecking,
            modifier = Modifier.fillMaxWidth().testTag("topup.shielded.checkNow"),
        ) {
            if (isChecking) {
                CircularProgressIndicator(
                    modifier = Modifier.size(16.dp).padding(end = 4.dp),
                    strokeWidth = 2.dp,
                )
            }
            Text("Check Now")
        }
        CheckNowStatusLine(checkStatus)
        Text(
            ShieldedTopUpMessages.PENDING_FOOTER,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/**
 * Greedily select funding inputs from [candidates] (assumed sorted
 * largest-balance-first) until their combined credits cover [target],
 * spending only what's needed from the final address. Returns an empty
 * list when the candidates can't cover [target]. Mirrors Swift's
 * `TopUpIdentityView.buildInputs`.
 */
private fun packInputs(candidates: List<FundingInput>, target: Long): List<FundingInput> {
    if (target <= 0) return emptyList()
    var remaining = target
    val picked = ArrayList<FundingInput>()
    for (input in candidates) {
        if (remaining <= 0) break
        val spend = minOf(input.credits, remaining)
        picked.add(input.copy(credits = spend))
        remaining -= spend
    }
    return if (remaining <= 0) picked else emptyList()
}
