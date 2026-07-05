package org.dashfoundation.example.ui.wallet

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.compose.runtime.rememberCoroutineScope
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.keywallet.Mnemonic
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
 * confirmation). On success the flow hands off to [SeedBackup] — the
 * Android ordering creates the wallet first and the backup screen re-reads
 * the stored phrase by wallet id, so the mnemonic never travels through
 * navigation state (iOS pushes SeedBackupView with the phrase in-memory
 * before creating; same user-visible flow, safer arg plumbing).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CreateWalletScreen(navController: NavHostController) {
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()

    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()

    var name by rememberSaveable { mutableStateOf("") }
    var error by rememberSaveable { mutableStateOf<String?>(null) }
    var isCreating by rememberSaveable { mutableStateOf(false) }

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
                "Name your wallet. A new 12-word recovery phrase will be " +
                    "generated on the next screen for you to back up.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            FormSection(title = "New Wallet") {
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
            }

            SubmitButton(
                text = "Create Wallet",
                isLoading = isCreating,
                enabled = name.isNotBlank() && !isCreating,
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("createWallet.submit"),
            ) {
                scope.launch {
                    isCreating = true
                    try {
                        val manager = container.walletManagerStore.activeManager.value
                            ?: error("Wallet manager is not active")
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
}
