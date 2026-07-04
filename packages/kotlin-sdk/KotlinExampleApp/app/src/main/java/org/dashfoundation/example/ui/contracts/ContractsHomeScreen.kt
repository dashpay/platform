package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.ContractDetail
import org.dashfoundation.example.navigation.LocalContracts
import org.dashfoundation.example.navigation.QueriesList
import org.dashfoundation.example.navigation.QuickBasicToken
import org.dashfoundation.example.navigation.RegisterContractSource
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.navigation.TokenSearch
import org.dashfoundation.example.navigation.TokensHome
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.formatRelative
import org.dashfoundation.example.util.toHex

/**
 * Contracts tab root — port of `ContractsTabView.swift`: locally-persisted
 * data contracts scoped to the current network. Rows drill into
 * [ContractDetail]; the "+" toolbar action opens the fetch-contract screen
 * ([LocalContracts], ← `LoadDataContractView`), and the list action opens
 * the platform-queries explorer (← the `DocumentsView`/`QueryDetailView`
 * browse affordance in the iOS toolbar).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ContractsHomeScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()

    val contractsFlow = remember(network) {
        container.database.dataContractDao().observeByNetwork(network.ffiValue)
    }
    val contracts by contractsFlow.collectAsStateWithLifecycle(initialValue = emptyList())

    // Wallet-owned identities are the possible contract owners; the manual
    // register screen registers under the first one (matching the Swift
    // flow, which registers under the current identity).
    val ownedIdentitiesFlow = remember(network) {
        container.database.identityDao().observeWalletOwnedByNetwork(network.ffiValue)
    }
    val ownedIdentities by ownedIdentitiesFlow.collectAsStateWithLifecycle(initialValue = emptyList())
    val registerOwnerIdHex = ownedIdentities.firstOrNull()?.identityId?.toHex()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Contracts") },
                actions = {
                    IconButton(
                        onClick = { navController.navigate(QueriesList) },
                        modifier = Modifier.testTag("contracts.browseDocuments"),
                    ) {
                        Icon(Icons.AutoMirrored.Filled.List, contentDescription = "Platform Queries")
                    }
                    IconButton(
                        onClick = { navController.navigate(LocalContracts) },
                        modifier = Modifier.testTag("contracts.addContract"),
                    ) {
                        Icon(Icons.Default.Add, contentDescription = "Add Contract or Token")
                    }
                },
            )
        },
    ) { padding ->
        if (contracts.isEmpty()) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                TokensEntrySection(navController, registerOwnerIdHex)
                Column(
                    modifier = Modifier.padding(top = 24.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Text("No Contracts", style = MaterialTheme.typography.titleMedium)
                    Text(
                        "Fetch a data contract to browse its documents and tokens.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        } else {
            LazyColumn(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentPadding = PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                // Tokens live under the Contracts tab, mirroring how
                // `ContractsTabView.swift` hosts the token surfaces.
                item { TokensEntrySection(navController, registerOwnerIdHex) }
                item {
                    Text(
                        "CONTRACTS",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                items(
                    contracts.sortedByDescending { it.lastAccessedAt },
                    key = { it.id.toHex() },
                ) { contract ->
                    Card(
                        modifier = Modifier
                            .clickable {
                                navController.navigate(ContractDetail(contract.id.toHex()))
                            }
                            .testTag("contracts.row.${Base58.encode(contract.id)}"),
                    ) {
                        ListItem(
                            headlineContent = { Text(contract.name) },
                            supportingContent = {
                                Column {
                                    Text(
                                        Base58.encode(contract.id),
                                        style = MaterialTheme.typography.bodySmall,
                                        maxLines = 1,
                                    )
                                    Text(
                                        "Last used: ${formatRelative(contract.lastAccessedAt)}",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            },
                        )
                    }
                }
            }
        }
    }
}

/**
 * Tokens entry rows on the Contracts tab home — the Android placement
 * of the token surfaces `ContractsTabView.swift` hosts (browse / search
 * / quick register).
 */
@Composable
private fun TokensEntrySection(
    navController: NavHostController,
    registerOwnerIdHex: String?,
) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(
            "TOKENS",
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Card(
            modifier = Modifier
                .clickable { navController.navigate(TokensHome) }
                .testTag("contracts.tokens"),
        ) {
            ListItem(
                headlineContent = { Text("Tokens") },
                supportingContent = { Text("Balances and actions per identity") },
            )
        }
        Card(
            modifier = Modifier
                .clickable { navController.navigate(TokenSearch) }
                .testTag("contracts.tokenSearch"),
        ) {
            ListItem(
                headlineContent = { Text("Token Search") },
                supportingContent = { Text("Filter local tokens by capability") },
            )
        }
        Card(
            modifier = Modifier
                .clickable { navController.navigate(QuickBasicToken) }
                .testTag("contracts.quickBasicToken"),
        ) {
            ListItem(
                headlineContent = { Text("Quick Basic Token") },
                supportingContent = { Text("Register a single-token contract") },
            )
        }
        if (registerOwnerIdHex != null) {
            Card(
                modifier = Modifier
                    .clickable {
                        navController.navigate(RegisterContractSource(registerOwnerIdHex))
                    }
                    .testTag("contracts.registerContract"),
            ) {
                ListItem(
                    headlineContent = { Text("Register Contract (JSON)") },
                    supportingContent = { Text("Broadcast a contract from raw JSON") },
                )
            }
        }
    }
}
