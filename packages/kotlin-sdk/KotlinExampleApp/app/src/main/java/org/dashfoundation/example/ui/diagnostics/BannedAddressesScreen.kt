package org.dashfoundation.example.ui.diagnostics

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection

/**
 * Read-only DAPI address ban list — port of `BannedAddressesView.swift`.
 * The iOS view renders the snapshot from
 * `PlatformWalletManager.addressBanInfo()`
 * (`platform_wallet_manager_address_ban_info` in platform-wallet-ffi);
 * that export is not bridged into the JNI shim yet, so the list can never
 * populate here. The screen keeps the iOS empty-state semantics (an empty
 * list already legitimately means "no bans or unseeded pool") and the
 * Refresh affordance surfaces the named-missing-export dialog so the gap
 * is explicit rather than silent.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BannedAddressesScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    var notBridged by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Banned Addresses") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = { notBridged = true },
                        modifier = Modifier.testTag("bannedAddresses.refresh"),
                    ) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
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
            FormSection(title = "Addresses (0)") {
                Text(
                    "No banned addresses.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.testTag("bannedAddresses.empty"),
                )
                Text(
                    "This list reflects the current SDK session. An empty list " +
                        "can mean either that no DAPI addresses have been banned, " +
                        "or that the address pool has not yet been seeded.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 8.dp),
                )
            }

            FormSection(title = "Session") {
                Text(
                    if (manager == null) {
                        "Wallet manager not active — no SDK session to inspect."
                    } else {
                        "Wallet manager active on ${manager?.network?.displayName}."
                    },
                    style = MaterialTheme.typography.bodyMedium,
                )
            }

            FormSection(title = "Note") {
                Text(
                    "Reading the Rust-side ban state requires the " +
                        "platform_wallet_manager_address_ban_info JNI export.",
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
            title = { Text("Ban List Not Available Yet") },
            text = {
                Text(
                    "Reading the DAPI address ban list requires the " +
                        "`platform_wallet_manager_address_ban_info` FFI " +
                        "(platform-wallet-ffi) to be bridged into the JNI shim. " +
                        "Until then this screen always shows the empty state.",
                )
            },
            confirmButton = { TextButton(onClick = { notBridged = false }) { Text("OK") } },
        )
    }
}
