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
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.truncateMiddle

/**
 * Update an existing data contract (DC-04) — the Kotlin counterpart of the
 * iOS contract-update flow (`ManagedPlatformWallet.updateDataContract` →
 * `platform_wallet_update_data_contract_with_signer`). The wallet fetches the
 * live contract, bumps its version, and *merges* the supplied schema sections
 * additively, so omitted document types / tokens / groups keep their on-chain
 * definition.
 *
 * The owner identity is picked on-screen (the contract owner must sign);
 * [prefillContractIdHex] pre-populates the contract-id field when the screen
 * is reached from the transition catalog with a chosen contract.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun UpdateContractScreen(
    prefillContractIdHex: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    val identities by container.database.identityDao()
        .observeWalletOwnedByNetwork(network.ffiValue)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    var owner by remember { mutableStateOf<IdentityEntity?>(null) }
    LaunchedEffect(identities) {
        if (owner == null) owner = identities.firstOrNull()
    }

    // The contract id starts base58 in the field. If we were handed a hex
    // prefill (nav arg), render it as base58 for consistency with the app's
    // other id fields.
    val initialContractId = remember(prefillContractIdHex) {
        prefillContractIdHex.takeIf { it.isNotEmpty() }
            ?.let { runCatching { Base58.encode(it.hexToBytesOrEmpty()) }.getOrNull() }
            ?: ""
    }
    var contractIdText by rememberSaveable(prefillContractIdHex) {
        mutableStateOf(initialContractId)
    }
    var documentsJson by rememberSaveable { mutableStateOf("{}") }
    var tokensJson by rememberSaveable { mutableStateOf("") }
    var groupsJson by rememberSaveable { mutableStateOf("") }
    var keywords by rememberSaveable { mutableStateOf("") }
    var description by rememberSaveable { mutableStateOf("") }

    var error by remember { mutableStateOf<String?>(null) }
    var isSubmitting by remember { mutableStateOf(false) }
    var updatedContractIdHex by remember { mutableStateOf<String?>(null) }

    val contractIdBytes = remember(contractIdText) {
        Base58.decodeIdentifier(contractIdText.trim())
    }
    val hasDocuments = documentsJson.trim().let { it.isNotEmpty() && it != "{}" }
    val hasTokens = tokensJson.trim().isNotEmpty()
    val hasGroups = groupsJson.trim().isNotEmpty()
    // Keywords/description are valid stand-alone updates (the FFI submits
    // them independently of the schema overlays), so a metadata-only edit
    // must be able to enable the button.
    val hasKeywords = keywords.trim().isNotEmpty()
    val hasDescription = description.trim().isNotEmpty()
    val canSubmit = owner != null && manager != null && contractIdBytes != null &&
        (hasDocuments || hasTokens || hasGroups || hasKeywords || hasDescription) &&
        !isSubmitting

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Update Contract") },
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
            FormSection(title = "Contract") {
                OutlinedTextField(
                    value = contractIdText,
                    onValueChange = { contractIdText = it },
                    label = { Text("Data contract ID (base58 or hex)") },
                    singleLine = true,
                    isError = contractIdText.trim().isNotEmpty() && contractIdBytes == null,
                    enabled = !isSubmitting,
                    textStyle = MaterialTheme.typography.bodyMedium.copy(fontFamily = FontFamily.Monospace),
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("updateContract.contractIdField"),
                )
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
                        testTag = "updateContract.ownerPicker",
                        onSelected = { owner = it },
                    )
                    Text(
                        "Must be the contract's owner — the update is signed by this identity.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            FormSection(title = "Document Schemas (JSON)") {
                OutlinedTextField(
                    value = documentsJson,
                    onValueChange = { documentsJson = it },
                    label = { Text("documentSchemas") },
                    enabled = !isSubmitting,
                    textStyle = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .testTag("updateContract.documentsJson"),
                )
                Text(
                    "Merged onto the live contract — new document types are added, " +
                        "existing ones replaced key-by-key. Use {} to leave documents " +
                        "unchanged and only touch tokens / groups.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            FormSection(title = "Token Schemas (JSON, optional)") {
                OutlinedTextField(
                    value = tokensJson,
                    onValueChange = { tokensJson = it },
                    label = { Text("tokenSchemas") },
                    enabled = !isSubmitting,
                    textStyle = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .testTag("updateContract.tokensJson"),
                )
            }

            FormSection(title = "Groups (JSON, optional)") {
                OutlinedTextField(
                    value = groupsJson,
                    onValueChange = { groupsJson = it },
                    label = { Text("groups") },
                    enabled = !isSubmitting,
                    textStyle = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .testTag("updateContract.groupsJson"),
                )
            }

            FormSection(title = "Metadata (Optional)") {
                OutlinedTextField(
                    value = keywords,
                    onValueChange = { keywords = it },
                    label = { Text("Keywords (comma separated)") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .testTag("updateContract.keywords"),
                )
                OutlinedTextField(
                    value = description,
                    onValueChange = { description = it },
                    label = { Text("Description") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .testTag("updateContract.description"),
                )
            }

            SubmitButton(
                text = "Update / Broadcast",
                isLoading = isSubmitting,
                enabled = canSubmit,
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("updateContract.submitButton"),
            ) {
                val ownerId = owner ?: return@SubmitButton
                val mgr = manager ?: return@SubmitButton
                val contractId = contractIdBytes ?: run {
                    error = "Contract ID must be base58 or 64-char hex."
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
                        val id = wallet.dataContracts.update(
                            ownerIdentityId = ownerId.identityId,
                            contractId = contractId,
                            documentsSchemaJson = documentsJson.trim().ifEmpty { "{}" },
                            tokensSchemaJson = tokensJson.trim().ifEmpty { null },
                            groupsSchemaJson = groupsJson.trim().ifEmpty { null },
                            keywordsJson = keywordsAsJsonArray(keywords),
                            description = description.trim().ifEmpty { null },
                            signerHandle = mgr.signerHandle,
                        )
                        updatedContractIdHex = id.toHex()
                    } catch (t: Throwable) {
                        error = t.message ?: "Failed to update the contract."
                    } finally {
                        isSubmitting = false
                    }
                }
            }
        }
    }

    updatedContractIdHex?.let { idHex ->
        AlertDialog(
            onDismissRequest = {
                updatedContractIdHex = null
                navController.popBackStack()
            },
            title = { Text("Contract Updated") },
            text = {
                Text(
                    "Contract id:\n$idHex",
                    modifier = Modifier.testTag("updateContract.success"),
                )
            },
            confirmButton = {
                TextButton(onClick = {
                    updatedContractIdHex = null
                    navController.popBackStack()
                }) { Text("Done") }
            },
        )
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

/** Decode a hex string to bytes, or an empty array if it is not valid hex. */
private fun String.hexToBytesOrEmpty(): ByteArray =
    runCatching {
        val clean = trim()
        require(clean.length % 2 == 0)
        clean.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
    }.getOrElse { ByteArray(0) }

/**
 * Turn a comma-separated keyword string into a JSON string array
 * (`["a","b"]`), or null when empty — the shape the update FFI expects for
 * its `keywords_json` argument.
 */
private fun keywordsAsJsonArray(raw: String): String? {
    val items = raw.split(",").map { it.trim() }.filter { it.isNotEmpty() }
    if (items.isEmpty()) return null
    return items.joinToString(prefix = "[", postfix = "]") { kw ->
        "\"" + kw.replace("\\", "\\\\").replace("\"", "\\\"") + "\""
    }
}
