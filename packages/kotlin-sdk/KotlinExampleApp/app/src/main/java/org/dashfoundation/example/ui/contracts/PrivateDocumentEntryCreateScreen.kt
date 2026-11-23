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
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.wallet.shieldedOperationMayHaveGoneThrough
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.formatCredits
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

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

    var isSubmitting by remember { mutableStateOf(false) }
    var outcome by remember { mutableStateOf<PrivateEntryCreateOutcome?>(null) }
    // The inputs of the last create that may have gone through: Create stays disabled while
    // the form still describes it, as another tap would pay for a second entry. Leaving the
    // screen drops it with the rest of this state.
    var heldInputs by remember { mutableStateOf<PrivateEntryCreateInputs?>(null) }

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

    val inputs = PrivateEntryCreateInputs(
        walletHex = selectedWalletHex,
        senderHex = selectedSenderHex,
        recipientText = recipientText,
        recipientKeyId = selectedRecipientKeyId,
        auth = auth,
        fields = listOf(textValues.toMap(), boolValues.toMap(), touchedBools.toSet(), listValues.toMap()),
    )
    val canSubmit = canSubmitPrivateEntryCreate(
        walletLoaded = selectedWalletHex != null && selectedWalletHex in loadedWalletHexes,
        hasSender = selectedSenderHex != null,
        keysApply = keysApply,
        hasRecipientKey = selectedRecipientKeyId != null,
        inputs = inputs,
        heldInputs = heldInputs,
    )

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

    fun submit() {
        val walletHex = selectedWalletHex ?: return
        val senderHex = selectedSenderHex ?: return
        val recipient = recipientKeysFor ?: return
        val keyId = selectedRecipientKeyId ?: return
        val activeManager = manager ?: return
        val propertiesJson = try {
            buildPropertiesJson(properties, required, textValues, boolValues, touchedBools, listValues)
        } catch (e: Exception) {
            outcome = PrivateEntryCreateOutcome.Failed("Could not encode the properties: ${e.message}")
            return
        }
        val submitted = inputs
        isSubmitting = true
        outcome = null
        heldInputs = null
        scope.launch {
            try {
                val created = activeManager.createPrivateDocumentEntry(
                    walletId = walletHex.hexToBytes(),
                    account = PRIVATE_ENTRY_CREATE_ACCOUNT,
                    contractId = contractId,
                    documentTypeName = documentTypeName,
                    senderIdentityId = senderHex.hexToBytes(),
                    recipientIdentityId = recipient,
                    recipientKeyId = keyId,
                    auth = auth,
                    propertiesJson = propertiesJson,
                )
                outcome = PrivateEntryCreateOutcome.Created(created)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                val failed = PrivateEntryCreateOutcome.of(e)
                outcome = failed
                // Rust keeps the notes reserved until a shielded sync sees the transition:
                // another tap would pay for a second entry
                if (failed is PrivateEntryCreateOutcome.Unconfirmed) heldInputs = submitted
            } finally {
                isSubmitting = false
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("New $documentTypeName Entry") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }, enabled = !isSubmitting) {
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
            FormSection(title = "Pay From") {
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
                        outcome = null
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

            SubmitButton(
                text = if (isSubmitting) "Proving and broadcasting…" else "Create Entry",
                isLoading = isSubmitting,
                enabled = canSubmit,
                modifier = Modifier.fillMaxWidth().testTag("privateEntryCreate.createButton"),
            ) { submit() }

            outcome?.let { current ->
                OutcomeSection(current) {
                    navController.navigate(PrivateDocumentEntries(contractIdHex, documentTypeName))
                }
            }
        }
    }
}

/** The fee preview's state. */
private sealed class FeePreview {
    data object Loading : FeePreview()
    data class Known(val credits: Long) : FeePreview()
    data class Unknown(val reason: String) : FeePreview()
}

/**
 * Everything the user chose on the form, compared to the inputs of a create that may have
 * gone through so the same create is not sent twice. Ids are compared as hex, so equality is
 * by value.
 */
internal data class PrivateEntryCreateInputs(
    val walletHex: String?,
    val senderHex: String?,
    val recipientText: String,
    val recipientKeyId: Long?,
    val auth: PrivateDocumentSenderAuth,
    val fields: List<Any>,
)

/**
 * Whether Create may be tapped: a loaded wallet, a sender, keys read for the recipient typed
 * now and one of them chosen, and a form that is not the one of a create that may have gone
 * through (← Swift `canSubmit`). What a create needs beyond that is Rust's to refuse.
 */
internal fun canSubmitPrivateEntryCreate(
    walletLoaded: Boolean,
    hasSender: Boolean,
    keysApply: Boolean,
    hasRecipientKey: Boolean,
    inputs: PrivateEntryCreateInputs,
    heldInputs: PrivateEntryCreateInputs?,
): Boolean = walletLoaded && hasSender && keysApply && hasRecipientKey && inputs != heldInputs

/** The typed recipient as a 32-byte identifier (base58 or hex), or null. */
internal fun parsePrivateEntryRecipientId(text: String): ByteArray? = Base58.decodeIdentifier(text)

/** How a create ended, each shown in its own state. */
internal sealed class PrivateEntryCreateOutcome {
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
