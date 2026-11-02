package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.DialogProperties
import androidx.compose.ui.window.SecureFlagPolicy
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import java.util.Date
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.voting.DecisionProposal
import org.dashfoundation.dashsdk.voting.DecisionProposalEdit
import org.dashfoundation.dashsdk.voting.YesNoAbstainVoteChoice
import org.dashfoundation.dashsdk.voting.YesNoMinimumVotingPower
import org.dashfoundation.dashsdk.voting.YesNoVotePollState
import org.dashfoundation.dashsdk.voting.YesNoVotePollStatus
import org.dashfoundation.example.LocalSecureScreen
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.formatDate
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.truncateMiddle

/**
 * One decision proposal: port of `DecisionProposalDetailView.swift`.
 *
 * Reads the proposal, its poll's state and the poll's id through `Sdk.decisionProposals`
 * (each independently, so one failing read leaves the others shown) and shows the rule,
 * the tallies while voting is open and the result once it finished. Offers the masternode
 * vote ([DecisionProposalVoteDialog], the same credentials [ContestDetailScreen] collects)
 * and, when a local wallet identity filed the proposal, the text edit
 * ([EditDecisionProposalDialog]).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DecisionProposalDetailScreen(
    proposalId: String,
    navController: NavHostController,
) {
    val appState = LocalAppState.current
    val container = LocalAppContainer.current
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val walletIdentities by container.database.identityDao()
        .observeWalletOwnedByNetwork(network.ffiValue)
        .collectAsStateWithLifecycle(initialValue = emptyList())

    var proposal by remember { mutableStateOf<DecisionProposal?>(null) }
    var pollState by remember { mutableStateOf<YesNoVotePollState?>(null) }
    var pollId by remember { mutableStateOf<String?>(null) }
    var proposalError by remember { mutableStateOf<String?>(null) }
    var pollError by remember { mutableStateOf<String?>(null) }
    var isLoading by remember { mutableStateOf(false) }
    var refreshTick by remember { mutableStateOf(0) }

    var showVote by remember { mutableStateOf(false) }
    var showEdit by remember { mutableStateOf(false) }
    var actionMessage by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(sdk, proposalId, refreshTick) {
        val currentSdk = sdk ?: run {
            proposalError = "SDK not initialized"
            return@LaunchedEffect
        }
        isLoading = true
        proposalError = null
        pollError = null
        try {
            proposal = readOrNull({ proposalError = "Loading the proposal failed: ${it.message}" }) {
                currentSdk.decisionProposals.fetch(proposalId)
            }
            if (proposal == null && proposalError == null) {
                proposalError = "Platform has no proposal $proposalId."
            }
            pollState = readOrNull({ pollError = "Loading the poll failed: ${it.message}" }) {
                currentSdk.decisionProposals.votePollState(proposalId)
            }
            pollId = readOrNull({}) { currentSdk.decisionProposals.votePollId(proposalId) }
        } finally {
            isLoading = false
        }
    }

    // The local wallet identity that filed the proposal, which may edit it.
    val owner = remember(proposal, walletIdentities) {
        proposal?.let { shown ->
            walletIdentities.firstOrNull { Base58.encode(it.identityId) == shown.ownerId }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Proposal") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = { refreshTick++ },
                        enabled = !isLoading,
                        modifier = Modifier.testTag("decisionProposal.refresh"),
                    ) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
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
            proposalError?.let { message ->
                Text(
                    message,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.testTag("decisionProposal.error"),
                )
            }

            val shown = proposal
            if (shown != null) {
                ProposalSection(shown, isOwnerLocal = owner != null)
                RuleSection(shown)
            }

            PollSection(
                state = pollState,
                pollId = pollId,
                isLoading = isLoading,
                error = pollError,
            )

            FormSection(title = "Actions") {
                actionMessage?.let { message ->
                    Text(
                        message,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.testTag("decisionProposal.actionResult"),
                    )
                }
                SubmitButton(
                    text = "Vote as a Masternode…",
                    isLoading = false,
                    // Platform takes votes only while the poll is open, as on iOS
                    enabled = sdk != null && shown != null && pollState?.result == null,
                    modifier = Modifier.fillMaxWidth().testTag("decisionProposal.vote"),
                ) {
                    actionMessage = null
                    showVote = true
                }
                Text(
                    "Only masternodes vote, with the voting key of a pro_tx_hash. A " +
                        "regular wallet has no voting key, so Platform refuses its vote.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                if (owner != null) {
                    SubmitButton(
                        text = "Edit Text…",
                        isLoading = false,
                        enabled = sdk != null && shown != null,
                        modifier = Modifier.fillMaxWidth().testTag("decisionProposal.edit"),
                    ) {
                        actionMessage = null
                        showEdit = true
                    }
                    Text(
                        "Platform accepts an edit of the title, description and url for a " +
                            "day after the proposal is created (or a fifth of its voting " +
                            "time, if that is shorter).",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }

    val shown = proposal
    if (showVote && shown != null) {
        DecisionProposalVoteDialog(
            proposal = shown,
            onDismiss = { showVote = false },
            onVoted = { message ->
                showVote = false
                actionMessage = message
                refreshTick++ // pull fresh tallies after an accepted vote
            },
        )
    }
    val editor = owner
    if (showEdit && shown != null && editor != null) {
        EditDecisionProposalDialog(
            proposal = shown,
            owner = editor,
            onDismiss = { showEdit = false },
            onEdited = { message ->
                showEdit = false
                actionMessage = message
                refreshTick++
            },
        )
    }
}

/**
 * [read]'s value, or null after handing its failure to [onError], so one failed read
 * leaves the others shown. Cancellation is rethrown, never reported as a failure.
 */
private suspend fun <T> readOrNull(onError: (Exception) -> Unit, read: suspend () -> T?): T? =
    try {
        read()
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        onError(e)
        null
    }

@Composable
private fun ProposalSection(proposal: DecisionProposal, isOwnerLocal: Boolean) {
    FormSection(title = "Proposal") {
        Text(
            proposal.title,
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.testTag("decisionProposal.title"),
        )
        Text(
            proposal.description,
            style = MaterialTheme.typography.bodyMedium,
            modifier = Modifier.testTag("decisionProposal.description"),
        )
        proposal.url?.let { LabeledContent("URL", it) }
        LabeledContent("ID", truncateMiddle(proposal.id, 12, 8))
        LabeledContent(
            "Proposer",
            truncateMiddle(proposal.ownerId, 10, 6) + if (isOwnerLocal) " (local)" else "",
        )
        proposal.revision?.let { LabeledContent("Revision", "$it") }
        proposal.createdAtMs?.let { LabeledContent("Created", formatDate(Date(it))) }
        proposal.updatedAtMs?.let { LabeledContent("Updated", formatDate(Date(it))) }
    }
}

@Composable
private fun RuleSection(proposal: DecisionProposal) {
    FormSection(title = "Rule") {
        LabeledContent("Passes when yes is more than", "${proposal.passPercent}% of yes plus no")
        LabeledContent("Minimum turnout", "${proposal.minimumTurnoutPercent}% of voting power")
        LabeledContent("Voting ends", formatDate(Date(proposal.endDateMs)))
        Text(
            "The rule and the end date never change after the proposal is created.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun PollSection(
    state: YesNoVotePollState?,
    pollId: String?,
    isLoading: Boolean,
    error: String?,
) {
    FormSection(title = "Poll") {
        when {
            error != null -> Text(
                error,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
            )
            state == null && isLoading -> LabeledContent("Status", "Loading…")
            state == null -> Text(
                "Platform has no poll for this proposal.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            else -> PollStateRows(state)
        }
        pollId?.let { LabeledContent("Poll ID", truncateMiddle(it, 12, 8)) }
        Text(
            "Voting power counts a masternode 1 and an evonode 4. Abstaining is recorded " +
                "but plays no part in the result.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun PollStateRows(state: YesNoVotePollState) {
    when (val status = state.status) {
        is YesNoVotePollStatus.Started -> {
            LabeledContent("Status", "Voting open", modifier = Modifier.testTag("decisionProposal.pollStatus"))
            LabeledContent("Opened at block", "${status.startBlock.height}")
            LabeledContent("Yes", "${state.yesVotingPower}")
            LabeledContent("No", "${state.noVotingPower}")
            LabeledContent("Abstain", "${state.abstainVotingPower}")
        }
        is YesNoVotePollStatus.Finished -> {
            val result = status.result
            LabeledContent(
                "Status",
                if (result.passed) "Passed" else "Did not pass",
                modifier = Modifier.testTag("decisionProposal.pollStatus"),
            )
            LabeledContent("Yes", "${result.yesVotingPower}")
            LabeledContent("No", "${result.noVotingPower}")
            LabeledContent("Abstain", "${result.abstainVotingPower}")
            LabeledContent("Required yes plus no", "${result.requiredVotingPower}")
            LabeledContent("Closed at block", "${result.finalizationBlock.height}")
        }
    }
    val poll = state.votePoll
    LabeledContent(
        "Supermajority",
        "more than ${poll.supermajorityNumerator}/${poll.supermajorityDenominator} of yes plus no",
    )
    LabeledContent(
        "Minimum voting power",
        when (val minimum = poll.minimumVotingPower) {
            is YesNoMinimumVotingPower.Absolute -> "${minimum.votingPower} voting power"
            is YesNoMinimumVotingPower.FractionOfTotal ->
                "${minimum.numerator}/${minimum.denominator} of total, rounded " +
                    minimum.rounding.jsonName
        },
    )
}

/**
 * Picks an answer and takes masternode credentials: port of `DecisionProposalVoteSheet`
 * in `DecisionProposalDetailView.swift`. Votes through
 * `Sdk.decisionProposals.castVote`, and reads the masternode's current answer through
 * `Sdk.decisionProposals.masternodeVote`.
 *
 * The voting private key is masked as it is typed, and screen capture is blocked while the
 * dialog is open (see [HoldSecureWindow]). The dialog cannot be dismissed while a call runs.
 */
@Composable
private fun DecisionProposalVoteDialog(
    proposal: DecisionProposal,
    onDismiss: () -> Unit,
    onVoted: (String) -> Unit,
) {
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    HoldSecureWindow()

    var choice by remember { mutableStateOf(YesNoAbstainVoteChoice.YES) }
    var proTxHashHex by remember { mutableStateOf("") }
    var votingKeyHex by remember { mutableStateOf("") }
    var isBusy by remember { mutableStateOf(false) }
    var message by remember { mutableStateOf<String?>(null) }
    var messageIsError by remember { mutableStateOf(false) }

    AlertDialog(
        onDismissRequest = { if (!isBusy) onDismiss() },
        properties = secureDialogProperties(dismissible = !isBusy),
        title = { Text("Vote · ${truncateMiddle(proposal.title, 20, 8)}") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                AccessiblePicker(
                    label = "Answer",
                    options = YesNoAbstainVoteChoice.entries,
                    selected = choice,
                    optionLabel = { it.label() },
                    testTag = "decisionProposalVote.choice",
                    enabled = !isBusy,
                    onSelected = { choice = it },
                )
                OutlinedTextField(
                    value = proTxHashHex,
                    onValueChange = { proTxHashHex = it },
                    label = { Text("pro_tx_hash (64 hex chars, wire order)") },
                    singleLine = true,
                    enabled = !isBusy,
                    textStyle = MaterialTheme.typography.bodySmall
                        .copy(fontFamily = FontFamily.Monospace),
                    modifier = Modifier.fillMaxWidth().testTag("decisionProposalVote.proTxHash"),
                )
                OutlinedTextField(
                    value = votingKeyHex,
                    onValueChange = { votingKeyHex = it },
                    label = { Text("voting private key (64 hex chars)") },
                    singleLine = true,
                    enabled = !isBusy,
                    textStyle = MaterialTheme.typography.bodySmall
                        .copy(fontFamily = FontFamily.Monospace),
                    // KeyboardType.Password only configures the keyboard; the
                    // transformation masks what the field shows.
                    visualTransformation = PasswordVisualTransformation(),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                    modifier = Modifier.fillMaxWidth().testTag("decisionProposalVote.votingKey"),
                )
                Text(
                    "The pro_tx_hash goes in wire order, the reverse of the hex Core " +
                        "displays. The voting public key and signer are derived from the " +
                        "private key on the Rust side; the key bytes are not stored.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                TextButton(
                    enabled = !isBusy && proTxHashHex.isNotBlank(),
                    onClick = {
                        val currentSdk = sdk ?: return@TextButton
                        val proTxHash = decode32ByteHex(proTxHashHex)
                        if (proTxHash == null) {
                            message = "pro_tx_hash must be 32 bytes (64 hex characters)."
                            messageIsError = true
                            return@TextButton
                        }
                        isBusy = true
                        scope.launch {
                            try {
                                val answer = currentSdk.decisionProposals
                                    .masternodeVote(proTxHash, proposal.id)
                                message = answer?.let {
                                    "This masternode answered ${it.choice.label()} " +
                                        "(${it.timesVoted} vote" +
                                        (if (it.timesVoted == 1) "" else "s") + ")."
                                } ?: "This masternode has no answer on the open poll."
                                messageIsError = false
                            } catch (e: Exception) {
                                message = "Reading the answer failed: ${e.message}"
                                messageIsError = true
                            } finally {
                                isBusy = false
                            }
                        }
                    },
                    modifier = Modifier.testTag("decisionProposalVote.lookUp"),
                ) { Text("Read This Masternode's Answer") }
                message?.let {
                    Text(
                        it,
                        style = MaterialTheme.typography.bodySmall,
                        color = if (messageIsError) {
                            MaterialTheme.colorScheme.error
                        } else {
                            MaterialTheme.colorScheme.primary
                        },
                        modifier = Modifier.testTag("decisionProposalVote.result"),
                    )
                }
            }
        },
        confirmButton = {
            TextButton(
                enabled = !isBusy && proTxHashHex.isNotBlank() && votingKeyHex.isNotBlank(),
                onClick = {
                    val currentSdk = sdk ?: return@TextButton
                    val proTxHash = decode32ByteHex(proTxHashHex)
                    if (proTxHash == null) {
                        message = "pro_tx_hash must be 32 bytes (64 hex characters)."
                        messageIsError = true
                        return@TextButton
                    }
                    val votingKey = decode32ByteHex(votingKeyHex)
                    if (votingKey == null) {
                        message = "Voting private key must be 32 bytes (64 hex characters)."
                        messageIsError = true
                        return@TextButton
                    }
                    isBusy = true
                    scope.launch {
                        try {
                            currentSdk.decisionProposals.castVote(
                                proposalId = proposal.id,
                                choice = choice,
                                proTxHash = proTxHash,
                                votingPrivateKey = votingKey,
                            )
                            onVoted("Vote accepted by Platform.")
                        } catch (e: Exception) {
                            message = "Vote rejected: ${e.message}"
                            messageIsError = true
                        } finally {
                            votingKey.fill(0)
                            isBusy = false
                        }
                    }
                },
                modifier = Modifier.testTag("decisionProposalVote.submit"),
            ) { Text(if (isBusy) "Broadcasting…" else "Submit Vote") }
        },
        dismissButton = {
            TextButton(enabled = !isBusy, onClick = onDismiss) { Text("Cancel") }
        },
    )
}

/**
 * Block screenshots, screen recording and the recents thumbnail of the app window while the
 * calling composable is shown (the app's FLAG_SECURE toggle, [LocalSecureScreen]). Shared
 * with the contested-name vote dialog, which takes the same voting private key.
 */
@Composable
internal fun HoldSecureWindow() {
    val setSecure = LocalSecureScreen.current
    DisposableEffect(Unit) {
        setSecure(true)
        onDispose { setSecure(false) }
    }
}

/**
 * The properties of a dialog that takes a voting private key. A dialog is a window of its
 * own: FLAG_SECURE on the app window reaches it only when the dialog reads its parent's
 * flags, so the dialog sets the flag itself. [dismissible] false holds Back and taps outside
 * while a call runs, so the dialog (and the call in its scope) stays.
 */
internal fun secureDialogProperties(dismissible: Boolean): DialogProperties = DialogProperties(
    dismissOnBackPress = dismissible,
    dismissOnClickOutside = dismissible,
    securePolicy = SecureFlagPolicy.SecureOn,
)

/** Display name of a vote answer. */
private fun YesNoAbstainVoteChoice.label(): String = when (this) {
    YesNoAbstainVoteChoice.YES -> "Yes"
    YesNoAbstainVoteChoice.NO -> "No"
    YesNoAbstainVoteChoice.ABSTAIN -> "Abstain"
}

/**
 * Gives a proposal a local identity filed new text: port of `EditDecisionProposalView`
 * (`DecisionProposalFormViews.swift`). Submits through
 * `PlatformWalletManager.decisionProposals.edit`, one platform wallet call that reads the
 * stored proposal, applies the edit (rs-sdk refuses one that changes nothing), signs with a
 * key it picks and waits for Platform to confirm.
 *
 * The dialog cannot be dismissed (Back or a tap outside) while the edit runs, so the paid
 * replace running in its scope is not cancelled with it. Should the dialog still leave
 * composition mid-edit (a rotation), the detail screen reads the proposal again, and an edit
 * repeating one that landed changes nothing, which rs-sdk refuses before anything is
 * broadcast.
 */
@Composable
private fun EditDecisionProposalDialog(
    proposal: DecisionProposal,
    owner: IdentityEntity,
    onDismiss: () -> Unit,
    onEdited: (String) -> Unit,
) {
    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    var title by remember { mutableStateOf(proposal.title) }
    var description by remember { mutableStateOf(proposal.description) }
    var url by remember { mutableStateOf(proposal.url.orEmpty()) }
    var isSubmitting by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    AlertDialog(
        onDismissRequest = { if (!isSubmitting) onDismiss() },
        properties = DialogProperties(
            dismissOnBackPress = !isSubmitting,
            dismissOnClickOutside = !isSubmitting,
        ),
        title = { Text("Edit Proposal") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    value = title,
                    onValueChange = { title = it },
                    label = { Text("Title") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    modifier = Modifier.fillMaxWidth().testTag("editDecisionProposal.title"),
                )
                OutlinedTextField(
                    value = description,
                    onValueChange = { description = it },
                    label = { Text("Description") },
                    minLines = 3,
                    enabled = !isSubmitting,
                    modifier = Modifier.fillMaxWidth().testTag("editDecisionProposal.description"),
                )
                OutlinedTextField(
                    value = url,
                    onValueChange = { url = it },
                    label = { Text("URL (empty for none)") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
                    modifier = Modifier.fillMaxWidth().testTag("editDecisionProposal.url"),
                )
                Text(
                    "Only the title, description and url can change, and only for a day " +
                        "after the proposal was created (or a fifth of its voting time, if " +
                        "that is shorter).",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                error?.let {
                    Text(
                        it,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("editDecisionProposal.error"),
                    )
                }
            }
        },
        confirmButton = {
            TextButton(
                enabled = !isSubmitting && manager != null,
                onClick = {
                    val mgr = manager ?: return@TextButton
                    val wallet = owner.walletId?.toHex()?.let { mgr.wallets.value[it] }
                    if (wallet == null) {
                        error = "The proposer's wallet is not loaded in the wallet manager."
                        return@TextButton
                    }
                    val proposalIdBytes = Base58.decodeIdentifier(proposal.id)
                    if (proposalIdBytes == null || proposalIdBytes.size != 32) {
                        error = "The proposal id is not a 32-byte base58 identifier."
                        return@TextButton
                    }
                    // Only the fields the user changed from the proposal this dialog
                    // shows are sent: an untouched field must not write back the text
                    // it was opened with over a newer edit by another client. An
                    // emptied url field removes the url.
                    val trimmedUrl = url.trim()
                    val urlChange = when {
                        trimmedUrl == proposal.url.orEmpty() -> DecisionProposalEdit.UrlChange.Keep
                        trimmedUrl.isEmpty() -> DecisionProposalEdit.UrlChange.Remove
                        else -> DecisionProposalEdit.UrlChange.Set(trimmedUrl)
                    }
                    val edit = DecisionProposalEdit(
                        title = title.takeIf { it != proposal.title },
                        description = description.takeIf { it != proposal.description },
                        url = urlChange,
                    )
                    isSubmitting = true
                    error = null
                    scope.launch {
                        try {
                            mgr.decisionProposals.edit(
                                walletHandle = wallet.handle,
                                ownerId = owner.identityId,
                                proposalId = proposalIdBytes,
                                edit = edit,
                                signerHandle = mgr.signerHandle,
                            )
                            onEdited("Edit accepted by Platform.")
                        } catch (e: Exception) {
                            error = e.message ?: "Edit failed"
                        } finally {
                            isSubmitting = false
                        }
                    }
                },
                modifier = Modifier.testTag("editDecisionProposal.submitButton"),
            ) { Text(if (isSubmitting) "Broadcasting…" else "Replace / Broadcast") }
        },
        dismissButton = {
            TextButton(enabled = !isSubmitting, onClick = onDismiss) { Text("Cancel") }
        },
    )
}
