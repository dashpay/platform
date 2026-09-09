package org.dashfoundation.example.ui.diagnostics

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
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
import kotlinx.coroutines.launch
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.truncateMiddle

/**
 * Platform-address balance/nonce queries — port of
 * `AddressQueriesView.swift`'s single + batch address-info forms
 * (`dash_sdk_address_fetch_info` / `dash_sdk_addresses_fetch_infos`, now
 * bridged as `Sdk.addresses.fetchInfo` / `fetchInfos`).
 *
 * The Kotlin `Addresses` surface takes RAW address bytes (the caller owns
 * decoding), so this screen decodes at the edge. iOS accepts bech32m
 * (tdashevo1…) and hex; bech32m decoding is not yet ported to Kotlin, so
 * this screen accepts **hex** (21 bytes = 42 hex chars, e.g. a `00`
 * type-byte prefix + 20-byte hash — the iOS "Use Hex Test" fixtures). A
 * bech32m address is rejected with a clear message rather than mis-decoded.
 *
 * What also stays live: the locally-synced Platform addresses (Room
 * `platform_addresses`, written by BLAST address sync) render below the
 * forms so the screen doubles as an address browser.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AddressQueriesScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()

    val sdk by appState.sdk.collectAsStateWithLifecycle()

    val localAddresses by remember {
        container.database.platformAddressDao().observeNonZeroBalances()
    }.collectAsStateWithLifecycle(initialValue = emptyList())

    var singleAddress by rememberSaveable { mutableStateOf("") }
    var batchAddresses by rememberSaveable { mutableStateOf("") }

    var singleLoading by remember { mutableStateOf(false) }
    var batchLoading by remember { mutableStateOf(false) }
    var singleResult by remember { mutableStateOf<String?>(null) }
    var batchResult by remember { mutableStateOf<String?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Address Queries") },
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
            FormSection(title = "Get Address Info") {
                Text(
                    "Fetch balance and nonce for a single Platform address " +
                        "(hex, 42 chars = 21 bytes).",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                OutlinedTextField(
                    value = singleAddress,
                    onValueChange = { singleAddress = it },
                    label = { Text("Address (hex)") },
                    placeholder = { Text("001234…") },
                    singleLine = true,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("addressQueries.singleAddress"),
                )
                SubmitButton(
                    text = "Fetch Info",
                    isLoading = singleLoading,
                    enabled = singleAddress.isNotBlank() && sdk != null && !singleLoading,
                    modifier = Modifier.testTag("addressQueries.fetchInfo"),
                ) {
                    val currentSdk = sdk ?: return@SubmitButton
                    val bytes = decodeHexAddress(singleAddress)
                    if (bytes == null) {
                        singleResult = INVALID_HEX_MESSAGE
                        return@SubmitButton
                    }
                    scope.launch {
                        singleLoading = true
                        singleResult = runCatching { currentSdk.addresses.fetchInfo(bytes) }
                            .fold(
                                onSuccess = { it ?: "Address not found on Platform." },
                                onFailure = { it.message ?: "Query failed." },
                            )
                        singleLoading = false
                    }
                }
                singleResult?.let { result ->
                    LabeledContent("Result", "")
                    Text(
                        result,
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                        modifier = Modifier.testTag("addressQueries.singleResult"),
                    )
                }
            }

            FormSection(title = "Get Addresses Infos") {
                Text(
                    "Fetch balance and nonce for multiple Platform addresses " +
                        "(one hex address per line).",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                OutlinedTextField(
                    value = batchAddresses,
                    onValueChange = { batchAddresses = it },
                    label = { Text("Addresses (one per line)") },
                    placeholder = { Text("001234…\n00abcd…") },
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("addressQueries.batchAddresses"),
                )
                SubmitButton(
                    text = "Fetch Infos",
                    isLoading = batchLoading,
                    enabled = batchAddresses.isNotBlank() && sdk != null && !batchLoading,
                    modifier = Modifier.testTag("addressQueries.fetchInfos"),
                ) {
                    val currentSdk = sdk ?: return@SubmitButton
                    val lines = batchAddresses.split("\n", ",")
                        .map { it.trim() }.filter { it.isNotEmpty() }
                    val decoded = lines.map { decodeHexAddress(it) }
                    if (decoded.any { it == null }) {
                        batchResult = INVALID_HEX_MESSAGE
                        return@SubmitButton
                    }
                    val bytes = decoded.filterNotNull()
                    scope.launch {
                        batchLoading = true
                        batchResult = runCatching { currentSdk.addresses.fetchInfos(bytes) }
                            .fold(
                                onSuccess = { it ?: "No address info returned." },
                                onFailure = { it.message ?: "Query failed." },
                            )
                        batchLoading = false
                    }
                }
                batchResult?.let { result ->
                    LabeledContent("Result", "")
                    Text(
                        result,
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                        modifier = Modifier.testTag("addressQueries.batchResult"),
                    )
                }
            }

            FormSection(title = "Local Addresses (${localAddresses.size})") {
                Text(
                    "Platform addresses with a non-zero balance in the local " +
                        "store (written by BLAST address sync).",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                if (localAddresses.isEmpty()) {
                    Text(
                        "No synced addresses with balance.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(top = 8.dp),
                    )
                } else {
                    localAddresses.forEach { entry ->
                        LabeledContent(
                            label = truncateMiddle(entry.address, 14, 8),
                            value = "${entry.balance} credits",
                        )
                    }
                }
            }
        }
    }
}

private const val INVALID_HEX_MESSAGE =
    "Enter each address as hex (21 bytes = 42 hex chars). bech32m " +
        "(tdashevo1…) input is not yet supported on Android."

/**
 * Decode a hex-encoded Platform address into raw bytes. Returns null unless
 * the input is an even-length hex string of 21 bytes (the type byte + 20-byte
 * hash the address queries expect).
 */
private fun decodeHexAddress(input: String): ByteArray? {
    val trimmed = input.trim()
    if (trimmed.length != 42 || trimmed.length % 2 != 0) return null
    if (!trimmed.all { it.isDigit() || it in 'a'..'f' || it in 'A'..'F' }) return null
    return runCatching {
        trimmed.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
    }.getOrNull()
}
