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
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.navigation.NavHostController
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.security.BiometricGate
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.truncateMiddle
import java.security.KeyStore

/**
 * Read-only secret-store inspector — port of `KeychainExplorerView.swift`
 * onto the Android layout: the `WalletStorage` DataStore entries (masked;
 * ciphertext is never decoded for the listing) plus the AndroidKeyStore
 * aliases that wrap them. One deliberate extension over iOS v1: a
 * mnemonic row's Reveal action, gated through [BiometricGate] — the same
 * gate `SeedPhraseRevealSheet` uses — because Android's explorer doubles
 * as the recovery-phrase surface during UAT. Private-key entries never
 * reveal.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun KeystoreExplorerScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()

    var reloadKey by remember { mutableIntStateOf(0) }
    var entryNames by remember { mutableStateOf<List<String>>(emptyList()) }
    var keystoreAliases by remember { mutableStateOf<List<String>>(emptyList()) }
    var loadError by remember { mutableStateOf<String?>(null) }
    var revealedMnemonic by remember { mutableStateOf<Pair<String, String>?>(null) }
    var revealError by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(reloadKey) {
        loadError = null
        try {
            entryNames = container.walletStorage.listEntryNames()
            keystoreAliases = withContext(Dispatchers.IO) {
                java.util.Collections.list(
                    KeyStore.getInstance("AndroidKeyStore")
                        .apply { load(null) }
                        .aliases(),
                ).sorted()
            }
        } catch (e: Exception) {
            loadError = e.message ?: e.toString()
        }
    }

    val mnemonicEntries = entryNames.filter { it.startsWith("mnemonic.") }
    val privkeyEntries = entryNames.filter { it.startsWith("privkey.") }
    val otherEntries = entryNames.filterNot {
        it.startsWith("mnemonic.") || it.startsWith("privkey.")
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Keystore Explorer") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = { reloadKey++ },
                        modifier = Modifier.testTag("keystoreExplorer.refresh"),
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
            loadError?.let { message ->
                FormSection(title = "Error") {
                    Text(
                        message,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("keystoreExplorer.error"),
                    )
                }
            }

            FormSection(title = "Per-Wallet Mnemonics (${mnemonicEntries.size})") {
                if (mnemonicEntries.isEmpty()) {
                    Text(
                        "No stored mnemonics.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                mnemonicEntries.forEach { entry ->
                    val walletIdHex = entry.removePrefix("mnemonic.")
                    ListItem(
                        headlineContent = { Text("Wallet ${truncateMiddle(walletIdHex, 6, 6)}") },
                        supportingContent = {
                            Text(
                                entry,
                                style = MaterialTheme.typography.bodySmall,
                                fontFamily = FontFamily.Monospace,
                            )
                        },
                        trailingContent = {
                            TextButton(
                                onClick = {
                                    scope.launch {
                                        revealError = null
                                        val outcome = container.biometricGate.authenticate(
                                            title = "Reveal mnemonic",
                                            subtitle = "Authenticate to decrypt the recovery phrase",
                                        )
                                        if (outcome == BiometricGate.AuthOutcome.AUTHORIZED) {
                                            try {
                                                val phrase = container.walletStorage
                                                    .retrieveMnemonic(walletIdHex.hexToBytes())
                                                if (phrase != null) {
                                                    revealedMnemonic = walletIdHex to phrase
                                                } else {
                                                    revealError = "No mnemonic stored for this wallet."
                                                }
                                            } catch (e: Exception) {
                                                revealError = e.message ?: e.toString()
                                            }
                                        } else {
                                            revealError = "Authentication ${outcome.name.lowercase()}."
                                        }
                                    }
                                },
                                modifier = Modifier.testTag("keystoreExplorer.reveal.$walletIdHex"),
                            ) {
                                Text("Reveal")
                            }
                        },
                        modifier = Modifier.testTag("keystoreExplorer.mnemonic.$walletIdHex"),
                    )
                }
            }

            FormSection(title = "Identity Private Keys (${privkeyEntries.size})") {
                if (privkeyEntries.isEmpty()) {
                    Text(
                        "No stored identity keys.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                privkeyEntries.forEach { entry ->
                    val pubkeyHex = entry.removePrefix("privkey.")
                    LabeledContent(
                        label = "Key ${truncateMiddle(pubkeyHex, 8, 6)}",
                        value = "Sealed",
                    )
                }
            }

            if (otherEntries.isNotEmpty()) {
                FormSection(title = "Other Entries (${otherEntries.size})") {
                    otherEntries.forEach { entry ->
                        Text(
                            entry,
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace,
                        )
                    }
                }
            }

            FormSection(title = "AndroidKeyStore Aliases (${keystoreAliases.size})") {
                if (keystoreAliases.isEmpty()) {
                    Text(
                        "No keystore aliases.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                keystoreAliases.forEach { alias ->
                    Text(
                        alias,
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                        modifier = Modifier.testTag("keystoreExplorer.alias.$alias"),
                    )
                }
            }

            FormSection(title = "About") {
                Text(
                    "Secret material stays sealed in the listing — rows show " +
                        "entry names only. Values are AES-GCM ciphertext under " +
                        "non-exportable AndroidKeyStore master keys; a mnemonic's " +
                        "Reveal decrypts only after biometric/credential auth. " +
                        "Identity private keys are additionally bound to a " +
                        "30-second auth window and never reveal here.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }

    revealError?.let { message ->
        AlertDialog(
            onDismissRequest = { revealError = null },
            title = { Text("Reveal Failed") },
            text = { Text(message) },
            confirmButton = { TextButton(onClick = { revealError = null }) { Text("OK") } },
        )
    }

    revealedMnemonic?.let { (walletIdHex, phrase) ->
        AlertDialog(
            onDismissRequest = { revealedMnemonic = null },
            title = { Text("Wallet ${truncateMiddle(walletIdHex, 6, 6)}") },
            text = {
                Text(
                    phrase,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.testTag("keystoreExplorer.revealedPhrase"),
                )
            },
            confirmButton = {
                TextButton(onClick = { revealedMnemonic = null }) { Text("Done") }
            },
        )
    }
}
