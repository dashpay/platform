package org.dashfoundation.example.ui.wallet

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.navigation.NavHostController
import org.dashfoundation.example.LocalSecureScreen
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.WalletsHome

/**
 * Seed backup + confirmation — port of `SeedBackupView.swift`, extended
 * with a tap-words-in-order confirmation quiz after the "I wrote it down"
 * gate. The phrase is re-read from WalletStorage by wallet id (never a nav
 * arg), FLAG_SECURE is held for the whole flow, and backing out early
 * raises the "You haven't backed up" warning.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class, ExperimentalStdlibApi::class)
@Composable
fun SeedBackupScreen(
    walletIdHex: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val setSecure = LocalSecureScreen.current

    // Block screenshots / recents thumbnails while the phrase is visible.
    DisposableEffect(Unit) {
        setSecure(true)
        onDispose { setSecure(false) }
    }

    var words by remember { mutableStateOf<List<String>?>(null) }
    var loadError by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(walletIdHex) {
        try {
            val phrase = container.walletStorage.retrieveMnemonic(walletIdHex.hexToByteArray())
            if (phrase.isNullOrBlank()) {
                loadError = "No recovery phrase is stored for this wallet."
            } else {
                words = phrase.trim().split(Regex("\\s+"))
            }
        } catch (e: Exception) {
            loadError = e.message ?: "Failed to read the recovery phrase."
        }
    }

    // 0 = show phrase, 1 = confirmation quiz.
    var step by rememberSaveable { mutableStateOf(0) }
    var wroteItDown by rememberSaveable { mutableStateOf(false) }
    var quizDone by rememberSaveable { mutableStateOf(false) }
    var showLeaveDialog by rememberSaveable { mutableStateOf(false) }

    fun finish() {
        // Backup confirmed (or explicitly abandoned) → back to the wallet
        // list (popUpTo(WalletsHome) semantics of the create flow).
        navController.popBackStack(WalletsHome, inclusive = false)
    }

    fun requestLeave() {
        if (quizDone) finish() else showLeaveDialog = true
    }

    BackHandler { requestLeave() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Backup Seed") },
                navigationIcon = {
                    IconButton(onClick = { requestLeave() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        val currentWords = words
        when {
            loadError != null -> Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterVertically),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text("Recovery Phrase Unavailable", style = MaterialTheme.typography.titleMedium)
                Text(
                    loadError ?: "",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.error,
                )
                TextButton(onClick = { finish() }) { Text("Back to Wallets") }
            }

            currentWords == null -> Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentAlignment = Alignment.Center,
            ) { CircularProgressIndicator() }

            step == 0 -> PhraseStep(
                words = currentWords,
                wroteItDown = wroteItDown,
                onWroteItDownChange = { wroteItDown = it },
                onContinue = { step = 1 },
                modifier = Modifier
                    .padding(padding)
                    .padding(16.dp),
            )

            else -> QuizStep(
                words = currentWords,
                onDoneChange = { quizDone = it },
                onFinish = { finish() },
                modifier = Modifier
                    .padding(padding)
                    .padding(16.dp),
            )
        }
    }

    if (showLeaveDialog) {
        AlertDialog(
            onDismissRequest = { showLeaveDialog = false },
            title = { Text("You haven't backed up") },
            text = {
                Text(
                    "Your recovery phrase hasn't been verified. Without a " +
                        "backup you will lose access to this wallet's funds " +
                        "if this device is lost.",
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        showLeaveDialog = false
                        finish()
                    },
                    modifier = Modifier.testTag("seedBackup.leaveAnyway"),
                ) { Text("Leave Anyway") }
            },
            dismissButton = {
                TextButton(onClick = { showLeaveDialog = false }) { Text("Stay") }
            },
        )
    }
}

/** Step 1: the numbered word grid + "I wrote it down" gate (← SeedBackupView). */
@Composable
private fun PhraseStep(
    words: List<String>,
    wroteItDown: Boolean,
    onWroteItDownChange: (Boolean) -> Unit,
    onContinue: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Text("Recovery Phrase", style = MaterialTheme.typography.titleLarge)
        Text(
            "Write down these ${words.size} words in order and store them " +
                "somewhere safe. Do not take screenshots or share them with anyone.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        WordGrid(words)

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text("I wrote it down")
            Switch(
                checked = wroteItDown,
                onCheckedChange = onWroteItDownChange,
                modifier = Modifier.testTag("seedBackup.wroteItDownToggle"),
            )
        }

        Button(
            onClick = onContinue,
            enabled = wroteItDown,
            modifier = Modifier
                .fillMaxWidth()
                .testTag("seedBackup.continueButton"),
        ) { Text("Verify Backup") }
    }
}

/**
 * Step 2: tap-the-words-in-order confirmation quiz. A wrong tap resets the
 * sequence; duplicates in the phrase are handled by matching on the word
 * text at the expected position (any chip carrying the right word counts).
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun QuizStep(
    words: List<String>,
    onDoneChange: (Boolean) -> Unit,
    onFinish: () -> Unit,
    modifier: Modifier = Modifier,
) {
    // Stable shuffled chip order for the lifetime of the quiz.
    val shuffled = remember(words) { words.indices.shuffled() }
    var consumed by remember(words) { mutableStateOf(setOf<Int>()) }
    var mistake by remember { mutableStateOf(false) }

    val progress = consumed.size
    val done = progress == words.size
    LaunchedEffect(done) { onDoneChange(done) }

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Text("Confirm Your Phrase", style = MaterialTheme.typography.titleLarge)
        Text(
            "Tap the words in the order of your recovery phrase.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        LinearProgressIndicator(
            progress = { progress.toFloat() / words.size },
            modifier = Modifier.fillMaxWidth(),
        )
        Text(
            "$progress of ${words.size} words",
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        if (mistake) {
            Text(
                "That wasn't the next word — starting over.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.error,
            )
        }

        // The words tapped so far, in order.
        if (progress > 0) {
            Card(Modifier.fillMaxWidth()) {
                Text(
                    words.take(progress).joinToString(" "),
                    style = MaterialTheme.typography.bodyMedium,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.padding(12.dp),
                )
            }
        }

        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            shuffled.forEach { wordIndex ->
                val used = consumed.contains(wordIndex)
                FilterChip(
                    selected = used,
                    enabled = !used && !done,
                    onClick = {
                        if (words[wordIndex] == words[progress]) {
                            // Consume the chip whose word matches the next
                            // expected word (duplicate-safe: prefer this
                            // exact chip; identity by index keeps chips
                            // one-shot).
                            consumed = consumed + wordIndex
                            mistake = false
                        } else {
                            consumed = emptySet()
                            mistake = true
                        }
                    },
                    label = { Text(words[wordIndex]) },
                    modifier = Modifier.testTag("seedBackup.quizWord.${words[wordIndex]}"),
                )
            }
        }

        Button(
            onClick = onFinish,
            enabled = done,
            modifier = Modifier
                .fillMaxWidth()
                .testTag("seedBackup.createWalletButton"),
        ) { Text(if (done) "Done" else "Tap the words in order") }
    }
}

/** Two-column numbered word grid shared with the reveal sheet. */
@Composable
internal fun WordGrid(words: List<String>) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        words.chunked(2).forEachIndexed { rowIndex, pair ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                pair.forEachIndexed { colIndex, word ->
                    val number = rowIndex * 2 + colIndex + 1
                    Card(Modifier.weight(1f)) {
                        Row(
                            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            Text(
                                "%2d.".format(number),
                                style = MaterialTheme.typography.bodyMedium,
                                fontFamily = FontFamily.Monospace,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            Text(
                                word,
                                style = MaterialTheme.typography.bodyMedium,
                                fontFamily = FontFamily.Monospace,
                            )
                        }
                    }
                }
                if (pair.size == 1) Box(Modifier.weight(1f))
            }
        }
    }
}
