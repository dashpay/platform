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
import org.dashfoundation.dashsdk.credits.FundingInput
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.hexToBytes

/**
 * Top up an identity's credit balance from Platform-payment addresses —
 * port of `TopUpIdentityView.swift`. On submit it enumerates the wallet's
 * balance-carrying Platform-payment addresses via the now-bridged
 * `platform_address_wallet_addresses_with_balances`
 * ([org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.addressesWithBalances]),
 * greedily packs them up to the requested amount (largest-balance-first,
 * matching Swift's `buildInputs`), then credits the identity through
 * `platform_wallet_top_up_from_addresses_with_signer`
 * ([org.dashfoundation.dashsdk.credits.IdentityCredits.topUpFromAddresses]).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TopUpIdentityScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()

    val identity by container.database.identityDao()
        .observeByIdentityId(idBytes)
        .collectAsStateWithLifecycle(initialValue = null)
    val wallet = rememberManagedWalletFor(identity?.walletId)

    var amountText by rememberSaveable { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var isSubmitting by remember { mutableStateOf(false) }
    var done by remember { mutableStateOf(false) }
    // The top-up may have been broadcast and its outcome is not confirmed
    // (code 20): shown as pending, and the same top-up cannot be sent again.
    var pending by remember { mutableStateOf(false) }

    val amount = amountText.toLongOrNull()
    val canSubmit = wallet != null && manager != null && amount != null && amount > 0 &&
        !isSubmitting

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Top Up Identity") },
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
            FormSection(title = "Identity") {
                LabeledContent("Identity", identity?.mainDpnsName ?: identityIdHex.take(16) + "…")
                LabeledContent("Current balance", "${identity?.balance ?: 0} credits")
            }

            FormSection(title = "Funding") {
                Text(
                    "Funded from the wallet's Platform-payment addresses.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                OutlinedTextField(
                    value = amountText,
                    onValueChange = { amountText = it.filter(Char::isDigit) },
                    label = { Text("Amount (credits)") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier.fillMaxWidth().testTag("topUpIdentity.amount"),
                )
            }

            if (pending) {
                Text(
                    "Top-up pending: it may still complete. Don't send it again; " +
                        "a later sync refreshes the balance but can't confirm this top-up.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.testTag("topup.pending"),
                )
            }

            SubmitButton(
                text = when {
                    done -> "Topped Up"
                    pending -> "Top-Up Pending"
                    else -> "Top Up Balance"
                },
                isLoading = isSubmitting,
                enabled = canSubmit && !done && !pending,
                modifier = Modifier.fillMaxWidth().testTag("topUpIdentity.submit"),
            ) {
                val mgr = manager ?: return@SubmitButton
                val w = wallet ?: return@SubmitButton
                val amt = amount ?: return@SubmitButton
                isSubmitting = true
                scope.launch {
                    try {
                        // Enumerate balance-carrying addresses, then greedily
                        // pack up to the requested amount (largest first) —
                        // matching TopUpIdentityView.buildInputs.
                        val candidates = w.addressesWithBalances()
                            .sortedByDescending { it.credits }
                        val inputs = packInputs(candidates, amt)
                        if (inputs.isEmpty()) {
                            error = "Not enough Platform-address balance to fund " +
                                "$amt credits."
                            return@launch
                        }
                        mgr.identityCredits.topUpFromAddresses(
                            walletHandle = w.handle,
                            identityId = idBytes,
                            inputs = inputs,
                            signerHandle = mgr.signerHandle,
                        )
                        done = true
                    } catch (e: DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed) {
                        // Not a failure: the top-up may still execute, so no
                        // retry is offered.
                        pending = true
                    } catch (e: Exception) {
                        error = e.message ?: "Top up failed"
                    } finally {
                        isSubmitting = false
                    }
                }
            }
        }
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

/**
 * Greedily select funding inputs from [candidates] (assumed sorted
 * largest-balance-first) until their combined credits cover [target],
 * spending only what's needed from the final address. Returns an empty
 * list when the candidates can't cover [target]. Mirrors Swift's
 * `TopUpIdentityView.buildInputs`.
 */
private fun packInputs(candidates: List<FundingInput>, target: Long): List<FundingInput> {
    if (target <= 0) return emptyList()
    var remaining = target
    val picked = ArrayList<FundingInput>()
    for (input in candidates) {
        if (remaining <= 0) break
        val spend = minOf(input.credits, remaining)
        picked.add(input.copy(credits = spend))
        remaining -= spend
    }
    return if (remaining <= 0) picked else emptyList()
}
