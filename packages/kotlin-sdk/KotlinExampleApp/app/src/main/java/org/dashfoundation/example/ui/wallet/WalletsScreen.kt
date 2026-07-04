package org.dashfoundation.example.ui.wallet

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
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
import kotlinx.coroutines.flow.map
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.WalletDetail

/**
 * Wallets tab root — port of `WalletsContentView.swift`: the persisted
 * wallet list for the current network, with the add-wallet entry point.
 * Rows come straight from the Room Flow (= the SwiftData `@Query`).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WalletsScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()

    val walletsFlow = remember(network) {
        container.database.walletDao().observeByNetwork(network.ffiValue)
    }
    val wallets by walletsFlow.collectAsStateWithLifecycle(initialValue = emptyList())

    Scaffold(
        topBar = { TopAppBar(title = { Text("Wallets") }) },
        floatingActionButton = {
            FloatingActionButton(
                onClick = { navController.navigate(org.dashfoundation.example.navigation.CreateWallet) },
                modifier = Modifier.testTag("wallets.add"),
            ) {
                Icon(Icons.Default.Add, contentDescription = "Create wallet")
            }
        },
    ) { padding ->
        if (wallets.isEmpty()) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterVertically),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text("No Wallets", style = MaterialTheme.typography.titleMedium)
                Text(
                    "Create a wallet to get started on ${network.displayName}.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        } else {
            LazyColumn(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(wallets, key = { it.walletId.toHexString() }) { wallet ->
                    Card(
                        onClick = {
                            navController.navigate(WalletDetail(wallet.walletId.toHexString()))
                        },
                    ) {
                        ListItem(
                            headlineContent = { Text(wallet.name ?: "Wallet") },
                            supportingContent = {
                                Text("Synced height: ${wallet.syncedHeight}")
                            },
                        )
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalStdlibApi::class)
internal fun ByteArray.toHexString(): String = toHexString(HexFormat.Default)
