package org.dashfoundation.example.ui.credits

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
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
import org.dashfoundation.dashsdk.credits.FundingInput
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.formatCredits
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * Transfer an identity's credits to Platform (DIP-17) addresses — the ID-11
 * path, the identity-side counterpart of ADDR-02's
 * [TransferPlatformAddressScreen]. Drives
 * [org.dashfoundation.dashsdk.credits.IdentityCredits.transferToAddresses]
 * (→ `platform_wallet_transfer_credits_to_addresses_with_signer`), signed by
 * the identity's transfer key via the wallet's Keystore signer; no private
 * key is entered. Recipient is one of the wallet's own Platform addresses
 * (the credits stay in-wallet and the recipient balance reconciles from the
 * proof) or a pasted external 40-hex P2PKH hash.
 *
 * The amount is gated `>= minOutput` (DPP rejects a sub-minimum output) and
 * `<= the identity's credit balance`. The `minOutput` floor comes from the
 * wallet's version-locked `minAmounts()` (resolved on appear); an unresolved
 * floor keeps the gate CLOSED rather than substituting a default, mirroring
 * the ADDR-02 screen.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TransferIdentityToAddressScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()

    val identity by container.database.identityDao()
        .observeByIdentityId(idBytes)
        .collectAsStateWithLifecycle(initialValue = null)
    val wallet = rememberManagedWalletFor(identity?.walletId)

    val platformAddresses by container.database.platformAddressDao()
        .observeByWallet(identity?.walletId ?: ByteArray(0))
        .collectAsStateWithLifecycle(initialValue = emptyList())

    // Version-locked floor (resolved once on appear; null = gate closed).
    var minOutput by remember { mutableStateOf<Long?>(null) }
    LaunchedEffect(wallet) {
        minOutput = wallet?.let { runCatching { it.minAmounts().minOutput }.getOrNull() }
    }

    val balance = identity?.balance ?: 0L

    // Destination: own-wallet address (20-byte hash) or external 40-hex hash.
    var useOwnWallet by remember { mutableStateOf(true) }
    var selectedOwnHashHex by rememberSaveable { mutableStateOf("") }
    var externalHashHex by rememberSaveable { mutableStateOf("") }
    var amountText by rememberSaveable { mutableStateOf("") }
    var isSubmitting by remember { mutableStateOf(false) }
    var submitGeneration by remember { mutableStateOf(0) }
    var error by remember { mutableStateOf<String?>(null) }
    var done by remember { mutableStateOf(false) }

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
        minOutput != null && amount >= minOutput!! && amount <= balance
    val canSubmit = wallet != null && manager != null && destinationHash != null &&
        amountOk && !isSubmitting && !done

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Transfer to Platform Address") },
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
            FormSection(title = "Source Identity") {
                LabeledContent("Identity", identity?.mainDpnsName ?: identityIdHex.take(16) + "…")
                LabeledContent("Balance", formatCredits(balance))
            }

            FormSection(title = "Destination") {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    FilterChip(
                        selected = useOwnWallet,
                        onClick = { useOwnWallet = true },
                        label = { Text("My Wallet") },
                        modifier = Modifier.testTag("transferToAddress.destOwn"),
                    )
                    FilterChip(
                        selected = !useOwnWallet,
                        onClick = { useOwnWallet = false },
                        label = { Text("External") },
                        modifier = Modifier.testTag("transferToAddress.destExternal"),
                    )
                }
                if (useOwnWallet) {
                    if (ownRecipients.isEmpty()) {
                        Text(
                            "No platform addresses yet. Fund a Platform address first, or " +
                                "paste an external hash.",
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
                        modifier = Modifier.fillMaxWidth().testTag("transferToAddress.externalHash"),
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
                    modifier = Modifier.fillMaxWidth().testTag("transferToAddress.amount"),
                )
                minOutput?.let { LabeledContent("Minimum", formatCredits(it)) }
                LabeledContent("Available", formatCredits(balance))
            }

            if (done) {
                Text(
                    "Transfer submitted.",
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.testTag("transferToAddress.done"),
                )
            }

            SubmitButton(
                text = "Transfer Credits",
                isLoading = isSubmitting,
                enabled = canSubmit,
                modifier = Modifier.fillMaxWidth().testTag("transferToAddress.submit"),
            ) {
                val mgr = manager ?: return@SubmitButton
                val w = wallet ?: return@SubmitButton
                val hash = destinationHash ?: return@SubmitButton
                val amt = amount ?: return@SubmitButton
                isSubmitting = true
                val generation = ++submitGeneration
                scope.launch {
                    try {
                        mgr.identityCredits.transferToAddresses(
                            walletHandle = w.handle,
                            fromIdentityId = idBytes,
                            // Platform-address recipients are P2PKH only (type 0).
                            outputs = listOf(FundingInput(addressType = 0, hash = hash, credits = amt)),
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
