package org.dashfoundation.example.ui.components

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable

/**
 * Error dialog driven by a nullable message — reproduces the
 * `AlertMessage: Identifiable` + `.alert(item:)` pattern used across the
 * iOS app's forms. Render unconditionally; shows only while [message]
 * is non-null.
 */
@Composable
fun ErrorAlertDialog(
    message: String?,
    title: String = "Error",
    onDismiss: () -> Unit,
) {
    if (message == null) return
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = { Text(message) },
        confirmButton = {
            TextButton(onClick = onDismiss) { Text("OK") }
        },
    )
}
