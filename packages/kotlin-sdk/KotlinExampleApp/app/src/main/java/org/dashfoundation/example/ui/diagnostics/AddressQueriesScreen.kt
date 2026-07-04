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
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.truncateMiddle

/**
 * Platform-address balance/nonce queries — port of
 * `AddressQueriesView.swift` (single + batch address info forms). The
 * backing reads (`dash_sdk_address_fetch_info`,
 * `dash_sdk_addresses_fetch_infos` in rs-sdk-ffi) are not bridged into
 * the JNI shim yet, so Execute surfaces the named-missing-export dialog.
 * What IS live: the locally-synced Platform addresses (Room
 * `platform_addresses`, written by BLAST address sync) render below the
 * forms so the screen is still a useful address browser.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AddressQueriesScreen(navController: NavHostController) {
    val container = LocalAppContainer.current

    val localAddresses by remember {
        container.database.platformAddressDao().observeNonZeroBalances()
    }.collectAsStateWithLifecycle(initialValue = emptyList())

    var singleAddress by rememberSaveable { mutableStateOf("") }
    var batchAddresses by rememberSaveable { mutableStateOf("") }
    var notBridged by remember { mutableStateOf(false) }

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
                    "Fetch balance and nonce for a single Platform address.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                OutlinedTextField(
                    value = singleAddress,
                    onValueChange = { singleAddress = it },
                    label = { Text("Address (bech32m)") },
                    placeholder = { Text("tdash1…") },
                    singleLine = true,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("addressQueries.singleAddress"),
                )
                SubmitButton(
                    text = "Fetch Info",
                    isLoading = false,
                    enabled = singleAddress.isNotBlank(),
                    modifier = Modifier.testTag("addressQueries.fetchInfo"),
                ) {
                    notBridged = true
                }
            }

            FormSection(title = "Get Addresses Infos") {
                Text(
                    "Fetch balance and nonce for multiple Platform addresses.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                OutlinedTextField(
                    value = batchAddresses,
                    onValueChange = { batchAddresses = it },
                    label = { Text("Addresses (comma-separated)") },
                    placeholder = { Text("tdash1…, tdash1…") },
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("addressQueries.batchAddresses"),
                )
                SubmitButton(
                    text = "Fetch Infos",
                    isLoading = false,
                    enabled = batchAddresses.isNotBlank(),
                    modifier = Modifier.testTag("addressQueries.fetchInfos"),
                ) {
                    notBridged = true
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

            FormSection(title = "Note") {
                Text(
                    "Network execution requires the dash_sdk_address_fetch_info / " +
                        "dash_sdk_addresses_fetch_infos JNI exports.",
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }

    if (notBridged) {
        AlertDialog(
            onDismissRequest = { notBridged = false },
            title = { Text("Address Queries Not Available Yet") },
            text = {
                Text(
                    "Fetching Platform address balances/nonces requires the " +
                        "`dash_sdk_address_fetch_info` and " +
                        "`dash_sdk_addresses_fetch_infos` FFI exports (rs-sdk-ffi) " +
                        "to be bridged into the JNI shim. The forms and the local " +
                        "address browser are wired; only the network read is pending.",
                )
            },
            confirmButton = { TextButton(onClick = { notBridged = false }) { Text("OK") } },
        )
    }
}
