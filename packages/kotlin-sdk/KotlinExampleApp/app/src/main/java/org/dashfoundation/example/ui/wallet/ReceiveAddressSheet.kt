package org.dashfoundation.example.ui.wallet

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.flatMapLatest
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.persistence.entities.CoreAddressEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.services.faucet.TestnetFaucet
import org.dashfoundation.example.services.faucet.TestnetFaucetOutcome
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.generateQrBitmap
import org.dashfoundation.example.util.toHex

/** Which receive-address family the sheet is showing. */
private enum class ReceiveTab(val label: String) {
    CORE("Core"),
    PLATFORM("Platform"),
}

/** A displayable receive address (any family), flattened for the UI. */
private data class ReceiveAddr(
    val address: String,
    val derivationPath: String,
    val publicKey: ByteArray,
)

/**
 * Receive sheet — port of `ReceiveAddressView.swift`. Segmented **Core** /
 * **Platform** tabs, each showing the lowest-indexed unused address on its
 * pool with a QR code, copy / share, and (Core + testnet) a one-tap testnet
 * faucet. The **Shielded** tab is owned by the shielded workstream and slots
 * into [ReceiveTab] alongside these.
 *
 * "Never used" approximates iOS's `addr.txos.isEmpty` with
 * `!isUsed && balance == 0` — the persisted flags Rust maintains for the
 * same pool rows.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalCoroutinesApi::class)
@Composable
fun ReceiveAddressSheet(
    walletId: ByteArray,
    onDismiss: () -> Unit,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val network by appState.currentNetwork.collectAsStateWithLifecycle()

    var tab by remember { mutableStateOf(ReceiveTab.CORE) }

    // Core: primary BIP44 account → its external pool, lowest unused index
    // (← nextCoreReceiveAddress).
    val coreAddress by remember {
        container.database.accountDao()
            .observeByWalletAndType(walletId, accountType = 0)
            .map { accounts ->
                accounts.filter { it.standardTag == 0 }.minByOrNull { it.accountIndex }
            }
            .flatMapLatest { account ->
                if (account == null) {
                    flowOf<CoreAddressEntity?>(null)
                } else {
                    container.database.coreAddressDao().observeByAccount(account.id)
                        .map { pool ->
                            pool.filter { it.poolTypeTag == 0 && !it.isUsed && it.balance == 0L }
                                .minByOrNull { it.addressIndex }
                        }
                }
            }
    }.collectAsStateWithLifecycle(initialValue = null)

    // Platform: dedicated DIP-17 Platform-address store, lowest unused index
    // (← nextPlatformReceiveAddress).
    val platformAddress by remember {
        container.database.platformAddressDao().observeByWallet(walletId)
            .map { rows ->
                rows.filter { !it.isUsed }.minByOrNull { it.addressIndex }
            }
    }.collectAsStateWithLifecycle(initialValue = null)

    val current: ReceiveAddr? = when (tab) {
        ReceiveTab.CORE -> coreAddress?.let {
            ReceiveAddr(it.address, it.derivationPath, it.publicKey)
        }
        ReceiveTab.PLATFORM -> platformAddress?.let {
            ReceiveAddr(it.address, it.derivationPath, it.publicKey)
        }
    }

    var copied by remember { mutableStateOf(false) }
    LaunchedEffect(copied) {
        if (copied) {
            delay(2_000)
            copied = false
        }
    }

    var faucetStatus by remember { mutableStateOf<String?>(null) }
    var faucetLoading by remember { mutableStateOf(false) }

    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 24.dp)
                .padding(bottom = 32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text("Receive Dash", style = MaterialTheme.typography.titleLarge)

            SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                ReceiveTab.entries.forEachIndexed { index, entry ->
                    SegmentedButton(
                        selected = tab == entry,
                        onClick = { tab = entry },
                        shape = SegmentedButtonDefaults.itemShape(index, ReceiveTab.entries.size),
                        modifier = Modifier.testTag("receive.tab.${entry.label}"),
                    ) { Text(entry.label) }
                }
            }

            if (current == null) {
                Text(
                    when (tab) {
                        ReceiveTab.CORE ->
                            "No unused receive address available yet — sync the wallet to extend the pool."
                        ReceiveTab.PLATFORM ->
                            "No Platform receive address available yet — create a wallet with Platform address persistence."
                    },
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.padding(vertical = 24.dp),
                )
            } else {
                val qr = remember(current.address) { generateQrBitmap(current.address) }
                qr?.let {
                    Image(
                        bitmap = it.asImageBitmap(),
                        contentDescription = "Receive address QR code",
                        modifier = Modifier
                            .size(240.dp)
                            .background(Color.White, RoundedCornerShape(12.dp))
                            .padding(8.dp),
                    )
                }

                Text(
                    "Your ${tab.label} Address",
                    style = MaterialTheme.typography.labelMedium,
                )
                Text(
                    current.address,
                    style = MaterialTheme.typography.bodyMedium,
                    fontFamily = FontFamily.Monospace,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.testTag("receive.address"),
                )

                Column(Modifier.fillMaxWidth()) {
                    LabeledContent("Path", current.derivationPath)
                    if (current.publicKey.isNotEmpty()) {
                        LabeledContent(
                            "Public Key",
                            current.publicKey.toHex().take(20) + "…",
                        )
                    }
                }

                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    Button(
                        onClick = {
                            copyToClipboard(context, current.address)
                            copied = true
                        },
                        modifier = Modifier
                            .weight(1f)
                            .testTag("receive.copyButton"),
                    ) { Text(if (copied) "Copied!" else "Copy Address") }

                    OutlinedButton(
                        onClick = { shareText(context, current.address) },
                        modifier = Modifier
                            .weight(1f)
                            .testTag("receive.shareButton"),
                    ) { Text("Share") }
                }

                // Dev tool: public testnet faucet (faucet.thepasta.org), Core
                // tab + testnet only (← requestFromTestnetFaucet).
                if (tab == ReceiveTab.CORE && network == Network.TESTNET) {
                    Button(
                        onClick = {
                            if (faucetLoading) return@Button
                            val addr = current.address
                            scope.launch {
                                faucetLoading = true
                                faucetStatus = "Solving captcha…"
                                val outcome = TestnetFaucet().requestCoreDash(addr)
                                faucetStatus = when (outcome) {
                                    is TestnetFaucetOutcome.Sent ->
                                        "Sent ${outcome.amount} tDASH! tx ${outcome.txid.take(12)}…"
                                    is TestnetFaucetOutcome.RateLimited -> {
                                        openWebFaucet(context, addr)
                                        outcome.message
                                    }
                                    is TestnetFaucetOutcome.Failed -> {
                                        openWebFaucet(context, addr)
                                        outcome.reason
                                    }
                                }
                                faucetLoading = false
                                delay(6_000)
                                faucetStatus = null
                            }
                        },
                        enabled = !faucetLoading,
                        colors = ButtonDefaults.buttonColors(containerColor = Color(0xFFED8A19)),
                        modifier = Modifier
                            .fillMaxWidth()
                            .testTag("receive.faucetButton"),
                    ) {
                        if (faucetLoading) {
                            CircularProgressIndicator(
                                modifier = Modifier
                                    .size(18.dp)
                                    .padding(end = 4.dp),
                                strokeWidth = 2.dp,
                                color = Color.White,
                            )
                        }
                        Text(faucetStatus ?: "Get 1 tDASH — Testnet Faucet")
                    }
                }
            }
        }
    }
}

private fun copyToClipboard(context: Context, text: String) {
    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    clipboard.setPrimaryClip(ClipData.newPlainText("Dash address", text))
}

private fun shareText(context: Context, text: String) {
    val intent = Intent(Intent.ACTION_SEND).apply {
        type = "text/plain"
        putExtra(Intent.EXTRA_TEXT, text)
    }
    context.startActivity(Intent.createChooser(intent, "Share address"))
}

/** Fallback when the API path is rate-limited / fails: copy + open the web faucet. */
private fun openWebFaucet(context: Context, address: String) {
    copyToClipboard(context, address)
    runCatching {
        context.startActivity(
            Intent(Intent.ACTION_VIEW, Uri.parse(TestnetFaucet.WEB_URL)).apply {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            },
        )
    }
}
