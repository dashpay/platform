package org.dashfoundation.example.ui.identity

import androidx.activity.compose.BackHandler
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
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import java.util.Date
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.voting.DescriptionDocumentParams
import org.dashfoundation.dashsdk.voting.MultipleChoiceProposal
import org.dashfoundation.dashsdk.voting.MultipleChoiceProposalEdit
import org.dashfoundation.dashsdk.voting.MultipleChoiceProposalVote
import org.dashfoundation.dashsdk.voting.MultipleChoiceVoteChoice
import org.dashfoundation.dashsdk.voting.ProposalChoice
import org.dashfoundation.dashsdk.voting.ProposalChoiceParams
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.tokens.displayLabel
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.formatCredits
import org.dashfoundation.example.util.formatDate
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.truncateMiddle

/**
 * One multiple choice proposal: port of `MultipleChoiceProposalDetailView.swift`, beside
 * the yes/no [DecisionProposalDetailScreen].
 *
 * Reads the proposal, the vote among its choices and, once the vote ended, the choice it
 * kept through `Sdk.multipleChoiceProposals` (each independently, so one failing read
 * leaves the others shown). Shows the choices with their tallies while the vote runs and
 * how it ended afterwards. Offers the masternode vote ([MultipleChoiceVoteDialog], the same
 * credentials [ContestDetailScreen] collects), entering a choice for a local wallet
 * identity ([EnterChoiceDialog]) and, when a local wallet identity filed the proposal, the
 * edit ([EditMultipleChoiceProposalDialog]).
 *
 * Entering a choice pays the contest fund, so it runs in [choiceEntry], which the screen's
 * back-stack entry holds, and Back is held while it runs.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MultipleChoiceProposalDetailScreen(
    proposalId: String,
    navController: NavHostController,
    choiceEntry: MultipleChoiceDocumentCreateViewModel = viewModel(),
) {
    val appState = LocalAppState.current
    val container = LocalAppContainer.current
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val walletIdentities by container.database.identityDao()
        .observeWalletOwnedByNetwork(network.ffiValue)
        .collectAsStateWithLifecycle(initialValue = emptyList())

    var proposal by remember { mutableStateOf<MultipleChoiceProposal?>(null) }
    var vote by remember { mutableStateOf<MultipleChoiceProposalVote?>(null) }
    var keptChoice by remember { mutableStateOf<ProposalChoice?>(null) }
    var proposalError by remember { mutableStateOf<String?>(null) }
    var voteError by remember { mutableStateOf<String?>(null) }
    var isLoading by remember { mutableStateOf(false) }
    var refreshTick by remember { mutableStateOf(0) }

    var showVote by remember { mutableStateOf(false) }
    var showEnterChoice by remember { mutableStateOf(false) }
    var showEdit by remember { mutableStateOf(false) }
    var actionMessage by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(sdk, proposalId, refreshTick) {
        val currentSdk = sdk ?: run {
            proposalError = "SDK not initialized"
            return@LaunchedEffect
        }
        isLoading = true
        proposalError = null
        voteError = null
        try {
            proposal = readOrNull({ proposalError = "Loading the proposal failed: ${it.message}" }) {
                currentSdk.multipleChoiceProposals.fetch(proposalId)
            }
            if (proposal == null && proposalError == null) {
                proposalError = "Platform has no multiple choice proposal $proposalId."
            }
            val read = readOrNull({ voteError = "Loading the vote failed: ${it.message}" }) {
                currentSdk.multipleChoiceProposals.vote(proposalId)
            }
            vote = read
            // The choices not kept are gone once the vote ended; the kept one is a read of
            // its own.
            keptChoice = if (read?.result != null) {
                readOrNull({ voteError = "Loading the kept choice failed: ${it.message}" }) {
                    currentSdk.multipleChoiceProposals.keptChoice(proposalId)
                }
            } else {
                null
            }
        } finally {
            isLoading = false
        }
    }

    // A choice Platform confirmed, also when its dialog left composition meanwhile.
    val enteredChoiceId = choiceEntry.createdId
    LaunchedEffect(enteredChoiceId) {
        if (enteredChoiceId != null) {
            showEnterChoice = false
            actionMessage = "Choice entered: ${truncateMiddle(enteredChoiceId, 12, 8)}"
            choiceEntry.acknowledge()
            refreshTick++
        }
    }

    // The local wallet identity that filed the proposal, which may edit it.
    val owner = remember(proposal, walletIdentities) {
        proposal?.let { shown ->
            walletIdentities.firstOrNull { Base58.encode(it.identityId) == shown.ownerId }
        }
    }

    val isEnteringChoice = choiceEntry.isSubmitting
    // System Back would pop the entry and clear the paid choice entry with it; the toolbar
    // Back is disabled for the same reason.
    BackHandler(enabled = isEnteringChoice) {}

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Multiple Choice Proposal") },
                navigationIcon = {
                    IconButton(
                        onClick = { navController.popBackStack() },
                        enabled = !isEnteringChoice,
                    ) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = { refreshTick++ },
                        enabled = !isLoading,
                        modifier = Modifier.testTag("multipleChoiceProposal.refresh"),
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
                    modifier = Modifier.testTag("multipleChoiceProposal.error"),
                )
            }

            val shown = proposal
            if (shown != null) {
                MultipleChoiceProposalSection(shown, isOwnerLocal = owner != null)
            }

            MultipleChoiceVoteSection(
                vote = vote,
                keptChoice = keptChoice,
                isLoading = isLoading,
                error = voteError,
            )

            FormSection(title = "Actions") {
                actionMessage?.let { message ->
                    Text(
                        message,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.testTag("multipleChoiceProposal.actionResult"),
                    )
                }
                SubmitButton(
                    text = "Vote as a Masternode…",
                    isLoading = false,
                    // Platform takes votes only while the vote runs
                    enabled = sdk != null && shown != null && vote?.result == null,
                    modifier = Modifier.fillMaxWidth().testTag("multipleChoiceProposal.vote"),
                ) {
                    actionMessage = null
                    showVote = true
                }
                Text(
                    "Only masternodes vote, with the voting key of a pro_tx_hash: for one " +
                        "choice, for none of the choices, or abstaining. A regular wallet " +
                        "has no voting key, so Platform refuses its vote.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                if (walletIdentities.isNotEmpty()) {
                    SubmitButton(
                        text = if (isEnteringChoice) "Entering a Choice…" else "Enter a Choice…",
                        isLoading = isEnteringChoice,
                        enabled = sdk != null && shown != null && vote?.result == null,
                        modifier = Modifier
                            .fillMaxWidth()
                            .testTag("multipleChoiceProposal.enterChoice"),
                    ) {
                        actionMessage = null
                        choiceEntry.error = null
                        showEnterChoice = true
                    }
                    // Shown here when the dialog that started the entry is gone (a rotation).
                    if (!showEnterChoice) {
                        choiceEntry.error?.let { message ->
                            Text(
                                message,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.error,
                                modifier = Modifier.testTag("multipleChoiceProposal.enterChoiceError"),
                            )
                        }
                    }
                    Text(
                        "A choice pays the contest fund on top of the fees and is never " +
                            "edited. For this proposal, " +
                            (shown?.whoEntersChoices() ?: "Platform says who enters choices") + ".",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                if (owner != null) {
                    SubmitButton(
                        text = "Edit…",
                        isLoading = false,
                        enabled = sdk != null && shown != null,
                        modifier = Modifier.fillMaxWidth().testTag("multipleChoiceProposal.edit"),
                    ) {
                        actionMessage = null
                        showEdit = true
                    }
                    Text(
                        "Platform accepts an edit of the title, description document and " +
                            "url for a short time after the proposal is created, and " +
                            "refuses it afterwards. Whether the proposal is open never " +
                            "changes.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }

    val shown = proposal
    if (showVote && shown != null) {
        MultipleChoiceVoteDialog(
            proposal = shown,
            vote = vote,
            onDismiss = { showVote = false },
            onVoted = { message ->
                showVote = false
                actionMessage = message
                refreshTick++ // pull fresh tallies after an accepted vote
            },
        )
    }
    if (showEnterChoice && shown != null && walletIdentities.isNotEmpty()) {
        EnterChoiceDialog(
            proposal = shown,
            identities = walletIdentities,
            proposer = owner,
            entry = choiceEntry,
            onDismiss = { showEnterChoice = false },
        )
    }
    val editor = owner
    if (showEdit && shown != null && editor != null) {
        EditMultipleChoiceProposalDialog(
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

@Composable
private fun MultipleChoiceProposalSection(proposal: MultipleChoiceProposal, isOwnerLocal: Boolean) {
    FormSection(title = "Proposal") {
        Text(
            proposal.title,
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.testTag("multipleChoiceProposal.title"),
        )
        proposal.url?.let { LabeledContent("URL", it) }
        proposal.descriptionDocument?.let { described ->
            LabeledContent(
                "Description document",
                truncateMiddle(described.id, 12, 8),
                modifier = Modifier.testTag("multipleChoiceProposal.descriptionDocument"),
            )
            LabeledContent("In contract", truncateMiddle(described.contractId, 12, 8))
            LabeledContent("Of type", described.documentType)
        }
        LabeledContent(
            "Choices",
            proposal.whoEntersChoices(),
            modifier = Modifier.testTag("multipleChoiceProposal.open"),
        )
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
private fun MultipleChoiceVoteSection(
    vote: MultipleChoiceProposalVote?,
    keptChoice: ProposalChoice?,
    isLoading: Boolean,
    error: String?,
) {
    FormSection(title = "Vote") {
        error?.let { message ->
            Text(
                message,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.testTag("multipleChoiceProposal.voteError"),
            )
        }
        when {
            vote != null -> MultipleChoiceVoteRows(vote, keptChoice)
            isLoading -> LabeledContent("Status", "Loading…")
        }
        Text(
            "A vote names a choice by its id. The choice with the most votes is kept as an " +
                "ordinary document and the others are gone, unless the votes for none of " +
                "the choices outnumber it. Abstaining is recorded but plays no part in " +
                "the result.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun MultipleChoiceVoteRows(vote: MultipleChoiceProposalVote, keptChoice: ProposalChoice?) {
    val result = vote.result
    LabeledContent(
        "Status",
        when {
            result != null -> result.outcome.label()
            vote.choices.isEmpty() -> "No choice entered"
            else -> "Voting open"
        },
        modifier = Modifier.testTag("multipleChoiceProposal.voteStatus"),
    )
    if (result != null) {
        result.keptChoiceId?.let { keptId ->
            LabeledContent(
                "Kept choice",
                keptChoice?.title ?: truncateMiddle(keptId, 12, 8),
                modifier = Modifier.testTag("multipleChoiceProposal.keptChoice"),
            )
            keptChoice?.description?.let { Text(it, style = MaterialTheme.typography.bodyMedium) }
            LabeledContent("Kept choice ID", truncateMiddle(keptId, 12, 8))
        }
        LabeledContent("Closed at block", "${result.finalizationBlock.height}")
    }
    vote.choices.forEach { entry ->
        val kept = result?.keptChoiceId == entry.id
        // Once the vote ended the choices are gone from the vote; the kept one is known.
        val title = entry.choice?.title
            ?: keptChoice?.takeIf { kept }?.title
            ?: truncateMiddle(entry.id, 12, 8)
        LabeledContent(
            title + if (kept) " (kept)" else "",
            entry.voteTally.tally(),
            modifier = Modifier.testTag("multipleChoiceProposal.choice.${entry.id}"),
        )
        entry.choice?.description?.let {
            Text(
                it,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        entry.choice?.let {
            Text(
                "${truncateMiddle(entry.id, 12, 8)} · entered by ${truncateMiddle(it.ownerId, 10, 6)}",
                style = MaterialTheme.typography.labelSmall.copy(fontFamily = FontFamily.Monospace),
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
    LabeledContent(
        "None of the choices",
        vote.noneOfTheChoicesVoteTally.tally(),
        modifier = Modifier.testTag("multipleChoiceProposal.noneOfTheChoices"),
    )
    LabeledContent(
        "Abstain",
        vote.abstainVoteTally.tally(),
        modifier = Modifier.testTag("multipleChoiceProposal.abstain"),
    )
}

/** A tally as text; Platform does not report every tally. */
private fun Long?.tally(): String = this?.let { "$it" } ?: "not reported"

/** How a finished vote ended, as the status row says it. */
private fun MultipleChoiceProposalVote.Outcome.label(): String = when (this) {
    MultipleChoiceProposalVote.Outcome.KEPT_CHOICE -> "Ended: a choice was kept"
    MultipleChoiceProposalVote.Outcome.NONE_OF_THE_CHOICES -> "Ended: none of the choices was kept"
    MultipleChoiceProposalVote.Outcome.NO_WINNER -> "Ended without a winner"
}

/** Display name of a vote answer; a choice shows its title when the vote still holds it. */
private fun MultipleChoiceVoteChoice.label(vote: MultipleChoiceProposalVote?): String = when (this) {
    is MultipleChoiceVoteChoice.Choice -> {
        val title = vote?.choices?.firstOrNull { it.id == choiceId }?.choice?.title
        val id = truncateMiddle(choiceId, 8, 6)
        if (title != null) "$title ($id)" else id
    }
    MultipleChoiceVoteChoice.NoneOfTheChoices -> "None of the choices"
    MultipleChoiceVoteChoice.Abstain -> "Abstain"
}

/**
 * Picks an answer and takes masternode credentials: port of
 * `MultipleChoiceProposalVoteSheet` in `MultipleChoiceProposalDetailView.swift`. Votes
 * through `Sdk.multipleChoiceProposals.castVote`, and reads the masternode's current answer
 * through `Sdk.multipleChoiceProposals.masternodeVote`. The answers offered are the choices
 * the vote holds, each by its document id, none of the choices, and abstaining.
 *
 * The voting private key is masked as it is typed, and screen capture is blocked while the
 * dialog is open (see [HoldSecureWindow]). The dialog cannot be dismissed, by Back or a tap
 * outside, while a call runs.
 */
@Composable
private fun MultipleChoiceVoteDialog(
    proposal: MultipleChoiceProposal,
    vote: MultipleChoiceProposalVote?,
    onDismiss: () -> Unit,
    onVoted: (String) -> Unit,
) {
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    HoldSecureWindow()

    val answers: List<MultipleChoiceVoteChoice> = remember(vote) {
        vote?.choices.orEmpty().map { MultipleChoiceVoteChoice.Choice(it.id) } +
            listOf(MultipleChoiceVoteChoice.NoneOfTheChoices, MultipleChoiceVoteChoice.Abstain)
    }
    var choice by remember { mutableStateOf(answers.first()) }
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
                    options = answers,
                    selected = choice,
                    optionLabel = { it.label(vote) },
                    testTag = "multipleChoiceProposalVote.choice",
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
                    modifier = Modifier.fillMaxWidth().testTag("multipleChoiceProposalVote.proTxHash"),
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
                    modifier = Modifier.fillMaxWidth().testTag("multipleChoiceProposalVote.votingKey"),
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
                                val answer = currentSdk.multipleChoiceProposals
                                    .masternodeVote(proTxHash, proposal.id)
                                message = answer?.let {
                                    "This masternode answered ${it.label(vote)}."
                                } ?: "This masternode has no answer in the running vote."
                                messageIsError = false
                            } catch (e: CancellationException) {
                                throw e
                            } catch (e: Exception) {
                                message = "Reading the answer failed: ${e.message}"
                                messageIsError = true
                            } finally {
                                isBusy = false
                            }
                        }
                    },
                    modifier = Modifier.testTag("multipleChoiceProposalVote.lookUp"),
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
                        modifier = Modifier.testTag("multipleChoiceProposalVote.result"),
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
                            currentSdk.multipleChoiceProposals.castVote(
                                proposalId = proposal.id,
                                choice = choice,
                                proTxHash = proTxHash,
                                votingPrivateKey = votingKey,
                            )
                            onVoted("Vote accepted by Platform.")
                        } catch (e: CancellationException) {
                            throw e
                        } catch (e: Exception) {
                            message = "Vote rejected: ${e.message}"
                            messageIsError = true
                        } finally {
                            votingKey.fill(0)
                            isBusy = false
                        }
                    }
                },
                modifier = Modifier.testTag("multipleChoiceProposalVote.submit"),
            ) { Text(if (isBusy) "Broadcasting…" else "Submit Vote") }
        },
        dismissButton = {
            TextButton(enabled = !isBusy, onClick = onDismiss) { Text("Cancel") }
        },
    )
}

/**
 * Enters a choice for a proposal as a local wallet identity: port of
 * `EnterProposalChoiceView` (`MultipleChoiceProposalFormViews.swift`). Submits through
 * `PlatformWalletManager.multipleChoiceProposals.enterChoice`, one platform wallet call that
 * builds the choice, states the contest fund, signs with a key it picks and waits for
 * Platform to confirm. The fund is read through `Sdk.multipleChoiceProposals.choiceFundToJoin`
 * and shown before the submit.
 *
 * The entry runs in [entry], held by the detail screen's back-stack entry, so a dialog that
 * leaves composition (a rotation) does not cancel a paid choice; the detail screen reports
 * its outcome. The dialog cannot be dismissed while the entry runs. Who may enter a choice
 * is Platform's to say: any local wallet identity can be picked.
 */
@Composable
private fun EnterChoiceDialog(
    proposal: MultipleChoiceProposal,
    identities: List<IdentityEntity>,
    proposer: IdentityEntity?,
    entry: MultipleChoiceDocumentCreateViewModel,
    onDismiss: () -> Unit,
) {
    val appState = LocalAppState.current
    val container = LocalAppContainer.current
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    var owner by remember { mutableStateOf(proposer ?: identities.first()) }
    var title by remember { mutableStateOf("") }
    var description by remember { mutableStateOf("") }
    val isSubmitting = entry.isSubmitting

    // The fund a choice entered now pays, read rather than assumed: it rises as the vote
    // fills up.
    var fund by remember { mutableStateOf<ULong?>(null) }
    var fundError by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(sdk, proposal.id) {
        fund = null
        fundError = null
        val currentSdk = sdk ?: return@LaunchedEffect
        try {
            fund = currentSdk.multipleChoiceProposals.choiceFundToJoin(proposal.id)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            fundError = "Reading the contest fund failed: ${e.message}"
        }
    }

    AlertDialog(
        onDismissRequest = { if (!isSubmitting) onDismiss() },
        properties = DialogProperties(
            dismissOnBackPress = !isSubmitting,
            dismissOnClickOutside = !isSubmitting,
        ),
        title = { Text("Enter a Choice") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                AccessiblePicker(
                    label = "Entered by",
                    options = identities,
                    selected = owner,
                    optionLabel = { it.displayLabel() },
                    testTag = "enterProposalChoice.ownerPicker",
                    enabled = !isSubmitting,
                    onSelected = { owner = it },
                )
                OutlinedTextField(
                    value = title,
                    onValueChange = { title = it },
                    label = { Text("Title: the answer, in one line") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    modifier = Modifier.fillMaxWidth().testTag("enterProposalChoice.title"),
                )
                OutlinedTextField(
                    value = description,
                    onValueChange = { description = it },
                    label = { Text("Description: what voting for it means (optional)") },
                    minLines = 3,
                    enabled = !isSubmitting,
                    modifier = Modifier.fillMaxWidth().testTag("enterProposalChoice.description"),
                )
                LabeledContent(
                    "Contest fund",
                    fund?.let { formatCredits(it) }
                        ?: if (fundError == null) "Loading…" else "Unavailable",
                    modifier = Modifier.testTag("enterProposalChoice.fund"),
                )
                Text(
                    "The choice pays the contest fund on top of the fees and is never " +
                        "edited. For this proposal, ${proposal.whoEntersChoices()}.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                (entry.error ?: fundError)?.let {
                    Text(
                        it,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("enterProposalChoice.error"),
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
                        entry.error = "The identity's wallet is not loaded in the wallet manager."
                        return@TextButton
                    }
                    val proposalIdBytes = Base58.decodeIdentifier(proposal.id)
                    if (proposalIdBytes == null) {
                        entry.error = "The proposal id is not a 32-byte base58 identifier."
                        return@TextButton
                    }
                    val enteredBy = owner.identityId
                    val params = ProposalChoiceParams(
                        title = title,
                        description = description.ifBlank { null },
                    )
                    entry.submit {
                        mgr.multipleChoiceProposals.enterChoice(
                            walletHandle = wallet.handle,
                            ownerId = enteredBy,
                            proposalId = proposalIdBytes,
                            params = params,
                            signerHandle = mgr.signerHandle,
                        ).id
                    }
                },
                modifier = Modifier.testTag("enterProposalChoice.submitButton"),
            ) { Text(if (isSubmitting) "Broadcasting…" else "Enter / Broadcast") }
        },
        dismissButton = {
            TextButton(enabled = !isSubmitting, onClick = onDismiss) { Text("Cancel") }
        },
    )
}

/**
 * Gives a proposal a local identity filed a new title, url or description document: port
 * of `EditMultipleChoiceProposalView` (`MultipleChoiceProposalFormViews.swift`). Submits
 * through `PlatformWalletManager.multipleChoiceProposals.edit`, one platform wallet call
 * that reads the stored proposal, applies the edit (rs-sdk refuses one that changes
 * nothing), signs with a key it picks and waits for Platform to confirm.
 *
 * The dialog cannot be dismissed (Back or a tap outside) while the edit runs, so the
 * replace running in its scope is not cancelled with it. Should the dialog still leave
 * composition mid-edit (a rotation), the detail screen reads the proposal again, and an edit
 * repeating one that landed changes nothing, which rs-sdk refuses before anything is
 * broadcast.
 */
@Composable
private fun EditMultipleChoiceProposalDialog(
    proposal: MultipleChoiceProposal,
    owner: IdentityEntity,
    onDismiss: () -> Unit,
    onEdited: (String) -> Unit,
) {
    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    val described = proposal.descriptionDocument
    var title by remember { mutableStateOf(proposal.title) }
    var url by remember { mutableStateOf(proposal.url.orEmpty()) }
    var describedId by remember { mutableStateOf(described?.id.orEmpty()) }
    var describedContractId by remember { mutableStateOf(described?.contractId.orEmpty()) }
    var describedType by remember { mutableStateOf(described?.documentType.orEmpty()) }
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
            Column(
                verticalArrangement = Arrangement.spacedBy(12.dp),
                modifier = Modifier.verticalScroll(rememberScrollState()),
            ) {
                OutlinedTextField(
                    value = title,
                    onValueChange = { title = it },
                    label = { Text("Title") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    modifier = Modifier.fillMaxWidth().testTag("editMultipleChoiceProposal.title"),
                )
                OutlinedTextField(
                    value = url,
                    onValueChange = { url = it },
                    label = { Text("URL (empty for none)") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
                    modifier = Modifier.fillMaxWidth().testTag("editMultipleChoiceProposal.url"),
                )
                OutlinedTextField(
                    value = describedId,
                    onValueChange = { describedId = it },
                    label = { Text("Description document ID (empty for none)") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("editMultipleChoiceProposal.descriptionDocumentId"),
                )
                OutlinedTextField(
                    value = describedContractId,
                    onValueChange = { describedContractId = it },
                    label = { Text("Its contract ID") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("editMultipleChoiceProposal.descriptionDocumentContractId"),
                )
                OutlinedTextField(
                    value = describedType,
                    onValueChange = { describedType = it },
                    label = { Text("Its document type") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("editMultipleChoiceProposal.descriptionDocumentType"),
                )
                Text(
                    "Only the title, url and description document can change, and only " +
                        "for a short time after the proposal was created. Platform " +
                        "refuses an edit that leaves the proposal with neither a " +
                        "description document nor a url.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                error?.let {
                    Text(
                        it,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("editMultipleChoiceProposal.error"),
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
                    if (proposalIdBytes == null) {
                        error = "The proposal id is not a 32-byte base58 identifier."
                        return@TextButton
                    }
                    val edit = when (
                        val built = multipleChoiceProposalEdit(
                            proposal = proposal,
                            title = title,
                            url = url,
                            describedId = describedId,
                            describedContractId = describedContractId,
                            describedType = describedType,
                        )
                    ) {
                        is BuiltMultipleChoiceEdit.Unreadable -> {
                            error = built.message
                            return@TextButton
                        }
                        is BuiltMultipleChoiceEdit.Edit -> built.edit
                    }
                    isSubmitting = true
                    error = null
                    scope.launch {
                        try {
                            mgr.multipleChoiceProposals.edit(
                                walletHandle = wallet.handle,
                                ownerId = owner.identityId,
                                proposalId = proposalIdBytes,
                                edit = edit,
                                signerHandle = mgr.signerHandle,
                            )
                            onEdited("Edit accepted by Platform.")
                        } catch (e: CancellationException) {
                            throw e
                        } catch (e: Exception) {
                            error = e.message ?: "Edit failed"
                        } finally {
                            isSubmitting = false
                        }
                    }
                },
                modifier = Modifier.testTag("editMultipleChoiceProposal.submitButton"),
            ) { Text(if (isSubmitting) "Broadcasting…" else "Replace / Broadcast") }
        },
        dismissButton = {
            TextButton(enabled = !isSubmitting, onClick = onDismiss) { Text("Cancel") }
        },
    )
}

/** The edit an edit form's fields make, or why a field could not be read. */
internal sealed interface BuiltMultipleChoiceEdit {
    data class Edit(val edit: MultipleChoiceProposalEdit) : BuiltMultipleChoiceEdit

    /** A description document id field does not hold the 32 bytes the wallet call takes. */
    data class Unreadable(val message: String) : BuiltMultipleChoiceEdit
}

/**
 * The edit that turns [proposal], as the form was opened with it, into what the form's
 * fields now hold. Only the fields the user changed are sent: an untouched field must not
 * write back the value the form was opened with over a newer edit by another client. An
 * emptied url removes the url, and three emptied description document fields remove the
 * description document. Whether the result is an acceptable proposal is not looked at here.
 */
internal fun multipleChoiceProposalEdit(
    proposal: MultipleChoiceProposal,
    title: String,
    url: String,
    describedId: String,
    describedContractId: String,
    describedType: String,
): BuiltMultipleChoiceEdit {
    val trimmedUrl = url.trim()
    val urlChange = when {
        trimmedUrl == proposal.url.orEmpty() -> MultipleChoiceProposalEdit.Change.Keep
        trimmedUrl.isEmpty() -> MultipleChoiceProposalEdit.Change.Remove
        else -> MultipleChoiceProposalEdit.Change.Set(trimmedUrl)
    }
    val described = proposal.descriptionDocument
    val describedUntouched = describedId.trim() == described?.id.orEmpty() &&
        describedContractId.trim() == described?.contractId.orEmpty() &&
        describedType.trim() == described?.documentType.orEmpty()
    val describedChange: MultipleChoiceProposalEdit.Change<DescriptionDocumentParams> =
        if (describedUntouched) {
            MultipleChoiceProposalEdit.Change.Keep
        } else {
            when (val parsed = parseDescriptionDocument(describedId, describedContractId, describedType)) {
                is ParsedDescriptionDocument.Unreadable ->
                    return BuiltMultipleChoiceEdit.Unreadable(parsed.message)
                ParsedDescriptionDocument.None -> MultipleChoiceProposalEdit.Change.Remove
                is ParsedDescriptionDocument.Document ->
                    MultipleChoiceProposalEdit.Change.Set(parsed.document)
            }
        }
    return BuiltMultipleChoiceEdit.Edit(
        MultipleChoiceProposalEdit(
            title = title.takeIf { it != proposal.title },
            descriptionDocument = describedChange,
            url = urlChange,
        ),
    )
}
