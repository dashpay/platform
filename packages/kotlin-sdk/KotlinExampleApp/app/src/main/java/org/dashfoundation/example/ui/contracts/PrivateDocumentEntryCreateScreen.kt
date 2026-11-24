package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Person
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
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
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.mutableStateSetOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.currentStateAsState
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.WalletEntity
import org.dashfoundation.dashsdk.wallet.PrivateDocumentEntryCreated
import org.dashfoundation.dashsdk.wallet.PrivateDocumentRecipientKey
import org.dashfoundation.dashsdk.wallet.PrivateDocumentSenderAuth
import org.dashfoundation.dashsdk.wallet.createPrivateDocumentEntry
import org.dashfoundation.dashsdk.wallet.privateDocumentCreateFee
import org.dashfoundation.dashsdk.wallet.privateDocumentRecipientKeys
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.PrivateDocumentEntries
import org.dashfoundation.example.services.shielded.PrivateDocumentEntryCreateCoordinator
import org.dashfoundation.example.ui.identity.HoldSecureWindow
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.wallet.shieldedOperationMayHaveGoneThrough
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.formatCredits
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex
import java.util.Collections
import java.util.IdentityHashMap

/** The shielded account a create pays from, as on iOS. */
internal const val PRIVATE_ENTRY_CREATE_ACCOUNT = 0

/**
 * Creates one entry of a private document type (protocol version 15): an encrypted entry
 * sealed by one of a wallet's identities to a key of any identity, appended by an anonymous
 * transition (type 29) paid from the wallet's shielded account 0. Port of
 * `PrivateDocumentEntryCreateView.swift`. Covers TEST_PLAN DOC-19 (Swift DOC-19).
 *
 * Everything that decides something is Rust's: which recipient keys qualify
 * (`privateDocumentRecipientKeys`), which of the sender's keys seals, the schema check, the fee
 * (`privateDocumentCreateFee`) and the outcome (`createPrivateDocumentEntry`). This screen
 * gathers the choices, shows what Rust answers, and keeps nothing. The schema is read only to
 * build the property form, with the same editors as New Document. The wallet is looked up from
 * the active manager at tap time, so no native handle is held in composition.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PrivateDocumentEntryCreateScreen(
    contractIdHex: String,
    documentTypeName: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val contractId = remember(contractIdHex) { contractIdHex.hexToBytes() }
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val protocolVersion by appState.platformProtocolVersion.collectAsStateWithLifecycle()
    val wallets by remember(network) {
        container.database.walletDao().observeByNetwork(network.ffiValue)
    }.collectAsStateWithLifecycle(initialValue = emptyList())
    val identities by remember(network) {
        container.database.identityDao().observeByNetwork(network.ffiValue)
    }.collectAsStateWithLifecycle(initialValue = emptyList())
    // The ids of the wallets the manager has loaded, not the wallets: a screen holds no native handle
    val loadedWalletHexes by remember(manager) {
        manager?.wallets?.map { it.keys } ?: flowOf(emptySet())
    }.collectAsStateWithLifecycle(initialValue = emptySet())
    val contract by remember(contractIdHex) {
        container.database.dataContractDao().observeById(contractId)
    }.collectAsStateWithLifecycle(initialValue = null)
    val schema = remember(contract?.lastUpdated, documentTypeName) {
        contract?.let { ParsedContract.from(it)?.documentTypes?.get(documentTypeName) }
    }
    val properties = schema?.objectField("properties") ?: JsonObject(emptyMap())
    val required = schema?.arrayField("required")
        ?.mapNotNull { (it as? JsonPrimitive)?.content }
        ?.toSet()
        .orEmpty()
    val sortedProps = remember(properties) { properties.entries.sortedBy { it.key } }

    var selectedWalletHex by remember { mutableStateOf<String?>(null) }
    var selectedSenderHex by remember { mutableStateOf<String?>(null) }
    // The recipient identity id as typed (base58 or hex)
    var recipientText by remember { mutableStateOf("") }
    // The recipient's keys an entry may be sealed to, as Rust read them for recipientKeysFor
    var recipientKeys by remember { mutableStateOf<List<PrivateDocumentRecipientKey>?>(null) }
    var recipientKeysFor by remember { mutableStateOf<ByteArray?>(null) }
    var recipientKeysError by remember { mutableStateOf<String?>(null) }
    var isLoadingKeys by remember { mutableStateOf(false) }
    var selectedRecipientKeyId by remember { mutableStateOf<Long?>(null) }
    var auth by remember { mutableStateOf(PrivateDocumentSenderAuth.ANONYMOUS) }
    val textValues = remember { mutableStateMapOf<String, String>() }
    val boolValues = remember { mutableStateMapOf<String, Boolean>() }
    val touchedBools = remember { mutableStateSetOf<String>() }
    val listValues = remember { mutableStateMapOf<String, List<String>>() }

    val coordinator = container.privateDocumentEntryCreateCoordinator
    val createStates by coordinator.states.collectAsStateWithLifecycle()
    // The creates as they stood when this screen opened: what one of them left is shown, and only
    // the outcome of a create that finishes from here on is scrolled to
    val statesAtOpen = remember { coordinator.states.value }
    val heldRequests by coordinator.held.collectAsStateWithLifecycle()
    val hasShielded = remember { runCatching { Sdk.hasShielded() }.getOrDefault(false) }
    // Why the last tap sent nothing (the properties did not encode, the form changed under it)
    var localRefusal by remember { mutableStateOf<String?>(null) }

    // The wallet's create lives in the coordinator, past this screen: going back or a rotation
    // keeps it running and keeps its outcome
    val walletState = selectedWalletHex?.let { createStates[it] }
    val isSubmitting = walletState is PrivateDocumentEntryCreateCoordinator.State.InFlight
    val outcome = (walletState as? PrivateDocumentEntryCreateCoordinator.State.Done)
        ?.takeIf { it.request.contractIdHex == contractIdHex.lowercase() && it.request.documentTypeName == documentTypeName }
        ?.outcome

    val recipientId = parsePrivateEntryRecipientId(recipientText)
    // Keys read for another identity do not apply to the one typed now
    val keysApply = recipientKeysFor != null && recipientId != null &&
        recipientKeysFor.contentEquals(recipientId)

    val senders = identities.filter { it.walletId?.toHex() == selectedWalletHex }

    // Default to the first wallet of this network and its first identity
    LaunchedEffect(wallets, identities) {
        val current = selectedWalletHex
        if (current == null || wallets.none { it.walletId.toHex() == current }) {
            selectedWalletHex = wallets.firstOrNull()?.walletId?.toHex()
            selectedSenderHex = null
        }
        val walletSenders = identities.filter { it.walletId?.toHex() == selectedWalletHex }
        if (walletSenders.none { it.identityId.toHex() == selectedSenderHex }) {
            selectedSenderHex = walletSenders.firstOrNull()?.identityId?.toHex()
        }
    }

    val shieldedBalance by remember(selectedWalletHex) {
        selectedWalletHex?.let {
            container.shieldedService.accountBalance(it.hexToBytes(), PRIVATE_ENTRY_CREATE_ACCOUNT)
        } ?: flowOf(null)
    }.collectAsStateWithLifecycle(initialValue = null)

    // Rust's fee for one entry, at the network's protocol version (read again when it is published)
    val fee by produceState<FeePreview>(FeePreview.Loading, manager, protocolVersion) {
        val activeManager = manager
        value = if (activeManager == null) {
            FeePreview.Loading
        } else {
            try {
                FeePreview.Known(
                    activeManager.privateDocumentCreateFee(contractId, documentTypeName, entryCount = 1),
                )
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                FeePreview.Unknown("Fee not known: ${e.message ?: e.toString()}")
            }
        }
    }

    fun encodeProperties(): String = buildPropertiesJson(
        properties, required, textValues, boolValues, touchedBools, listValues, trimStrings = false,
    )

    // What a tap would send now, judged on the live form; null while it is not complete
    val encodedProperties = runCatching { encodeProperties() }.getOrNull()
    val request = privateEntryCreateRequest(
        walletIdHex = selectedWalletHex,
        senderIdentityIdHex = selectedSenderHex,
        contractIdHex = contractIdHex,
        documentTypeName = documentTypeName,
        recipientText = recipientText,
        recipientKeysFor = recipientKeysFor,
        recipientKeys = recipientKeys,
        recipientKeyId = selectedRecipientKeyId,
        auth = auth,
        propertiesJson = encodedProperties ?: "",
    )
    // The same request may already have gone through: it is not sent again
    val isHeld = encodedProperties != null && request != null && request in heldRequests
    val canSubmit = hasShielded && !isSubmitting && !isHeld && request != null &&
        selectedWalletHex in loadedWalletHexes

    fun loadRecipientKeys() {
        val recipient = recipientId ?: return
        val activeManager = manager ?: return
        isLoadingKeys = true
        recipientKeysError = null
        scope.launch {
            try {
                val keys = activeManager.privateDocumentRecipientKeys(recipient)
                // The user typed another id while this read ran
                if (!recipient.contentEquals(parsePrivateEntryRecipientId(recipientText))) return@launch
                recipientKeys = keys
                recipientKeysFor = recipient
                selectedRecipientKeyId = keys.firstOrNull()?.keyId
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                if (!recipient.contentEquals(parsePrivateEntryRecipientId(recipientText))) return@launch
                recipientKeys = null
                recipientKeysFor = null
                selectedRecipientKeyId = null
                recipientKeysError = e.message ?: e.toString()
            } finally {
                isLoadingKeys = false
            }
        }
    }

    // Judged again from the live form when the tap is handled, not from the last composition: a
    // recipient edited just before the tap must not send to the keys loaded for the previous one
    fun submit() {
        localRefusal = null
        val activeManager = manager ?: return
        val propertiesJson = try {
            encodeProperties()
        } catch (e: Exception) {
            localRefusal = "Could not encode the properties: ${e.message}"
            return
        }
        val live = privateEntryCreateRequest(
            walletIdHex = selectedWalletHex,
            senderIdentityIdHex = selectedSenderHex,
            contractIdHex = contractIdHex,
            documentTypeName = documentTypeName,
            recipientText = recipientText,
            recipientKeysFor = recipientKeysFor,
            recipientKeys = recipientKeys,
            recipientKeyId = selectedRecipientKeyId,
            auth = auth,
            propertiesJson = propertiesJson,
        )
        if (live == null) {
            localRefusal = "The recipient changed: load its keys again."
            return
        }
        val refusal = coordinator.start(live) {
            activeManager.createPrivateDocumentEntry(
                walletId = live.walletIdHex.hexToBytes(),
                account = PRIVATE_ENTRY_CREATE_ACCOUNT,
                contractId = live.contractIdHex.hexToBytes(),
                documentTypeName = live.documentTypeName,
                senderIdentityId = live.senderIdentityIdHex.hexToBytes(),
                recipientIdentityId = live.recipientIdentityIdHex.hexToBytes(),
                recipientKeyId = live.recipientKeyId,
                auth = live.auth,
                propertiesJson = live.propertiesJson,
            )
        }
        localRefusal = when (refusal) {
            null -> null
            PrivateDocumentEntryCreateCoordinator.Refusal.BUSY -> "A create from this wallet is still running."
            PrivateDocumentEntryCreateCoordinator.Refusal.HELD -> HELD_MESSAGE
        }
    }

    // The app's FLAG_SECURE toggle (LocalSecureScreen), held while this screen is shown: the
    // draft, the sender and the recipient are private before Rust encrypts them
    HoldSecureWindow()
    val lifecycleState by LocalLifecycleOwner.current.lifecycle.currentStateAsState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("New $documentTypeName Entry") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Box(Modifier.fillMaxSize().padding(padding)) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .imePadding()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Pay From") {
                if (!hasShielded) {
                    Text(
                        "This build has no shielded support, so it cannot pay for an entry.",
                        style = MaterialTheme.typography.bodySmall,
                        color = WarningOrange,
                        modifier = Modifier.testTag("privateEntryCreate.noShielded"),
                    )
                }
                if (wallets.isEmpty()) {
                    Footer("No wallets on this network. Create or import a wallet first.")
                } else {
                    val selected = wallets.firstOrNull { it.walletId.toHex() == selectedWalletHex }
                        ?: wallets.first()
                    AccessiblePicker(
                        label = "Wallet",
                        options = wallets,
                        selected = selected,
                        optionLabel = ::walletLabel,
                        testTag = "privateEntryCreate.walletPicker",
                        enabled = !isSubmitting,
                    ) { picked ->
                        selectedWalletHex = picked.walletId.toHex()
                        selectedSenderHex = identities
                            .firstOrNull { it.walletId?.toHex() == selectedWalletHex }
                            ?.identityId?.toHex()
                    }
                    if (selectedWalletHex != null && selectedWalletHex !in loadedWalletHexes) {
                        Text(
                            "This wallet is not loaded yet.",
                            style = MaterialTheme.typography.bodySmall,
                            color = WarningOrange,
                        )
                    }
                    shieldedBalance?.let {
                        LabeledContent(
                            "Shielded balance (account 0)",
                            "${formatCredits(it)} credits",
                            modifier = Modifier.testTag("privateEntryCreate.shieldedBalance"),
                        )
                    }
                }
                Footer(
                    "The entry is appended by an anonymous transition paid from this wallet's " +
                        "shielded account 0: nothing on chain names the sender or the recipient.",
                )
            }

            FormSection(title = "Sender") {
                if (senders.isEmpty()) {
                    Footer("This wallet holds no identity on this network.")
                } else {
                    val selected = senders.firstOrNull { it.identityId.toHex() == selectedSenderHex }
                        ?: senders.first()
                    AccessiblePicker(
                        label = "Sender",
                        options = senders,
                        selected = selected,
                        optionLabel = { identityName(it.identityId, identities) },
                        testTag = "privateEntryCreate.senderPicker",
                        enabled = !isSubmitting,
                    ) { selectedSenderHex = it.identityId.toHex() }
                }
                Footer(
                    "The entry is sealed with one of the sender's encryption keys, so this " +
                        "wallet's Read Entries recovers it as sent. An identity without an " +
                        "encryption key cannot send entries.",
                )
            }

            FormSection(title = "Recipient") {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    OutlinedTextField(
                        value = recipientText,
                        onValueChange = {
                            recipientText = it
                            // A refusal of another identity's keys does not apply to this one
                            recipientKeysError = null
                        },
                        label = { Text("Recipient identity ID (base58)") },
                        singleLine = true,
                        enabled = !isSubmitting,
                        textStyle = MaterialTheme.typography.bodyMedium.copy(fontFamily = FontFamily.Monospace),
                        modifier = Modifier.weight(1f).testTag("privateEntryCreate.recipientId"),
                    )
                    if (identities.isNotEmpty()) {
                        LocalRecipientMenu(identities, enabled = !isSubmitting) {
                            recipientText = Base58.encode(it.identityId)
                        }
                    }
                }
                OutlinedButton(
                    onClick = ::loadRecipientKeys,
                    enabled = recipientId != null && !isLoadingKeys && !isSubmitting && manager != null,
                    modifier = Modifier.padding(top = 8.dp).testTag("privateEntryCreate.loadKeysButton"),
                ) {
                    if (isLoadingKeys) {
                        CircularProgressIndicator(modifier = Modifier.padding(end = 8.dp).size(16.dp), strokeWidth = 2.dp)
                    }
                    Text("Load Keys")
                }
                val keysError = recipientKeysError
                val keys = recipientKeys
                when {
                    keysError != null && !keysApply -> SelectionContainer {
                        Text(
                            keysError,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.testTag("privateEntryCreate.recipientKeysError"),
                        )
                    }
                    keys != null && keysApply && keys.isEmpty() -> Text(
                        "This identity has no enabled encryption or decryption key, so no entry " +
                            "can be sealed to it.",
                        style = MaterialTheme.typography.bodySmall,
                        color = WarningOrange,
                        modifier = Modifier.testTag("privateEntryCreate.noRecipientKeys"),
                    )
                    keys != null && keysApply -> AccessiblePicker(
                        label = "Key",
                        options = keys,
                        selected = keys.firstOrNull { it.keyId == selectedRecipientKeyId } ?: keys.first(),
                        optionLabel = { "Key ${it.keyId} (${it.purposeName})" },
                        testTag = "privateEntryCreate.recipientKeyPicker",
                        enabled = !isSubmitting,
                    ) { selectedRecipientKeyId = it.keyId }
                }
                Footer("The entry is sealed to one key of the recipient. Its owner reads the entry with that key.")
            }

            FormSection(title = "Authentication") {
                val modes = PrivateDocumentSenderAuth.entries
                SingleChoiceSegmentedButtonRow(
                    modifier = Modifier.fillMaxWidth().testTag("privateEntryCreate.authPicker"),
                ) {
                    modes.forEachIndexed { index, mode ->
                        SegmentedButton(
                            selected = auth == mode,
                            onClick = { auth = mode },
                            enabled = !isSubmitting,
                            shape = SegmentedButtonDefaults.itemShape(index, modes.size),
                            modifier = Modifier.testTag("privateEntryCreate.authPicker.${mode.name}"),
                        ) { Text(authLabel(mode)) }
                    }
                }
                Footer(
                    when (auth) {
                        PrivateDocumentSenderAuth.ANONYMOUS ->
                            "The recipient learns nothing of who wrote the entry."
                        PrivateDocumentSenderAuth.DENIABLE_MAC ->
                            "The entry names the sender and its encryption key. The recipient can " +
                                "tell who wrote it but cannot prove that to anyone else."
                    },
                )
            }

            FormSection(title = "Properties") {
                Column(Modifier.testTag("privateEntryCreate.fields")) {
                    if (schema == null) {
                        Footer("The contract is not stored on this device.")
                    } else if (sortedProps.isEmpty()) {
                        Footer("This document type has no properties.")
                    }
                    sortedProps.forEach { (name, propEl) ->
                        DocumentPropertyField(
                            name = name,
                            prop = propEl as? JsonObject ?: JsonObject(emptyMap()),
                            isRequired = name in required,
                            enabled = !isSubmitting,
                            textValues = textValues,
                            boolValues = boolValues,
                            touchedBools = touchedBools,
                            listValues = listValues,
                            tagPrefix = "privateEntryCreate.field",
                        )
                    }
                }
                if (required.isNotEmpty()) {
                    Footer("Required: ${required.sorted().joinToString(", ")}")
                }
            }

            FormSection(title = "Cost") {
                when (val preview = fee) {
                    is FeePreview.Known -> LabeledContent(
                        "Fee (at least)",
                        "${formatCredits(preview.credits)} credits",
                        modifier = Modifier.testTag("privateEntryCreate.fee"),
                    )
                    is FeePreview.Unknown -> Text(
                        preview.reason,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.testTag("privateEntryCreate.feeError"),
                    )
                    FeePreview.Loading -> CircularProgressIndicator()
                }
                Footer("Paid from the shielded notes when one or two notes are spent; each further note adds to it.")
            }

            PrivateEntryCreateSubmitArea(
                isSubmitting = isSubmitting,
                canSubmit = canSubmit,
                isHeld = isHeld,
                localRefusal = localRefusal,
                outcome = outcome,
                outcomeSinceOpened = walletState !== selectedWalletHex?.let { statesAtOpen[it] },
                onSubmit = ::submit,
                onReadEntries = {
                    navController.navigate(PrivateDocumentEntries(contractIdHex, documentTypeName))
                },
            )
        }

        if (!lifecycleState.isAtLeast(Lifecycle.State.RESUMED)) {
            PrivacyCover(text = "Private entry hidden", testTag = "privateEntryCreate.privacyCover")
        }
        }
    }
}

/**
 * The Create Entry button and, under it, what a tap led to: the hold notice, why nothing was
 * sent, and the create's outcome. It is the last thing in the form, so the button is tapped at
 * the bottom edge of the screen and whatever appears under it appears out of sight: the form
 * scrolls to show it. Left there, a create that went through looks like a tap that did nothing,
 * with the same entry one tap away from being paid for again.
 *
 * The form scrolls to what is under the button, not to the button: in a window too short for
 * both, the answer is what has to be seen. It scrolls once per answer. An outcome it already
 * scrolled to is shown again without moving the form when another wallet is chosen and then this
 * one again.
 *
 * [outcomeSinceOpened] says the outcome is of a create that finished while this screen was shown.
 * The outcome of an earlier one is shown but not scrolled to, so a reopened form starts at its
 * top. The hold notice is not scrolled to by itself either: it also appears while a field is
 * being edited.
 */
@Composable
internal fun PrivateEntryCreateSubmitArea(
    isSubmitting: Boolean,
    canSubmit: Boolean,
    isHeld: Boolean,
    localRefusal: String?,
    outcome: PrivateEntryCreateOutcome?,
    outcomeSinceOpened: Boolean,
    onSubmit: () -> Unit,
    onReadEntries: () -> Unit,
) {
    val answer = remember { BringIntoViewRequester() }
    // Counted so a tap answered with the same words as the last one scrolls again
    var taps by remember { mutableIntStateOf(0) }
    LaunchedEffect(taps, localRefusal) {
        if (localRefusal != null) answer.bringIntoView()
    }
    // By identity: two creates that may have gone through give equal outcomes
    val scrolledTo = remember {
        Collections.newSetFromMap(IdentityHashMap<PrivateEntryCreateOutcome, Boolean>())
    }
    LaunchedEffect(outcome) {
        if (outcome != null && outcomeSinceOpened && scrolledTo.add(outcome)) answer.bringIntoView()
    }
    Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
        SubmitButton(
            text = if (isSubmitting) "Proving and broadcasting…" else "Create Entry",
            isLoading = isSubmitting,
            enabled = canSubmit,
            modifier = Modifier.fillMaxWidth().testTag("privateEntryCreate.createButton"),
            onClick = {
                taps++
                onSubmit()
            },
        )

        if (isHeld || localRefusal != null || outcome != null) {
            Column(
                modifier = Modifier.bringIntoViewRequester(answer),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                if (isHeld) {
                    Text(
                        HELD_MESSAGE,
                        style = MaterialTheme.typography.bodySmall,
                        color = WarningOrange,
                        modifier = Modifier.testTag("privateEntryCreate.held"),
                    )
                }

                localRefusal?.let {
                    SelectionContainer {
                        Text(
                            it,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.testTag("privateEntryCreate.refusal"),
                        )
                    }
                }

                outcome?.let { OutcomeSection(it, onReadEntries) }
            }
        }
    }
}

/** Shown, and nothing sent, for a request whose create may already have gone through. */
private const val HELD_MESSAGE =
    "This entry may already have gone through: it is not created again. Change what it sends to " +
        "create another."

/** The fee preview's state. */
private sealed class FeePreview {
    data object Loading : FeePreview()
    data class Known(val credits: Long) : FeePreview()
    data class Unknown(val reason: String) : FeePreview()
}

/**
 * What one create would send, from the form as it is now, or null while it is not complete: a
 * wallet and a sender, a recipient that decodes to the identity whose keys were loaded, and one
 * of those keys chosen (← Swift `canSubmit`). Ids are lowercase hex, so two spellings of the same
 * recipient give the same request. What a create needs beyond that is Rust's to refuse.
 */
@Suppress("LongParameterList")
internal fun privateEntryCreateRequest(
    walletIdHex: String?,
    senderIdentityIdHex: String?,
    contractIdHex: String,
    documentTypeName: String,
    recipientText: String,
    recipientKeysFor: ByteArray?,
    recipientKeys: List<PrivateDocumentRecipientKey>?,
    recipientKeyId: Long?,
    auth: PrivateDocumentSenderAuth,
    propertiesJson: String,
): PrivateDocumentEntryCreateCoordinator.Request? {
    val wallet = walletIdHex ?: return null
    val sender = senderIdentityIdHex ?: return null
    val recipient = parsePrivateEntryRecipientId(recipientText) ?: return null
    if (recipientKeysFor == null || !recipientKeysFor.contentEquals(recipient)) return null
    val keyId = recipientKeyId ?: return null
    if (recipientKeys == null || recipientKeys.none { it.keyId == keyId }) return null
    return PrivateDocumentEntryCreateCoordinator.Request(
        walletIdHex = wallet.lowercase(),
        senderIdentityIdHex = sender.lowercase(),
        contractIdHex = contractIdHex.lowercase(),
        documentTypeName = documentTypeName,
        recipientIdentityIdHex = recipient.toHex(),
        recipientKeyId = keyId,
        auth = auth,
        propertiesJson = propertiesJson,
    )
}

/** The typed recipient as a 32-byte identifier (base58 or hex), or null. */
internal fun parsePrivateEntryRecipientId(text: String): ByteArray? = Base58.decodeIdentifier(text)

/** How a create ended, each shown in its own state. */
sealed class PrivateEntryCreateOutcome {
    data class Created(val created: PrivateDocumentEntryCreated) : PrivateEntryCreateOutcome()

    /** The entry may already be in the store: it is not reported as failed, and not sent again. */
    data class Unconfirmed(val message: String) : PrivateEntryCreateOutcome()

    /** The network does not serve private stores yet. */
    data object NotActive : PrivateEntryCreateOutcome()

    /** Nothing was created; shown as Rust gave it. */
    data class Failed(val message: String) : PrivateEntryCreateOutcome()

    companion object {
        const val UNCONFIRMED_MESSAGE =
            "The entry may have gone through; a shielded sync settles it. Do not create it " +
                "again. Read Entries shows it once it is in the store."

        /** A failed create's outcome: one that may have gone through is never shown as failed. */
        fun of(error: Throwable): PrivateEntryCreateOutcome = when {
            shieldedOperationMayHaveGoneThrough(error) -> Unconfirmed(UNCONFIRMED_MESSAGE)
            error is DashSdkError.PlatformWallet.ProtocolVersionNotActive -> NotActive
            else -> Failed(error.message ?: error.toString())
        }
    }
}

@Composable
private fun OutcomeSection(outcome: PrivateEntryCreateOutcome, onReadEntries: () -> Unit) {
    FormSection {
        when (outcome) {
            is PrivateEntryCreateOutcome.Created -> {
                Column(Modifier.testTag("privateEntryCreate.success")) {
                    Text(
                        "Entry created",
                        style = MaterialTheme.typography.titleSmall,
                        color = SuccessGreen,
                    )
                    SelectionContainer {
                        Text(
                            "Commitment: ${outcome.created.commitment.toHex()}",
                            style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                        )
                    }
                    Text(
                        "Sealed with sender key: ${outcome.created.senderKeyId}",
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
                TextButton(
                    onClick = onReadEntries,
                    modifier = Modifier.testTag("privateEntryCreate.readEntriesLink"),
                ) { Text("Read Entries") }
            }
            is PrivateEntryCreateOutcome.Unconfirmed -> Text(
                outcome.message,
                style = MaterialTheme.typography.bodySmall,
                color = WarningOrange,
                modifier = Modifier.testTag("privateEntryCreate.unconfirmed"),
            )
            PrivateEntryCreateOutcome.NotActive -> Text(
                "This network does not serve private document stores yet: they arrive with " +
                    "protocol version 15.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.testTag("privateEntryCreate.notActive"),
            )
            is PrivateEntryCreateOutcome.Failed -> SelectionContainer {
                Text(
                    outcome.message,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.testTag("privateEntryCreate.error"),
                )
            }
        }
    }
}

/** The device's identities on this network, to fill the recipient (← `localRecipientMenu`). */
@Composable
private fun LocalRecipientMenu(
    identities: List<IdentityEntity>,
    enabled: Boolean,
    onPicked: (IdentityEntity) -> Unit,
) {
    var expanded by remember { mutableStateOf(false) }
    Box {
        IconButton(
            onClick = { expanded = true },
            enabled = enabled,
            modifier = Modifier.testTag("privateEntryCreate.localRecipientMenu"),
        ) {
            Icon(Icons.Filled.Person, contentDescription = "Pick one of this device's identities")
        }
        DropdownMenu(expanded = expanded && enabled, onDismissRequest = { expanded = false }) {
            identities.forEach { identity ->
                DropdownMenuItem(
                    text = { Text(identityName(identity.identityId, identities)) },
                    onClick = {
                        expanded = false
                        onPicked(identity)
                    },
                    modifier = Modifier.testTag(
                        "privateEntryCreate.localRecipientMenu.${Base58.encode(identity.identityId)}",
                    ),
                )
            }
        }
    }
}

private fun authLabel(mode: PrivateDocumentSenderAuth): String = when (mode) {
    PrivateDocumentSenderAuth.ANONYMOUS -> "Anonymous"
    PrivateDocumentSenderAuth.DENIABLE_MAC -> "Deniable MAC"
}

private fun walletLabel(wallet: WalletEntity): String {
    val fingerprint = wallet.walletId.take(4).joinToString("") { "%02x".format(it) }
    return "${wallet.name ?: "Wallet"} ($fingerprint)"
}

/** A form section's explanatory footer. */
@Composable
private fun Footer(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(top = 8.dp),
    )
}

private val SuccessGreen = Color(0xFF2E7D32)
private val WarningOrange = Color(0xFFEF6C00)
