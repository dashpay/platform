package org.dashfoundation.example.ui.dashpay

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
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.AlertDialog
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
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.LifecycleResumeEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.wallet.ShieldedIdentityBalance
import org.dashfoundation.dashsdk.wallet.ShieldedTipRecipient
import org.dashfoundation.dashsdk.wallet.ShieldedTipRecipientChange
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.services.shielded.ShieldedIdentityPaymentCoordinator
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.shielded.CheckNowStatus
import org.dashfoundation.example.ui.shielded.CheckNowStatusLine
import org.dashfoundation.example.ui.shielded.ShieldedPaymentOutcome
import org.dashfoundation.example.ui.wallet.formatCreditsExact
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.DashAddress
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * Send a shielded tip to a DashPay username: the shielded address the
 * name's DashPay profile publishes is paid from the shielded pool. Port of
 * the tip flow of SwiftExampleApp's DashPay tab.
 *
 * The flow is amount and name, then **Review tip**: the wallet parses the
 * amount ([PlatformWalletManager.parseShieldedTipAmount]) and resolves the
 * name ([PlatformWalletManager.reviewShieldedTip]); the screen shows what it
 * resolved, how that compares with the recipient last tipped, and whether
 * an earlier payment to it is unresolved. **Send tip** passes the reviewed
 * recipient and amount back unchanged
 * ([PlatformWalletManager.sendShieldedTip]).
 *
 * The send runs on the application-scoped
 * [ShieldedIdentityPaymentCoordinator], keyed by wallet and sending
 * identity, so a rotation or a tab switch while it proves can neither drop
 * its outcome nor start a second tip. Across a relaunch the wallet itself
 * refuses a repeat: an unresolved payment comes back as
 * [org.dashfoundation.dashsdk.wallet.ShieldedTipReview.unresolved] on review
 * and as `ShieldedTipPending` on send, until a shielded sync resolves it.
 *
 * @param contact set when opened from a saved contact: its DPNS name fixes
 *   the username, and its identity goes to the wallet with the review and
 *   the send, so a name that moved to another identity is refused there.
 *   Null when opened from the DashPay tab.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SendShieldedTipScreen(
    identityIdHex: String,
    contact: ShieldedTipContact?,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    // Copied out of Room as hex: entities compare by id only.
    val walletIdHex by remember(identityIdHex) {
        container.database.identityDao().observeByIdentityId(idBytes).map { it?.walletId?.toHex() }
    }.collectAsStateWithLifecycle(initialValue = null)
    val walletId = remember(walletIdHex) { walletIdHex?.hexToBytes() }

    var username by rememberSaveable { mutableStateOf(contact?.username.orEmpty()) }
    // From a contact the wallet also gets the contact's identity, and refuses
    // a name that now belongs to someone else (ShieldedTipRecipientMismatch).
    val contactIdentityId = remember(contact) { contact?.identityIdHex?.hexToBytes() }
    var amountText by rememberSaveable { mutableStateOf("") }
    var memo by rememberSaveable { mutableStateOf("") }
    var sourceChoice by rememberSaveable { mutableStateOf(ShieldedTipSourceChoice.IDENTITY) }
    var reviewed by remember { mutableStateOf<ReviewedShieldedTip?>(null) }
    var changedReview by remember { mutableStateOf<ReviewedShieldedTip?>(null) }
    var reviewing by remember { mutableStateOf(false) }
    var formError by remember { mutableStateOf<String?>(null) }
    var preparing by remember { mutableStateOf(false) }
    var checkStatus by remember { mutableStateOf<CheckNowStatus>(CheckNowStatus.Idle) }
    var holdEnded by remember { mutableStateOf(false) }

    val paymentCoordinator = container.shieldedIdentityPaymentCoordinator
    val paymentStates by paymentCoordinator.states.collectAsStateWithLifecycle()
    val paymentKey = walletIdHex?.let {
        ShieldedIdentityPaymentCoordinator.Key(
            ShieldedIdentityPaymentCoordinator.Kind.SHIELDED_TIP,
            it,
            identityIdHex.lowercase(),
        )
    }
    val paymentState = paymentKey?.let { paymentStates[it] }
    val sending = paymentState is ShieldedIdentityPaymentCoordinator.State.InFlight
    val finished = paymentState as? ShieldedIdentityPaymentCoordinator.State.Done
    val busy = sending || reviewing || preparing || checkStatus == CheckNowStatus.Checking

    // Balances, as the wallet reports them. A failed or busy read keeps the
    // last state; a resume, a preparation or a finished shielded sync reads again.
    val syncPasses by remember {
        container.shieldedService.state.map { it.syncCountSinceLaunch }.distinctUntilChanged()
    }.collectAsStateWithLifecycle(initialValue = 0)
    var balanceEpoch by remember { mutableIntStateOf(0) }
    LifecycleResumeEffect(Unit) {
        balanceEpoch++
        onPauseOrDispose {}
    }
    val identityBalance by produceState<ShieldedIdentityBalance?>(
        null, manager, walletIdHex, balanceEpoch, syncPasses,
    ) {
        val m = manager ?: return@produceState
        val wid = walletId ?: return@produceState
        try {
            value = m.shieldedIdentityBalance(wid, idBytes)
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            // Keep the last state; the next trigger reads again.
        }
    }
    val mainBalance by remember(walletIdHex) {
        if (walletId != null) {
            container.shieldedService.accountBalance(walletId, SHIELDED_TIP_MAIN_ACCOUNT)
        } else {
            flowOf(null)
        }
    }.collectAsStateWithLifecycle(initialValue = null)

    // A review is for the draft it started from: the fields are locked while
    // it runs, and a result for a draft that changed meanwhile is dropped.
    // [recheck] is the automatic read again of a review that said an earlier
    // payment is unresolved.
    fun review(recheck: Boolean = false) {
        val m = manager ?: return
        val wid = walletId ?: return
        if (reviewing) return
        val draft = ShieldedTipDraft(username, amountText)
        reviewing = true
        formError = null
        if (!recheck) holdEnded = false
        scope.launch {
            try {
                val credits = m.parseShieldedTipAmount(draft.amountText)
                val tip = ReviewedShieldedTip(draft.username, credits, m.reviewShieldedTip(wid, draft.username, contactIdentityId))
                when (val step = shieldedTipReviewStep(draft, ShieldedTipDraft(username, amountText), recheck, tip)) {
                    ShieldedTipReviewStep.Discard -> Unit
                    ShieldedTipReviewStep.HoldEnded -> {
                        reviewed = null
                        holdEnded = true
                    }
                    is ShieldedTipReviewStep.ConfirmChange -> {
                        reviewed = null
                        changedReview = step.tip
                    }
                    is ShieldedTipReviewStep.Show -> reviewed = step.tip
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                if (draft == ShieldedTipDraft(username, amountText)) {
                    reviewed = null
                    formError = ShieldedTipMessages.reviewError(e)
                }
            } finally {
                reviewing = false
            }
        }
    }

    // While the review says an earlier payment is unresolved, each finished
    // shielded sync asks the wallet again. When the hold ends the review is
    // dropped with a notice, not turned into an enabled Send: the earlier
    // payment may have executed, and another tip is a new payment.
    LaunchedEffect(syncPasses) {
        if (reviewed?.review?.unresolved == true) review(recheck = true)
    }

    fun syncNow() {
        val m = manager ?: return
        checkStatus = CheckNowStatus.Checking
        scope.launch {
            checkStatus = try {
                m.syncShieldedNow()
                CheckNowStatus.Finished
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                CheckNowStatus.Failed(e.message ?: "unknown error")
            }
            balanceEpoch++
            if (reviewed?.review?.unresolved == true) review(recheck = true)
        }
    }

    fun prepareIdentityAccount() {
        val m = manager ?: return
        val wid = walletId ?: return
        preparing = true
        formError = null
        scope.launch {
            try {
                m.prepareShieldedIdentityAddress(wid, idBytes)
                balanceEpoch++
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                formError = e.message ?: "Could not prepare this identity's shielded account."
            } finally {
                preparing = false
            }
        }
    }

    fun send() {
        val m = manager ?: return
        val wid = walletId ?: return
        val key = paymentKey ?: return
        val tip = reviewed ?: return
        val source = sourceChoice.source(idBytes)
        val memoText = memo.ifEmpty { null }
        // Whatever the outcome, the next tip is reviewed again.
        reviewed = null
        formError = null
        // The call and its outcome belong to the coordinator, not to this screen.
        val started = paymentCoordinator.start(key, tip.credits) {
            val error = try {
                m.sendShieldedTip(
                    wid, source, tip.username, tip.review.recipient, tip.credits, memoText, contactIdentityId,
                )
                null
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                e
            }
            ShieldedTipMessages.outcome(error)
        }
        if (!started) formError = ShieldedTipMessages.ALREADY_RUNNING
    }

    // Leaving a shown result counts as having seen it; a rotation or tab
    // switch keeps it. A running send cannot be left from here.
    val resultShown = finished != null && finished.outcome !is ShieldedPaymentOutcome.Failed
    val leave: () -> Unit = {
        if (resultShown && paymentKey != null) paymentCoordinator.acknowledge(paymentKey)
        navController.popBackStack()
    }
    BackHandler(enabled = sending || resultShown) {
        if (!sending) leave()
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Send Shielded Tip") },
                navigationIcon = {
                    IconButton(onClick = leave, enabled = !sending) {
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
            if (finished != null && finished.outcome !is ShieldedPaymentOutcome.Failed && paymentKey != null) {
                // Replaces the form so the tip cannot be sent again straight away.
                TipOutcomeSection(
                    outcome = finished.outcome,
                    credits = finished.amount,
                    checkStatus = checkStatus,
                    canCheck = manager != null,
                    onCheckNow = ::syncNow,
                    onDone = {
                        paymentCoordinator.acknowledge(paymentKey)
                        navController.popBackStack()
                    },
                )
                return@Column
            }

            if (sending) {
                FormSection(title = "Sending") {
                    Row(
                        modifier = Modifier.fillMaxWidth().testTag("dashpay.tip.sending"),
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
                        Text(ShieldedTipMessages.SENDING, style = MaterialTheme.typography.bodySmall)
                    }
                    paymentState?.let { LabeledContent("Amount", formatCreditsExact(it.amount)) }
                }
            }

            FormSection(title = "Tip") {
                OutlinedTextField(
                    value = username,
                    onValueChange = { username = it; reviewed = null; changedReview = null; holdEnded = false },
                    label = { Text("Username") },
                    singleLine = true,
                    readOnly = contact != null,
                    enabled = !sending && !reviewing,
                    modifier = Modifier.fillMaxWidth().testTag("dashpay.tip.username"),
                )
                OutlinedTextField(
                    value = amountText,
                    onValueChange = { amountText = it; reviewed = null; changedReview = null; holdEnded = false },
                    label = { Text("Amount (DASH)") },
                    singleLine = true,
                    enabled = !sending && !reviewing,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                    modifier = Modifier.fillMaxWidth().testTag("dashpay.tip.amount"),
                )
                OutlinedTextField(
                    value = memo,
                    onValueChange = { memo = it },
                    label = { Text("Memo (optional)") },
                    singleLine = true,
                    enabled = !sending,
                    supportingText = { Text("Up to 32 bytes, on the recipient's note.") },
                    modifier = Modifier.fillMaxWidth().testTag("dashpay.tip.memo"),
                )
            }

            FormSection(title = "Pay from") {
                SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                    ShieldedTipSourceChoice.entries.forEachIndexed { index, choice ->
                        SegmentedButton(
                            selected = sourceChoice == choice,
                            onClick = { sourceChoice = choice },
                            enabled = !sending,
                            shape = SegmentedButtonDefaults.itemShape(index, ShieldedTipSourceChoice.entries.size),
                            modifier = Modifier.testTag(choice.tag),
                        ) { Text(choice.label) }
                    }
                }
                when (sourceChoice) {
                    ShieldedTipSourceChoice.IDENTITY -> {
                        LabeledContent(
                            "Identity's shielded balance",
                            ShieldedTipMessages.identityBalanceText(identityBalance),
                            modifier = Modifier.testTag("dashpay.tip.identityBalance"),
                        )
                        if (identityBalance == ShieldedIdentityBalance.Unverified) {
                            TextButton(
                                onClick = ::prepareIdentityAccount,
                                enabled = !busy && manager != null && walletId != null,
                                modifier = Modifier.testTag("dashpay.tip.prepareIdentityAccount"),
                            ) { Text("Prepare this identity's shielded account") }
                        }
                    }
                    ShieldedTipSourceChoice.MAIN_ACCOUNT ->
                        LabeledContent(
                            "Main shielded balance",
                            ShieldedTipMessages.mainBalanceText(mainBalance),
                            modifier = Modifier.testTag("dashpay.tip.mainBalance"),
                        )
                }
                Text(
                    "The wallet checks the paying account when the tip is sent. The fee is " +
                        "paid from the same notes on top of the amount.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            reviewed?.let { tip ->
                TipReviewSection(
                    tip = tip,
                    sourceLabel = sourceChoice.label,
                    manager = manager,
                    checkStatus = checkStatus,
                    onSyncNow = ::syncNow,
                    syncEnabled = !busy,
                )
            }

            if (holdEnded) {
                Text(
                    ShieldedTipMessages.HOLD_ENDED,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.tertiary,
                    modifier = Modifier.testTag("dashpay.tip.holdEnded"),
                )
            }

            formError?.let {
                Text(
                    it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.testTag("dashpay.tip.error"),
                )
            }

            val tip = reviewed
            if (tip == null) {
                SubmitButton(
                    text = "Review tip",
                    isLoading = reviewing,
                    enabled = !busy && manager != null && walletId != null &&
                        username.isNotBlank() && amountText.isNotBlank(),
                    modifier = Modifier.fillMaxWidth().testTag("dashpay.tip.review"),
                    onClick = ::review,
                )
            } else {
                SubmitButton(
                    text = "Send tip",
                    isLoading = sending,
                    enabled = !busy && !tip.review.unresolved,
                    modifier = Modifier.fillMaxWidth().testTag("dashpay.tip.send"),
                    onClick = ::send,
                )
            }
        }
    }

    changedReview?.let { changed ->
        val previous = (changed.review.change as? ShieldedTipRecipientChange.Changed)?.previous
        AlertDialog(
            onDismissRequest = { changedReview = null },
            title = { Text(ShieldedTipMessages.RECIPIENT_CHANGED_TITLE) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(ShieldedTipMessages.RECIPIENT_CHANGED)
                    previous?.let { RecipientLines("Previously", it, manager) }
                    RecipientLines("Now", changed.review.recipient, manager)
                }
            },
            confirmButton = {
                TextButton(
                    onClick = { reviewed = changed; changedReview = null },
                    modifier = Modifier.testTag("dashpay.tip.changed.review"),
                ) { Text("Review new recipient") }
            },
            dismissButton = {
                TextButton(
                    onClick = { changedReview = null },
                    modifier = Modifier.testTag("dashpay.tip.changed.cancel"),
                ) { Text("Cancel") }
            },
        )
    }

    // A refused or failed tip, held by the coordinator until shown.
    val failed = finished?.outcome as? ShieldedPaymentOutcome.Failed
    if (failed != null && paymentKey != null) {
        ErrorAlertDialog(
            message = failed.message,
            title = "Tip not sent",
            onDismiss = { paymentCoordinator.acknowledge(paymentKey) },
        )
    }
}

/** What the wallet resolved, for the user to confirm. */
@Composable
private fun TipReviewSection(
    tip: ReviewedShieldedTip,
    sourceLabel: String,
    manager: PlatformWalletManager?,
    checkStatus: CheckNowStatus,
    onSyncNow: () -> Unit,
    syncEnabled: Boolean,
) {
    FormSection(title = "Review") {
        Column(
            modifier = Modifier.testTag("dashpay.tip.reviewed"),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            LabeledContent("To", tip.username)
            RecipientLines("Recipient", tip.review.recipient, manager)
            LabeledContent("Amount", formatCreditsExact(tip.credits))
            LabeledContent("From", sourceLabel)
            Text(
                ShieldedTipMessages.changeText(tip.review.change),
                style = MaterialTheme.typography.bodySmall,
                color = if (tip.review.change is ShieldedTipRecipientChange.Changed) {
                    MaterialTheme.colorScheme.tertiary
                } else {
                    MaterialTheme.colorScheme.onSurfaceVariant
                },
                modifier = Modifier.testTag("dashpay.tip.change"),
            )
            if (tip.review.unresolved) {
                Text(
                    ShieldedTipMessages.UNRESOLVED,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.tertiary,
                    modifier = Modifier.testTag("dashpay.tip.unresolved"),
                )
                OutlinedButton(
                    onClick = onSyncNow,
                    enabled = syncEnabled && manager != null,
                    modifier = Modifier.fillMaxWidth().testTag("dashpay.tip.syncNow"),
                ) { Text("Sync shielded now") }
                CheckNowStatusLine(checkStatus)
            }
        }
    }
}

/** A recipient's identity id and shielded address, selectable for comparison. */
@Composable
private fun RecipientLines(label: String, recipient: ShieldedTipRecipient, manager: PlatformWalletManager?) {
    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Text(label, style = MaterialTheme.typography.labelMedium)
        SelectionContainer {
            Column {
                Text(Base58.encode(recipient.identityId), style = MaterialTheme.typography.bodySmall)
                Text(
                    manager?.let { DashAddress.encodeOrchard(recipient.address, it.network) }
                        ?: recipient.address.toHex(),
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }
    }
}

/** A tip that returned or may have gone through. Neither offers a resend. */
@Composable
private fun TipOutcomeSection(
    outcome: ShieldedPaymentOutcome,
    credits: Long,
    checkStatus: CheckNowStatus,
    canCheck: Boolean,
    onCheckNow: () -> Unit,
    onDone: () -> Unit,
) {
    FormSection(title = "Result") {
        Column(
            modifier = Modifier.testTag("dashpay.tip.outcome"),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            when (outcome) {
                is ShieldedPaymentOutcome.Failed -> Unit
                ShieldedPaymentOutcome.Submitted -> {
                    Text(
                        "Tip submitted",
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.primary,
                    )
                    Text(ShieldedTipMessages.submittedText(credits))
                }
                ShieldedPaymentOutcome.MayHaveGoneThrough -> {
                    Text(
                        "May have gone through",
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.tertiary,
                    )
                    Text(ShieldedTipMessages.mayHaveGoneThroughText(credits))
                    OutlinedButton(
                        onClick = onCheckNow,
                        enabled = canCheck && checkStatus != CheckNowStatus.Checking,
                        modifier = Modifier.fillMaxWidth().testTag("dashpay.tip.outcome.syncNow"),
                    ) { Text("Sync shielded now") }
                    CheckNowStatusLine(checkStatus)
                }
            }
            OutlinedButton(
                onClick = onDone,
                modifier = Modifier.fillMaxWidth().testTag("dashpay.tip.outcome.done"),
            ) { Text("Done") }
        }
    }
}
