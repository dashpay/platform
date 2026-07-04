package org.dashfoundation.example.ui.wallet

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
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
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
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
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.flatMapLatest
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.map
import org.dashfoundation.dashsdk.persistence.entities.CoreAddressEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.generateQrBitmap
import org.dashfoundation.example.util.toHex

/**
 * Receive sheet — port of `ReceiveAddressView.swift`'s Core tab: the
 * lowest-indexed never-used external address on the primary BIP44 account,
 * a QR code (zxing ← CoreImage), tap/button copy, and share via
 * ACTION_SEND. The Platform / Shielded tabs and the faucet dev tools are
 * iOS-only for now (their backing flows aren't bridged).
 *
 * "Never used" approximates iOS's `addr.txos.isEmpty` with
 * `!isUsed && balance == 0` — the persisted flags Rust maintains for the
 * same pool rows.
 */
@OptIn(ExperimentalMaterial3Api::class, kotlinx.coroutines.ExperimentalCoroutinesApi::class)
@Composable
fun ReceiveAddressSheet(
    walletId: ByteArray,
    onDismiss: () -> Unit,
) {
    val container = LocalAppContainer.current
    val context = LocalContext.current

    // Primary BIP44 account → its external pool, lowest unused index
    // (← nextCoreReceiveAddress).
    val receiveAddress by remember {
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

    var copied by remember { mutableStateOf(false) }
    LaunchedEffect(copied) {
        if (copied) {
            delay(2_000)
            copied = false
        }
    }

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

            val address = receiveAddress
            if (address == null) {
                Text(
                    "No unused receive address available yet — sync the wallet to extend the pool.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.padding(vertical = 24.dp),
                )
            } else {
                val qr = remember(address.address) { generateQrBitmap(address.address) }
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

                Text("Your Core Address", style = MaterialTheme.typography.labelMedium)
                Text(
                    address.address,
                    style = MaterialTheme.typography.bodyMedium,
                    fontFamily = FontFamily.Monospace,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.testTag("receive.address"),
                )

                Column(Modifier.fillMaxWidth()) {
                    LabeledContent("Path", address.derivationPath)
                    if (address.publicKey.isNotEmpty()) {
                        LabeledContent(
                            "Public Key",
                            address.publicKey.toHex().take(20) + "…",
                        )
                    }
                }

                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    Button(
                        onClick = {
                            copyToClipboard(context, address.address)
                            copied = true
                        },
                        modifier = Modifier
                            .weight(1f)
                            .testTag("receive.copyButton"),
                    ) { Text(if (copied) "Copied!" else "Copy Address") }

                    OutlinedButton(
                        onClick = { shareText(context, address.address) },
                        modifier = Modifier
                            .weight(1f)
                            .testTag("receive.shareButton"),
                    ) { Text("Share") }
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
