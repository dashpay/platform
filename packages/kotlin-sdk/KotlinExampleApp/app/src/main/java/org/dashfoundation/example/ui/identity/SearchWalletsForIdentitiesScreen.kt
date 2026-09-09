package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.toHex

/**
 * Re-scan a wallet's identity-authentication tree for forgotten identities —
 * port of `SearchWalletsForIdentitiesView.swift`. Runs the single FFI
 * gap-limit scan `platform_wallet_discover_identities`; on an empty result it
 * shows the derivation-path keys that were probed
 * (`platform_wallet_preview_identity_registration_keys`), matching the Swift
 * "here are the keys we scanned for" panel. The FFI scan is synchronous
 * (Rust-side gap-limit walk); no streaming progress is reported by the entry
 * point, so this shows a spinner rather than per-slot progress.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SearchWalletsForIdentitiesScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()

    val walletsMap by remember(manager) {
        manager?.wallets ?: MutableStateFlow(emptyMap<String, ManagedPlatformWallet>())
    }.collectAsStateWithLifecycle()
    val wallets = remember(walletsMap) { walletsMap.values.toList() }
    var selected by remember(wallets) { mutableStateOf(wallets.firstOrNull()) }

    var isSearching by remember { mutableStateOf(false) }
    var summary by remember { mutableStateOf<String?>(null) }
    var previewPaths by remember { mutableStateOf<List<String>>(emptyList()) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Search Wallets") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text(
                "Scans a wallet's identity-authentication tree for identities " +
                    "registered but not yet loaded on this device.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            FormSection(title = "Wallet") {
                if (wallets.isEmpty()) {
                    Text(
                        "No wallets on ${network.displayName}.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    AccessiblePicker(
                        label = "Wallet",
                        options = wallets,
                        selected = selected ?: wallets.first(),
                        optionLabel = { it.walletIdHex.take(12) + "…" },
                        testTag = "searchWallets.picker",
                        onSelected = { selected = it },
                    )
                }
            }

            summary?.let {
                FormSection(title = "Result") {
                    Text(it, modifier = Modifier.testTag("searchWallets.summary"))
                    if (previewPaths.isNotEmpty()) {
                        Text(
                            "Keys probed:",
                            style = MaterialTheme.typography.labelMedium,
                            modifier = Modifier.padding(top = 8.dp),
                        )
                        previewPaths.forEach { path ->
                            Text(path, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace)
                        }
                    }
                }
            }

            SubmitButton(
                text = "Search",
                isLoading = isSearching,
                enabled = selected != null,
                modifier = Modifier.fillMaxWidth().testTag("searchWallets.submit"),
            ) {
                val wallet = selected ?: return@SubmitButton
                val mgr = manager ?: return@SubmitButton
                isSearching = true
                summary = null
                previewPaths = emptyList()
                scope.launch {
                    try {
                        val found = mgr.identityRegistration.discoverIdentities(
                            walletHandle = wallet.handle,
                            mnemonicResolverHandle = mgr.mnemonicResolverHandle,
                        )
                        summary = "Found ${found.size} identity(ies)."
                        if (found.isEmpty()) {
                            previewPaths = mgr.identityRegistration.previewRegistrationKeys(
                                walletHandle = wallet.handle,
                                mnemonicResolverHandle = mgr.mnemonicResolverHandle,
                                startIndex = 0,
                                count = -1,
                            ).map { it.derivationPath }
                        } else {
                            // Discovered identities are folded into Rust's
                            // IdentityManager and land in Room via the persister.
                            summary += " " + found.joinToString(", ") { it.toHex().take(12) + "…" }
                        }
                    } catch (e: Exception) {
                        summary = "Search failed: ${e.message}"
                    } finally {
                        isSearching = false
                    }
                }
            }
        }
    }
}
