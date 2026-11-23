package org.dashfoundation.example.ui.shielded

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.dashfoundation.dashsdk.wallet.ShieldedIdentityDebitRecoveryRecord

/**
 * "In-Progress Identity Shields": the durable Type 21 records Rust keeps,
 * shown above Pending so an interrupted shield is visible after the app is
 * killed and relaunched. Renders nothing when there are no records and the
 * last read did not fail. Port of `ShieldedIdentityDebitRecoverySection` in
 * `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Core/Views/ShieldedIdentityDebitRecoveryView.swift`;
 * the accessibility identifiers are reused as test tags.
 */
@Composable
internal fun IdentityShieldRecoverySection(
    state: RecoveryReadState<ShieldedIdentityDebitRecoveryRecord>,
    identityLabels: Map<String, String>,
    isChecking: Boolean,
    canCheck: Boolean,
    isAbandoning: Boolean,
    checkStatus: CheckNowStatus,
    onCheckNow: () -> Unit,
    onStopRetrying: (ShieldedIdentityDebitRecoveryRecord) -> Unit,
) {
    var pendingStop by remember { mutableStateOf<ShieldedIdentityDebitRecoveryRecord?>(null) }

    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        if (state.records.isNotEmpty()) {
            Text(
                IdentityShieldRecoveryDisplay.HEADER.uppercase(),
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            state.records.forEach { record ->
                RecordRow(
                    record = record,
                    identityLabels = identityLabels,
                    isAbandoning = isAbandoning,
                    onStop = { pendingStop = record },
                )
            }
            OutlinedButton(
                onClick = onCheckNow,
                enabled = !isChecking && canCheck,
                modifier = Modifier.fillMaxWidth().testTag("shieldedActivity.identityShields.checkNow"),
            ) {
                if (isChecking) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(16.dp).padding(end = 4.dp),
                        strokeWidth = 2.dp,
                    )
                }
                Text("Check Now")
            }
            CheckNowStatusLine(checkStatus)
            Text(
                IdentityShieldRecoveryDisplay.FOOTER,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        state.failureMessage?.let { message ->
            Text(
                "In-progress identity shields could not be read: $message",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.testTag("shieldedActivity.identityShields.loadError"),
            )
        }
    }

    pendingStop?.let { record ->
        AlertDialog(
            onDismissRequest = { pendingStop = null },
            title = { Text(IdentityShieldRecoveryDisplay.STOP_TITLE) },
            text = { Text(IdentityShieldRecoveryDisplay.stopRetryingWarning(record.identityId)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        pendingStop = null
                        onStopRetrying(record)
                    },
                    modifier = Modifier.testTag("shieldedActivity.identityShields.confirmStop"),
                ) { Text("Stop Retrying", color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = {
                TextButton(onClick = { pendingStop = null }) { Text("Cancel") }
            },
        )
    }
}

@Composable
private fun RecordRow(
    record: ShieldedIdentityDebitRecoveryRecord,
    identityLabels: Map<String, String>,
    isAbandoning: Boolean,
    onStop: () -> Unit,
) {
    Card(modifier = Modifier.fillMaxWidth().testTag("shieldedActivity.identityShields.row")) {
        Column(
            modifier = Modifier.padding(12.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    IdentityShieldRecoveryDisplay.identityLabel(record.identityId, identityLabels),
                    style = MaterialTheme.typography.titleSmall,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f),
                )
                record.amount?.let {
                    Text(formatCreditsExact(it), fontWeight = FontWeight.SemiBold)
                }
            }
            Text(
                IdentityShieldRecoveryDisplay.statusTitle(record.status, record.identityId),
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.tertiary,
            )
            Text(
                IdentityShieldRecoveryDisplay.statusDetail(record.status, record.identityId),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            if (IdentityShieldRecoveryDisplay.canStopRetrying(record.status)) {
                TextButton(
                    onClick = onStop,
                    enabled = !isAbandoning,
                    modifier = Modifier.testTag("shieldedActivity.identityShields.stopRetrying"),
                ) { Text("Stop Retrying", color = MaterialTheme.colorScheme.error) }
            }
        }
    }
}

/** The line under a Check Now button. ← Swift `ShieldedCheckNowStatusLine`. */
@Composable
internal fun CheckNowStatusLine(status: CheckNowStatus) {
    status.text?.let {
        Text(
            it,
            style = MaterialTheme.typography.bodySmall,
            color = if (status.isFailure) {
                MaterialTheme.colorScheme.error
            } else {
                MaterialTheme.colorScheme.onSurfaceVariant
            },
            modifier = Modifier.testTag("shielded.checkNow.status"),
        )
    }
}
