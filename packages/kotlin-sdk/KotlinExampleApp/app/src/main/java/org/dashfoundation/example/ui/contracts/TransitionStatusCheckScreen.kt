package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.queries.TransitionStatusByHash
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton

/**
 * Checks a state transition's status from its hash (protocol version 15): port of
 * `TransitionStatusCheckView.swift`, over `PlatformBlocks.getStateTransitionStatusByHash`
 * (`dash_sdk_system_get_state_transition_status_by_hash`). Covers TEST_PLAN SYS-13 (Swift
 * SYS-12).
 *
 * Rust asks the node and, when the node says committed, reads that block with its proof and
 * looks for the transition. Only "in a block" is proved; every other outcome is the node's
 * word and is shown as unproved, with the node's height.
 *
 * The outcome shown answers the hash it was checked for, through the SDK it was read with:
 * editing the hash or replacing the SDK clears it, and a check that completes after either is
 * dropped.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TransitionStatusCheckScreen(navController: NavHostController) {
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val sdk by appState.sdk.collectAsStateWithLifecycle()

    var hashText by rememberSaveable { mutableStateOf("") }
    var isRunning by remember { mutableStateOf(false) }
    var status by remember { mutableStateOf<TransitionStatusByHash?>(null) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    // The hash the outcome shown was checked for
    var checkedHash by remember { mutableStateOf<String?>(null) }
    val submissions = remember { Submissions() }

    // The hash was edited or the SDK replaced: the outcome shown, and a check still running,
    // answer a hash or a connection no longer on screen
    fun clearOutcome() {
        submissions.invalidate()
        status = null
        errorMessage = null
        checkedHash = null
        isRunning = false
    }
    val connection = rememberConnection(sdk) { clearOutcome() }
    val trimmedHash = hashText.trim()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Transition Status") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .imePadding()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "State transition hash") {
                OutlinedTextField(
                    value = hashText,
                    onValueChange = {
                        hashText = it
                        clearOutcome()
                    },
                    placeholder = { Text("64 hex characters") },
                    textStyle = MaterialTheme.typography.bodySmall
                        .copy(fontFamily = FontFamily.Monospace),
                    modifier = Modifier
                        .fillMaxWidth()
                        .semantics { contentDescription = "State transition hash" }
                        .testTag("txStatus.hashField"),
                )
                SubmitButton(
                    text = if (isRunning) "Checking…" else "Check Status",
                    isLoading = isRunning,
                    enabled = sdk != null && trimmedHash.isNotEmpty(),
                    modifier = Modifier.fillMaxWidth().padding(top = 8.dp)
                        .testTag("txStatus.checkButton"),
                ) {
                    val currentSdk = sdk
                    if (currentSdk == null) {
                        errorMessage = "SDK not initialized"
                        return@SubmitButton
                    }
                    // The hash as it is now: the outcome is shown beside it, and only while
                    // the field and the SDK are unchanged
                    val hash = trimmedHash
                    val readConnection = connection.value
                    val submission = submissions.submit()
                    isRunning = true
                    errorMessage = null
                    status = null
                    checkedHash = null
                    scope.launch {
                        val result = runCatching {
                            currentSdk.blocks.getStateTransitionStatusByHash(hash)
                        }
                        if (!submissions.isCurrent(submission) || connection.value != readConnection) {
                            return@launch
                        }
                        result
                            .onSuccess {
                                status = it
                                checkedHash = hash
                            }
                            .onFailure { errorMessage = it.message ?: "Check failed" }
                        isRunning = false
                    }
                }
                Text(
                    TransitionStatusPrivacy.NOTE,
                    style = MaterialTheme.typography.bodySmall,
                    color = UnprovedOrange,
                    modifier = Modifier.padding(top = 8.dp).testTag("txStatus.privacyNote"),
                )
                SectionFooter("Proved only when the block the node names holds the transition.")
            }

            val message = errorMessage
            val outcome = status
            if (message != null) {
                FormSection(title = "Error") {
                    SelectionContainer {
                        Text(
                            message,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.testTag("txStatus.errorText"),
                        )
                    }
                }
            } else if (outcome != null) {
                OutcomeSection(outcome, checkedHash)
            }
        }
    }
}

@Composable
private fun OutcomeSection(status: TransitionStatusByHash, checkedHash: String?) {
    FormSection(title = "Outcome") {
        ProvedBadge(
            proved = status.proved,
            provedText = "Proved: a block the validator quorum signed holds it",
            unprovedText = "Unproved: one node's word",
            testTag = "txStatus.provedBadge",
        )
        Text(
            headline(status),
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.testTag("txStatus.outcomeText"),
        )
        checkedHash?.let {
            BlockField(
                "Hash checked",
                it,
                monospaced = true,
                modifier = Modifier.testTag("txStatus.checkedHashText"),
            )
        }

        if (status.status == TransitionStatusByHash.Status.IN_BLOCK) {
            status.blockHeight?.let { BlockField("Block height", "$it") }
            status.blockHash?.let { BlockField("Block hash", it, monospaced = true) }
            status.blockTimeMs?.let { BlockField("Block time", BlockFormatting.time(it)) }
            status.transaction?.let { transaction ->
                BlockField(
                    "Result",
                    if (transaction.executed) {
                        "Executed (code 0)"
                    } else {
                        "Failed, paid for error code ${transaction.code}"
                    },
                    modifier = Modifier.testTag("txStatus.resultText"),
                )
                BlockTransactionRow(transaction)
            }
        } else {
            status.height?.let { BlockField("Height the node named", "$it") }
            status.error?.let { error ->
                BlockField("Error the node reports (unproved)", "Code ${error.code}: ${error.message}")
                error.consensusError?.let { BlockField("Consensus error", it) }
            }
            BlockField(
                "Node height when it answered",
                status.nodeHeight?.toString() ?: "Not reported",
                modifier = Modifier.testTag("txStatus.nodeHeightText"),
            )
        }
    }
}

private fun headline(status: TransitionStatusByHash): String = when (status.status) {
    TransitionStatusByHash.Status.IN_BLOCK -> "In block ${status.blockHeight ?: ""}"
    TransitionStatusByHash.Status.COMMITTED_NOT_PROVED ->
        "The node says it is in block ${status.height ?: ""}, but no proved block came back"
    TransitionStatusByHash.Status.COMMITTED_REFUTED ->
        "Refuted: the proved block ${status.height ?: ""} does not hold it"
    TransitionStatusByHash.Status.PENDING -> "Pending in the node's mempool"
    TransitionStatusByHash.Status.UNKNOWN -> "Unknown to the node"
}

/**
 * What the transition status reads tell before a hash is sent: the hash goes to the app's
 * nodes, not to the node the transition was broadcast to (← `TransitionStatusPrivacy`).
 */
internal object TransitionStatusPrivacy {
    const val NOTE = "For public transitions only. The hash goes to the app's nodes, not to " +
        "the node the transition was broadcast to, so checking it ties you to that transition " +
        "in front of them. Do not use this for shielded or private document transitions."
}
