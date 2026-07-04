package org.dashfoundation.example.ui.wallet

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import org.dashfoundation.example.LocalSecureScreen

/**
 * Read-only recovery-phrase reveal, gated by biometric auth on the caller
 * side — port of `SeedPhraseRevealSheet` (WalletDetailView.swift): warning
 * banner, numbered word grid, copy-to-clipboard. Holds FLAG_SECURE while
 * presented.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SeedPhraseRevealSheet(
    mnemonic: String,
    onDismiss: () -> Unit,
) {
    val context = LocalContext.current
    val setSecure = LocalSecureScreen.current
    DisposableEffect(Unit) {
        setSecure(true)
        onDispose { setSecure(false) }
    }

    var copied by remember { mutableStateOf(false) }
    LaunchedEffect(copied) {
        if (copied) {
            delay(2_000)
            copied = false
        }
    }

    val words = remember(mnemonic) { mnemonic.trim().split(Regex("\\s+")) }

    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 24.dp)
                .padding(bottom = 32.dp),
            verticalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(16.dp),
        ) {
            Text("Recovery Phrase", style = MaterialTheme.typography.titleLarge)

            Text(
                "Never share this phrase. Anyone who sees it can spend your funds.",
                style = MaterialTheme.typography.bodyMedium,
                color = Color.White,
                modifier = Modifier
                    .fillMaxWidth()
                    .background(MaterialTheme.colorScheme.error, RoundedCornerShape(10.dp))
                    .padding(12.dp),
            )

            WordGrid(words)

            OutlinedButton(
                onClick = {
                    val clipboard =
                        context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                    clipboard.setPrimaryClip(ClipData.newPlainText("Recovery phrase", mnemonic))
                    copied = true
                },
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("seedReveal.copyButton"),
            ) { Text(if (copied) "Copied!" else "Copy to Clipboard") }
        }
    }
}
