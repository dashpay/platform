package org.dashfoundation.example.ui.dashpay

import android.content.Context
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import java.math.BigDecimal
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.tokens.ShieldedTipRecipient
import org.dashfoundation.dashsdk.tokens.ShieldedTipRecipientHistory
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.DashAddress

/** Username tipping uses the same verified recipient and confirmation boundary as Swift. */
@Composable
fun ShieldedTipSheet(manager: PlatformWalletManager, wallet: ManagedPlatformWallet, walletId: ByteArray, tipAccount: Int) {
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    val history = remember(context) {
        ShieldedTipRecipientHistory(context.getSharedPreferences("dashpay.tipRecipients", Context.MODE_PRIVATE))
    }
    var changedRecipient by remember { mutableStateOf<ShieldedTipRecipient?>(null) }
    var username by remember { mutableStateOf("") }
    var amount by remember { mutableStateOf("") }
    var recipient by remember { mutableStateOf<ShieldedTipRecipient?>(null) }
    var confirmedAmount by remember { mutableStateOf<Long?>(null) }
    var busy by remember { mutableStateOf(false) }
    var submitted by remember { mutableStateOf(false) }
    var spendTips by remember { mutableStateOf(false) }
    var message by remember { mutableStateOf<String?>(null) }

    changedRecipient?.let { changed ->
        AlertDialog(
            onDismissRequest = { changedRecipient = null },
            title = { Text("Tip recipient changed") },
            text = { Text("The identity or shielded address for this username differs from your previous confirmation. Verify the change with the recipient before continuing.") },
            confirmButton = {
                TextButton(onClick = { recipient = changed; changedRecipient = null }) { Text("Review new recipient") }
            },
            dismissButton = {
                TextButton(onClick = { changedRecipient = null }) { Text("Cancel") }
            },
        )
    }

    Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Send a shielded tip")
        OutlinedTextField(username, { username = it; recipient = null }, label = { Text("Username") }, enabled = !busy && !submitted)
        OutlinedTextField(amount, { amount = it; recipient = null }, label = { Text("Amount (DASH)") }, enabled = !busy && !submitted)
        Row {
            Checkbox(checked = spendTips, enabled = !busy && !submitted,
                onCheckedChange = { spendTips = it; recipient = null })
            Text("Spend from my dedicated tip account")
        }
        recipient?.let {
            Text("Recipient: ${Base58.encode(it.identityId)}")
            DashAddress.encodeOrchard(it.address, manager.network)?.let { address ->
                SelectionContainer { Text(address) }
            }
            Text("Send $amount DASH from your ${if (spendTips) "tip" else "main shielded"} account to $username?")
        }
        message?.let { Text(it) }
        SubmitButton(
            text = if (recipient == null) "Review tip" else "Confirm and send",
            isLoading = busy, enabled = !busy && !submitted && changedRecipient == null, modifier = Modifier.fillMaxWidth(),
        ) {
            busy = true
            message = null
            scope.launch {
                try {
                    val selected = recipient
                    if (selected == null) {
                        val credits = BigDecimal(amount.trim()).movePointRight(11).longValueExact()
                        require(credits > 0) { "Enter a positive amount" }
                        confirmedAmount = credits
                        val resolved = wallet.dashpay.resolveShieldedTip(username.trim())
                        if (history.hasChanged(manager.network, walletId, username, resolved)) {
                            changedRecipient = resolved
                        } else {
                            recipient = resolved
                        }
                    } else {
                        // Ambiguous or unclassified outcomes remain locked; definitive failures permit a fresh review.
                        history.confirm(manager.network, walletId, username, selected)
                        submitted = true
                        manager.sendShieldedTip(walletId, username.trim(), selected, requireNotNull(confirmedAmount), account = if (spendTips) tipAccount else 0)
                        message = "Shielded tip sent."
                    }
                } catch (e: Exception) {
                    message = e.message ?: "Unable to send tip"
                    if (canReviewShieldedTipAfterFailure(e)) submitted = false
                    recipient = null
                } finally { busy = false }
            }
        }
    }
}
