package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
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
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.ContractDetail
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.truncateMiddle

/**
 * Fetch-a-contract screen — port of `LocalDataContractsView.swift` with the
 * `LoadDataContractView` sheet folded in as an inline form (Compose pushes
 * one destination instead of presenting a sheet). Well-known contract
 * buttons come from the `exampleContracts` list in
 * LocalDataContractsView.swift:212-219.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LocalDataContractsScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()

    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val sdk by appState.sdk.collectAsStateWithLifecycle()

    val contractsFlow = remember(network) {
        container.database.dataContractDao().observeByNetwork(network.ffiValue)
    }
    val contracts by contractsFlow.collectAsStateWithLifecycle(initialValue = emptyList())

    var inputId by rememberSaveable { mutableStateOf("") }
    var contractName by rememberSaveable { mutableStateOf("") }
    var isLoading by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var fetchedSummary by remember { mutableStateOf<String?>(null) }

    fun load(id: String, name: String?) {
        val currentSdk = sdk
        if (currentSdk == null) {
            errorMessage = "SDK not initialized"
            return
        }
        scope.launch {
            isLoading = true
            fetchedSummary = null
            try {
                val result = ContractDownloader.downloadAndPersistContract(
                    contractIdBase58 = id,
                    suggestedName = name,
                    sdk = currentSdk,
                    database = container.database,
                    network = network,
                )
                if (result.alreadyExisted) {
                    errorMessage = "This contract is already saved locally"
                } else {
                    fetchedSummary = "Saved ${result.contract.name}"
                }
            } catch (e: ContractDownloader.ContractNotFoundException) {
                errorMessage = "Contract not found on ${network.displayName}. This contract " +
                    "may exist on a different network or the ID may be incorrect."
            } catch (e: Exception) {
                errorMessage = "Failed to load contract: ${e.message}"
            } finally {
                isLoading = false
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Local Data Contracts") },
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
            FormSection(title = "Contract Details") {
                OutlinedTextField(
                    value = inputId,
                    onValueChange = { inputId = it },
                    label = { Text("Contract ID (Base58)") },
                    singleLine = true,
                    enabled = !isLoading,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("localContracts.contractIdField"),
                )
                OutlinedTextField(
                    value = contractName,
                    onValueChange = { contractName = it },
                    label = { Text("Name (Optional)") },
                    singleLine = true,
                    enabled = !isLoading,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 8.dp)
                        .testTag("localContracts.nameField"),
                )
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 8.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    SubmitButton(
                        text = "Load",
                        isLoading = isLoading,
                        enabled = inputId.isNotBlank(),
                        modifier = Modifier.testTag("localContracts.loadButton"),
                    ) {
                        load(inputId, contractName.takeIf { it.isNotBlank() })
                    }
                    if (isLoading) {
                        Text(
                            "Loading contract from network…",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                Text(
                    "Connected to: ${network.displayName}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                fetchedSummary?.let {
                    Text(
                        it,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                }
            }

            FormSection(title = "Common System Contracts (${network.displayName})") {
                KNOWN_CONTRACTS.forEach { (name, id) ->
                    ListItem(
                        headlineContent = { Text(name, style = MaterialTheme.typography.bodyMedium) },
                        supportingContent = {
                            Text(
                                truncateMiddle(id, head = 12, tail = 8),
                                style = MaterialTheme.typography.bodySmall,
                            )
                        },
                        modifier = Modifier
                            .clickable(enabled = !isLoading) {
                                inputId = id
                                contractName = name
                                load(id, name)
                            }
                            .testTag("localContracts.example.$name"),
                    )
                }
            }

            FormSection(title = "Saved Contracts (${contracts.size})") {
                if (contracts.isEmpty()) {
                    Text(
                        "Load data contracts from the network to use them offline",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(vertical = 8.dp),
                    )
                } else {
                    contracts.sortedByDescending { it.lastAccessedAt }.forEach { contract ->
                        Card(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(vertical = 4.dp)
                                .clickable {
                                    navController.navigate(ContractDetail(contract.id.toHex()))
                                },
                        ) {
                            ListItem(
                                headlineContent = { Text(contract.name) },
                                supportingContent = {
                                    Text(
                                        Base58.encode(contract.id),
                                        style = MaterialTheme.typography.bodySmall,
                                        maxLines = 1,
                                    )
                                },
                                trailingContent = {
                                    IconButton(onClick = {
                                        scope.launch {
                                            try {
                                                container.database.dataContractDao()
                                                    .delete(contract)
                                            } catch (e: Exception) {
                                                errorMessage =
                                                    "Failed to delete contract: ${e.message}"
                                            }
                                        }
                                    }) {
                                        Icon(
                                            Icons.Default.Delete,
                                            contentDescription = "Delete ${contract.name}",
                                            modifier = Modifier.size(20.dp),
                                        )
                                    }
                                },
                            )
                        }
                    }
                }
            }

            if (isLoading) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.Center,
                ) {
                    CircularProgressIndicator(modifier = Modifier.size(24.dp), strokeWidth = 2.dp)
                }
            }
        }
    }

    ErrorAlertDialog(message = errorMessage, onDismiss = { errorMessage = null })
}

/**
 * Well-known testnet system contracts — verbatim from
 * `LoadDataContractView.exampleContracts` (LocalDataContractsView.swift:212-219).
 */
private val KNOWN_CONTRACTS = listOf(
    "DPNS Contract" to "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
    "DashPay Contract" to "Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7",
    "Withdrawals Contract" to "4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6",
    "Wallet Utils" to "7CSFGeF4WNzgDmx94zwvHkYaG3Dx4XEe5LFsFgJswLbm",
    "Token History" to "43gujrzZgXqcKBiScLa4T8XTDnRhenR9BLx8GWVHjPxF",
    "Keyword Search" to "BsjE6tQxG47wffZCRQCovFx5rYrAYYC3rTVRWKro27LA",
)
