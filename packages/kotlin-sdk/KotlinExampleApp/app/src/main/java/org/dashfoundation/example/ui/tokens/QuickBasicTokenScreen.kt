package org.dashfoundation.example.ui.tokens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
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
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.json.JsonUnquotedLiteral
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonObject
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.prettyPrintJson
import org.dashfoundation.example.util.toHex

/**
 * Streamlined register-a-single-token-contract form — port of
 * `QuickBasicTokenView.swift`: singular/plural names, decimals, base /
 * max supply, optional keywords + description, with the same pre-flight
 * validation, and the identical three-level `$formatVersion: "0"`
 * `tokenSchemas` JSON synthesis (`synthesizedInputs`).
 *
 * Submit synthesizes the `tokenSchemas` JSON and broadcasts a
 * data-contract-create via `ManagedPlatformWallet.dataContracts.create` →
 * `IdentityNative.createDataContract` →
 * `platform_wallet_create_data_contract_with_signer` — the Kotlin port of
 * the iOS `TransitionDetailView` submit path. The contract is registered
 * under the first wallet-owned identity (the app's current owner), with an
 * empty `documentSchemas` (`{}`) since this is a token-only contract.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun QuickBasicTokenScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val network by appState.currentNetwork.collectAsStateWithLifecycle()

    val ownedIdentitiesFlow = remember(network) {
        container.database.identityDao().observeWalletOwnedByNetwork(network.ffiValue)
    }
    val ownedIdentities by ownedIdentitiesFlow.collectAsStateWithLifecycle(initialValue = emptyList())
    val owner = ownedIdentities.firstOrNull()

    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by (
        manager?.wallets
            ?: remember { MutableStateFlow(emptyMap<String, ManagedPlatformWallet>()) }
        ).collectAsStateWithLifecycle()
    val ownerWallet = owner?.walletId?.toHex()?.let { walletsMap[it] }
    var singular by rememberSaveable { mutableStateOf("Coin") }
    var plural by rememberSaveable { mutableStateOf("Coins") }
    var decimals by rememberSaveable { mutableStateOf("8") }
    var baseSupply by rememberSaveable { mutableStateOf("1000000") }
    var maxSupply by rememberSaveable { mutableStateOf("") }
    var shouldCapitalize by rememberSaveable { mutableStateOf(true) }
    var keywords by rememberSaveable { mutableStateOf("") }
    var description by rememberSaveable { mutableStateOf("") }

    var validationError by remember { mutableStateOf<String?>(null) }
    var synthesizedJson by remember { mutableStateOf<String?>(null) }
    var isSubmitting by remember { mutableStateOf(false) }
    var createdContractIdHex by remember { mutableStateOf<String?>(null) }
    var submitError by remember { mutableStateOf<String?>(null) }

    // Pre-flight validation — port of `isValid` / `validate()`.
    fun validate(): String? {
        val trimmedSingular = singular.trim()
        val trimmedPlural = plural.trim()
        if (trimmedSingular.isEmpty() || trimmedPlural.isEmpty()) {
            return "Singular and plural names are required."
        }
        val d = decimals.trim().toIntOrNull()
        if (d == null || d < 0 || d > 18) {
            return "Decimals must be a whole number between 0 and 18."
        }
        val base = baseSupply.trim().toULongOrNull()
            ?: return "Base supply must be a whole, non-negative number."
        val trimmedMax = maxSupply.trim()
        if (trimmedMax.isNotEmpty()) {
            val max = trimmedMax.toULongOrNull()
                ?: return "Max supply must be a whole number, or blank for unlimited."
            if (max < base) {
                return "Max supply must be greater than or equal to base supply."
            }
        }
        return null
    }

    val isValid = validate() == null

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Quick Basic Token") },
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
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Names") {
                OutlinedTextField(
                    value = singular,
                    onValueChange = { singular = it },
                    label = { Text("Singular") },
                    singleLine = true,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .testTag("quickToken.singular"),
                )
                OutlinedTextField(
                    value = plural,
                    onValueChange = { plural = it },
                    label = { Text("Plural") },
                    singleLine = true,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .testTag("quickToken.plural"),
                )
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        "Capitalize in display",
                        style = MaterialTheme.typography.bodyMedium,
                        modifier = Modifier.weight(1f),
                    )
                    Switch(
                        checked = shouldCapitalize,
                        onCheckedChange = { shouldCapitalize = it },
                        modifier = Modifier.testTag("quickToken.capitalize"),
                    )
                }
            }

            FormSection(title = "Supply") {
                OutlinedTextField(
                    value = decimals,
                    onValueChange = { decimals = it },
                    label = { Text("Decimals") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .testTag("quickToken.decimals"),
                )
                OutlinedTextField(
                    value = baseSupply,
                    onValueChange = { baseSupply = it },
                    label = { Text("Base Supply") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .testTag("quickToken.baseSupply"),
                )
                OutlinedTextField(
                    value = maxSupply,
                    onValueChange = { maxSupply = it },
                    label = { Text("Max Supply (optional)") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .testTag("quickToken.maxSupply"),
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
                        .testTag("quickToken.keywords"),
                )
                OutlinedTextField(
                    value = description,
                    onValueChange = { description = it },
                    label = { Text("Description") },
                    singleLine = true,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .testTag("quickToken.description"),
                )
            }

            SubmitButton(
                text = "Register",
                isLoading = isSubmitting,
                enabled = isValid && ownerWallet != null && !isSubmitting,
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("quickToken.continueButton"),
            ) {
                val error = validate()
                if (error != null) {
                    validationError = error
                    return@SubmitButton
                }
                validationError = null
                val tokenSchemas = synthesizeTokenSchemas(
                    singular = singular.trim(),
                    plural = plural.trim(),
                    shouldCapitalize = shouldCapitalize,
                    decimals = decimals.trim().toIntOrNull() ?: 8,
                    baseSupply = baseSupply.trim().toULongOrNull() ?: 0u,
                    maxSupply = maxSupply.trim().toULongOrNull(),
                )
                synthesizedJson = tokenSchemas

                val activeManager = manager
                val w = ownerWallet
                val ownerId = owner?.identityId
                if (activeManager == null || w == null || ownerId == null) {
                    submitError = "No wallet-owned identity is available to own the contract."
                    return@SubmitButton
                }
                val kw = keywords.split(",").map { it.trim() }.filter { it.isNotEmpty() }
                val keywordsJson = if (kw.isEmpty()) null else kw.joinToString(
                    prefix = "[", postfix = "]",
                ) { "\"" + it.replace("\\", "\\\\").replace("\"", "\\\"") + "\"" }
                val desc = description.trim().ifEmpty { null }

                isSubmitting = true
                scope.launch {
                    try {
                        val contractId = w.dataContracts.create(
                            ownerIdentityId = ownerId,
                            documentsSchemaJson = "{}",
                            tokensSchemaJson = tokenSchemas,
                            keywordsJson = keywordsJson,
                            description = desc,
                            signerHandle = activeManager.signerHandle,
                        )
                        createdContractIdHex = contractId.toHex()
                    } catch (t: Throwable) {
                        submitError = t.message ?: "Failed to register the token contract."
                    } finally {
                        isSubmitting = false
                    }
                }
            }

            validationError?.let {
                Text(
                    it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.testTag("quickToken.validationError"),
                )
            }

            synthesizedJson?.let { json ->
                FormSection(title = "Synthesized tokenSchemas JSON") {
                    Text(
                        prettyPrintJson(json),
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                        modifier = Modifier.testTag("quickToken.schemaPreview"),
                    )
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
            title = { Text("Token Contract Registered") },
            text = {
                Text(
                    "Contract id:\n$idHex",
                    modifier = Modifier.testTag("quickToken.successId"),
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

    submitError?.let { msg ->
        AlertDialog(
            onDismissRequest = { submitError = null },
            title = { Text("Registration Failed") },
            text = { Text(msg) },
            confirmButton = {
                TextButton(onClick = { submitError = null }) { Text("OK") }
            },
        )
    }
}

/**
 * Build the `tokenSchemas` JSON — port of `synthesizedInputs()`
 * (QuickBasicTokenView.swift:149-204). Every level carries an explicit
 * `$formatVersion: "0"` for serde's tagged-enum dispatch; supplies are
 * emitted as numbers, matching the Swift `JSONSerialization` output.
 */
@OptIn(ExperimentalSerializationApi::class)
internal fun synthesizeTokenSchemas(
    singular: String,
    plural: String,
    shouldCapitalize: Boolean,
    decimals: Int,
    baseSupply: ULong,
    maxSupply: ULong?,
): String = buildJsonObject {
    putJsonObject("0") {
        put("\$formatVersion", "0")
        putJsonObject("conventions") {
            put("\$formatVersion", "0")
            put("decimals", decimals)
            putJsonObject("localizations") {
                putJsonObject("en") {
                    put("\$formatVersion", "0")
                    put("shouldCapitalize", shouldCapitalize)
                    put("singularForm", singular)
                    put("pluralForm", plural)
                }
            }
        }
        put("baseSupply", JsonUnquotedLiteral(baseSupply.toString()))
        maxSupply?.let { put("maxSupply", JsonUnquotedLiteral(it.toString())) }
    }
}.toString()
