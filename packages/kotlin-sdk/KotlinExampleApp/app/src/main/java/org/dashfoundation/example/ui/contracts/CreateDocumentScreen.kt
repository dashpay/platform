package org.dashfoundation.example.ui.contracts

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
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.mutableStateSetOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.add
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonArray
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.prettyPrintJson
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.truncateMiddle

/**
 * Schema-driven **New Document** create + broadcast form — port of iOS
 * `CreateDocumentView` + `DocumentFieldsView` (`DocumentsView.swift`),
 * reached from `DocumentTypeDetailsScreen`'s "New Document" action so the
 * contract + document type are already fixed.
 *
 * Collects a value per schema property (parsed on demand from the stored
 * contract JSON via [ParsedContract], the same source the type detail
 * screen reads), marshals them into a properties JSON string, and
 * broadcasts a real revision-1 document state transition through
 * `PlatformWalletManager.documentTransactions.create`
 * (`platform_wallet_create_document_with_signer`). The signing key is
 * selected on the Rust side — an AUTHENTICATION + ECDSA key on the owner
 * whose security level satisfies the document type — so this screen only
 * supplies the wallet handle, owner id, and the wallet's shared signer
 * handle (like `DocumentWithPriceScreen`).
 *
 * Byte-array fields are entered as hex and identifier fields as base58;
 * both ride to Rust as strings, where the schema-driven sanitize step
 * decodes them to native bytes. The confirmed canonical JSON the FFI
 * returns is shown on success — its `$id` is the on-chain document id a
 * DOC-01 browse (chain query) then surfaces.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CreateDocumentScreen(
    contractIdHex: String,
    typeName: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    val contractIdBytes = remember(contractIdHex) { contractIdHex.hexToBytes() }
    val contractIdBase58 = remember(contractIdHex) { Base58.encode(contractIdBytes) }

    val contractFlow = remember(contractIdHex) {
        container.database.dataContractDao().observeById(contractIdBytes)
    }
    val contract by contractFlow.collectAsStateWithLifecycle(initialValue = null)

    val identities by container.database.identityDao()
        .observeWalletOwnedByNetwork(network.ffiValue)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    var owner by remember { mutableStateOf<IdentityEntity?>(null) }
    LaunchedEffect(identities) {
        if (owner == null) owner = identities.firstOrNull()
    }

    // Field state keyed by property name — text for every non-boolean input,
    // a separate boolean map plus a "touched" set so untouched optional
    // booleans stay out of the payload (mirror of iOS `touchedBoolFields`).
    val textValues = remember { mutableStateMapOf<String, String>() }
    val boolValues = remember { mutableStateMapOf<String, Boolean>() }
    val touchedBools = remember { mutableStateSetOf<String>() }

    var isSubmitting by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var createdDocId by remember { mutableStateOf<String?>(null) }
    var createdJson by remember { mutableStateOf<String?>(null) }

    val schema = remember(contract?.lastUpdated, typeName) {
        contract?.let { ParsedContract.from(it)?.documentTypes?.get(typeName) }
    }
    val properties = schema?.objectField("properties") ?: JsonObject(emptyMap())
    val required = schema?.arrayField("required")
        ?.mapNotNull { (it as? JsonPrimitive)?.content }
        ?.toSet()
        .orEmpty()
    val sortedProps = remember(properties) { properties.entries.sortedBy { it.key } }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("New Document") },
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
            val successJson = createdJson
            if (createdDocId != null && successJson != null) {
                FormSection(title = "Document Created") {
                    Text(
                        "Broadcast confirmed on-chain.",
                        color = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.testTag("createDocument.success"),
                    )
                    LabeledContent("ID", truncateMiddle(createdDocId!!, 12, 8))
                    Text(
                        prettyPrintJson(successJson),
                        style = MaterialTheme.typography.bodySmall,
                        modifier = Modifier.testTag("createDocument.resultJson"),
                    )
                }
                SubmitButton(
                    text = "Done",
                    isLoading = false,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("createDocument.doneButton"),
                ) { navController.popBackStack() }
                return@Column
            }

            FormSection(title = "Document") {
                LabeledContent("Contract", truncateMiddle(contractIdBase58, 10, 6))
                LabeledContent("Type", typeName)
            }

            FormSection(title = "Owner Identity") {
                if (identities.isEmpty()) {
                    Text(
                        "No wallet-owned identities on ${network.networkName} — " +
                            "create or load one first.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    AccessiblePicker(
                        label = "Owner",
                        options = identities,
                        selected = owner ?: identities.first(),
                        optionLabel = {
                            it.alias ?: it.mainDpnsName
                                ?: truncateMiddle(Base58.encode(it.identityId), 10, 6)
                        },
                        testTag = "createDocument.ownerPicker",
                        onSelected = { owner = it },
                    )
                    Text(
                        "The identity that owns and signs for this document.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            FormSection(title = "Fields") {
                if (sortedProps.isEmpty()) {
                    Text(
                        "This document type has no properties.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    sortedProps.forEach { (name, propEl) ->
                        DocumentPropertyField(
                            name = name,
                            prop = propEl as? JsonObject ?: JsonObject(emptyMap()),
                            isRequired = name in required,
                            enabled = !isSubmitting,
                            textValues = textValues,
                            boolValues = boolValues,
                            touchedBools = touchedBools,
                        )
                    }
                    if (required.isNotEmpty()) {
                        Text(
                            "Required: ${required.sorted().joinToString(", ")}",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }

            SubmitButton(
                text = "Create / Broadcast",
                isLoading = isSubmitting,
                enabled = !isSubmitting && manager != null && owner != null,
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("createDocument.submitButton"),
            ) {
                val ownerId = owner ?: return@SubmitButton
                val mgr = manager ?: return@SubmitButton
                val propertiesJson = try {
                    buildPropertiesJson(properties, required, textValues, boolValues, touchedBools)
                } catch (e: Exception) {
                    error = "Could not encode document fields: ${e.message}"
                    return@SubmitButton
                }
                val walletIdHex = ownerId.walletId?.toHex()
                if (walletIdHex == null) {
                    error = "Selected identity has no wallet linkage."
                    return@SubmitButton
                }
                val wallet = mgr.wallets.value[walletIdHex]
                if (wallet == null) {
                    error = "Wallet not loaded in the wallet manager."
                    return@SubmitButton
                }
                isSubmitting = true
                scope.launch {
                    try {
                        val json = mgr.documentTransactions.create(
                            walletHandle = wallet.handle,
                            ownerId = ownerId.identityId,
                            contractId = contractIdBytes,
                            documentType = typeName,
                            propertiesJson = propertiesJson,
                            signerHandle = mgr.signerHandle,
                        )
                        createdJson = json
                        createdDocId = parseDocumentId(json) ?: "(id in JSON)"
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

/**
 * One schema-property editor, dispatched on the JSON-schema `type`. Shared
 * by the create form and the DOC-03 replace form ([DocumentActionsScreen]),
 * which pass distinct [tagPrefix]es so their fields get non-colliding
 * testTags.
 */
@Composable
internal fun DocumentPropertyField(
    name: String,
    prop: JsonObject,
    isRequired: Boolean,
    enabled: Boolean,
    textValues: MutableMap<String, String>,
    boolValues: MutableMap<String, Boolean>,
    touchedBools: MutableSet<String>,
    tagPrefix: String = "createDocument.field",
) {
    val type = prop.stringField("type")
    val isByteArray = prop.boolField("byteArray") == true
    val isIdentifier = prop.stringField("contentMediaType")?.contains("identifier") == true
    val tag = "$tagPrefix.$name"

    Column(modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
        Row {
            Text(name, style = MaterialTheme.typography.bodyMedium)
            if (isRequired) {
                Text(" *", color = MaterialTheme.colorScheme.error)
            }
        }
        when {
            type == "boolean" -> Switch(
                checked = boolValues[name] ?: false,
                onCheckedChange = {
                    boolValues[name] = it
                    touchedBools.add(name)
                },
                enabled = enabled,
                modifier = Modifier.testTag(tag),
            )

            type == "integer" || type == "number" -> OutlinedTextField(
                value = textValues[name].orEmpty(),
                onValueChange = { new ->
                    textValues[name] = new.filter { it.isDigit() || it == '.' || it == '-' }
                },
                label = { Text(numericHint(prop)) },
                singleLine = true,
                enabled = enabled,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.fillMaxWidth().testTag(tag),
            )

            type == "object" -> OutlinedTextField(
                value = textValues[name].orEmpty(),
                onValueChange = { textValues[name] = it },
                label = { Text("JSON object") },
                enabled = enabled,
                minLines = 3,
                modifier = Modifier.fillMaxWidth().testTag(tag),
            )

            else -> OutlinedTextField(
                value = textValues[name].orEmpty(),
                onValueChange = { textValues[name] = it },
                label = {
                    Text(
                        when {
                            isIdentifier -> "Base58 identifier"
                            type == "array" && isByteArray -> "Hex bytes"
                            type == "array" -> "Comma-separated values"
                            else -> stringHint(prop)
                        },
                    )
                },
                singleLine = type != "array",
                enabled = enabled,
                modifier = Modifier.fillMaxWidth().testTag(tag),
            )
        }
        prop.stringField("description")?.let {
            Text(
                it,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 2.dp),
            )
        }
    }
}

internal fun stringHint(prop: JsonObject): String {
    val min = prop.intField("minLength")
    val max = prop.intField("maxLength")
    return when {
        min != null && max != null -> "Text ($min–$max chars)"
        max != null -> "Text (max $max chars)"
        min != null -> "Text (min $min chars)"
        else -> "Text"
    }
}

internal fun numericHint(prop: JsonObject): String {
    val min = prop.intField("minimum")
    val max = prop.intField("maximum")
    return when {
        min != null && max != null -> "Number ($min–$max)"
        max != null -> "Number (≤ $max)"
        min != null -> "Number (≥ $min)"
        else -> "Number"
    }
}

/**
 * Marshal the collected field state into a properties JSON string the Rust
 * `create_document_with_signer` sanitize step accepts. Empty inputs are
 * omitted; booleans ride only when required or user-toggled (absence ≠
 * `false` for some schemas). Byte-array (hex) and identifier (base58)
 * fields pass through as strings — the schema-driven sanitize decodes
 * them Rust-side. `object` fields are parsed so they serialize as nested
 * objects rather than a JSON string. Throws on invalid `object` JSON.
 */
internal fun buildPropertiesJson(
    properties: JsonObject,
    required: Set<String>,
    textValues: Map<String, String>,
    boolValues: Map<String, Boolean>,
    touchedBools: Set<String>,
): String = buildJsonObject {
    for ((name, propEl) in properties) {
        val prop = propEl as? JsonObject ?: continue
        when (prop.stringField("type")) {
            "boolean" -> {
                if (name in required || name in touchedBools) {
                    put(name, boolValues[name] ?: false)
                }
            }

            "integer", "number" -> {
                val raw = textValues[name]?.trim().orEmpty()
                if (raw.isNotEmpty()) {
                    val long = raw.toLongOrNull()
                    if (long != null) {
                        put(name, long)
                    } else {
                        raw.toDoubleOrNull()?.let { put(name, it) }
                    }
                }
            }

            "array" -> {
                val raw = textValues[name]?.trim().orEmpty()
                if (raw.isNotEmpty()) {
                    if (prop.boolField("byteArray") == true) {
                        // Hex (byte array) or base58 (identifier) — string.
                        put(name, raw)
                    } else {
                        putJsonArray(name) {
                            raw.split(",")
                                .map { it.trim() }
                                .filter { it.isNotEmpty() }
                                .forEach { add(it) }
                        }
                    }
                }
            }

            "object" -> {
                val raw = textValues[name]?.trim().orEmpty()
                if (raw.isNotEmpty()) {
                    put(name, LenientJson.parseToJsonElement(raw))
                }
            }

            else -> {
                val raw = textValues[name]?.trim().orEmpty()
                if (raw.isNotEmpty()) put(name, raw)
            }
        }
    }
}.toString()

/** Read the confirmed document id (`$id`, base58) off the canonical JSON. */
internal fun parseDocumentId(json: String): String? = try {
    val root = LenientJson.parseToJsonElement(json).jsonObject
    ((root["\$id"] ?: root["id"]) as? JsonPrimitive)?.content
} catch (_: Exception) {
    null
}
