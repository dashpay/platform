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
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * Simplified "Register Contract" source screen — port of the manual /
 * JSON-editor branch of `RegisterContractSourceView.swift`.
 *
 * A JSON editor (documents schema + optional token schemas / keywords /
 * description) that broadcasts a data-contract-create for [identityIdHex]
 * via `ManagedPlatformWallet.dataContracts.create` →
 * `IdentityNative.createDataContract` →
 * `platform_wallet_create_data_contract_with_signer`. The full Swift view
 * additionally offers pasteboard / QR / quick-token sources; those routes
 * live elsewhere (`QuickBasicTokenScreen`) — this screen covers the manual
 * JSON path, which is the one that exercises the create bridge directly.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RegisterContractSourceScreen(
    identityIdHex: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()

    val identityId = remember(identityIdHex) { identityIdHex.hexToBytes() }
    val identityFlow = remember(identityIdHex) {
        container.database.identityDao().observeByIdentityId(identityId)
    }
    val identity by identityFlow.collectAsStateWithLifecycle(initialValue = null)

    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by (
        manager?.wallets
            ?: remember { MutableStateFlow(emptyMap<String, ManagedPlatformWallet>()) }
        ).collectAsStateWithLifecycle()
    val wallet = identity?.walletId?.toHex()?.let { walletsMap[it] }

    var documentsJson by rememberSaveable { mutableStateOf("{}") }
    var tokensJson by rememberSaveable { mutableStateOf("") }
    var keywords by rememberSaveable { mutableStateOf("") }
    var description by rememberSaveable { mutableStateOf("") }

    var error by remember { mutableStateOf<String?>(null) }
    var isSubmitting by remember { mutableStateOf(false) }
    var createdContractIdHex by remember { mutableStateOf<String?>(null) }

    // At least one of documents (non-"{}") or tokens must carry content —
    // mirrors the Swift "at least one schema" guard.
    val hasDocuments = documentsJson.trim().let { it.isNotEmpty() && it != "{}" }
    val hasTokens = tokensJson.trim().isNotEmpty()
    val canSubmit = wallet != null && (hasDocuments || hasTokens) && !isSubmitting

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Register Contract") },
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
            FormSection(title = "Owner") {
                LabeledContent("Identity", identityIdHex)
                LabeledContent(
                    "Wallet",
                    if (wallet != null) "loaded" else "not loaded",
                )
            }

            FormSection(title = "Documents Schema (JSON)") {
                OutlinedTextField(
                    value = documentsJson,
                    onValueChange = { documentsJson = it },
                    label = { Text("documentSchemas") },
                    textStyle = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .testTag("registerContract.documentsJson"),
                )
                Text(
                    "Use {} for a token-only contract.",
                    style = MaterialTheme.typography.bodySmall,
                )
            }

            FormSection(title = "Token Schemas (JSON, optional)") {
                OutlinedTextField(
                    value = tokensJson,
                    onValueChange = { tokensJson = it },
                    label = { Text("tokenSchemas") },
                    textStyle = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .testTag("registerContract.tokensJson"),
                )
            }

            FormSection(title = "Metadata (Optional)") {
                OutlinedTextField(
                    value = keywords,
                    onValueChange = { keywords = it },
                    label = { Text("Keywords (comma separated)") },
                    singleLine = true,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .testTag("registerContract.keywords"),
                )
                OutlinedTextField(
                    value = description,
                    onValueChange = { description = it },
                    label = { Text("Description") },
                    singleLine = true,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .testTag("registerContract.description"),
                )
            }

            SubmitButton(
                text = "Register",
                isLoading = isSubmitting,
                enabled = canSubmit,
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("registerContract.submitButton"),
            ) {
                val activeManager = manager
                val w = wallet
                if (activeManager == null || w == null) {
                    error = "Wallet is not loaded for this identity."
                    return@SubmitButton
                }
                isSubmitting = true
                scope.launch {
                    try {
                        val contractId = w.dataContracts.create(
                            ownerIdentityId = identityId,
                            documentsSchemaJson = documentsJson.trim().ifEmpty { "{}" },
                            tokensSchemaJson = tokensJson.trim().ifEmpty { null },
                            keywordsJson = keywordsAsJsonArray(keywords),
                            description = description.trim().ifEmpty { null },
                            signerHandle = activeManager.signerHandle,
                        )
                        createdContractIdHex = contractId.toHex()
                    } catch (t: Throwable) {
                        error = t.message ?: "Failed to register the contract."
                    } finally {
                        isSubmitting = false
                    }
                }
            }
        }
    }

    createdContractIdHex?.let { idHex ->
        AlertDialog(
            onDismissRequest = {
                createdContractIdHex = null
                navController.popBackStack()
            },
            title = { Text("Contract Registered") },
            text = {
                Text(
                    "Contract id:\n$idHex",
                    modifier = Modifier.testTag("registerContract.successId"),
                )
            },
            confirmButton = {
                TextButton(onClick = {
                    createdContractIdHex = null
                    navController.popBackStack()
                }) { Text("Done") }
            },
        )
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

/**
 * Turn a comma-separated keyword string into a JSON string array
 * (`["a","b"]`), or null when empty — the shape
 * `platform_wallet_create_data_contract_with_signer` expects for its
 * `keywords_json` argument.
 */
private fun keywordsAsJsonArray(raw: String): String? {
    val items = raw.split(",").map { it.trim() }.filter { it.isNotEmpty() }
    if (items.isEmpty()) return null
    return items.joinToString(prefix = "[", postfix = "]") { kw ->
        "\"" + kw.replace("\\", "\\\\").replace("\"", "\\\"") + "\""
    }
}
