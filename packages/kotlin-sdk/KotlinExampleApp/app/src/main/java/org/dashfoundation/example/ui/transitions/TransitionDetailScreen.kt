package org.dashfoundation.example.ui.transitions

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.dashsdk.voting.VoteChoice
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.ContestDetail
import org.dashfoundation.example.navigation.CreateIdentity
import org.dashfoundation.example.navigation.DocumentActions
import org.dashfoundation.example.navigation.DocumentWithPrice
import org.dashfoundation.example.navigation.RegisterContractSource
import org.dashfoundation.example.navigation.TokenAction
import org.dashfoundation.example.navigation.TopUpIdentity
import org.dashfoundation.example.navigation.TransferCredits
import org.dashfoundation.example.navigation.UpdateContract
import org.dashfoundation.example.navigation.WithdrawCredits
import org.dashfoundation.example.services.IdentityKeyAdditionFlow
import org.dashfoundation.example.services.transitions.DedicatedTransition
import org.dashfoundation.example.services.transitions.StateTransitionDefinitions
import org.dashfoundation.example.services.transitions.TransitionInput
import org.dashfoundation.example.services.transitions.TransitionInputType
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.contracts.ParsedContract
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.truncateMiddle

/**
 * Dynamic input form for one transition — port of `TransitionDetailView.swift`
 * + `TransitionInputView.swift`: renders each
 * [org.dashfoundation.example.services.transitions.TransitionInput] by its
 * type (identity / token / contract pickers are live Room-backed pickers),
 * validates required fields, and executes.
 *
 * Execution routing (the Swift dedicated-flow split, re-audited for the
 * bridged surface):
 * - [DedicatedTransition] entries navigate to the fully wired dedicated
 *   screen with the picked context (credits flows, identity create,
 *   contract register / update, document price/purchase, the owned-document
 *   replace/delete/transfer actions, the eight token action forms, and the
 *   DPNS contest drill-in).
 * - `identityUpdate` executes inline via
 *   `PlatformWalletManager.identityUpdates` (disable path; the add-keys
 *   path names the pubkey-returning slot-derive gap, see
 *   [IdentityKeyAdditionFlow.UnbridgedSlotDeriver]).
 * - `masternodeVote` executes inline via
 *   `PlatformWalletManager.voteCasting.castVote`.
 * - The remaining definitions (document create) surface the
 *   named-missing-export dialog on submit — their platform-wallet FFIs are
 *   not bridged into `rs-unified-sdk-jni`.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TransitionDetailScreen(transitionKey: String, navController: NavHostController) {
    val definition = remember(transitionKey) { StateTransitionDefinitions.byKey(transitionKey) }
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()

    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    val identities by container.database.identityDao()
        .observeWalletOwnedByNetwork(network.ffiValue)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val tokens by container.database.tokenDao()
        .observeTokensByNetwork(network.ffiValue)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val contracts by container.database.dataContractDao()
        .observeByNetwork(network.ffiValue)
        .collectAsStateWithLifecycle(initialValue = emptyList())

    // Per-field form state (survives config change / picker pops).
    val textInputs = remember { mutableStateMapOf<String, String>() }
    val boolInputs = remember { mutableStateMapOf<String, Boolean>() }
    var initialized by rememberSaveable(transitionKey) { mutableStateOf(false) }
    if (definition != null && !initialized) {
        definition.inputs.forEach { input ->
            when (input.type) {
                TransitionInputType.CHECKBOX ->
                    boolInputs[input.name] = input.defaultValue == "true"
                else ->
                    textInputs[input.name] = input.defaultValue ?: ""
            }
        }
        initialized = true
    }

    var notBridged by remember { mutableStateOf(false) }
    var isSubmitting by remember { mutableStateOf(false) }
    var resultMessage by remember { mutableStateOf<String?>(null) }
    var resultIsError by remember { mutableStateOf(false) }

    if (definition == null) {
        Scaffold(topBar = { TopAppBar(title = { Text("Transition") }) }) { padding ->
            Text(
                "Unknown transition.",
                modifier = Modifier.padding(padding).padding(24.dp),
                color = MaterialTheme.colorScheme.error,
            )
        }
        return
    }

    val requiredFilled = definition.inputs.all { input ->
        if (!input.required || input.type == TransitionInputType.CHECKBOX) {
            true
        } else {
            (textInputs[input.name]?.isNotBlank() == true)
        }
    }

    /** Show an inline execution outcome. */
    fun report(message: String, isError: Boolean) {
        resultMessage = message
        resultIsError = isError
    }

    /** Resolve the wallet a picked identity belongs to, or report why not. */
    suspend fun resolveWalletFor(identityIdHex: String):
        Pair<org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet, ByteArray>? {
        val idBytes = Base58.decodeIdentifier(identityIdHex)
            ?: run { report("Invalid identity id.", true); return null }
        val identity = container.database.identityDao().getByIdentityId(idBytes)
            ?: run { report("Identity not found on this device.", true); return null }
        val walletIdHex = identity.walletId?.toHex()
            ?: run { report("Identity has no wallet linkage.", true); return null }
        val wallet = manager?.wallets?.value?.get(walletIdHex)
            ?: run { report("Wallet not loaded in the wallet manager.", true); return null }
        return wallet to idBytes
    }

    /** The `identityUpdate` inline path (disable keys; adds name the gap). */
    fun executeIdentityUpdate() {
        val mgr = manager ?: run { report("Wallet manager not active.", true); return }
        val addJson = textInputs["addPublicKeys"].orEmpty().trim()
        if (addJson.isNotEmpty()) {
            // The add-keys path needs the wallet-derived public half —
            // named gap, same copy AddIdentityKeyScreen surfaces.
            report(IdentityKeyAdditionFlow.UnbridgedSlotDeriver.EXPORT_NOTE, true)
            return
        }
        val disableIds = textInputs["disablePublicKeys"].orEmpty()
            .split(',').mapNotNull { it.trim().toIntOrNull() }
        if (disableIds.isEmpty()) {
            report("Provide at least one key id to disable (e.g. 2,3,5).", true)
            return
        }
        isSubmitting = true
        scope.launch {
            try {
                val resolved = resolveWalletFor(textInputs["identityId"].orEmpty())
                    ?: return@launch
                val (wallet, idBytes) = resolved
                mgr.identityUpdates.disableKeys(
                    walletHandle = wallet.handle,
                    identityId = idBytes,
                    keyIds = disableIds,
                    signerHandle = mgr.signerHandle,
                )
                report("Disabled key ids ${disableIds.joinToString()} on-chain.", false)
            } catch (e: Exception) {
                report(e.message ?: "Identity update failed", true)
            } finally {
                isSubmitting = false
            }
        }
    }

    /** The `masternodeVote` inline path over `VoteCasting.castVote`. */
    fun executeMasternodeVote() {
        val mgr = manager ?: run { report("Wallet manager not active.", true); return }
        val currentSdk = sdk ?: run { report("SDK not initialized.", true); return }
        val contractInput = textInputs["contractId"].orEmpty().trim()
        val contractBase58 = Base58.decodeIdentifier(contractInput)?.let { Base58.encode(it) }
            ?: run { report("Contract ID must be base58 or 64-char hex.", true); return }
        val proTxHash = decodeHex32(textInputs["proTxHash"].orEmpty())
            ?: run { report("pro_tx_hash must be 32 bytes (64 hex characters).", true); return }
        val votingKey = decodeHex32(textInputs["votingPrivateKey"].orEmpty())
            ?: run {
                report("Voting private key must be 32 bytes (64 hex characters).", true)
                return
            }
        val choice = when (textInputs["voteChoice"].orEmpty().ifEmpty { "abstain" }) {
            "lock" -> VoteChoice.Lock
            "towardsIdentity" -> {
                val target = Base58.decodeIdentifier(textInputs["targetIdentity"].orEmpty())
                    ?: run {
                        report("Towards-identity votes need a target identity.", true)
                        return
                    }
                VoteChoice.TowardsIdentity(Base58.encode(target))
            }
            else -> VoteChoice.Abstain
        }
        val indexValues = textInputs["indexValues"].orEmpty()
            .split(',').map { it.trim() }.filter { it.isNotEmpty() }
        isSubmitting = true
        scope.launch {
            try {
                mgr.voteCasting.castVote(
                    sdkHandle = currentSdk.handle,
                    dataContractId = contractBase58,
                    documentTypeName = textInputs["documentType"].orEmpty().trim(),
                    indexName = textInputs["indexName"].orEmpty().trim(),
                    indexValues = indexValues,
                    choice = choice,
                    proTxHash = proTxHash,
                    votingPrivateKey = votingKey,
                    networkOrd = network.ffiValue,
                )
                report("Vote accepted by Platform.", false)
            } catch (e: Exception) {
                report("Vote rejected: ${e.message}", true)
            } finally {
                isSubmitting = false
            }
        }
    }

    /** Navigate an executable dedicated transition with the picked context. */
    fun executeDedicated(route: DedicatedTransition) {
        val identityHex = Base58.decodeIdentifier(textInputs["identityId"].orEmpty())
            ?.toHex()
        when (route) {
            DedicatedTransition.CREATE_IDENTITY -> navController.navigate(CreateIdentity)
            DedicatedTransition.TOP_UP_IDENTITY -> identityHex
                ?.let { navController.navigate(TopUpIdentity(it)) }
                ?: report("Pick the identity to top up.", true)
            DedicatedTransition.TRANSFER_CREDITS -> identityHex
                ?.let { navController.navigate(TransferCredits(it)) }
                ?: report("Pick the source identity.", true)
            DedicatedTransition.WITHDRAW_CREDITS -> identityHex
                ?.let { navController.navigate(WithdrawCredits(it)) }
                ?: report("Pick the source identity.", true)
            DedicatedTransition.REGISTER_CONTRACT -> identityHex
                ?.let { navController.navigate(RegisterContractSource(it)) }
                ?: report("Pick the owner identity.", true)
            DedicatedTransition.UPDATE_CONTRACT -> {
                // The contract id is optional here — the update screen lets the
                // user type / edit it. Prefill it (as hex) when one was picked.
                val contractHex = textInputs["dataContractId"].orEmpty().trim()
                    .takeIf { it.isNotEmpty() }
                    ?.let { Base58.decodeIdentifier(it)?.toHex() }
                    ?: ""
                navController.navigate(UpdateContract(contractIdHex = contractHex))
            }
            DedicatedTransition.DOCUMENT_WITH_PRICE -> {
                val contractHex = Base58
                    .decodeIdentifier(textInputs["contractId"].orEmpty())?.toHex()
                    ?: run { report("Pick a data contract.", true); return }
                val typeName = textInputs["documentType"].orEmpty().trim()
                if (typeName.isEmpty()) {
                    report("Pick a document type.", true)
                    return
                }
                navController.navigate(
                    DocumentWithPrice(
                        contractIdHex = contractHex,
                        typeName = typeName,
                        documentId = textInputs["documentId"].orEmpty().trim(),
                    ),
                )
            }
            DedicatedTransition.DOCUMENT_ACTIONS -> {
                val contractHex = Base58
                    .decodeIdentifier(textInputs["contractId"].orEmpty())?.toHex()
                    ?: run { report("Pick a data contract.", true); return }
                val typeName = textInputs["documentType"].orEmpty().trim()
                if (typeName.isEmpty()) {
                    report("Pick a document type.", true)
                    return
                }
                navController.navigate(
                    DocumentActions(
                        contractIdHex = contractHex,
                        typeName = typeName,
                        documentId = textInputs["documentId"].orEmpty().trim(),
                    ),
                )
            }
            DedicatedTransition.TOKEN_ACTION -> {
                val actionId = definition.tokenActionId
                    ?: run { report("Missing token action mapping.", true); return }
                val tokenHex = textInputs["token"].orEmpty().trim()
                if (tokenHex.isEmpty() || identityHex == null) {
                    report("Pick a token and the acting identity.", true)
                    return
                }
                navController.navigate(TokenAction(actionId, tokenHex, identityHex))
            }
            DedicatedTransition.CONTEST_DETAIL -> {
                val label = textInputs["contestedUsername"].orEmpty().trim().lowercase()
                if (label.isEmpty()) {
                    report("Enter the contested username.", true)
                    return
                }
                navController.navigate(ContestDetail(label, identityHex ?: ""))
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(definition.label) },
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
            FormSection(title = definition.label) {
                Text(
                    definition.description,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            definition.inputs.forEach { input ->
                TransitionInputField(
                    input = input,
                    textValue = textInputs[input.name] ?: "",
                    boolValue = boolInputs[input.name] ?: false,
                    identities = identities,
                    tokens = tokens,
                    contracts = contracts,
                    selectedContractIdHex = textInputs["contractId"] ?: "",
                    onTextChange = { textInputs[input.name] = it },
                    onBoolChange = { boolInputs[input.name] = it },
                )
            }

            if (!definition.executable) {
                Text(
                    "This transition's state-transition FFI is not bridged in " +
                        "rs-unified-sdk-jni yet. The form is wired; submit shows the " +
                        "missing-export note.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            resultMessage?.let { message ->
                Text(
                    message,
                    style = MaterialTheme.typography.bodyMedium,
                    color = if (resultIsError) {
                        MaterialTheme.colorScheme.error
                    } else {
                        MaterialTheme.colorScheme.primary
                    },
                    modifier = Modifier.testTag("transition.${definition.key}.result"),
                )
            }

            SubmitButton(
                text = "Execute",
                isLoading = isSubmitting,
                enabled = requiredFilled && !isSubmitting,
                modifier = Modifier.fillMaxWidth().testTag("transition.${definition.key}.execute"),
            ) {
                resultMessage = null
                when {
                    !definition.executable -> notBridged = true
                    definition.dedicatedRoute != null ->
                        executeDedicated(definition.dedicatedRoute)
                    definition.key == "identityUpdate" -> executeIdentityUpdate()
                    definition.key == "masternodeVote" -> executeMasternodeVote()
                    else -> notBridged = true
                }
            }
        }
    }

    if (notBridged) {
        AlertDialog(
            onDismissRequest = { notBridged = false },
            title = { Text("Not Bridged Yet") },
            text = {
                Text(
                    "The composite state-transition FFI for " +
                        "\"${definition.label}\" is not bridged in " +
                        "rs-unified-sdk-jni yet. The catalog form is fully " +
                        "wired — only the final execution is pending the bridge.",
                )
            },
            confirmButton = { TextButton(onClick = { notBridged = false }) { Text("OK") } },
        )
    }
}

/** Decode exactly 32 bytes of hex; null otherwise. */
private fun decodeHex32(input: String): ByteArray? {
    val trimmed = input.trim()
    if (trimmed.length != 64 ||
        !trimmed.all { it.isDigit() || it in 'a'..'f' || it in 'A'..'F' }
    ) {
        return null
    }
    return trimmed.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
}

/**
 * Render one input by its [TransitionInputType] — `TransitionInputView.swift`.
 * Identity / token / contract / document-type pickers are live Room-backed
 * pickers (falling back to plain id entry when the device has no rows to
 * offer); document ids stay free text.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TransitionInputField(
    input: TransitionInput,
    textValue: String,
    boolValue: Boolean,
    identities: List<IdentityEntity>,
    tokens: List<TokenEntity>,
    contracts: List<DataContractEntity>,
    selectedContractIdHex: String,
    onTextChange: (String) -> Unit,
    onBoolChange: (Boolean) -> Unit,
) {
    val tag = "transition.${input.name}"
    when (input.type) {
        TransitionInputType.CHECKBOX -> {
            androidx.compose.foundation.layout.Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(input.label, style = MaterialTheme.typography.bodyLarge)
                Switch(
                    checked = boolValue,
                    onCheckedChange = onBoolChange,
                    modifier = Modifier.testTag(tag),
                )
            }
        }
        TransitionInputType.SELECT -> {
            val selected = input.options.firstOrNull { it.value == textValue } ?: input.options.firstOrNull()
            if (selected != null) {
                AccessiblePicker(
                    label = input.label,
                    options = input.options,
                    selected = selected,
                    optionLabel = { it.label },
                    testTag = "$tag.selectPicker",
                    onSelected = { onTextChange(it.value) },
                )
            }
        }
        TransitionInputType.JSON, TransitionInputType.TEXTAREA -> {
            OutlinedTextField(
                value = textValue,
                onValueChange = onTextChange,
                label = { Text(input.label + if (input.required) " *" else "") },
                placeholder = input.placeholder?.let { { Text(it) } },
                supportingText = input.help?.let { { Text(it) } },
                textStyle = if (input.type == TransitionInputType.JSON) {
                    MaterialTheme.typography.bodyMedium.copy(fontFamily = FontFamily.Monospace)
                } else {
                    MaterialTheme.typography.bodyMedium
                },
                modifier = Modifier.fillMaxWidth().heightIn(min = 100.dp).testTag(tag),
            )
        }
        TransitionInputType.NUMBER -> {
            OutlinedTextField(
                value = textValue,
                onValueChange = { onTextChange(it.filter(Char::isDigit)) },
                label = { Text(input.label + if (input.required) " *" else "") },
                placeholder = input.placeholder?.let { { Text(it) } },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.fillMaxWidth().testTag(tag),
            )
        }
        TransitionInputType.IDENTITY_PICKER -> {
            if (identities.isEmpty()) {
                PlainIdField(input, textValue, tag, onTextChange)
            } else {
                val selected = identities.firstOrNull {
                    Base58.encode(it.identityId) == textValue || it.identityId.toHex() == textValue
                }
                AccessiblePicker(
                    label = input.label,
                    options = listOf<IdentityEntity?>(null) + identities,
                    selected = selected,
                    optionLabel = { identity ->
                        identity?.let {
                            it.alias ?: it.mainDpnsName
                                ?: truncateMiddle(Base58.encode(it.identityId), 10, 6)
                        } ?: "Select identity"
                    },
                    testTag = "$tag.identityPicker",
                    onSelected = { identity ->
                        onTextChange(identity?.identityId?.toHex() ?: "")
                    },
                )
            }
        }
        TransitionInputType.TOKEN_PICKER -> {
            if (tokens.isEmpty()) {
                PlainIdField(input, textValue, tag, onTextChange)
            } else {
                val selected = tokens.firstOrNull { it.id.toHex() == textValue }
                AccessiblePicker(
                    label = input.label,
                    options = listOf<TokenEntity?>(null) + tokens,
                    selected = selected,
                    optionLabel = { it?.name ?: "Select token" },
                    testTag = "$tag.tokenPicker",
                    onSelected = { token -> onTextChange(token?.id?.toHex() ?: "") },
                )
            }
        }
        TransitionInputType.CONTRACT_PICKER -> {
            if (contracts.isEmpty()) {
                PlainIdField(input, textValue, tag, onTextChange)
            } else {
                val selected = contracts.firstOrNull { it.id.toHex() == textValue }
                AccessiblePicker(
                    label = input.label,
                    options = listOf<DataContractEntity?>(null) + contracts,
                    selected = selected,
                    optionLabel = { it?.name ?: "Select contract" },
                    testTag = "$tag.contractPicker",
                    onSelected = { contract -> onTextChange(contract?.id?.toHex() ?: "") },
                )
            }
        }
        TransitionInputType.DOCUMENT_TYPE_PICKER -> {
            val typeNames = remember(contracts, selectedContractIdHex) {
                contracts.firstOrNull { it.id.toHex() == selectedContractIdHex }
                    ?.let { ParsedContract.from(it)?.documentTypes?.keys?.sorted() }
                    .orEmpty()
            }
            if (typeNames.isEmpty()) {
                PlainIdField(input, textValue, tag, onTextChange)
            } else {
                AccessiblePicker(
                    label = input.label,
                    options = listOf("") + typeNames,
                    selected = textValue.takeIf { it in typeNames } ?: "",
                    optionLabel = { it.ifEmpty { "Select document type" } },
                    testTag = "$tag.docTypePicker",
                    onSelected = onTextChange,
                )
            }
        }
        TransitionInputType.TEXT,
        TransitionInputType.DOCUMENT_PICKER,
        TransitionInputType.DOCUMENT_WITH_PRICE,
        -> PlainIdField(input, textValue, tag, onTextChange)
    }
}

/** Free-text id entry — the picker fallback and document-id shape. */
@Composable
private fun PlainIdField(
    input: TransitionInput,
    textValue: String,
    tag: String,
    onTextChange: (String) -> Unit,
) {
    OutlinedTextField(
        value = textValue,
        onValueChange = onTextChange,
        label = { Text(input.label + if (input.required) " *" else "") },
        placeholder = input.placeholder?.let { { Text(it) } },
        singleLine = true,
        supportingText = input.help?.let { { Text(it) } },
        modifier = Modifier.fillMaxWidth().testTag(tag),
    )
}
