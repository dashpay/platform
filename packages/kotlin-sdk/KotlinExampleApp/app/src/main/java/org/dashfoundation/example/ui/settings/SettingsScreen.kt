package org.dashfoundation.example.ui.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent

/**
 * Settings tab root — port of `SettingsView` + `OptionsView.swift`'s
 * Network section. Network switching rebuilds the SDK (new manager
 * instance, never reconfiguration); the picker is disabled while a switch
 * is in flight. Devnet requires a quorum URL, matching the iOS gating.
 *
 * The SPV-peers, Platform-overrides, faucet, and data-management sections
 * attach in B-M1's follow-up as their backing services land.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(navController: NavHostController) {
    val appState = LocalAppState.current
    val container = org.dashfoundation.example.di.LocalAppContainer.current
    val scope = rememberCoroutineScope()

    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val isLoading by appState.isLoading.collectAsStateWithLifecycle()
    // Gate the network picker while a registration holds a slot — switching
    // networks mid-flight tears down the FFI manager and would abort the
    // in-flight call (← RegistrationCoordinator.hasInFlightRegistrations
    // driving the picker's `.disabled(_:)`). Observe the coordinator map so
    // the gate re-evaluates as phases change.
    val controllers by container.registrationCoordinator.controllers.collectAsStateWithLifecycle()
    val registrationInFlight = remember(controllers) {
        controllers.values.any { it.phase.value.isActive }
    }
    val errorMessage by appState.errorMessage.collectAsStateWithLifecycle()
    val useDocker by appState.useDockerSetup.collectAsStateWithLifecycle()

    var quorumUrl by remember { mutableStateOf("") }
    var showAbout by remember { mutableStateOf(false) }

    Scaffold(
        topBar = { TopAppBar(title = { Text("Settings") }) },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Network") {
                AccessiblePicker(
                    label = "Network",
                    options = Network.entries,
                    selected = network,
                    optionLabel = { it.displayName },
                    testTag = "settings.networkPicker",
                ) { selected ->
                    // Ignore selection while a registration is in flight — the
                    // note below explains why (1:1 with the iOS disabled gate).
                    if (registrationInFlight) return@AccessiblePicker
                    scope.launch {
                        appState.switchNetwork(
                            selected,
                            quorumUrl = quorumUrl.takeIf { it.isNotEmpty() },
                        )
                    }
                }
                if (registrationInFlight) {
                    Text(
                        "Network switching is locked while an identity registration is in progress.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.testTag("settings.networkLocked"),
                    )
                }
                if (isLoading) {
                    Text(
                        "Switching network…",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 6.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text("Use local dashmate (Docker)")
                    Switch(
                        checked = useDocker,
                        onCheckedChange = { appState.setUseDockerSetup(it) },
                    )
                }
                if (network == Network.DEVNET) {
                    androidx.compose.material3.OutlinedTextField(
                        value = quorumUrl,
                        onValueChange = { quorumUrl = it },
                        label = { Text("Quorum URL (required for devnet)") },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                }
            }

            // SPV peer override (← OptionsView.swift's "Use Custom SPV
            // Peers" toggle + peers field; read by the Sync tab's Start).
            FormSection(title = "SPV Peers") {
                val useLocalCore by appState.useLocalhostCore.collectAsStateWithLifecycle()
                val localPeers by appState.localCorePeers.collectAsStateWithLifecycle()
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 6.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text("Use custom SPV peers")
                    Switch(
                        checked = useLocalCore,
                        onCheckedChange = { appState.setUseLocalhostCore(it) },
                        modifier = Modifier.testTag("settings.useLocalhostCore"),
                    )
                }
                if (useLocalCore) {
                    androidx.compose.material3.OutlinedTextField(
                        value = localPeers,
                        onValueChange = { appState.setLocalCorePeers(it) },
                        label = { Text("Peers (host:port, comma-separated)") },
                        placeholder = { Text("10.0.2.2:20301") },
                        modifier = Modifier
                            .fillMaxWidth()
                            .testTag("settings.localCorePeers"),
                        singleLine = true,
                    )
                    Text(
                        "On the Android emulator, the host machine is 10.0.2.2 " +
                            "(not 127.0.0.1).",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            // Data section — mirrors OptionsView.swift's Data section
            // (Storage / Keychain / Wallet Memory / Banned Addresses /
            // Manage Local Data).
            FormSection(title = "Data") {
                androidx.compose.material3.TextButton(
                    onClick = { navController.navigate(org.dashfoundation.example.navigation.DataManagement) },
                ) {
                    Text("Data Management")
                }
                androidx.compose.material3.TextButton(
                    onClick = { navController.navigate(org.dashfoundation.example.navigation.StorageExplorer) },
                ) {
                    Text("Storage Explorer")
                }
                androidx.compose.material3.TextButton(
                    onClick = { navController.navigate(org.dashfoundation.example.navigation.KeystoreExplorer) },
                    modifier = Modifier.testTag("settings.keystoreExplorer"),
                ) {
                    Text("Keystore Explorer")
                }
                androidx.compose.material3.TextButton(
                    onClick = { navController.navigate(org.dashfoundation.example.navigation.WalletMemoryExplorer) },
                    modifier = Modifier.testTag("settings.walletMemoryExplorer"),
                ) {
                    Text("Wallet Memory Explorer")
                }
                androidx.compose.material3.TextButton(
                    onClick = { navController.navigate(org.dashfoundation.example.navigation.BannedAddresses) },
                    modifier = Modifier.testTag("settings.bannedAddresses"),
                ) {
                    Text("Banned Addresses")
                }
            }

            // Platform section — mirrors OptionsView.swift's Platform
            // section (Contracts / Queries / State Transitions + SDK
            // status), with Diagnostics as the run-all-queries entry point.
            // Contracts is demoted here from a top-level tab, matching iOS
            // hosting it as the first Platform NavigationLink.
            FormSection(title = "Platform") {
                androidx.compose.material3.TextButton(
                    onClick = { navController.navigate(org.dashfoundation.example.navigation.ContractsHome) },
                    modifier = Modifier.testTag("settings.contracts"),
                ) {
                    Text("Contracts")
                }
                androidx.compose.material3.TextButton(
                    onClick = { navController.navigate(org.dashfoundation.example.navigation.QueriesList) },
                    modifier = Modifier.testTag("settings.queries"),
                ) {
                    Text("Queries")
                }
                androidx.compose.material3.TextButton(
                    onClick = { navController.navigate(org.dashfoundation.example.navigation.StateTransitions) },
                    modifier = Modifier.testTag("settings.stateTransitions"),
                ) {
                    Text("State Transitions")
                }
                androidx.compose.material3.TextButton(
                    onClick = { navController.navigate(org.dashfoundation.example.navigation.Diagnostics) },
                    modifier = Modifier.testTag("settings.diagnostics"),
                ) {
                    Text("Diagnostics")
                }
            }

            FormSection(title = "About") {
                androidx.compose.material3.TextButton(
                    onClick = { showAbout = true },
                    modifier = Modifier.testTag("settings.about"),
                ) {
                    Text("About Dash SDK Example")
                }
                LabeledContent("SDK version", Sdk.version())
                LabeledContent(
                    "Shielded support",
                    if (Sdk.hasShielded()) "Enabled" else "Disabled",
                )
            }
        }
    }

    if (showAbout) {
        AboutSheet(onDismiss = { showAbout = false })
    }

    ErrorAlertDialog(message = errorMessage, onDismiss = { appState.clearError() })
}
