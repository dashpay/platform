package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
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
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.mutableStateSetOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.delay
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.RecipientPicker
import org.dashfoundation.example.ui.components.RecipientSelection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.truncateMiddle

/**
 * Owned-document actions for one document — the DOC-03 replace / DOC-04
 * delete / DOC-05 transfer flows the iOS document ops menu ships
 * (`ManagedPlatformWallet.replaceDocument` / `deleteDocument` /
 * `transferDocument`). Reached from a document row's "Actions…" button and
 * from the document replace/delete/transfer transition-catalog entries.
 *
 * The document is probed by id (debounced, like [DocumentWithPriceScreen]) so
 * the acting-identity ownership badge and the replace-field prefill reflect
 * on-chain state. Replace reuses [DocumentPropertyField] / [buildPropertiesJson]
 * (the create form's schema-driven field dispatch); all three actions sign
 * through [resolveSigning] / [submitDocumentTransaction] on the current
 * owner's wallet, so cross-wallet ownership is honored.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DocumentActionsScreen(
    contractIdHex: String,
    typeName: String,
    initialDocumentId: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val sdk by appState.sdk.collectAsStateWithLifecycle()
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
    var actingIdentity by remember { mutableStateOf<IdentityEntity?>(null) }

    var documentIdText by rememberSaveable { mutableStateOf(initialDocumentId) }
    var isLoading by remember { mutableStateOf(false) }
    var probeError by remember { mutableStateOf<String?>(null) }
    var probed by remember { mutableStateOf<ProbedActionDocument?>(null) }
    var probeTick by remember { mutableStateOf(0) }

    // Replace field state (mirror of CreateDocumentScreen), prefilled from the
    // probed document once per document id.
    val textValues = remember { mutableStateMapOf<String, String>() }
    val boolValues = remember { mutableStateMapOf<String, Boolean>() }
    val touchedBools = remember { mutableStateSetOf<String>() }
    // What the replace prefill seeded per key — a field that was seeded
    // non-blank and is blank at submit is an explicit REMOVE; a field that
    // was never seeded and stays blank is ABSENT (preserved by the merge).
    val seededTexts = remember { mutableStateMapOf<String, String>() }
    var seededForId by remember { mutableStateOf<String?>(null) }

    var recipient by remember { mutableStateOf<RecipientSelection?>(null) }
    var showDeleteConfirm by remember { mutableStateOf(false) }

    var isSubmitting by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var replaceSuccess by remember { mutableStateOf<String?>(null) }
    var deleteSuccess by remember { mutableStateOf<String?>(null) }
    var transferSuccess by remember { mutableStateOf<String?>(null) }

    val schema = remember(contract?.lastUpdated, typeName) {
        contract?.let { ParsedContract.from(it)?.documentTypes?.get(typeName) }
    }
    val properties = schema?.objectField("properties") ?: JsonObject(emptyMap())
    val required = schema?.arrayField("required")
        ?.mapNotNull { (it as? JsonPrimitive)?.content }
        ?.toSet()
        .orEmpty()
    val sortedProps = remember(properties) { properties.entries.sortedBy { it.key } }
    val isTransferable = (schema?.intField("transferable") ?: 0) == 1
    // Reproduce DPP's effective capability flags (see
    // `documentTypeCapabilities`): the per-type schema value, else the
    // contract config default, else `true`. Gating anything looser leaves
    // Replace/Delete enabled for transitions Drive rejects as paid-invalid.
    val contractConfig = remember(contract?.lastUpdated) {
        contract?.let { ParsedContract.from(it)?.root?.objectField("config") }
    }
    val capabilities = documentTypeCapabilities(schema, contractConfig)
    val documentsMutable = capabilities.documentsMutable
    val canBeDeleted = capabilities.canBeDeleted

    // Default acting identity to the on-chain owner when it's one of ours,
    // else the first identity — replace/delete/transfer require ownership.
    LaunchedEffect(identities, probed) {
        if (actingIdentity == null) {
            val ownerB58 = probed?.ownerBase58
            actingIdentity = identities.firstOrNull { Base58.encode(it.identityId) == ownerB58 }
                ?: identities.firstOrNull()
        }
    }

    // Debounced probe (← DocumentWithPriceScreen's 0.5 s debounce; also fires
    // for the pre-seeded id from the row / catalog).
    LaunchedEffect(documentIdText, sdk, probeTick) {
        probed = null
        probeError = null
        val currentSdk = sdk ?: return@LaunchedEffect
        val trimmed = documentIdText.trim()
        if (trimmed.isEmpty()) return@LaunchedEffect
        val docIdBytes = Base58.decodeIdentifier(trimmed)
        if (docIdBytes == null) {
            probeError = "Invalid document ID format"
            return@LaunchedEffect
        }
        delay(500)
        isLoading = true
        try {
            val json = currentSdk.contracts.fetch(contractIdBase58).use { c ->
                currentSdk.documents.fetch(c, typeName, Base58.encode(docIdBytes))
            }
            if (json == null) {
                probeError = "Document not found"
            } else {
                probed = parseActionDocument(json)
            }
        } catch (e: Exception) {
            probeError = e.message ?: "Fetch failed"
        } finally {
            isLoading = false
        }
    }

    // Seed the replace fields from the probed document, once per id, so the
    // full replacement payload starts from the current values.
    LaunchedEffect(probed, sortedProps) {
        val doc = probed ?: return@LaunchedEffect
        val trimmed = documentIdText.trim()
        if (seededForId == trimmed || sortedProps.isEmpty()) return@LaunchedEffect
        textValues.clear()
        boolValues.clear()
        touchedBools.clear()
        seedReplaceFields(doc.fields, properties, textValues, boolValues, touchedBools)
        seededTexts.clear()
        seededTexts.putAll(textValues)
        seededForId = trimmed
    }

    val actingB58 = actingIdentity?.let { Base58.encode(it.identityId) }
    val probedDoc = probed
    val isOwner = probedDoc != null && actingB58 != null && probedDoc.ownerBase58 == actingB58

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Document Actions") },
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
            FormSection(title = "Document") {
                LabeledContent("Contract", truncateMiddle(contractIdBase58, 10, 6))
                LabeledContent("Type", typeName)
                OutlinedTextField(
                    value = documentIdText,
                    onValueChange = { documentIdText = it },
                    label = { Text("Document ID (base58 or hex)") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    modifier = Modifier.fillMaxWidth().testTag("documentActions.documentId"),
                )
                when {
                    isLoading -> Text(
                        "Loading…",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    probeError != null -> Text(
                        probeError!!,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("documentActions.probeError"),
                    )
                    probedDoc != null -> {
                        LabeledContent("Status", "Document found")
                        LabeledContent(
                            "Owner",
                            truncateMiddle(probedDoc.ownerBase58 ?: "unknown", 10, 6),
                        )
                    }
                }
            }

            FormSection(title = "Acting Identity") {
                if (identities.isEmpty()) {
                    Text(
                        "No wallet-owned identities on ${network.networkName} — " +
                            "create or load one first.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    AccessiblePicker(
                        label = "Identity",
                        options = identities,
                        selected = actingIdentity ?: identities.first(),
                        optionLabel = {
                            it.alias ?: it.mainDpnsName
                                ?: truncateMiddle(Base58.encode(it.identityId), 10, 6)
                        },
                        testTag = "documentActions.identityPicker",
                        onSelected = { actingIdentity = it },
                    )
                    Text(
                        if (isOwner) {
                            "You own this document — replace, delete, or transfer it below."
                        } else {
                            "Only the current owner can replace, delete, or transfer a " +
                                "document. Pick the owning identity."
                        },
                        style = MaterialTheme.typography.bodySmall,
                        color = if (isOwner) {
                            MaterialTheme.colorScheme.onSurfaceVariant
                        } else {
                            MaterialTheme.colorScheme.error
                        },
                    )
                }
            }

            // ── Replace (DOC-03) ──────────────────────────────────────────
            FormSection(title = "Replace") {
                Text(
                    "Overwrite every field with the values below (revision bumps " +
                        "automatically).",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.testTag("documentActions.replace"),
                )
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
                            tagPrefix = "replaceDocument.field",
                        )
                    }
                }
                replaceSuccess?.let {
                    Text(
                        it,
                        color = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.testTag("replaceDocument.success"),
                    )
                }
                if (!documentsMutable) {
                    Text(
                        "This document type is not mutable — a replace will be " +
                            "rejected by consensus (fees are still charged).",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
                SubmitButton(
                    text = "Replace Document",
                    isLoading = isSubmitting,
                    enabled = !isSubmitting && isOwner && manager != null &&
                        documentsMutable,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("replaceDocument.submitButton"),
                ) {
                    val owner = actingIdentity ?: return@SubmitButton
                    val docIdBytes = Base58.decodeIdentifier(documentIdText.trim())
                        ?: return@SubmitButton
                    val propertiesJson = try {
                        // Rust-side replace is a FULL overwrite
                        // (`set_properties`), and the form can neither
                        // render composite (object/array) values nor
                        // distinguish "left blank" from "clear": overlay
                        // the form's values on the document's current
                        // fields so everything not re-entered keeps its
                        // existing value instead of being silently dropped.
                        mergeReplaceProperties(
                            probed?.fields,
                            buildPropertiesJson(
                                properties, required, textValues, boolValues, touchedBools,
                            ),
                            // Explicit removals: seeded non-blank, now blank.
                            clearedKeys = seededTexts.keys.filter { key ->
                                seededTexts[key].orEmpty().isNotBlank() &&
                                    textValues[key].orEmpty().isBlank()
                            }.toSet(),
                        )
                    } catch (e: Exception) {
                        error = "Could not encode document fields: ${e.message}"
                        return@SubmitButton
                    }
                    submitDocumentTransaction(
                        begin = { isSubmitting = true; replaceSuccess = null },
                        end = { isSubmitting = false },
                        fail = { error = it },
                        scope = scope,
                    ) {
                        val (wallet, mgr, signingKeyId) = resolveSigning(container, owner)
                        val json = mgr.documentTransactions.replace(
                            walletHandle = wallet,
                            ownerId = owner.identityId,
                            contractId = contractIdBytes,
                            documentType = typeName,
                            documentId = docIdBytes,
                            propertiesJson = propertiesJson,
                            signingKeyId = signingKeyId,
                            signerHandle = mgr.signerHandle,
                        )
                        replaceSuccess =
                            "Replaced on-chain (${parseDocumentId(json) ?: "id in JSON"})."
                        probeTick++
                    }
                }
            }

            // ── Delete (DOC-04) ───────────────────────────────────────────
            FormSection(title = "Delete") {
                Text(
                    "Permanently remove this document. This cannot be undone.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.testTag("documentActions.delete"),
                )
                deleteSuccess?.let {
                    Text(
                        it,
                        color = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.testTag("deleteDocument.success"),
                    )
                }
                if (!canBeDeleted) {
                    Text(
                        "This document type cannot be deleted — a delete will be " +
                            "rejected by consensus (fees are still charged).",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
                SubmitButton(
                    text = "Delete Document…",
                    isLoading = false,
                    enabled = !isSubmitting && isOwner && manager != null &&
                        deleteSuccess == null && canBeDeleted,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("deleteDocument.button"),
                ) { showDeleteConfirm = true }
            }

            // ── Transfer (DOC-05) ─────────────────────────────────────────
            FormSection(title = "Transfer") {
                Text(
                    "Send this document to another identity.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.testTag("documentActions.transfer"),
                )
                if (!isTransferable) {
                    Text(
                        "This document type is not marked transferable — a transfer " +
                            "will be rejected by consensus.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
                RecipientPicker(
                    selection = recipient,
                    onSelectionChange = { recipient = it },
                    networkRaw = network.ffiValue,
                    excludeIdentityIdHex = actingIdentity?.identityId?.toHex(),
                )
                transferSuccess?.let {
                    Text(
                        it,
                        color = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.testTag("transferDocument.success"),
                    )
                }
                SubmitButton(
                    text = "Transfer Document",
                    isLoading = isSubmitting,
                    enabled = !isSubmitting && isOwner && manager != null &&
                        recipient != null && isTransferable,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("transferDocument.submitButton"),
                ) {
                    val owner = actingIdentity ?: return@SubmitButton
                    val recipientSel = recipient ?: return@SubmitButton
                    val docIdBytes = Base58.decodeIdentifier(documentIdText.trim())
                        ?: return@SubmitButton
                    val recipientBytes = recipientSel.identityIdHex.hexToBytes()
                    submitDocumentTransaction(
                        begin = { isSubmitting = true; transferSuccess = null },
                        end = { isSubmitting = false },
                        fail = { error = it },
                        scope = scope,
                    ) {
                        val (wallet, mgr, signingKeyId) = resolveSigning(container, owner)
                        mgr.documentTransactions.transfer(
                            walletHandle = wallet,
                            ownerId = owner.identityId,
                            contractId = contractIdBytes,
                            documentType = typeName,
                            documentId = docIdBytes,
                            recipientId = recipientBytes,
                            signingKeyId = signingKeyId,
                            signerHandle = mgr.signerHandle,
                        )
                        transferSuccess = "Transferred to ${recipientSel.label}."
                        probeTick++
                    }
                }
            }
        }
    }

    if (showDeleteConfirm) {
        AlertDialog(
            onDismissRequest = { showDeleteConfirm = false },
            title = { Text("Delete document?") },
            text = {
                Text("This permanently removes the document ${truncateMiddle(documentIdText.trim(), 8, 6)}.")
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        showDeleteConfirm = false
                        val owner = actingIdentity ?: return@TextButton
                        val docIdBytes = Base58.decodeIdentifier(documentIdText.trim())
                            ?: return@TextButton
                        submitDocumentTransaction(
                            begin = { isSubmitting = true; deleteSuccess = null },
                            end = { isSubmitting = false },
                            fail = { error = it },
                            scope = scope,
                        ) {
                            val (wallet, mgr, signingKeyId) = resolveSigning(container, owner)
                            mgr.documentTransactions.delete(
                                walletHandle = wallet,
                                ownerId = owner.identityId,
                                contractId = contractIdBytes,
                                documentType = typeName,
                                documentId = docIdBytes,
                                signingKeyId = signingKeyId,
                                signerHandle = mgr.signerHandle,
                            )
                            deleteSuccess = "Deleted on-chain."
                            probed = null
                        }
                    },
                    modifier = Modifier.testTag("deleteDocument.confirm"),
                ) { Text("Delete") }
            },
            dismissButton = {
                TextButton(onClick = { showDeleteConfirm = false }) { Text("Cancel") }
            },
        )
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

/** The owner + current field values [DocumentActionsScreen]'s probe reads. */
private data class ProbedActionDocument(
    val ownerBase58: String?,
    /** The document's populated property values (for the replace prefill). */
    val fields: JsonObject,
)

/**
 * Read `$ownerId` and the document's field values off the fetched canonical
 * JSON. Fields live either nested under `data` or at the root alongside the
 * `$`-prefixed system fields, depending on the query surface — prefer `data`,
 * else strip the system fields from the root.
 */
private fun parseActionDocument(json: String): ProbedActionDocument? = try {
    val root = LenientJson.parseToJsonElement(json).jsonObject
    val data = root["data"] as? JsonObject
    val owner = ((root["\$ownerId"] ?: root["ownerId"]) as? JsonPrimitive)?.content
    val fields = data ?: JsonObject(
        root.filterKeys { !it.startsWith("$") && it != "id" && it != "ownerId" },
    )
    ProbedActionDocument(ownerBase58 = owner, fields = fields)
} catch (_: Exception) {
    null
}

/**
 * Overlay the replace form's encoded values on the document's existing
 * (system-stripped) fields. `set_properties` on the Rust side replaces the
 * whole property map, so a field absent from the form output — composite
 * values the form can't represent, or a field that was never populated —
 * carries over from [existing]; a field the user explicitly cleared
 * ([clearedKeys]) is dropped, making preserve / replace / remove three
 * distinct outcomes.
 */
private fun mergeReplaceProperties(
    existing: JsonObject?,
    formJson: String,
    clearedKeys: Set<String> = emptySet(),
): String {
    if (existing == null) return formJson
    val form = LenientJson.parseToJsonElement(formJson).jsonObject
    return JsonObject(
        existing.toMutableMap().apply {
            // Tri-state: a key the user explicitly CLEARED (seeded
            // non-blank → blank) is removed from the replacement; a key the
            // form simply can't represent (composites) or never seeded
            // stays preserved; a filled key is overwritten below.
            clearedKeys.forEach { remove(it) }
            putAll(form)
        },
    ).toString()
}

/**
 * Prefill the replace form's field state from the document's current scalar
 * values, so a replace starts from the existing content. Only JSON-primitive
 * values are seeded (objects / arrays are left blank for the user to re-enter,
 * since their canonical encoding may not round-trip through the string form).
 */
private fun seedReplaceFields(
    fields: JsonObject,
    properties: JsonObject,
    textValues: MutableMap<String, String>,
    boolValues: MutableMap<String, Boolean>,
    touchedBools: MutableSet<String>,
) {
    for ((name, propEl) in properties) {
        val prop = propEl as? JsonObject ?: continue
        val value = fields[name] as? JsonPrimitive ?: continue
        if (prop.stringField("type") == "boolean") {
            value.content.toBooleanStrictOrNull()?.let {
                boolValues[name] = it
                touchedBools.add(name)
            }
        } else {
            textValues[name] = value.content
        }
    }
}
