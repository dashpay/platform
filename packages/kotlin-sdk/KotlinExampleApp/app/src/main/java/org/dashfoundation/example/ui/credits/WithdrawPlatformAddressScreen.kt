package org.dashfoundation.example.ui.credits

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.formatCredits
import org.dashfoundation.example.util.hexToBytes

/**
 * Wallet-signed withdrawal of Platform-address (DIP-17) credits to a Core
 * L1 address — port of `WithdrawPlatformAddressView.swift` (ADDR-04). Drives
 * [ManagedPlatformWallet.withdrawCredits] with the wallet's platform-address
 * signer. Full-account-balance AUTO withdrawal (no amount field); the Core
 * address is network-checked Rust-side.
 *
 * Submit is gated on a [ManagedPlatformWallet.preflightWithdrawal] whose
 * `canWithdraw` flag is authoritative — the preflight re-reads on-chain
 * balances and sizes the same plan the spend would, so the gate can't
 * approve what the spend rejects. The offered fee-per-byte values are the
 * non-zero Fibonacci rates DPP accepts (any other rate deterministically
 * fails structure validation on submit).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WithdrawPlatformAddressScreen(walletIdHex: String, navController: NavHostController) {
    val walletId = remember(walletIdHex) { walletIdHex.hexToBytes() }
    val container = LocalAppContainer.current
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val wallet = rememberManagedWalletFor(walletId)
    val scope = rememberCoroutineScope()

    val platformAddresses by container.database.platformAddressDao()
        .observeByWallet(walletId)
        .collectAsStateWithLifecycle(initialValue = emptyList())

    // Source accounts = distinct DIP-17 accountIndex values holding credits.
    val sourceAccounts = remember(platformAddresses) {
        platformAddresses
            .groupBy { it.accountIndex }
            .mapValues { (_, rows) -> rows.sumOf { it.balance } }
            .filter { it.value > 0 }
            .toSortedMap()
            .map { (index, bal) -> index to bal }
    }
    var sourceAccountIndex by remember { mutableStateOf<Int?>(null) }
    LaunchedEffect(sourceAccounts) {
        if (sourceAccountIndex == null) sourceAccountIndex = sourceAccounts.firstOrNull()?.first
    }

    var coreFeePerByte by remember { mutableStateOf(1) }
    var toAddress by rememberSaveable { mutableStateOf("") }
    var preflight by remember { mutableStateOf<ManagedPlatformWallet.WithdrawalPreflight?>(null) }
    var preflightReason by remember { mutableStateOf<String?>(null) }
    var isSubmitting by remember { mutableStateOf(false) }
    var submitGeneration by remember { mutableStateOf(0) }
    var error by remember { mutableStateOf<String?>(null) }
    var done by remember { mutableStateOf(false) }

    // Recompute the preflight on appear and on any source/fee change — the
    // gate must reflect what the spend would accept (← Swift recomputePreflight).
    // When the account can't fund a withdrawal, also surface the advisory
    // "why not" the preflight recorded (← the Swift `WithdrawalPreflightFFI`
    // success_with_message reason, via `preflightWithdrawalReason`).
    LaunchedEffect(wallet, sourceAccountIndex, coreFeePerByte) {
        val w = wallet
        val acct = sourceAccountIndex
        preflight = if (w != null && acct != null) {
            runCatching { w.preflightWithdrawal(accountIndex = acct, coreFeePerByte = coreFeePerByte) }
                .getOrNull()
        } else {
            null
        }
        preflightReason = if (w != null && acct != null && preflight?.canWithdraw == false) {
            runCatching {
                w.preflightWithdrawalReason(accountIndex = acct, coreFeePerByte = coreFeePerByte)
            }.getOrNull()
        } else {
            null
        }
    }

    val trimmedAddr = toAddress.trim()
    // Light L1 validation (base58check payload = 25 bytes); the network-aware
    // check runs Rust-side inside the FFI.
    val addressLooksValid = trimmedAddr.isNotEmpty() && Base58.decode(trimmedAddr)?.size == 25
    val canSubmit = wallet != null && manager != null && sourceAccountIndex != null &&
        addressLooksValid && preflight?.canWithdraw == true

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Withdraw Platform Credits") },
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
            FormSection(title = "Source Account") {
                if (sourceAccounts.isEmpty()) {
                    Text(
                        "No platform account with a balance. Sync first.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    sourceAccounts.forEach { (index, bal) ->
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier
                                .fillMaxWidth()
                                .selectable(
                                    selected = sourceAccountIndex == index,
                                    onClick = { sourceAccountIndex = index },
                                )
                                .testTag("withdrawPlatform.account.$index"),
                        ) {
                            RadioButton(
                                selected = sourceAccountIndex == index,
                                onClick = { sourceAccountIndex = index },
                            )
                            Text("Account $index — ${formatCredits(bal)}")
                        }
                    }
                }
            }

            FormSection(title = "Destination") {
                OutlinedTextField(
                    value = toAddress,
                    onValueChange = { toAddress = it },
                    label = { Text("Core (L1) Dash address") },
                    singleLine = true,
                    isError = trimmedAddr.isNotEmpty() && !addressLooksValid,
                    supportingText = {
                        if (trimmedAddr.isNotEmpty() && !addressLooksValid) {
                            Text("Not a valid Dash address")
                        }
                    },
                    modifier = Modifier.fillMaxWidth().testTag("withdrawPlatform.address"),
                )
            }

            FormSection(title = "Fee per byte") {
                Row(
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                ) {
                    fibonacciFeeRates(MAX_FEE_PER_BYTE).forEach { rate ->
                        FilterChip(
                            selected = coreFeePerByte == rate,
                            onClick = { coreFeePerByte = rate },
                            label = { Text(rate.toString()) },
                            modifier = Modifier.testTag("withdrawPlatform.feeRate.$rate"),
                        )
                    }
                }
            }

            FormSection(title = "Preflight") {
                val pf = preflight
                when {
                    pf == null -> LabeledContent("Status", "Estimating…")
                    pf.canWithdraw -> {
                        LabeledContent("Net withdrawable", formatCredits(pf.netWithdrawable))
                        LabeledContent("Estimated fee", formatCredits(pf.estimatedFee))
                    }
                    else -> {
                        LabeledContent("Status", "Cannot withdraw from this account right now")
                        preflightReason?.let { reason ->
                            Text(
                                reason,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.testTag("withdrawPlatform.preflightReason"),
                            )
                        }
                    }
                }
            }

            if (done) {
                Text(
                    "Withdrawal submitted. The L1 payout is processed asynchronously " +
                        "by the network and may take a while to appear on-chain.",
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.testTag("withdrawPlatform.done"),
                )
            }

            SubmitButton(
                text = "Withdraw Credits",
                isLoading = isSubmitting,
                enabled = canSubmit,
                modifier = Modifier.fillMaxWidth().testTag("withdrawPlatform.submit"),
            ) {
                val mgr = manager ?: return@SubmitButton
                val w = wallet ?: return@SubmitButton
                val acct = sourceAccountIndex ?: return@SubmitButton
                val fee = coreFeePerByte
                isSubmitting = true
                val generation = ++submitGeneration
                scope.launch {
                    try {
                        // Re-run the preflight as a consent guard (net/fee may
                        // have drifted while the screen was open) — ← Swift submit().
                        val recheck = w.preflightWithdrawal(accountIndex = acct, coreFeePerByte = fee)
                        if (!recheck.canWithdraw) {
                            if (generation == submitGeneration) {
                                error = "Withdrawal no longer available for this account."
                            }
                            return@launch
                        }
                        w.withdrawCredits(
                            coreAddress = trimmedAddr,
                            coreFeePerByte = fee,
                            signerHandle = mgr.signerHandle,
                            accountIndex = acct,
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

/** App-side ceiling for the offered Fibonacci fee rates (← Swift `maxFeePerByte`). */
private const val MAX_FEE_PER_BYTE = 89

/**
 * The non-zero Fibonacci fee rates up to [ceiling] — port of Swift's
 * `WithdrawalCoreFeeRates.rates`. DPP's address-credit-withdrawal structure
 * validation rejects any non-Fibonacci `core_fee_per_byte`, so the UI only
 * offers these. Generated by a Fibonacci walk (not hardcoded) to stay in
 * lockstep with the protocol's definition.
 */
private fun fibonacciFeeRates(ceiling: Int): List<Int> {
    if (ceiling < 1) return emptyList()
    val rates = ArrayList<Int>()
    var previous = 1
    var current = 1
    while (previous <= ceiling) {
        if (rates.lastOrNull() != previous) rates.add(previous)
        val next = previous + current
        previous = current
        current = next
    }
    return rates
}
