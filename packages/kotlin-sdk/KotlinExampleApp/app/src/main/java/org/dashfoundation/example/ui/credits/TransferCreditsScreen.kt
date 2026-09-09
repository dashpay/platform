package org.dashfoundation.example.ui.credits

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.hexToBytes

/**
 * Transfer credits from one identity to another — port of
 * `TransferCreditsView.swift`. Wraps
 * `platform_wallet_transfer_credits_with_signer` via
 * [org.dashfoundation.dashsdk.credits.IdentityCredits.transfer]; the
 * sender's balance refreshes through the persistence changeset (no balance
 * returned). A `submitGeneration` guard ignores stale completions if the
 * screen is re-entered mid-broadcast (← Swift `submitGeneration`).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TransferCreditsScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }

    val identity by container.database.identityDao()
        .observeByIdentityId(idBytes)
        .collectAsStateWithLifecycle(initialValue = null)

    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val wallet = rememberManagedWalletFor(identity?.walletId)

    var recipientHex by rememberSaveable { mutableStateOf("") }
    var amountText by rememberSaveable { mutableStateOf("") }
    var isSubmitting by remember { mutableStateOf(false) }
    var submitGeneration by remember { mutableStateOf(0) }
    var error by remember { mutableStateOf<String?>(null) }
    var done by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    val amount = amountText.toLongOrNull()
    val recipientValid = runCatching { recipientHex.trim().hexToBytes().size == 32 }.getOrDefault(false)
    val balance = identity?.balance ?: 0L
    val canSubmit = wallet != null && recipientValid && amount != null && amount > 0 && amount <= balance

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Transfer Credits") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }, enabled = !isSubmitting) {
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
            FormSection(title = "From") {
                LabeledContent("Identity", identity?.mainDpnsName ?: identityIdHex.take(16) + "…")
                LabeledContent("Balance", "$balance credits")
            }

            FormSection(title = "Recipient") {
                OutlinedTextField(
                    value = recipientHex,
                    onValueChange = { recipientHex = it },
                    label = { Text("Recipient identity ID (hex)") },
                    singleLine = true,
                    isError = recipientHex.isNotBlank() && !recipientValid,
                    modifier = Modifier.fillMaxWidth().testTag("transferCredits.recipient"),
                )
            }

            FormSection(title = "Amount") {
                OutlinedTextField(
                    value = amountText,
                    onValueChange = { amountText = it.filter(Char::isDigit) },
                    label = { Text("Amount (credits)") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier.fillMaxWidth().testTag("transferCredits.amount"),
                )
            }

            if (done) {
                Text(
                    "Transfer submitted.",
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.testTag("transferCredits.done"),
                )
            }

            SubmitButton(
                text = "Transfer Credits",
                isLoading = isSubmitting,
                enabled = canSubmit,
                modifier = Modifier.fillMaxWidth().testTag("transferCredits.submit"),
            ) {
                val mgr = manager ?: return@SubmitButton
                val w = wallet ?: return@SubmitButton
                val amt = amount ?: return@SubmitButton
                val toId = recipientHex.trim().hexToBytes()
                isSubmitting = true
                val generation = ++submitGeneration
                scope.launch {
                    try {
                        mgr.identityCredits.transfer(
                            walletHandle = w.handle,
                            fromIdentityId = idBytes,
                            toIdentityId = toId,
                            amount = amt,
                            signerHandle = mgr.signerHandle,
                        )
                        if (generation == submitGeneration) done = true
                    } catch (e: Exception) {
                        if (generation == submitGeneration) error = e.message ?: "Transfer failed"
                    } finally {
                        if (generation == submitGeneration) isSubmitting = false
                    }
                }
            }
        }
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}
