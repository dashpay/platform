package org.dashfoundation.example.ui.contracts

import android.content.ClipboardManager
import android.content.Context
import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonPrimitive
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.identity.DataContractUpdatePreview
import org.dashfoundation.dashsdk.identity.DataContractUpdateSource
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.truncateMiddle

/**
 * The six editor strings of the Update Contract screen (← `UpdateContractImport.Fields`).
 * Equality compares the strings exactly (Kotlin `String` equality is by UTF-16 code units,
 * so two normalization forms of the same text differ, as the bytes the wallet receives do).
 */
data class UpdateContractFields(
    val documents: String = "{}",
    val tokens: String = "",
    val groups: String = "",
    val keywords: String = "",
    val description: String = "",
    val config: String = "",
) {
    /**
     * The fields in the form the wallet calls take (← `UpdateContractImport.WalletArguments`).
     * Converts text formats only; what the contents mean is the wallet's and Platform's.
     */
    fun walletArguments(): UpdateContractArguments = UpdateContractArguments(
        documents = documents.trim().ifEmpty { "{}" },
        tokens = tokens.trim().ifEmpty { null },
        groups = groups.trim().ifEmpty { null },
        keywords = keywordsJson(keywords),
        // Verbatim: only an empty field keeps the live description
        description = description.ifEmpty { null },
        config = config.trim().ifEmpty { null },
    )

    companion object {
        /**
         * The keywords field as the wallet's JSON array, or null to keep the live keywords. A
         * JSON array (text starting with `[` once leading whitespace is skipped) is passed
         * through as typed; anything else is the comma separated form.
         */
        fun keywordsJson(field: String): String? {
            if (field.trimStart().startsWith("[")) return field
            val keywords = field.split(',').map { it.trim() }.filter { it.isNotEmpty() }
            if (keywords.isEmpty()) return null
            return JsonArray(keywords.map(::JsonPrimitive)).toString()
        }

        /** The fields a split source fills; a section it does not carry is reset. */
        fun from(source: DataContractUpdateSource): UpdateContractFields {
            val empty = UpdateContractFields()
            return UpdateContractFields(
                documents = source.documents ?: empty.documents,
                tokens = source.tokens ?: empty.tokens,
                groups = source.groups ?: empty.groups,
                keywords = source.keywords ?: empty.keywords,
                description = source.description ?: empty.description,
                config = source.config ?: empty.config,
            )
        }

        /**
         * The most bytes the screen reads from one source before splitting it: a UI input
         * budget that keeps a huge file from being read whole, not a Platform limit.
         */
        const val MAX_INPUT_BYTES = 4 * 1024 * 1024
    }
}

/**
 * The contract an update targets and the identity that owns it, as hex: what a review was
 * made for. A review stands only while the screen still targets the same pair.
 */
data class UpdateContractTarget(val contractIdHex: String, val ownerIdHex: String)

/** The wallet's update arguments for one set of [UpdateContractFields]. */
data class UpdateContractArguments(
    val documents: String,
    val tokens: String?,
    val groups: String?,
    val keywords: String?,
    val description: String?,
    val config: String?,
)

/**
 * Whether Update Contract may be tapped (← `UpdateContractView.canSubmit`): only for a review
 * of the fields and the contract and owner on screen that Rust says changes something, with
 * the owner's acknowledgment
 * when Rust says the configuration changes, and never after an update whose outcome is
 * unknown or while a sent update's refresh is pending.
 */
fun canSubmitUpdate(
    review: UpdateContractViewModel.Review?,
    fields: UpdateContractFields,
    target: UpdateContractTarget?,
    configChangeAcknowledged: Boolean,
    unconfirmedUpdateSent: Boolean,
    refreshPending: Boolean,
): Boolean {
    if (review == null || unconfirmedUpdateSent || refreshPending) return false
    return review.fields == fields &&
        review.target == target &&
        review.preview.hasChanges &&
        (!review.preview.configChanged || configChangeAcknowledged)
}

/**
 * The review and broadcast of [UpdateContractScreen], held by the screen's back-stack entry
 * rather than by its composition, so a rotation or a tab switch cannot cancel a paid update
 * Platform already accepted. Back is held while it runs.
 */
class UpdateContractViewModel : ViewModel() {

    /** A review on screen and the fields it was made with; the update sends exactly these. */
    data class Review(
        val fields: UpdateContractFields,
        val target: UpdateContractTarget,
        val arguments: UpdateContractArguments,
        val preview: DataContractUpdatePreview,
    )

    /** An alert the screen shows; [dismissesScreen] goes back to the contract on OK. */
    data class Outcome(
        val title: String,
        val message: String,
        val dismissesScreen: Boolean,
        val testTag: String,
    )

    var review by mutableStateOf<Review?>(null)
        private set

    /** Whether the owner confirmed the configuration change of the review on screen. */
    var configChangeAcknowledged by mutableStateOf(false)

    var isReviewing by mutableStateOf(false)
        private set
    var isSubmitting by mutableStateOf(false)
        private set
    var isRefreshing by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
    var outcome by mutableStateOf<Outcome?>(null)

    /** Set once an update was sent whose outcome could not be confirmed; holds Update Contract off. */
    var unconfirmedUpdateSent by mutableStateOf(false)
        private set

    /** The lowest version a pending refresh accepts, after a sent update's refresh failed. */
    var pendingRefreshFloor by mutableStateOf<Long?>(null)
        private set
    var refreshFailure by mutableStateOf<String?>(null)
        private set

    val isBusy: Boolean get() = isReviewing || isSubmitting || isRefreshing

    /** Replace or remove the review; the acknowledgment, given for one review, goes with it. */
    fun replaceReview(newReview: Review?) {
        review = newReview
        configChangeAcknowledged = false
    }

    /**
     * Ask the wallet what an update with [fields] would change ([previewUpdate] is one
     * `platform_wallet_preview_data_contract_update` call). [currentFields] reads the fields
     * when the review returns: an edit made while it ran is not what was reviewed.
     */
    fun review(
        fields: UpdateContractFields,
        target: UpdateContractTarget,
        current: () -> Pair<UpdateContractFields, UpdateContractTarget?>,
        previewUpdate: suspend (UpdateContractArguments, UpdateContractTarget) -> DataContractUpdatePreview,
    ) {
        if (isBusy) return
        val arguments = fields.walletArguments()
        // A failed review must not leave an older review, or its acknowledgment, standing
        replaceReview(null)
        isReviewing = true
        viewModelScope.launch {
            try {
                val preview = previewUpdate(arguments, target)
                // An edit, or another contract or owner, chosen while the review ran is not
                // what was reviewed
                if (current() == fields to target) replaceReview(Review(fields, target, arguments, preview))
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                error = describe(e)
            } finally {
                isReviewing = false
            }
        }
    }

    /**
     * Broadcast the reviewed update, bound to the reviewed contract version, then refresh the
     * stored contract. [update] is one `platform_wallet_update_data_contract_with_signer`
     * call; [refresh] re-reads the stored contract, refusing a version below its floor, and
     * returns why it failed, or null.
     */
    fun submit(
        fields: UpdateContractFields,
        target: UpdateContractTarget?,
        update: suspend (UpdateContractArguments, Long, UpdateContractTarget) -> ByteArray,
        refresh: suspend (Long) -> String?,
    ) {
        val review = review ?: return
        if (isBusy ||
            !canSubmitUpdate(
                review,
                fields,
                target,
                configChangeAcknowledged,
                unconfirmedUpdateSent,
                pendingRefreshFloor != null,
            )
        ) {
            return
        }
        isSubmitting = true
        viewModelScope.launch {
            try {
                // Sent to the reviewed contract and owner, with the reviewed arguments
                val contractId = update(review.arguments, review.preview.currentVersion, review.target)
                // Platform holds at least the reviewed new version now, so an older copy from
                // a lagging node is not stored
                val newVersion = review.preview.newVersion
                val failure = refresh(newVersion)
                var message = "Contract id:\n${Base58.encode(contractId)}\n\nVersion: $newVersion"
                if (failure != null) {
                    replaceReview(null)
                    pendingRefreshFloor = newVersion
                    refreshFailure = failure
                    message += "\n\nThe contract was updated, but the local copy could not be " +
                        "refreshed: $failure\n\nUse Refresh Stored Contract to try again."
                }
                outcome = Outcome("Contract Updated", message, failure == null, "updateContract.success")
            } catch (e: CancellationException) {
                throw e
            } catch (e: DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed) {
                // The update may have executed, or may still. It is not retried, and no other
                // update is offered from this screen. The review proved the contract was at
                // least at its current version, so that is the refresh's floor
                replaceReview(null)
                unconfirmedUpdateSent = true
                val currentVersion = review.preview.currentVersion
                val failure = refresh(currentVersion)
                var message = "The update was sent, but its outcome is unknown: it may have " +
                    "executed, or may still. It was not retried, and this screen will not send " +
                    "another. Later, reopen it and review again: the review reads the live " +
                    "version. An unchanged version does not prove the first update failed, and " +
                    "a second update sent while the first is pending can be refused for a fee." +
                    "\n\n${e.message.orEmpty()}"
                if (failure != null) {
                    pendingRefreshFloor = currentVersion
                    refreshFailure = failure
                    message += "\n\nThe local copy could not be refreshed: $failure"
                }
                outcome = Outcome("Update Unconfirmed", message, false, "updateContract.unconfirmed")
            } catch (e: Exception) {
                // A failed submit may leave the reviewed contract behind, so the review is
                // cleared and Update Contract stays off until the changes are reviewed again
                replaceReview(null)
                error = describe(e)
            } finally {
                isSubmitting = false
            }
        }
    }

    /** Retry only the refresh after a sent update whose refresh failed; never signs or broadcasts. */
    fun retryRefresh(refresh: suspend (Long) -> String?) {
        val floor = pendingRefreshFloor ?: return
        if (isBusy) return
        isRefreshing = true
        viewModelScope.launch {
            try {
                val failure = refresh(floor)
                if (failure != null) {
                    refreshFailure = failure
                } else {
                    pendingRefreshFloor = null
                    refreshFailure = null
                    outcome = Outcome(
                        "Contract Refreshed",
                        "The stored contract is up to date.",
                        true,
                        "updateContract.refreshed",
                    )
                }
            } finally {
                isRefreshing = false
            }
        }
    }

    companion object {
        /** Rust's message, and the consensus code when Platform refused the transition. */
        fun describe(error: Throwable): String {
            val message = error.message ?: error.toString()
            val consensus = (error as? DashSdkError)?.consensusError
            return if (consensus != null) "$message\n\nConsensus error code: ${consensus.code}" else message
        }
    }
}

/**
 * Update a data contract a local identity owns (DC-04): port of `UpdateContractView.swift`,
 * reached from the contract's detail screen (`contractDetails.updateContract`) and from the
 * Settings builder.
 *
 * The six inputs are the wallet's update arguments; the wallet fetches the live contract and
 * merges them onto it. "Review Changes" asks the wallet what the update would change
 * (`DataContracts.previewUpdate`, one Rust call that also checks the owner and builds the
 * contract as the update does); "Update Contract" is offered only when that review reports a
 * change, and is sent with exactly the reviewed arguments, bound to the reviewed version.
 * The review's content and any refusal are Rust's; this screen renders them and checks no
 * contract update rule. Paste from Clipboard and Import from File fill the fields through
 * Rust's split (`DataContractUpdateSource.split`), section text as the source spells it.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun UpdateContractScreen(
    prefillContractIdHex: String,
    navController: NavHostController,
    state: UpdateContractViewModel = viewModel(),
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val context = LocalContext.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val identities by container.database.identityDao()
        .observeWalletOwnedByNetwork(network.ffiValue)
        .collectAsStateWithLifecycle(initialValue = emptyList())

    var contractIdText by rememberSaveable(prefillContractIdHex) {
        mutableStateOf(
            prefillContractIdHex.takeIf { it.isNotEmpty() }
                ?.let { runCatching { Base58.encode(it.hexToBytesOrEmpty()) }.getOrNull() }
                .orEmpty(),
        )
    }
    val contractIdBytes = remember(contractIdText) { Base58.decodeIdentifier(contractIdText.trim()) }
    val storedContractFlow = remember(contractIdBytes?.toHex()) {
        contractIdBytes?.let { container.database.dataContractDao().observeById(it) }
    }
    val storedContract by (storedContractFlow ?: kotlinx.coroutines.flow.flowOf(null))
        .collectAsStateWithLifecycle(initialValue = null)

    // The owner: the wallet-owned identity the stored contract names, else the one picked
    var pickedOwner by remember { mutableStateOf<IdentityEntity?>(null) }
    val contractOwner = storedContract?.ownerId?.let { ownerId ->
        identities.firstOrNull { it.identityId.contentEquals(ownerId) }
    }
    LaunchedEffect(identities) { if (pickedOwner == null) pickedOwner = identities.firstOrNull() }
    val owner = contractOwner ?: pickedOwner

    var documents by rememberSaveable { mutableStateOf("{}") }
    var tokens by rememberSaveable { mutableStateOf("") }
    var groups by rememberSaveable { mutableStateOf("") }
    var keywords by rememberSaveable { mutableStateOf("") }
    var descriptionText by rememberSaveable { mutableStateOf("") }
    var config by rememberSaveable { mutableStateOf("") }
    var sourceMessage by rememberSaveable { mutableStateOf<String?>(null) }
    var fieldsDropSharedDefinitions by rememberSaveable { mutableStateOf(false) }

    val fields = UpdateContractFields(documents, tokens, groups, keywords, descriptionText, config)
    // Any change of the inputs clears the review on screen
    val target = if (contractIdBytes != null && owner != null) {
        UpdateContractTarget(contractIdBytes.toHex(), owner.identityId.toHex())
    } else {
        null
    }
    // The target as composed last, for a review returning later to compare against
    val latestTarget by rememberUpdatedState(target)
    val currentTarget = { latestTarget }
    // Any change of the inputs, the contract or the owner clears the review on screen
    LaunchedEffect(fields, target) {
        val review = state.review
        if (review != null && (review.fields != fields || review.target != target)) state.replaceReview(null)
    }

    val wallet = owner?.walletId?.toHex()?.let { manager?.wallets?.value?.get(it) }
    val busy = state.isBusy

    fun fill(text: String, source: String) {
        if (text.toByteArray(Charsets.UTF_8).size > UpdateContractFields.MAX_INPUT_BYTES) {
            sourceMessage = "Nothing filled from $source: the input is larger than this " +
                "screen's import limit of ${UpdateContractFields.MAX_INPUT_BYTES / (1024 * 1024)} MiB."
            return
        }
        if (text.isBlank()) {
            sourceMessage = "The $source holds no contract JSON."
            return
        }
        val split = try {
            DataContractUpdateSource.split(text)
        } catch (e: Exception) {
            sourceMessage = "Nothing filled from $source: ${UpdateContractViewModel.describe(e)}"
            return
        }
        val filled = UpdateContractFields.from(split)
        documents = filled.documents
        tokens = filled.tokens
        groups = filled.groups
        keywords = filled.keywords
        descriptionText = filled.description
        config = filled.config
        fieldsDropSharedDefinitions = split.carriesSharedDefinitions
        sourceMessage = "Filled from $source." + if (split.carriesSharedDefinitions) {
            " This file has shared definitions (\$defs). This screen does not update them: " +
                "Platform keeps the live ones."
        } else {
            ""
        }
    }

    val importLauncher = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) {
            val name = uri.lastPathSegment ?: "the file"
            val text = readImportedText(context, uri)
            text.fold(onSuccess = { fill(it, name) }, onFailure = {
                sourceMessage = "Could not read $name: ${it.message}"
            })
        }
    }

    BackHandler(enabled = busy) { }

    val refresh: suspend (Long) -> String? = refresh@{ floor ->
        val sdk = appState.sdk.value ?: return@refresh "the SDK is not initialized"
        val id = contractIdBytes ?: return@refresh "no contract id"
        try {
            ContractDownloader.refreshPersistedContract(id, sdk, container.database, network, floor)
            null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            UpdateContractViewModel.describe(e)
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Update Contract") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }, enabled = !busy) {
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
            FormSection(title = "Contract") {
                if (prefillContractIdHex.isEmpty()) {
                    OutlinedTextField(
                        value = contractIdText,
                        onValueChange = { contractIdText = it },
                        label = { Text("Data contract ID (base58)") },
                        singleLine = true,
                        isError = contractIdText.isNotBlank() && contractIdBytes == null,
                        enabled = !busy,
                        textStyle = MaterialTheme.typography.bodyMedium.copy(fontFamily = FontFamily.Monospace),
                        modifier = Modifier
                            .fillMaxWidth()
                            .testTag("updateContract.contractIdField"),
                    )
                }
                storedContract?.let { LabeledContent("Name", it.name) }
                LabeledContent("ID", truncateMiddle(contractIdText, 12, 8))
                storedContract?.let {
                    LabeledContent("Stored Version", it.version?.toString() ?: "Unknown")
                }
                if (contractOwner == null) {
                    if (identities.isEmpty()) {
                        Text(
                            "No wallet-owned identity on ${network.networkName}, so this device " +
                                "cannot sign the update.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                    } else {
                        AccessiblePicker(
                            label = "Owner",
                            options = identities,
                            selected = pickedOwner ?: identities.first(),
                            optionLabel = {
                                it.alias ?: it.mainDpnsName ?: truncateMiddle(Base58.encode(it.identityId), 10, 6)
                            },
                            testTag = "updateContract.ownerPicker",
                            onSelected = { pickedOwner = it },
                        )
                        Text(
                            "The contract's owner is not a wallet identity here; the identity " +
                                "picked must own it, which the review checks.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                } else {
                    LabeledContent(
                        "Owner",
                        contractOwner.alias ?: contractOwner.mainDpnsName
                            ?: truncateMiddle(Base58.encode(contractOwner.identityId), 10, 6),
                    )
                }
                if (owner != null && wallet == null) {
                    Text(
                        "The owner identity's wallet is not loaded, so this device cannot sign the update.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
            }

            FormSection(title = "Source") {
                TextButton(
                    onClick = {
                        // Read only on the tap
                        val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                        val text = clipboard.primaryClip?.takeIf { it.itemCount > 0 }
                            ?.getItemAt(0)?.coerceToText(context)?.toString().orEmpty()
                        fill(text, "clipboard")
                    },
                    enabled = !busy,
                    modifier = Modifier.testTag("updateContract.pasteButton"),
                ) { Text("Paste from Clipboard") }
                TextButton(
                    onClick = { importLauncher.launch(arrayOf("application/json", "text/plain")) },
                    enabled = !busy,
                    modifier = Modifier.testTag("updateContract.importButton"),
                ) { Text("Import from File") }
                sourceMessage?.let {
                    Text(
                        it,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.testTag("updateContract.sourceMessage"),
                    )
                }
                Footer(
                    "Fills the fields below from contract JSON. Nothing is sent until you " +
                        "review the changes and tap Update Contract.",
                )
            }

            FormSection(title = "Document Schemas (JSON)") {
                JsonEditor(documents, { documents = it }, !busy, "updateContract.documentsJson")
                if (fieldsDropSharedDefinitions) SharedDefinitionsWarning("updateContract.sharedDefinitionsWarning")
                Footer(
                    "Merged onto the live contract: new document types are added, existing ones " +
                        "replaced key by key, and omitted ones keep their on-chain definition. " +
                        "Use {} to leave document types unchanged.",
                )
            }
            FormSection(title = "Token Schemas (JSON, optional)") {
                JsonEditor(tokens, { tokens = it }, !busy, "updateContract.tokensJson")
                Footer(
                    "Keyed by token position. This update can add new tokens; Platform refuses a " +
                        "change to an existing token. Omitted tokens keep their on-chain definition.",
                )
            }
            FormSection(title = "Groups (JSON, optional)") {
                JsonEditor(groups, { groups = it }, !busy, "updateContract.groupsJson")
                Footer(
                    "Keyed by group position. This update can add new groups; Platform refuses a " +
                        "change to an existing group. Omitted groups keep their on-chain definition.",
                )
            }
            FormSection(title = "Metadata (Optional)") {
                OutlinedTextField(
                    value = keywords,
                    onValueChange = { keywords = it },
                    label = { Text("Keywords (comma separated or a JSON array)") },
                    singleLine = true,
                    enabled = !busy,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("updateContract.keywords"),
                )
                OutlinedTextField(
                    value = descriptionText,
                    onValueChange = { descriptionText = it },
                    label = { Text("Description") },
                    singleLine = true,
                    enabled = !busy,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("updateContract.description"),
                )
                Footer(
                    "Replace the live keywords or description when given; left empty, the " +
                        "on-chain values are kept. Keywords are comma separated, or a JSON array " +
                        "such as [\"alpha,beta\", \"gamma\"] for keywords that contain a comma. " +
                        "The description is sent as typed.",
                )
            }
            FormSection(title = "Contract Configuration (JSON, optional)") {
                JsonEditor(config, { config = it }, !busy, "updateContract.configJson")
                Footer(
                    "Replaces the live configuration when given; left empty, the live " +
                        "configuration is kept. Platform decides which configuration changes it accepts.",
                )
            }

            FormSection {
                SubmitButton(
                    text = "Review Changes",
                    isLoading = state.isReviewing,
                    enabled = !busy && wallet != null && owner != null && contractIdBytes != null,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("updateContract.reviewButton"),
                ) {
                    val reviewWallet = wallet ?: return@SubmitButton
                    val reviewTarget = target ?: return@SubmitButton
                    state.review(
                        fields,
                        reviewTarget,
                        {
                            UpdateContractFields(documents, tokens, groups, keywords, descriptionText, config) to
                                currentTarget()
                        },
                    ) { arguments, reviewed ->
                        reviewWallet.dataContracts.previewUpdate(
                            ownerIdentityId = reviewed.ownerIdHex.hexToBytes(),
                            contractId = reviewed.contractIdHex.hexToBytes(),
                            documentsSchemaJson = arguments.documents,
                            tokensSchemaJson = arguments.tokens,
                            groupsSchemaJson = arguments.groups,
                            keywordsJson = arguments.keywords,
                            description = arguments.description,
                            configJson = arguments.config,
                        )
                    }
                }
                Footer(
                    "The review lists what the update would change. Platform decides whether " +
                        "the update is accepted when it is broadcast.",
                )
            }

            state.review?.let { UpdateContractPreviewSection(it.preview) }

            FormSection {
                if (fieldsDropSharedDefinitions) {
                    SharedDefinitionsWarning("updateContract.submitSharedDefinitionsWarning")
                }
                val review = state.review
                if (review?.preview?.configChanged == true) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            "I have checked the configuration change, including any moderator " +
                                "added or removed",
                            style = MaterialTheme.typography.bodyMedium,
                            modifier = Modifier.weight(1f),
                        )
                        Switch(
                            checked = state.configChangeAcknowledged,
                            onCheckedChange = { state.configChangeAcknowledged = it },
                            enabled = !busy,
                            modifier = Modifier.testTag("updateContract.acknowledgeConfigChange"),
                        )
                    }
                }
                val canSubmit = !busy && wallet != null && canSubmitUpdate(
                    review,
                    fields,
                    target,
                    state.configChangeAcknowledged,
                    state.unconfirmedUpdateSent,
                    state.pendingRefreshFloor != null,
                )
                SubmitButton(
                    text = "Update Contract",
                    isLoading = state.isSubmitting,
                    enabled = canSubmit,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("updateContract.submitButton"),
                ) {
                    val submitWallet = wallet ?: return@SubmitButton
                    val mgr = manager ?: return@SubmitButton
                    state.submit(fields, target, update = { arguments, expectedVersion, reviewed ->
                        submitWallet.dataContracts.update(
                            ownerIdentityId = reviewed.ownerIdHex.hexToBytes(),
                            contractId = reviewed.contractIdHex.hexToBytes(),
                            documentsSchemaJson = arguments.documents,
                            tokensSchemaJson = arguments.tokens,
                            groupsSchemaJson = arguments.groups,
                            keywordsJson = arguments.keywords,
                            description = arguments.description,
                            configJson = arguments.config,
                            // Bind the update to the reviewed contract: if it changed since the
                            // review, the wallet refuses the update
                            expectedCurrentVersion = expectedVersion,
                            signerHandle = mgr.signerHandle,
                        )
                    }, refresh = refresh)
                }
                if (state.unconfirmedUpdateSent) {
                    Text(
                        "An update was sent and its outcome is unknown: it may still land. This " +
                            "screen will not send another. Later, reopen it and review again: the " +
                            "review reads the live version. An unchanged version does not prove " +
                            "the first update failed, and a second update sent while the first is " +
                            "pending can be refused for a fee.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.tertiary,
                        modifier = Modifier.testTag("updateContract.unconfirmedHold"),
                    )
                }
                if (state.pendingRefreshFloor != null) {
                    Column(
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                        modifier = Modifier.testTag("updateContract.refreshRetry"),
                    ) {
                        Text(
                            if (state.unconfirmedUpdateSent) {
                                "The local copy of the contract could not be refreshed."
                            } else {
                                "The update was confirmed, but the local copy of the contract is not updated."
                            },
                            style = MaterialTheme.typography.bodySmall,
                        )
                        state.refreshFailure?.let {
                            Text(
                                it,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.error,
                                modifier = Modifier.testTag("updateContract.refreshRetryFailure"),
                            )
                        }
                        OutlinedButton(
                            onClick = { state.retryRefresh(refresh) },
                            enabled = !busy,
                            modifier = Modifier.testTag("updateContract.refreshRetryButton"),
                        ) { Text("Refresh Stored Contract") }
                    }
                }
                val footer = when {
                    state.unconfirmedUpdateSent || state.pendingRefreshFloor != null -> null
                    review?.preview?.hasChanges == false -> "The update would change nothing, so it is not offered."
                    review == null -> "Review the changes first."
                    review.preview.configChanged && !state.configChangeAcknowledged ->
                        "The update changes the contract configuration. Check the changes listed above, then confirm."
                    else -> null
                }
                footer?.let { Footer(it) }
            }
        }
    }

    state.outcome?.let { outcome ->
        val close = {
            state.outcome = null
            if (outcome.dismissesScreen) navController.popBackStack()
        }
        AlertDialog(
            onDismissRequest = { close() },
            title = { Text(outcome.title) },
            text = {
                SelectionContainer {
                    Text(outcome.message, modifier = Modifier.testTag(outcome.testTag))
                }
            },
            confirmButton = { TextButton(onClick = { close() }) { Text("OK") } },
        )
    }

    ErrorAlertDialog(message = state.error, title = "Update Failed", onDismiss = { state.error = null })
}

/** What a reviewed update would change, as the wallet reported it (← `UpdateContractPreviewSection`). */
@Composable
private fun UpdateContractPreviewSection(preview: DataContractUpdatePreview) {
    FormSection(title = "Changes", modifier = Modifier.testTag("updateContract.preview")) {
        LabeledContent("Version", "${preview.currentVersion} -> ${preview.newVersion}")
        if (preview.hasChanges) {
            SectionRows("Document types", preview.documentTypes)
            SectionRows("Tokens", preview.tokens)
            SectionRows("Groups", preview.groups)
            LabeledContent("Keywords", if (preview.keywordsChanged) "Changes" else "Unchanged")
            LabeledContent("Description", if (preview.descriptionChanged) "Changes" else "Unchanged")
            LabeledContent("Configuration", if (preview.configChanged) "Changes" else "Unchanged")
            if (preview.configChanged) {
                LabeledContent(
                    "Configuration fields changed",
                    preview.configChangedFields.joinToString(", "),
                    modifier = Modifier.testTag("updateContract.configFieldsChanged"),
                )
                if (preview.moderatorsAdded.isNotEmpty()) {
                    IdentityList("Moderator appointments added", preview.moderatorsAdded, "updateContract.moderatorsAdded")
                }
                if (preview.moderatorsRemoved.isNotEmpty()) {
                    IdentityList("Moderator appointments removed", preview.moderatorsRemoved, "updateContract.moderatorsRemoved")
                }
                if (preview.moderatorsAdded.isNotEmpty() || preview.moderatorsRemoved.isNotEmpty()) {
                    Text(
                        "The contract owner may always moderate, appointed or not.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.testTag("updateContract.ownerAlwaysModerates"),
                    )
                }
            }
        } else {
            Text("No changes", color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@Composable
private fun SectionRows(title: String, changes: DataContractUpdatePreview.SectionChanges) {
    if (changes.added.isNotEmpty()) LabeledContent("$title added", changes.added.joinToString(", "))
    if (changes.changed.isNotEmpty()) LabeledContent("$title changed", changes.changed.joinToString(", "))
    if (changes.added.isEmpty() && changes.changed.isEmpty()) LabeledContent(title, "Unchanged")
}

/** A title over one base58 identity per line, each selectable. */
@Composable
private fun IdentityList(title: String, ids: List<String>, testTag: String) {
    Column(Modifier.padding(vertical = 4.dp).testTag(testTag)) {
        Text(title, style = MaterialTheme.typography.bodyMedium)
        ids.forEach { identity ->
            SelectionContainer {
                Text(
                    identity,
                    style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.MiddleEllipsis,
                )
            }
        }
    }
}

@Composable
private fun JsonEditor(value: String, onChange: (String) -> Unit, enabled: Boolean, testTag: String) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        enabled = enabled,
        textStyle = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = 110.dp)
            .padding(vertical = 4.dp)
            .testTag(testTag),
    )
}

@Composable
private fun SharedDefinitionsWarning(testTag: String) {
    Text(
        "These fields were imported from a file with shared definitions (\$defs). This screen " +
            "does not send them: Platform keeps the live ones, and a document type that refers " +
            "to one is checked against the live definition.",
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.tertiary,
        modifier = Modifier.testTag(testTag),
    )
}

@Composable
private fun Footer(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(vertical = 4.dp),
    )
}

/**
 * The text of an imported file, refusing one over the input budget before reading it whole
 * (at most one byte past the budget is read).
 */
private fun readImportedText(context: Context, uri: Uri): Result<String> = runCatching {
    val stream = context.contentResolver.openInputStream(uri) ?: error("the file could not be opened")
    stream.use { input ->
        val buffer = ByteArray(UpdateContractFields.MAX_INPUT_BYTES + 1)
        var read = 0
        while (read < buffer.size) {
            val n = input.read(buffer, read, buffer.size - read)
            if (n < 0) break
            read += n
        }
        check(read <= UpdateContractFields.MAX_INPUT_BYTES) {
            "the file is larger than this screen's import limit of " +
                "${UpdateContractFields.MAX_INPUT_BYTES / (1024 * 1024)} MiB"
        }
        String(buffer, 0, read, Charsets.UTF_8)
    }
}

/** Decode a hex string to bytes, or an empty array if it is not valid hex. */
private fun String.hexToBytesOrEmpty(): ByteArray =
    runCatching {
        val clean = trim()
        require(clean.length % 2 == 0)
        clean.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
    }.getOrElse { ByteArray(0) }
