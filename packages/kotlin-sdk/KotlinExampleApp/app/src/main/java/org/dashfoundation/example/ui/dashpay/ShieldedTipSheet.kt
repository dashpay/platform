package org.dashfoundation.example.ui.dashpay

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
import androidx.compose.ui.unit.dp
import java.math.BigDecimal
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.tokens.ShieldedTipRecipientChange
import org.dashfoundation.dashsdk.tokens.ShieldedTipReview
import org.dashfoundation.dashsdk.tokens.ShieldedTipSource
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.DashAddress

/** A credit amount as DASH text (1e11 credits per DASH), shared by the tip confirmation and the profile balance. */
internal fun shieldedTipDashText(credits: Long): String =
    BigDecimal.valueOf(credits, 11).stripTrailingZeros().toPlainString()

/**
 * Username tipping with the same review and confirmation boundary as Swift's
 * `SendShieldedTipSheet`: the wallet parses the amount, resolves the recipient, flags a
 * change since this wallet last confirmed the name, and refuses a tip while an earlier one
 * to the recipient is unsettled. The sheet renders those answers.
 */
@Composable
fun ShieldedTipSheet(
    manager: PlatformWalletManager,
    walletId: ByteArray,
    identityId: ByteArray,
    submission: ShieldedTipSubmission,
) {
    val scope = rememberCoroutineScope()
    var username by remember { mutableStateOf("") }
    var amount by remember { mutableStateOf("") }
    var review by remember { mutableStateOf<ShieldedTipReview?>(null) }
    var changedReview by remember { mutableStateOf<ShieldedTipReview?>(null) }
    var confirmedAmount by remember { mutableStateOf<Long?>(null) }
    var reviewing by remember { mutableStateOf(false) }
    val busy = reviewing || submission.busy
    val submitted = submission.submitted
    var spendTips by remember { mutableStateOf(false) }
    var message by remember { mutableStateOf<String?>(null) }
    var confirmRelease by remember { mutableStateOf(false) }

    // The wallet reviews the recipient, after `before` if given; a changed recipient is
    // shown for confirmation first. Rust trims and normalizes the username.
    fun reviewRecipient(before: suspend () -> Unit = {}) {
        reviewing = true
        message = null
        scope.launch {
            try {
                before()
                confirmedAmount = manager.parseShieldedTipAmount(amount)
                val reviewed = manager.reviewShieldedTip(walletId, username)
                if (reviewed.change is ShieldedTipRecipientChange.Changed) {
                    review = null
                    changedReview = reviewed
                } else {
                    review = reviewed
                }
            } catch (e: kotlinx.coroutines.CancellationException) {
                throw e
            } catch (e: Exception) {
                message = e.message ?: "Unable to review tip"
                review = null
            } finally { reviewing = false }
        }
    }

    if (confirmRelease) {
        AlertDialog(
            onDismissRequest = { confirmRelease = false },
            title = { Text("Allow a new tip?") },
            text = { Text("The earlier tips from this wallet whose outcome is unknown may still be paid. If they are, a new tip pays the recipient again.") },
            confirmButton = {
                TextButton(onClick = {
                    confirmRelease = false
                    // The user accepted that the unresolved tips may still be paid.
                    reviewRecipient {
                        manager.abandonUnresolvedShieldedTips(walletId, acknowledgePossibleExecution = true)
                    }
                }) { Text("Allow, it may still be paid") }
            },
            dismissButton = {
                TextButton(onClick = { confirmRelease = false }) { Text("Keep waiting") }
            },
        )
    }

    changedReview?.let { changed ->
        AlertDialog(
            onDismissRequest = { changedReview = null },
            title = { Text("Tip recipient changed") },
            text = { Text("The identity or shielded address for this username differs from your previous confirmation. Verify the change with the recipient before continuing.") },
            confirmButton = {
                TextButton(onClick = { review = changed; changedReview = null }) { Text("Review new recipient") }
            },
            dismissButton = {
                TextButton(onClick = { changedReview = null }) { Text("Cancel") }
            },
        )
    }

    Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Send a shielded tip")
        OutlinedTextField(username, { username = it; review = null }, label = { Text("Username") }, enabled = !busy && !submitted)
        OutlinedTextField(amount, { amount = it; review = null }, label = { Text("Amount (DASH)") }, enabled = !busy && !submitted)
        Row {
            Checkbox(checked = spendTips, enabled = !busy && !submitted,
                onCheckedChange = { spendTips = it; review = null })
            Text("Spend from this identity's dedicated tip account")
        }
        review?.let {
            Text("Recipient: ${Base58.encode(it.recipient.identityId)}")
            DashAddress.encodeOrchard(it.recipient.address, manager.network)?.let { address ->
                SelectionContainer { Text(address) }
            }
            val shown = confirmedAmount?.let(::shieldedTipDashText) ?: "—"
            Text("Send $shown DASH from your ${if (spendTips) "tip" else "main shielded"} account to $username?")
            if (it.unresolved) {
                Text("An earlier payment to this recipient has not settled yet. Wait for shielded sync before sending another.")
                TextButton(onClick = { reviewRecipient() }, enabled = !busy) { Text("Check again") }
                TextButton(onClick = { confirmRelease = true }, enabled = !busy) { Text("Allow a new tip anyway") }
            }
        }
        (message ?: submission.message)?.let { Text(it) }
        if (submission.status == ShieldedTipSubmission.Status.Sent ||
            submission.status == ShieldedTipSubmission.Status.Unresolved
        ) {
            TextButton(onClick = { submission.startNewTip(); review = null }) { Text("Start a new tip") }
        }
        SubmitButton(
            text = if (review == null) "Review tip" else "Confirm and send",
            isLoading = busy,
            enabled = !busy && !submitted && changedReview == null && review?.unresolved != true,
            modifier = Modifier.fillMaxWidth(),
        ) {
            val selected = review
            if (selected != null) {
                val sendUsername = username
                val sendAmount = requireNotNull(confirmedAmount)
                val source = if (spendTips) ShieldedTipSource.Identity(identityId) else ShieldedTipSource.Account(0)
                // Whatever the outcome, the next attempt reviews the recipient again.
                review = null
                submission.submit {
                    manager.sendShieldedTip(walletId, sendUsername, selected.recipient, sendAmount, source)
                }
            } else {
                reviewRecipient()
            }
        }
    }
}
