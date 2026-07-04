package org.dashfoundation.example.ui.wallet

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
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import androidx.compose.runtime.rememberCoroutineScope
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.QrScanner
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.ScannedPayment
import org.dashfoundation.example.util.formatDuffs
import org.dashfoundation.example.util.parseDashToDuffs

/**
 * Send form — port of the Core (`coreToCore`) flow of
 * `SendTransactionView.swift` + `SendViewModel.swift`: recipient +
 * decimal-DASH amount with `Decimal`-backed duffs parsing, QR-scan entry,
 * spendable-balance display, and the static Core fee estimate.
 *
 * Submit calls `ManagedPlatformWallet.sendToAddresses` →
 * `WalletManagerNative.walletCoreSendToAddresses` → (Rust)
 * `platform_wallet_get_core` + `core_wallet_send_to_addresses`, which
 * builds, signs (via the manager's mnemonic resolver as the Core signer),
 * and broadcasts in one call — the Kotlin port of Swift's
 * `ManagedCoreWallet.sendToAddresses`.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalStdlibApi::class)
@Composable
fun SendTransactionScreen(
    walletIdHex: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current

    val scope = rememberCoroutineScope()

    var recipient by rememberSaveable { mutableStateOf("") }
    var amountText by rememberSaveable { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var isSending by remember { mutableStateOf(false) }
    var sentTxidHex by remember { mutableStateOf<String?>(null) }

    // Result hand-back from the QR scanner (← QRScannerView's onScan).
    val savedStateHandle = navController.currentBackStackEntry?.savedStateHandle
    LaunchedEffect(savedStateHandle) {
        savedStateHandle
            ?.getStateFlow<String?>(QrScanner.RESULT_KEY, null)
            ?.collect { raw ->
                if (raw != null) {
                    val parsed = ScannedPayment.parse(raw)
                    if (parsed == null) {
                        error = "The scanned code doesn't contain a Dash address."
                    } else {
                        recipient = parsed.address
                        parsed.amount?.let { amountText = it }
                    }
                    savedStateHandle[QrScanner.RESULT_KEY] = null
                }
            }
    }

    // Spendable Core balance from the live wallet handle.
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by remember(manager) {
        manager?.wallets ?: MutableStateFlow(emptyMap<String, ManagedPlatformWallet>())
    }.collectAsStateWithLifecycle()
    val managed = walletsMap[walletIdHex]
    var balance by remember { mutableStateOf<ManagedPlatformWallet.Balance?>(null) }
    LaunchedEffect(managed) {
        while (managed != null && !managed.isClosed) {
            balance = runCatching { managed.balance() }.getOrNull()
            delay(2_000)
        }
    }

    val amountDuffs = parseDashToDuffs(amountText)
    val trimmedRecipient = recipient.trim()
    // Light address validation — base58 decodes to the 25-byte
    // base58check payload (version + hash160 + checksum). Full network-
    // aware validation is `DashAddress.parse` → Rust on iOS; that FFI
    // isn't bridged, so this catches typos without judging the network.
    val recipientLooksValid = trimmedRecipient.isNotEmpty() &&
        Base58.decode(trimmedRecipient)?.size == 25
    val canSend = recipientLooksValid && (amountDuffs ?: 0) > 0

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Send Dash") },
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
            FormSection(title = "Recipient") {
                OutlinedTextField(
                    value = recipient,
                    onValueChange = { recipient = it },
                    label = { Text("Dash address") },
                    singleLine = true,
                    isError = trimmedRecipient.isNotEmpty() && !recipientLooksValid,
                    supportingText = {
                        if (trimmedRecipient.isNotEmpty() && !recipientLooksValid) {
                            Text("Not a valid Dash address")
                        }
                    },
                    trailingIcon = {
                        IconButton(
                            onClick = { navController.navigate(QrScanner) },
                            modifier = Modifier.testTag("send.scanQrButton"),
                        ) {
                            Icon(Icons.Default.QrCodeScanner, contentDescription = "Scan QR code")
                        }
                    },
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 8.dp)
                        .testTag("send.recipientField"),
                )
            }

            FormSection(title = "Amount") {
                OutlinedTextField(
                    value = amountText,
                    onValueChange = { amountText = it },
                    label = { Text("Amount (DASH)") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                    isError = amountText.isNotBlank() && amountDuffs == null,
                    supportingText = {
                        if (amountText.isNotBlank() && amountDuffs == null) {
                            Text("Enter a positive amount with at most 8 decimals")
                        }
                    },
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 8.dp)
                        .testTag("send.amountField"),
                )
                amountDuffs?.let { LabeledContent("Duffs", it.toString()) }
            }

            FormSection(title = "Summary") {
                LabeledContent(
                    "Spendable",
                    balance?.let { formatDuffs(it.confirmed) } ?: "—",
                )
                // Static Core fee estimate (← SendFlow.coreToCore.estimatedFee).
                LabeledContent("Estimated fee", formatDuffs(500_000))
            }

            SubmitButton(
                text = "Send",
                isLoading = isSending,
                enabled = canSend && !isSending,
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("send.submitButton"),
            ) {
                val activeManager = manager
                val wallet = managed
                val amount = amountDuffs
                if (activeManager == null || wallet == null || amount == null) {
                    error = "Wallet is not ready. Try again in a moment."
                    return@SubmitButton
                }
                isSending = true
                scope.launch {
                    try {
                        val txBytes = wallet.sendToAddresses(
                            recipients = listOf(trimmedRecipient to amount),
                            coreSignerHandle = activeManager.mnemonicResolverHandle,
                        )
                        sentTxidHex = txidHexOf(txBytes)
                    } catch (t: Throwable) {
                        error = t.message ?: "Failed to broadcast the transaction."
                    } finally {
                        isSending = false
                    }
                }
            }
        }
    }

    sentTxidHex?.let { txid ->
        androidx.compose.material3.AlertDialog(
            onDismissRequest = {
                sentTxidHex = null
                navController.popBackStack()
            },
            title = { Text("Payment Sent") },
            text = {
                Text(
                    "Your transaction was signed and broadcast.\n\nTxid:\n$txid",
                    modifier = Modifier.testTag("send.successTxid"),
                )
            },
            confirmButton = {
                androidx.compose.material3.TextButton(
                    onClick = {
                        sentTxidHex = null
                        navController.popBackStack()
                    },
                ) { Text("Done") }
            },
        )
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

/**
 * Standard Dash/Bitcoin txid of a serialized transaction: the
 * double-SHA-256 of the raw bytes, byte-reversed, rendered as lowercase
 * hex — matches what block explorers display. Computed on-device because
 * `core_wallet_send_to_addresses` returns the serialized tx, not the id.
 */
@OptIn(ExperimentalStdlibApi::class)
private fun txidHexOf(txBytes: ByteArray): String {
    val sha = java.security.MessageDigest.getInstance("SHA-256")
    val once = sha.digest(txBytes)
    sha.reset()
    val twice = sha.digest(once)
    twice.reverse()
    return twice.toHexString()
}
