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
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.hexToBytes

/**
 * Withdraw credits from an identity to a Core (L1) Dash address — port of
 * `WithdrawCreditsView.swift`. Wraps
 * `platform_wallet_withdraw_credits_with_signer` via
 * [org.dashfoundation.dashsdk.credits.IdentityCredits.withdraw]. The L1
 * payout is pooled + broadcast asynchronously (no txid returned), so the
 * success note mirrors Swift's async-payout caveat.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WithdrawCreditsScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }

    val identity by container.database.identityDao()
        .observeByIdentityId(idBytes)
        .collectAsStateWithLifecycle(initialValue = null)

    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val wallet = rememberManagedWalletFor(identity?.walletId)

    var toAddress by rememberSaveable { mutableStateOf("") }
    var amountText by rememberSaveable { mutableStateOf("") }
    var isSubmitting by remember { mutableStateOf(false) }
    var submitGeneration by remember { mutableStateOf(0) }
    var error by remember { mutableStateOf<String?>(null) }
    var done by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    val amount = amountText.toLongOrNull()
    val trimmedAddr = toAddress.trim()
    // Light L1 address validation (base58check payload = 25 bytes); the
    // network-aware check runs Rust-side in the FFI.
    val addressLooksValid = trimmedAddr.isNotEmpty() && Base58.decode(trimmedAddr)?.size == 25
    val balance = identity?.balance ?: 0L
    val canSubmit = wallet != null && addressLooksValid && amount != null && amount > 0 && amount <= balance

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Withdraw Credits") },
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

            FormSection(title = "Destination") {
                OutlinedTextField(
                    value = toAddress,
                    onValueChange = { toAddress = it },
                    label = { Text("L1 Dash address") },
                    singleLine = true,
                    isError = trimmedAddr.isNotEmpty() && !addressLooksValid,
                    supportingText = {
                        if (trimmedAddr.isNotEmpty() && !addressLooksValid) Text("Not a valid Dash address")
                    },
                    modifier = Modifier.fillMaxWidth().testTag("withdrawCredits.address"),
                )
            }

            FormSection(title = "Amount") {
                OutlinedTextField(
                    value = amountText,
                    onValueChange = { amountText = it.filter(Char::isDigit) },
                    label = { Text("Amount (credits)") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier.fillMaxWidth().testTag("withdrawCredits.amount"),
                )
            }

            if (done) {
                Text(
                    "Withdrawal submitted. The L1 payout is processed asynchronously " +
                        "by the network and may take a while to appear on-chain.",
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.testTag("withdrawCredits.done"),
                )
            }

            SubmitButton(
                text = "Withdraw Credits",
                isLoading = isSubmitting,
                enabled = canSubmit,
                modifier = Modifier.fillMaxWidth().testTag("withdrawCredits.submit"),
            ) {
                val mgr = manager ?: return@SubmitButton
                val w = wallet ?: return@SubmitButton
                val amt = amount ?: return@SubmitButton
                isSubmitting = true
                val generation = ++submitGeneration
                scope.launch {
                    try {
                        mgr.identityCredits.withdraw(
                            walletHandle = w.handle,
                            identityId = idBytes,
                            amount = amt,
                            toAddress = trimmedAddr,
                            signerHandle = mgr.signerHandle,
                        )
                        if (generation == submitGeneration) done = true
                    } catch (e: Exception) {
                        if (generation == submitGeneration) error = e.message ?: "Withdrawal failed"
                    } finally {
                        if (generation == submitGeneration) isSubmitting = false
                    }
                }
            }
        }
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}
