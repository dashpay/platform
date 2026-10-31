package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.clickable
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
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
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
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import java.util.Date
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.voting.DecisionProposal
import org.dashfoundation.dashsdk.voting.DecisionProposalParams
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.CreateDecisionProposal
import org.dashfoundation.example.navigation.DecisionProposalDetail
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.contracts.parseDocumentId
import org.dashfoundation.example.ui.tokens.displayLabel
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.formatDate
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.truncateMiddle

/** The three ways the list reads proposals, each one Platform listing (← `Listing`). */
private enum class ProposalListing(val label: String, val footer: String) {
    OPEN("Open", "Proposals whose voting closes from now on, closing soonest first."),
    MINE("Mine", "Proposals the selected identity filed, in the order it filed them."),
    NEWEST("Newest", "Every proposal, newest first."),
}

/** Proposals per page; a page shorter than this is the last. */
private const val PROPOSAL_PAGE_SIZE = 20

/**
 * Decision proposals (protocol version 15): port of `DecisionProposalsView.swift`.
 *
 * Lists proposals three ways, a page at a time, through `Sdk.decisionProposals`: the open
 * ones closing soonest first (`listByEndDate` from now), the ones a local identity filed
 * (`listByOwner`) and the newest (`listByCreatedAt`). Rows drill into
 * [DecisionProposalDetailScreen]; the + action opens [CreateDecisionProposalScreen].
 * Nothing here decides what a proposal is.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DecisionProposalsScreen(navController: NavHostController) {
    val appState = LocalAppState.current
    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val identities by container.database.identityDao()
        .observeByNetwork(network.ffiValue)
        .collectAsStateWithLifecycle(initialValue = emptyList())

    var listing by remember { mutableStateOf(ProposalListing.OPEN) }
    var owner by remember { mutableStateOf<IdentityEntity?>(null) }
    var proposals by remember { mutableStateOf<List<DecisionProposal>>(emptyList()) }
    var hasMore by remember { mutableStateOf(false) }
    var isLoading by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var refreshTick by remember { mutableStateOf(0) }
    // Bumped on every fresh load so a late "load more" page of a previous
    // listing is dropped instead of mixed in.
    var loadGeneration by remember { mutableStateOf(0) }

    // Start "Mine" on the first local identity rather than empty.
    LaunchedEffect(listing, identities) {
        if (listing == ProposalListing.MINE && owner == null) owner = identities.firstOrNull()
    }

    val ownerBase58 = owner?.let { Base58.encode(it.identityId) }

    LaunchedEffect(sdk, listing, ownerBase58, refreshTick) {
        loadGeneration++
        proposals = emptyList()
        hasMore = false
        errorMessage = null
        val currentSdk = sdk ?: run {
            errorMessage = "SDK not initialized"
            return@LaunchedEffect
        }
        if (listing == ProposalListing.MINE && ownerBase58 == null) return@LaunchedEffect
        isLoading = true
        try {
            val page = loadProposalPage(currentSdk, listing, ownerBase58, startAfter = null)
            proposals = page
            hasMore = page.size == PROPOSAL_PAGE_SIZE
        } catch (e: Exception) {
            errorMessage = "Loading failed: ${e.message}"
        } finally {
            isLoading = false
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Decision Proposals") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = { refreshTick++ },
                        enabled = !isLoading,
                        modifier = Modifier.testTag("decisionProposals.refresh"),
                    ) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                    IconButton(
                        onClick = { navController.navigate(CreateDecisionProposal) },
                        enabled = sdk != null,
                        modifier = Modifier.testTag("decisionProposals.create"),
                    ) {
                        Icon(Icons.Default.Add, contentDescription = "New decision proposal")
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
            FormSection(title = "Listing") {
                AccessiblePicker(
                    label = "Listing",
                    options = ProposalListing.entries,
                    selected = listing,
                    optionLabel = { it.label },
                    testTag = "decisionProposals.listing",
                    onSelected = { listing = it },
                )
                if (listing == ProposalListing.MINE) {
                    if (identities.isEmpty()) {
                        Text(
                            "No local identity on this network.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    } else {
                        AccessiblePicker(
                            label = "Identity",
                            options = identities,
                            selected = owner ?: identities.first(),
                            optionLabel = { it.displayLabel() },
                            testTag = "decisionProposals.ownerPicker",
                            onSelected = { owner = it },
                        )
                    }
                }
                Text(
                    listing.footer,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            FormSection(title = "Proposals") {
                errorMessage?.let { message ->
                    Text(
                        message,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("decisionProposals.error"),
                    )
                }
                when {
                    isLoading && proposals.isEmpty() -> CircularProgressIndicator(
                        modifier = Modifier.padding(8.dp),
                    )
                    listing == ProposalListing.MINE && ownerBase58 == null -> Text(
                        "Select an identity to list its proposals.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    proposals.isEmpty() && errorMessage == null -> Text(
                        "No proposals",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                proposals.forEach { proposal ->
                    DecisionProposalRow(proposal) {
                        navController.navigate(DecisionProposalDetail(proposal.id))
                    }
                }
                if (hasMore) {
                    OutlinedButton(
                        onClick = {
                            val currentSdk = sdk ?: return@OutlinedButton
                            val last = proposals.lastOrNull() ?: return@OutlinedButton
                            val generation = loadGeneration
                            isLoading = true
                            scope.launch {
                                try {
                                    val page = loadProposalPage(
                                        currentSdk,
                                        listing,
                                        ownerBase58,
                                        startAfter = last.id,
                                    )
                                    if (generation == loadGeneration) {
                                        proposals = proposals + page
                                        hasMore = page.size == PROPOSAL_PAGE_SIZE
                                    }
                                } catch (e: Exception) {
                                    if (generation == loadGeneration) {
                                        errorMessage = "Loading failed: ${e.message}"
                                    }
                                } finally {
                                    isLoading = false
                                }
                            }
                        },
                        enabled = !isLoading,
                        modifier = Modifier.fillMaxWidth().testTag("decisionProposals.loadMore"),
                    ) {
                        Text("Load More")
                    }
                }
            }
        }
    }
}

/** One page of [listing] after [startAfter] (null for the first page). */
private suspend fun loadProposalPage(
    sdk: Sdk,
    listing: ProposalListing,
    ownerBase58: String?,
    startAfter: String?,
): List<DecisionProposal> = when (listing) {
    ProposalListing.OPEN -> sdk.decisionProposals.listByEndDate(
        endDateFromMs = System.currentTimeMillis(),
        limit = PROPOSAL_PAGE_SIZE,
        startAfter = startAfter,
    )
    ProposalListing.MINE -> sdk.decisionProposals.listByOwner(
        ownerId = requireNotNull(ownerBase58) { "no identity selected" },
        limit = PROPOSAL_PAGE_SIZE,
        startAfter = startAfter,
    )
    ProposalListing.NEWEST -> sdk.decisionProposals.listByCreatedAt(
        limit = PROPOSAL_PAGE_SIZE,
        startAfter = startAfter,
    )
}

/** One proposal in a listing (← `DecisionProposalRow`). */
@Composable
private fun DecisionProposalRow(proposal: DecisionProposal, onClick: () -> Unit) {
    ListItem(
        headlineContent = { Text(proposal.title, maxLines = 2) },
        supportingContent = {
            Column {
                Text(
                    "Ends ${formatDate(Date(proposal.endDateMs))} · passes above " +
                        "${proposal.passPercent}% · turnout ${proposal.minimumTurnoutPercent}%",
                    style = MaterialTheme.typography.bodySmall,
                )
                Text(
                    truncateMiddle(proposal.id, 12, 8),
                    style = MaterialTheme.typography.labelSmall
                        .copy(fontFamily = FontFamily.Monospace),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        },
        modifier = Modifier
            .clickable(onClick = onClick)
            .testTag("decisionProposals.row.${proposal.id}"),
    )
}

/**
 * Files a new decision proposal for an identity held by a loaded wallet: port of
 * `CreateDecisionProposalView` (`DecisionProposalFormViews.swift`).
 *
 * Submits through `PlatformWalletManager.decisionProposals.create`, which has rs-sdk build
 * the proposal and the platform wallet put it; the create pays the decision proposal fund
 * from the proposer's credits. Nothing in the form is checked here: Platform refuses a rule
 * outside its limits.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CreateDecisionProposalScreen(navController: NavHostController) {
    val appState = LocalAppState.current
    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    // Identities on the active network held by a loaded wallet, so the
    // manager's signer can sign for them.
    val identities by container.database.identityDao()
        .observeWalletOwnedByNetwork(network.ffiValue)
        .collectAsStateWithLifecycle(initialValue = emptyList())

    var owner by remember { mutableStateOf<IdentityEntity?>(null) }
    LaunchedEffect(identities) {
        if (owner == null) owner = identities.firstOrNull()
    }
    var title by remember { mutableStateOf("") }
    var description by remember { mutableStateOf("") }
    var url by remember { mutableStateOf("") }
    var passPercent by remember { mutableStateOf("60") }
    var turnoutPercent by remember { mutableStateOf("10") }
    var endsInDays by remember { mutableStateOf("14") }

    var isSubmitting by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var createdProposalId by remember { mutableStateOf<String?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("New Proposal") },
                navigationIcon = {
                    IconButton(
                        onClick = { navController.popBackStack() },
                        enabled = !isSubmitting,
                    ) {
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
            val createdId = createdProposalId
            if (createdId != null) {
                FormSection(title = "Proposal Created") {
                    Text(
                        "Broadcast confirmed on-chain.",
                        color = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.testTag("createDecisionProposal.success"),
                    )
                    LabeledContent("ID", truncateMiddle(createdId, 12, 8))
                }
                SubmitButton(
                    text = "Done",
                    isLoading = false,
                    modifier = Modifier.fillMaxWidth().testTag("createDecisionProposal.doneButton"),
                ) { navController.popBackStack() }
                return@Column
            }

            FormSection(title = "Proposer") {
                if (identities.isEmpty()) {
                    Text(
                        "No identity held by a loaded wallet on this network.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    AccessiblePicker(
                        label = "Proposer",
                        options = identities,
                        selected = owner ?: identities.first(),
                        optionLabel = { it.displayLabel() },
                        testTag = "createDecisionProposal.ownerPicker",
                        enabled = !isSubmitting,
                        onSelected = { owner = it },
                    )
                }
                Text(
                    "The create pays the decision proposal fund (0.5 Dash) from this " +
                        "identity's credits, on top of the fees.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            FormSection(title = "Question") {
                OutlinedTextField(
                    value = title,
                    onValueChange = { title = it },
                    label = { Text("Title: the question, in one line") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    modifier = Modifier.fillMaxWidth().testTag("createDecisionProposal.title"),
                )
                OutlinedTextField(
                    value = description,
                    onValueChange = { description = it },
                    label = { Text("Description: what a yes and a no mean") },
                    minLines = 3,
                    enabled = !isSubmitting,
                    modifier = Modifier.fillMaxWidth().testTag("createDecisionProposal.description"),
                )
                OutlinedTextField(
                    value = url,
                    onValueChange = { url = it },
                    label = { Text("URL where it is discussed (optional)") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
                    modifier = Modifier.fillMaxWidth().testTag("createDecisionProposal.url"),
                )
                Text(
                    "The title, description and url can be edited for a day after the " +
                        "proposal is created.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            FormSection(title = "Rule") {
                OutlinedTextField(
                    value = passPercent,
                    onValueChange = { passPercent = it },
                    label = { Text("Passes when yes is more than (% of yes plus no)") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier.fillMaxWidth().testTag("createDecisionProposal.passPercent"),
                )
                OutlinedTextField(
                    value = turnoutPercent,
                    onValueChange = { turnoutPercent = it },
                    label = { Text("Minimum turnout (% of total voting power)") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier.fillMaxWidth().testTag("createDecisionProposal.turnoutPercent"),
                )
                OutlinedTextField(
                    value = endsInDays,
                    onValueChange = { endsInDays = it },
                    label = { Text("Voting ends in (days from now)") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier.fillMaxWidth().testTag("createDecisionProposal.endDate"),
                )
                Text(
                    "Yes must be more than the pass percent of yes plus no, and yes plus " +
                        "no must reach the turnout percent of the masternodes' total voting " +
                        "power. Platform accepts a pass percent of 50 to 80, a turnout of 0 " +
                        "to 50, and an end date one to four weeks after the block that " +
                        "creates the proposal. The rule never changes.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            SubmitButton(
                text = if (isSubmitting) "Broadcasting…" else "Create / Broadcast",
                isLoading = isSubmitting,
                enabled = sdk != null && manager != null && owner != null,
                modifier = Modifier.fillMaxWidth().testTag("createDecisionProposal.submitButton"),
            ) {
                val currentSdk = sdk ?: return@SubmitButton
                val mgr = manager ?: return@SubmitButton
                val proposer = owner ?: return@SubmitButton
                // Form parsing only: the limits on these values are Platform's.
                val pass = passPercent.trim().toIntOrNull()
                val turnout = turnoutPercent.trim().toIntOrNull()
                val days = endsInDays.trim().toLongOrNull()
                if (pass == null || turnout == null || days == null) {
                    error = "The pass percent, turnout and days must be whole numbers."
                    return@SubmitButton
                }
                val wallet = proposer.walletId?.toHex()?.let { mgr.wallets.value[it] }
                if (wallet == null) {
                    error = "The proposer's wallet is not loaded in the wallet manager."
                    return@SubmitButton
                }
                val params = DecisionProposalParams(
                    title = title,
                    description = description,
                    url = url.trim().ifEmpty { null },
                    passPercent = pass,
                    minimumTurnoutPercent = turnout,
                    endDateMs = System.currentTimeMillis() + days * MILLIS_PER_DAY,
                )
                isSubmitting = true
                scope.launch {
                    try {
                        val json = mgr.decisionProposals.create(
                            sdk = currentSdk,
                            walletHandle = wallet.handle,
                            ownerId = proposer.identityId,
                            params = params,
                            signerHandle = mgr.signerHandle,
                        )
                        createdProposalId = parseDocumentId(json) ?: "(id in JSON)"
                    } catch (e: Exception) {
                        error = e.message ?: "Create failed"
                    } finally {
                        isSubmitting = false
                    }
                }
            }
        }
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

private const val MILLIS_PER_DAY = 86_400_000L
