package org.dashfoundation.example.ui.dashpay

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.text.KeyboardOptions
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.example.ui.theme.appStatusColors
import org.dashfoundation.example.util.formatDuffs
import org.dashfoundation.example.util.parseDashToDuffs

/**
 * Send-a-Dash-payment sheet content — port of `SendDashPayPaymentSheet.swift`,
 * hosted inside a `ModalBottomSheet` by [ContactDetailScreen]. Recipient row +
 * amount (decimal DASH → duffs) with a balance check, then
 * `dashpay.sendPayment`. On success it fires [onSent] (the payment-durability
 * refresh in the parent), kicks a sync, and auto-closes after a short settle.
 *
 * No memo field: DashPay payments are plain Core-chain transactions with no
 * on-chain memo slot (matching iOS), so `memo = null`.
 */
@Composable
fun SendDashPayPaymentSheet(
    manager: PlatformWalletManager,
    walletId: ByteArray,
    senderIdentityId: ByteArray,
    contactId: ByteArray,
    contactDisplayName: String,
    contactDpnsName: String?,
    onSendingChange: (Boolean) -> Unit,
    onSent: () -> Unit,
    onClose: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    val wallet = remember(manager, walletId) { manager.wallet(forWalletId = walletId) }

    var amountText by remember { mutableStateOf("") }
    var isSending by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var successTxidHex by remember { mutableStateOf<String?>(null) }
    var recipientProfile by remember { mutableStateOf<DashPayProfile?>(null) }
    var senderBalanceDuffs by remember { mutableStateOf<Long?>(null) }

    LaunchedEffect(wallet) {
        val w = wallet ?: return@LaunchedEffect
        recipientProfile = parseDashPayProfile(w.dashpay.getContactProfile(senderIdentityId, contactId))
        // Kotlin Balance has no `spendable`; confirmed is the spendable slice.
        senderBalanceDuffs = runCatching { w.balance().confirmed }.getOrNull()
    }

    val amountDuffs = remember(amountText) { parseDashToDuffs(amountText) }
    val exceedsBalance = senderBalanceDuffs?.let { bal -> amountDuffs?.let { it > bal } } ?: false
    val recipientName = recipientProfile?.displayName?.trim()?.takeIf { it.isNotEmpty() }
        ?: contactDpnsName?.takeIf { it.isNotBlank() }
        ?: contactDisplayName
    val canSend = amountDuffs != null && amountDuffs > 0 && !exceedsBalance &&
        senderBalanceDuffs != 0L && !isSending

    fun send() {
        val w = wallet ?: run { errorMessage = "No wallet available for this identity"; return }
        val duffs = amountDuffs ?: return
        isSending = true
        onSendingChange(true)
        errorMessage = null
        scope.launch {
            performDashPaySend(
                sender = {
                    w.dashpay.sendPayment(
                        fromIdentityId = senderIdentityId,
                        toContactIdentityId = contactId,
                        amountDuffs = duffs,
                        coreSignerHandle = manager.mnemonicResolverHandle,
                        memo = null,
                    )
                },
                onSuccessTxid = { successTxidHex = it },
                onError = { errorMessage = it },
                onSent = onSent,
                settle = {
                    // Best-effort tail (kick a sweep + settle before auto-close).
                    kickDashPaySync(scope, manager)
                    delay(1500)
                },
                onClose = onClose,
                onSendingDone = {
                    isSending = false
                    onSendingChange(false)
                },
            )
        }
    }

    Column(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Text("Send Dash", style = MaterialTheme.typography.titleLarge)

        // Recipient
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            DashPayAvatar(recipientProfile?.avatarUrl, recipientName)
            Text(recipientName, style = MaterialTheme.typography.titleMedium)
        }

        if (senderBalanceDuffs == 0L) {
            Text(
                "Your balance is 0 DASH — top up your wallet before sending.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        } else {
            OutlinedTextField(
                value = amountText,
                onValueChange = { amountText = it },
                modifier = Modifier.fillMaxWidth().testTag("dashpay.send.amount"),
                label = { Text("Amount (DASH)") },
                placeholder = { Text("0.001") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
            )
        }
        senderBalanceDuffs?.let { bal ->
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(
                    "Your balance",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    formatDuffs(bal),
                    style = MaterialTheme.typography.bodySmall,
                    color = if (exceedsBalance) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        if (amountText.isNotEmpty() && amountDuffs == null) {
            Text(
                "Enter a valid decimal Dash amount",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
            )
        } else if (exceedsBalance) {
            Text(
                "Amount exceeds your spendable balance",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
            )
        }

        successTxidHex?.let { hex ->
            Text(
                "Sent! txid: ${hex.take(16)}…",
                style = MaterialTheme.typography.bodySmall,
                color = appStatusColors.success,
            )
        }
        errorMessage?.let { msg ->
            Text(msg, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(12.dp, Alignment.End),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            TextButton(
                onClick = onClose,
                enabled = !isSending,
                modifier = Modifier.testTag("dashpay.send.cancel"),
            ) { Text("Cancel") }
            if (isSending) {
                CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
            } else {
                androidx.compose.material3.Button(
                    onClick = { send() },
                    enabled = canSend,
                    modifier = Modifier.testTag("dashpay.send.confirm"),
                ) { Text("Send") }
            }
        }
    }
}

/** Broadcasts a DashPay payment; the sole suspend seam so [performDashPaySend] is JVM-testable. */
internal fun interface PaymentSender {
    suspend fun send(): ByteArray?
}

/**
 * The payment send flow, extracted from [SendDashPayPaymentSheet] so the
 * dispose-mid-send double-send guard is unit-testable. Runs in the CALLER's Job
 * (a plain `suspend fun`, no new scope — so cancellation semantics are the ones
 * under test): the broadcast + its durability bookkeeping ([onSuccessTxid] then
 * [onSent]) run inside [NonCancellable], so a teardown that cancels mid-send
 * cannot skip [onSent]. Losing [onSent] after the coin has left the wallet (the
 * JNI broadcast is uncancellable) would invite a double-send on retry. The
 * best-effort tail ([settle] then [onClose]) stays cancellable, and a
 * [CancellationException] still propagates so structured concurrency is intact.
 */
internal suspend fun performDashPaySend(
    sender: PaymentSender,
    onSuccessTxid: (String?) -> Unit,
    onError: (String) -> Unit,
    onSent: () -> Unit,
    settle: suspend () -> Unit,
    onClose: () -> Unit,
    onSendingDone: () -> Unit,
) {
    try {
        withContext(NonCancellable) {
            val txid = sender.send()
            onSuccessTxid(txid?.let { txidDisplayHex(it) })
            onSent()
        }
        settle()
        onClose()
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        onError(e.message ?: "Send failed")
    } finally {
        onSendingDone()
    }
}
