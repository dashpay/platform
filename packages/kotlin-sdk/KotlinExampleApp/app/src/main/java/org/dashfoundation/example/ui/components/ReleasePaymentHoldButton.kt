package org.dashfoundation.example.ui.components

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import org.dashfoundation.example.util.PaymentFailure

/**
 * The confirmed release of a failed payment's slot hold — port of the
 * "Payment checked in history — release" button + confirmation dialog in
 * the Swift progress views. Shown only for a hold without a lock to resume
 * ([PaymentFailure.releasableAfterReconciliation]); a held lock stays on the
 * resume path. [onRelease] runs only after the user confirmed in the dialog.
 */
@Composable
fun ReleasePaymentHoldButton(
    failure: PaymentFailure,
    testTagPrefix: String,
    onRelease: () -> Unit,
) {
    if (!failure.releasableAfterReconciliation) return
    var confirmingRelease by remember { mutableStateOf(false) }
    OutlinedButton(
        onClick = { confirmingRelease = true },
        modifier = Modifier.fillMaxWidth().testTag("$testTagPrefix.release"),
    ) { Text("Payment checked in history — release") }

    if (confirmingRelease) {
        AlertDialog(
            onDismissRequest = { confirmingRelease = false },
            title = { Text("Release this payment?") },
            text = {
                Text(
                    "Release only after the transaction history shows this payment as " +
                        "completed or failed. A new payment for this slot may otherwise pay twice.",
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmingRelease = false
                        onRelease()
                    },
                    modifier = Modifier.testTag("$testTagPrefix.release.confirm"),
                ) { Text("Release", color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = {
                TextButton(onClick = { confirmingRelease = false }) { Text("Cancel") }
            },
        )
    }
}
