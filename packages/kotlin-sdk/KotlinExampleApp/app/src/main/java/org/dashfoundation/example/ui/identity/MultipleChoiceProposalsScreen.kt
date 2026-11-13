package org.dashfoundation.example.ui.identity

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
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
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import java.util.Date
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.voting.DescriptionDocumentParams
import org.dashfoundation.dashsdk.voting.MultipleChoiceProposal
import org.dashfoundation.dashsdk.voting.MultipleChoiceProposalParams
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.CreateMultipleChoiceProposal
import org.dashfoundation.example.navigation.MultipleChoiceProposalDetail
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.tokens.displayLabel
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.formatCredits
import org.dashfoundation.example.util.formatDate
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.truncateMiddle

/** The two ways the list reads proposals, each one Platform listing (← `Listing`). */
private enum class MultipleChoiceListing(val label: String, val footer: String) {
    MINE("Mine", "Proposals the selected identity filed, in the order it filed them."),
    NEWEST("Newest", "Every multiple choice proposal, newest first."),
}

/** Proposals per page; a page shorter than this is the last. */
private const val MULTIPLE_CHOICE_PAGE_SIZE = 20

/**
 * Multiple choice proposals (protocol version 15): port of
 * `MultipleChoiceProposalsView.swift`, beside the yes/no [DecisionProposalsScreen].
 *
 * Lists proposals two ways, a page at a time, through `Sdk.multipleChoiceProposals`: the
 * ones a local identity filed (`listByOwner`) and the newest (`listByCreatedAt`). Rows drill
 * into [MultipleChoiceProposalDetailScreen]; the + action opens
 * [CreateMultipleChoiceProposalScreen]. Nothing here decides what a proposal is.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MultipleChoiceProposalsScreen(navController: NavHostController) {
    val appState = LocalAppState.current
    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val identities by container.database.identityDao()
        .observeByNetwork(network.ffiValue)
        .collectAsStateWithLifecycle(initialValue = emptyList())

    var listing by remember { mutableStateOf(MultipleChoiceListing.NEWEST) }
    var owner by remember { mutableStateOf<IdentityEntity?>(null) }
    var proposals by remember { mutableStateOf<List<MultipleChoiceProposal>>(emptyList()) }
    var hasMore by remember { mutableStateOf(false) }
    var isLoading by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var refreshTick by remember { mutableStateOf(0) }
    // Bumped on every fresh load so a late "load more" page of a previous
    // listing is dropped instead of mixed in.
    var loadGeneration by remember { mutableStateOf(0) }

    // Start "Mine" on the first local identity rather than empty.
    LaunchedEffect(listing, identities) {
        if (listing == MultipleChoiceListing.MINE && owner == null) owner = identities.firstOrNull()
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
        if (listing == MultipleChoiceListing.MINE && ownerBase58 == null) return@LaunchedEffect
        isLoading = true
        try {
            val page = loadMultipleChoicePage(currentSdk, listing, ownerBase58, startAfter = null)
            proposals = page
            hasMore = page.size == MULTIPLE_CHOICE_PAGE_SIZE
        } catch (e: Exception) {
            errorMessage = "Loading failed: ${e.message}"
        } finally {
            isLoading = false
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Multiple Choice Proposals") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = { refreshTick++ },
                        enabled = !isLoading,
                        modifier = Modifier.testTag("multipleChoiceProposals.refresh"),
                    ) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                    IconButton(
                        onClick = { navController.navigate(CreateMultipleChoiceProposal) },
                        enabled = sdk != null,
                        modifier = Modifier.testTag("multipleChoiceProposals.create"),
                    ) {
                        Icon(Icons.Default.Add, contentDescription = "New multiple choice proposal")
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
                    options = MultipleChoiceListing.entries,
                    selected = listing,
                    optionLabel = { it.label },
                    testTag = "multipleChoiceProposals.listing",
                    onSelected = { listing = it },
                )
                if (listing == MultipleChoiceListing.MINE) {
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
                            testTag = "multipleChoiceProposals.ownerPicker",
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
                        modifier = Modifier.testTag("multipleChoiceProposals.error"),
                    )
                }
                when {
                    isLoading && proposals.isEmpty() -> CircularProgressIndicator(
                        modifier = Modifier.padding(8.dp),
                    )
                    listing == MultipleChoiceListing.MINE && ownerBase58 == null -> Text(
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
                    MultipleChoiceProposalRow(proposal) {
                        navController.navigate(MultipleChoiceProposalDetail(proposal.id))
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
                                    val page = loadMultipleChoicePage(
                                        currentSdk,
                                        listing,
                                        ownerBase58,
                                        startAfter = last.id,
                                    )
                                    if (generation == loadGeneration) {
                                        proposals = proposals + page
                                        hasMore = page.size == MULTIPLE_CHOICE_PAGE_SIZE
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
                        modifier = Modifier
                            .fillMaxWidth()
                            .testTag("multipleChoiceProposals.loadMore"),
                    ) {
                        Text("Load More")
                    }
                }
            }
        }
    }
}

/** One page of [listing] after [startAfter] (null for the first page). */
private suspend fun loadMultipleChoicePage(
    sdk: Sdk,
    listing: MultipleChoiceListing,
    ownerBase58: String?,
    startAfter: String?,
): List<MultipleChoiceProposal> = when (listing) {
    MultipleChoiceListing.MINE -> sdk.multipleChoiceProposals.listByOwner(
        ownerId = requireNotNull(ownerBase58) { "no identity selected" },
        limit = MULTIPLE_CHOICE_PAGE_SIZE,
        startAfter = startAfter,
    )
    MultipleChoiceListing.NEWEST -> sdk.multipleChoiceProposals.listByCreatedAt(
        limit = MULTIPLE_CHOICE_PAGE_SIZE,
        startAfter = startAfter,
    )
}

/** One proposal in a listing (← `MultipleChoiceProposalRow`). */
@Composable
private fun MultipleChoiceProposalRow(proposal: MultipleChoiceProposal, onClick: () -> Unit) {
    ListItem(
        headlineContent = { Text(proposal.title, maxLines = 2) },
        supportingContent = {
            Column {
                Text(
                    listOfNotNull(
                        proposal.createdAtMs?.let { "Created ${formatDate(Date(it))}" },
                        proposal.whoEntersChoices(),
                    ).joinToString(" · "),
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
            .testTag("multipleChoiceProposals.row.${proposal.id}"),
    )
}

/** Who may enter choices for this proposal, as the screens say it. */
internal fun MultipleChoiceProposal.whoEntersChoices(): String =
    if (open) "anyone enters choices" else "only its owner enters choices"

/**
 * A paid write of the multiple choice proposal screens that creates a document (the
 * proposal on [CreateMultipleChoiceProposalScreen], a choice on
 * [MultipleChoiceProposalDetailScreen]), held by the screen's back-stack entry rather than
 * by its composition, as [CreateDecisionProposalViewModel] holds the yes/no create.
 *
 * Creating a proposal pays the processing fee and entering a choice pays the contest fund.
 * A coroutine of the composition is cancelled whenever the screen leaves composition (a
 * rotation, a switch to another bottom tab), which would drop a write Platform already
 * accepted and invite a second, paid retry. The back-stack entry outlives all of those, so
 * the write runs to its end and the screen shows the outcome when it composes again. Back,
 * which would clear the entry, is held by the screens while the write runs.
 */
class MultipleChoiceDocumentCreateViewModel : ViewModel() {

    /** Whether a write is running. */
    var isSubmitting by mutableStateOf(false)
        private set

    /** The message of the last failure, form errors included, until it is dismissed. */
    var error by mutableStateOf<String?>(null)

    /** The created document's id, base58, once Platform confirmed the write. */
    var createdId by mutableStateOf<String?>(null)
        private set

    /**
     * Run [create], which returns the created document's id, unless a write is running or
     * one succeeded and was not [acknowledge]d: the form is never submitted twice.
     */
    fun submit(create: suspend () -> String) {
        if (isSubmitting || createdId != null) return
        isSubmitting = true
        error = null
        viewModelScope.launch {
            try {
                createdId = create()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                error = e.message ?: "Broadcast failed"
            } finally {
                isSubmitting = false
            }
        }
    }

    /**
     * Forget a confirmed write once the screen showed it, so that another document (a
     * further choice) can be created. Does nothing while a write runs.
     */
    fun acknowledge() {
        if (!isSubmitting) createdId = null
    }
}

/**
 * Files a new multiple choice proposal for an identity held by a loaded wallet: port of
 * `CreateMultipleChoiceProposalView` (`MultipleChoiceProposalFormViews.swift`), beside the
 * yes/no [CreateDecisionProposalScreen].
 *
 * Submits through `PlatformWalletManager.multipleChoiceProposals.create`, one platform
 * wallet call that builds the proposal and waits for Platform to confirm; Platform charges
 * the proposer the processing fee, which the screen reads through
 * `Sdk.multipleChoiceProposals.processingFee` and shows before the submit. The submission
 * lives in [MultipleChoiceDocumentCreateViewModel] and Back is held while it runs, so
 * leaving the screen never cancels a paid create. Nothing in the form is checked here:
 * Platform refuses a proposal outside its limits.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CreateMultipleChoiceProposalScreen(
    navController: NavHostController,
    submission: MultipleChoiceDocumentCreateViewModel = viewModel(),
) {
    val appState = LocalAppState.current
    val container = LocalAppContainer.current
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
    var title by rememberSaveable { mutableStateOf("") }
    var url by rememberSaveable { mutableStateOf("") }
    var describedId by rememberSaveable { mutableStateOf("") }
    var describedContractId by rememberSaveable { mutableStateOf("") }
    var describedType by rememberSaveable { mutableStateOf("") }
    var open by rememberSaveable { mutableStateOf(true) }

    // The processing fee as Platform charges it on this network, read rather than assumed.
    var processingFee by remember { mutableStateOf<ULong?>(null) }
    var processingFeeError by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(sdk) {
        processingFee = null
        processingFeeError = null
        val currentSdk = sdk ?: return@LaunchedEffect
        try {
            processingFee = currentSdk.multipleChoiceProposals.processingFee()
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            processingFeeError = "Reading the processing fee failed: ${e.message}"
        }
    }

    val isSubmitting = submission.isSubmitting
    // System Back would pop the entry and clear the submission with it; the toolbar Back
    // is disabled for the same reason.
    BackHandler(enabled = isSubmitting) {}

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("New Multiple Choice Proposal") },
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
            val createdId = submission.createdId
            if (createdId != null) {
                FormSection(title = "Proposal Created") {
                    Text(
                        "Broadcast confirmed on-chain.",
                        color = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.testTag("createMultipleChoiceProposal.success"),
                    )
                    LabeledContent("ID", truncateMiddle(createdId, 12, 8))
                }
                SubmitButton(
                    text = "Done",
                    isLoading = false,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("createMultipleChoiceProposal.doneButton"),
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
                        testTag = "createMultipleChoiceProposal.ownerPicker",
                        enabled = !isSubmitting,
                        onSelected = { owner = it },
                    )
                }
                LabeledContent(
                    "Processing fee",
                    processingFee?.let { formatCredits(it.toLong()) }
                        ?: if (processingFeeError == null) "Loading…" else "Unavailable",
                    modifier = Modifier.testTag("createMultipleChoiceProposal.processingFee"),
                )
                processingFeeError?.let { message ->
                    Text(
                        message,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("createMultipleChoiceProposal.processingFeeError"),
                    )
                }
                Text(
                    "Platform charges this identity the processing fee for the create, on " +
                        "top of the fees.",
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
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("createMultipleChoiceProposal.title"),
                )
                OutlinedTextField(
                    value = url,
                    onValueChange = { url = it },
                    label = { Text("URL where it is described or discussed") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("createMultipleChoiceProposal.url"),
                )
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text("Anyone may enter choices", modifier = Modifier.weight(1f))
                    Switch(
                        checked = open,
                        onCheckedChange = { open = it },
                        enabled = !isSubmitting,
                        modifier = Modifier.testTag("createMultipleChoiceProposal.open"),
                    )
                }
                Text(
                    "The answers are the choices entered once the proposal exists: by " +
                        "anyone when it is open, by the proposer alone when it is not, " +
                        "which never changes. The title, url and description document can " +
                        "be edited for a short time after the proposal is created.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            FormSection(title = "Description Document (optional)") {
                OutlinedTextField(
                    value = describedId,
                    onValueChange = { describedId = it },
                    label = { Text("Document ID (base58)") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("createMultipleChoiceProposal.descriptionDocumentId"),
                )
                OutlinedTextField(
                    value = describedContractId,
                    onValueChange = { describedContractId = it },
                    label = { Text("Contract ID (base58)") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("createMultipleChoiceProposal.descriptionDocumentContractId"),
                )
                OutlinedTextField(
                    value = describedType,
                    onValueChange = { describedType = it },
                    label = { Text("Document type") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("createMultipleChoiceProposal.descriptionDocumentType"),
                )
                Text(
                    "The document saying what is being decided. Platform takes a proposal " +
                        "with a description document, a url or both, and refuses a " +
                        "document that is not of a moderated type.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            SubmitButton(
                text = if (isSubmitting) "Broadcasting…" else "Create / Broadcast",
                isLoading = isSubmitting,
                enabled = manager != null && owner != null,
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("createMultipleChoiceProposal.submitButton"),
            ) {
                val mgr = manager ?: return@SubmitButton
                val proposer = owner ?: return@SubmitButton
                val wallet = proposer.walletId?.toHex()?.let { mgr.wallets.value[it] }
                if (wallet == null) {
                    submission.error = "The proposer's wallet is not loaded in the wallet manager."
                    return@SubmitButton
                }
                // Form parsing only: what a proposal must hold is Platform's to say.
                val described = when (
                    val parsed = parseDescriptionDocument(describedId, describedContractId, describedType)
                ) {
                    is ParsedDescriptionDocument.Unreadable -> {
                        submission.error = parsed.message
                        return@SubmitButton
                    }
                    ParsedDescriptionDocument.None -> null
                    is ParsedDescriptionDocument.Document -> parsed.document
                }
                val params = MultipleChoiceProposalParams(
                    title = title,
                    descriptionDocument = described,
                    url = url.trim().ifEmpty { null },
                    open = open,
                )
                submission.submit {
                    mgr.multipleChoiceProposals.create(
                        walletHandle = wallet.handle,
                        ownerId = proposer.identityId,
                        params = params,
                        signerHandle = mgr.signerHandle,
                    ).id
                }
            }
        }
    }

    ErrorAlertDialog(message = submission.error, onDismiss = { submission.error = null })
}

/** What the three description document fields of a form hold. */
internal sealed interface ParsedDescriptionDocument {
    /** All three fields are empty: no description document. */
    data object None : ParsedDescriptionDocument

    /** The fields name a document. */
    data class Document(val document: DescriptionDocumentParams) : ParsedDescriptionDocument

    /** An id field does not hold the 32 bytes the wallet call takes. */
    data class Unreadable(val message: String) : ParsedDescriptionDocument
}

/**
 * Read a form's description document fields into the bytes the wallet call takes. Only the
 * text-to-bytes step can fail here; whether the document may describe a proposal is
 * Platform's to say.
 */
internal fun parseDescriptionDocument(
    id: String,
    contractId: String,
    documentType: String,
): ParsedDescriptionDocument {
    if (id.isBlank() && contractId.isBlank() && documentType.isBlank()) {
        return ParsedDescriptionDocument.None
    }
    val idBytes = Base58.decodeIdentifier(id)
        ?: return ParsedDescriptionDocument.Unreadable(
            "The description document id is not a 32-byte base58 identifier.",
        )
    val contractIdBytes = Base58.decodeIdentifier(contractId)
        ?: return ParsedDescriptionDocument.Unreadable(
            "The description document's contract id is not a 32-byte base58 identifier.",
        )
    return ParsedDescriptionDocument.Document(
        DescriptionDocumentParams(idBytes, contractIdBytes, documentType.trim()),
    )
}
