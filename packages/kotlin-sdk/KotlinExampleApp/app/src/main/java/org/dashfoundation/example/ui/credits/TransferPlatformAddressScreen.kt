package org.dashfoundation.example.ui.credits

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.text.KeyboardOptions
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
import androidx.compose.ui.text.input.KeyboardType
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
import org.dashfoundation.example.util.formatCredits
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * Wallet-signed transfer of Platform-address (DIP-17) credits — port of
 * `TransferPlatformAddressView.swift` (ADDR-02). Drives
 * [ManagedPlatformWallet.transferCredits] end-to-end with the wallet's
 * platform-address signer; no private key is entered. Input selection
 * (Auto), the `Σ inputs == Σ outputs` balancing, fee/nonce selection, and
 * signing all happen Rust-side — this screen only picks the source account,
 * the destination address (own-wallet or external 40-hex P2PKH hash), and
 * the amount.
 *
 * The submit gate sums only balances `>= minInput` (the Rust Auto selector
 * drops sub-minimum dust) and requires the amount `>= minOutput` (DPP
 * rejects a sub-minimum output). Both floors come from
 * [ManagedPlatformWallet.minAmounts] (version-locked, resolved on appear);
 * an unresolved floor keeps the gate CLOSED rather than substituting a
 * default, mirroring the Swift view's safe pattern.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TransferPlatformAddressScreen(walletIdHex: String, navController: NavHostController) {
    val walletId = remember(walletIdHex) { walletIdHex.hexToBytes() }
    val container = LocalAppContainer.current
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val wallet = rememberManagedWalletFor(walletId)
    val scope = rememberCoroutineScope()

    val platformAddresses by container.database.platformAddressDao()
        .observeByWallet(walletId)
        .collectAsStateWithLifecycle(initialValue = emptyList())

    // Version-locked floors (resolved once on appear; null = gate closed).
    var minAmounts by remember { mutableStateOf<ManagedPlatformWallet.MinAmounts?>(null) }
    LaunchedEffect(wallet) {
        minAmounts = wallet?.let { runCatching { it.minAmounts() }.getOrNull() }
    }
    val minInput = minAmounts?.minInput
    val minOutput = minAmounts?.minOutput

    // Source accounts = distinct DIP-17 accountIndex values whose spendable
    // (>= minInput) balance is positive. Auto selects the first.
    val sourceAccounts: List<Pair<Int, Long>> = remember(platformAddresses, minInput) {
        val floor = minInput ?: return@remember emptyList()
        platformAddresses
            .groupBy { it.accountIndex }
            .mapValues { (_, rows) -> rows.filter { it.balance >= floor }.sumOf { it.balance } }
            .filter { it.value > 0 }
            .toSortedMap()
            .map { (index, spendable) -> index to spendable }
    }
    var sourceAccountIndex by remember { mutableStateOf<Int?>(null) }
    LaunchedEffect(sourceAccounts) {
        if (sourceAccountIndex == null) sourceAccountIndex = sourceAccounts.firstOrNull()?.first
    }
    val spendable = sourceAccounts.firstOrNull { it.first == sourceAccountIndex }?.second ?: 0L

    // Destination: own-wallet address (20-byte hash) or external 40-hex hash.
    var useOwnWallet by remember { mutableStateOf(true) }
    var selectedOwnHashHex by rememberSaveable { mutableStateOf("") }
    var externalHashHex by rememberSaveable { mutableStateOf("") }
    var amountText by rememberSaveable { mutableStateOf("") }
    var isSubmitting by remember { mutableStateOf(false) }
    var submitGeneration by remember { mutableStateOf(0) }
    var error by remember { mutableStateOf<String?>(null) }
    var done by remember { mutableStateOf(false) }

    // Own-wallet recipient candidates: this wallet's platform addresses
    // (any account) rendered by hash. Auto-selects the first.
    val ownRecipients = remember(platformAddresses) {
        platformAddresses.sortedWith(compareBy({ it.accountIndex }, { it.addressIndex }))
    }
    LaunchedEffect(ownRecipients) {
        if (selectedOwnHashHex.isEmpty()) {
            selectedOwnHashHex = ownRecipients.firstOrNull()?.addressHash?.toHex() ?: ""
        }
    }

    val amount = amountText.toLongOrNull()
    val destinationHash: ByteArray? = remember(useOwnWallet, selectedOwnHashHex, externalHashHex) {
        val hex = (if (useOwnWallet) selectedOwnHashHex else externalHashHex).trim()
        runCatching { hex.hexToBytes() }.getOrNull()?.takeIf { it.size == 20 }
    }
    val amountOk = amount != null && amount > 0 &&
        minOutput != null && amount >= minOutput && amount <= spendable
    val canSubmit = wallet != null && manager != null && sourceAccountIndex != null &&
        destinationHash != null && amountOk && minInput != null

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Transfer Platform Credits") },
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
                        if (minInput == null) "Resolving minimum amounts…"
                        else "No platform account with a spendable balance. Sync first.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    sourceAccounts.forEach { (index, bal) ->
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .selectable(
                                    selected = sourceAccountIndex == index,
                                    onClick = { sourceAccountIndex = index },
                                )
                                .padding(vertical = 4.dp)
                                .testTag("transferPlatform.account.$index"),
                        ) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
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
            }

            FormSection(title = "Destination") {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    FilterChip(
                        selected = useOwnWallet,
                        onClick = { useOwnWallet = true },
                        label = { Text("My Wallet") },
                        modifier = Modifier.testTag("transferPlatform.destOwn"),
                    )
                    FilterChip(
                        selected = !useOwnWallet,
                        onClick = { useOwnWallet = false },
                        label = { Text("External") },
                        modifier = Modifier.testTag("transferPlatform.destExternal"),
                    )
                }
                if (useOwnWallet) {
                    if (ownRecipients.isEmpty()) {
                        Text(
                            "No platform addresses yet.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    } else {
                        ownRecipients.take(20).forEach { row ->
                            val hex = row.addressHash.toHex()
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                modifier = Modifier.selectable(
                                    selected = selectedOwnHashHex == hex,
                                    onClick = { selectedOwnHashHex = hex },
                                ),
                            ) {
                                RadioButton(
                                    selected = selectedOwnHashHex == hex,
                                    onClick = { selectedOwnHashHex = hex },
                                )
                                Text(
                                    "acct ${row.accountIndex} · ${hex.take(16)}…",
                                    style = MaterialTheme.typography.bodySmall,
                                )
                            }
                        }
                    }
                } else {
                    OutlinedTextField(
                        value = externalHashHex,
                        onValueChange = { externalHashHex = it.filter { c -> c.isLetterOrDigit() } },
                        label = { Text("Recipient P2PKH hash (40 hex chars)") },
                        singleLine = true,
                        isError = externalHashHex.isNotBlank() &&
                            runCatching { externalHashHex.trim().hexToBytes().size }.getOrNull() != 20,
                        modifier = Modifier.fillMaxWidth().testTag("transferPlatform.externalHash"),
                    )
                }
            }

            FormSection(title = "Amount") {
                OutlinedTextField(
                    value = amountText,
                    onValueChange = { amountText = it.filter(Char::isDigit) },
                    label = { Text("Amount (credits)") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier.fillMaxWidth().testTag("transferPlatform.amount"),
                )
                if (minOutput != null) {
                    LabeledContent("Minimum", formatCredits(minOutput))
                }
                LabeledContent("Spendable", formatCredits(spendable))
            }

            if (done) {
                Text(
                    "Transfer submitted.",
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.testTag("transferPlatform.done"),
                )
            }

            SubmitButton(
                text = "Transfer Credits",
                isLoading = isSubmitting,
                enabled = canSubmit,
                modifier = Modifier.fillMaxWidth().testTag("transferPlatform.submit"),
            ) {
                val mgr = manager ?: return@SubmitButton
                val w = wallet ?: return@SubmitButton
                val acct = sourceAccountIndex ?: return@SubmitButton
                val hash = destinationHash ?: return@SubmitButton
                val amt = amount ?: return@SubmitButton
                isSubmitting = true
                val generation = ++submitGeneration
                scope.launch {
                    try {
                        w.transferCredits(
                            outputs = listOf(
                                // Platform-address outputs are P2PKH only (type 0).
                                ManagedPlatformWallet.CreditOutput(
                                    addressType = 0,
                                    hash = hash,
                                    credits = amt,
                                ),
                            ),
                            signerHandle = mgr.signerHandle,
                            accountIndex = acct,
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
