package org.dashfoundation.example.ui.wallet

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
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
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.runtime.rememberCoroutineScope
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.keywallet.Mnemonic
import org.dashfoundation.dashsdk.wallet.WalletCreateRollbackException
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.SeedBackup
import org.dashfoundation.example.navigation.WalletsHome
import androidx.compose.material3.MaterialTheme
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton

/**
 * Wallet creation form — port of `CreateWalletView.swift` (name + network
 * confirmation + the **Import Existing Wallet** toggle, CORE-02). On
 * create-new success the flow hands off to [SeedBackup] — the Android
 * ordering creates the wallet first and the backup screen re-reads the
 * stored phrase by wallet id, so the mnemonic never travels through
 * navigation state (iOS pushes SeedBackupView with the phrase in-memory
 * before creating; same user-visible flow, safer arg plumbing). The
 * import path skips the backup handoff entirely — the user already holds
 * the phrase — and pops back to the wallets list, mirroring iOS's
 * "go straight to creation with provided mnemonic". Phrase validation
 * (BIP-39 word list + checksum) is Rust-side: an invalid phrase surfaces
 * through [ErrorAlertDialog], matching the iOS error path.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CreateWalletScreen(navController: NavHostController) {
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()

    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()

    var name by rememberSaveable { mutableStateOf("") }
    var importExisting by rememberSaveable { mutableStateOf(false) }
    var importMnemonic by rememberSaveable { mutableStateOf("") }
    var error by rememberSaveable { mutableStateOf<String?>(null) }
    var isCreating by rememberSaveable { mutableStateOf(false) }
    // Last-resort phrase surface: when creation fails AND the SDK could not
    // store the generated phrase durably, WalletCreateRollbackException
    // carries the only remaining copy — it must be shown for manual backup
    // before it is discarded. Plain remember (NOT rememberSaveable): the
    // plaintext must never be written into the saved-state Bundle.
    var unrecoverablePhrase by remember { mutableStateOf<String?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Create Wallet") },
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
                .imePadding()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text(
                if (importExisting) {
                    "Restore a wallet from its recovery phrase. Derived " +
                        "addresses and balances populate after the next sync."
                } else {
                    "Name your wallet. A new 12-word recovery phrase will be " +
                        "generated on the next screen for you to back up."
                },
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            FormSection(title = if (importExisting) "Import Wallet" else "New Wallet") {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("Wallet name") },
                    singleLine = true,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 8.dp)
                        .testTag("createWallet.name"),
                )
                LabeledContent("Network", network.displayName)
                // ← iOS `Toggle("Import Existing Wallet", isOn: $showImportOption)`.
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp),
                ) {
                    Text(
                        "Import Existing Wallet",
                        style = MaterialTheme.typography.bodyMedium,
                        modifier = Modifier.weight(1f),
                    )
                    Switch(
                        checked = importExisting,
                        onCheckedChange = { importExisting = it },
                        modifier = Modifier.testTag("createWallet.importToggle"),
                    )
                }
            }

            if (importExisting) {
                FormSection(title = "Recovery Phrase") {
                    OutlinedTextField(
                        value = importMnemonic,
                        onValueChange = { importMnemonic = it },
                        label = { Text("Enter recovery phrase (12–24 words)") },
                        minLines = 3,
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 8.dp)
                            .testTag("createWallet.importMnemonic"),
                    )
                    Text(
                        "Enter your recovery phrase words separated by spaces.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            SubmitButton(
                text = if (importExisting) "Import Wallet" else "Create Wallet",
                isLoading = isCreating,
                enabled = name.isNotBlank() && !isCreating &&
                    (!importExisting || importMnemonic.isNotBlank()),
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("createWallet.submit"),
            ) {
                scope.launch {
                    isCreating = true
                    try {
                        val manager = container.walletManagerStore.activeManager.value
                            ?: error("Wallet manager is not active")
                        if (importExisting) {
                            // CORE-02: restore from the typed phrase.
                            // Normalize whitespace (multi-line field);
                            // BIP-39 word-list + checksum validation is
                            // Rust-side and surfaces via ErrorAlertDialog.
                            val phrase = importMnemonic.trim()
                                .split(Regex("\\s+"))
                                .joinToString(" ")
                            // An imported mnemonic may already have on-chain
                            // history (incl. DashPay payments) from before
                            // this device — scan from genesis (birthHeight 0)
                            // so it is seen. A freshly generated phrase (else
                            // branch) has nothing before now, so it scans from
                            // the SPV tip (birthHeight null). ← SendViewModel/
                            // CreateWalletView `birthHeight: showImportOption ? 0 : nil`.
                            manager.createWallet(phrase, name = name.trim(), birthHeight = 0u)
                            // The user already holds the phrase — skip the
                            // backup handoff and return to the wallets list
                            // (← iOS "go straight to creation").
                            navController.popBackStack()
                        } else {
                            // Rust generates the phrase; Kotlin only displays and
                            // stores it (Mnemonic.swift parity).
                            val mnemonic = Mnemonic.generate(wordCount = 12)
                            val wallet = manager.createWallet(mnemonic, name = name.trim())
                            // Hand off to the backup + confirmation flow
                            // (← CreateWalletView's SeedBackupView push). The
                            // create form is popped so back from the backup
                            // screen can't re-submit.
                            navController.navigate(SeedBackup(wallet.walletIdHex)) {
                                popUpTo(WalletsHome)
                            }
                        }
                    } catch (e: WalletCreateRollbackException) {
                        // The phrase reached neither the Keystore nor the
                        // user (it lives only in the try-local above) —
                        // surface it for manual backup instead of losing it
                        // behind a plain error message.
                        val phrase = e.mnemonic
                        if (phrase != null) {
                            unrecoverablePhrase = phrase
                        } else {
                            error = e.message ?: "Wallet creation failed"
                        }
                    } catch (e: Exception) {
                        error = e.message ?: "Wallet creation failed"
                    } finally {
                        isCreating = false
                    }
                }
            }
        }
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })

    // Emergency backup dialog — the phrase shown here exists NOWHERE else.
    // Not dismissable by tapping outside; the user must acknowledge.
    unrecoverablePhrase?.let { phrase ->
        AlertDialog(
            onDismissRequest = { /* explicit acknowledgement required */ },
            title = { Text("Back up this phrase now") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        "Wallet creation failed and the recovery phrase could not " +
                            "be stored on this device. The phrase below is the ONLY " +
                            "copy — write it down before dismissing this dialog.",
                    )
                    SelectionContainer {
                        Text(
                            phrase,
                            fontFamily = FontFamily.Monospace,
                            modifier = Modifier.testTag("createWallet.unrecoverablePhrase"),
                        )
                    }
                }
            },
            confirmButton = {
                TextButton(
                    onClick = { unrecoverablePhrase = null },
                    modifier = Modifier.testTag("createWallet.unrecoverablePhrase.ack"),
                ) {
                    Text("I wrote it down")
                }
            },
        )
    }
}
