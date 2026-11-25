package org.dashfoundation.example.ui.contracts

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import java.text.DateFormat
import java.util.Date
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.queries.OwnerTransactionsInBlock
import org.dashfoundation.dashsdk.queries.PlatformBlock
import org.dashfoundation.dashsdk.queries.PlatformBlockList
import org.dashfoundation.dashsdk.queries.PlatformBlockTransaction
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton

/**
 * Read-only block explorer (protocol version 15): port of `PlatformBlocksView.swift`
 * (`PlatformBlocksView` and `PlatformBlockDetailView`). Reads up to ten consecutive blocks
 * with `PlatformBlocks.getBlocks` (`dash_sdk_system_get_blocks`), opens one to list its state
 * transitions, and from a block finds one identity's transitions in it with
 * `PlatformBlocks.getOwnerTransactionsInBlock`. Covers TEST_PLAN SYS-10 and SYS-11 (Swift
 * SYS-09 and SYS-10).
 *
 * Rust verifies the block proofs and says whether the answer is proved; this screen shows that
 * flag and nothing it decides itself. Below protocol version 15 the SDK's refusal is shown as
 * it comes.
 *
 * The opened block is shown inside this screen rather than as its own route, so no block
 * crosses a navigation argument; system back returns to the list. Blocks are read through the
 * SDK current when Read Blocks is tapped: once the app's SDK is replaced (another network),
 * the list and the block say so and the owner lookup does not run on the new SDK. The screen
 * keeps a connection count it bumps on every SDK change instead of holding the SDK itself.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PlatformBlocksScreen(navController: NavHostController) {
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val sdk by appState.sdk.collectAsStateWithLifecycle()

    var startHeightText by rememberSaveable { mutableStateOf("1") }
    var countText by rememberSaveable { mutableStateOf("10") }
    var isRunning by remember { mutableStateOf(false) }
    var answer by remember { mutableStateOf<LoadedBlocks?>(null) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var openedHeight by remember { mutableStateOf<Long?>(null) }
    val connection = rememberConnection(sdk) {
        // A read still running, or an error shown, answers for the replaced SDK; blocks
        // already shown stay, marked as read from it
        isRunning = false
        errorMessage = null
    }
    val submissions = remember { Submissions() }

    fun clearForEditedParameters() {
        submissions.invalidate()
        answer = null
        errorMessage = null
        isRunning = false
        openedHeight = null
    }

    val opened = openedHeight?.let { height ->
        answer?.blocks?.blocks?.firstOrNull { it.height == height }
    }
    BackHandler(enabled = opened != null) { openedHeight = null }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(opened?.let { "Block ${it.height}" } ?: "Block Explorer") },
                navigationIcon = {
                    IconButton(onClick = {
                        if (opened != null) openedHeight = null else navController.popBackStack()
                    }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        val loaded = answer
        if (opened != null && loaded != null) {
            PlatformBlockDetail(
                block = opened,
                proved = loaded.blocks.proved,
                networkChanged = loaded.connection != connection.value,
                sdk = sdk,
                connection = connection,
                readConnection = loaded.connection,
                modifier = Modifier.padding(padding),
            )
            return@Scaffold
        }
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .imePadding()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Blocks") {
                OutlinedTextField(
                    value = startHeightText,
                    onValueChange = {
                        startHeightText = it
                        clearForEditedParameters()
                    },
                    label = { Text("Start height") },
                    singleLine = true,
                    enabled = !isRunning,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier.fillMaxWidth().testTag("blocks.startHeightField"),
                )
                OutlinedTextField(
                    value = countText,
                    onValueChange = {
                        countText = it
                        clearForEditedParameters()
                    },
                    label = { Text("Count (1 to 10)") },
                    singleLine = true,
                    enabled = !isRunning,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier.fillMaxWidth().testTag("blocks.countField"),
                )
                SubmitButton(
                    text = if (isRunning) "Reading…" else "Read Blocks",
                    isLoading = isRunning,
                    enabled = sdk != null,
                    modifier = Modifier.fillMaxWidth().padding(top = 8.dp)
                        .testTag("blocks.runButton"),
                ) {
                    val currentSdk = sdk
                    if (currentSdk == null) {
                        errorMessage = "SDK not initialized"
                        return@SubmitButton
                    }
                    // Parsing the typed text is the form's job; the range the node serves is
                    // Rust's and the node's to refuse
                    val startHeight = startHeightText.trim().toLongOrNull()
                    val count = countText.trim().toIntOrNull()
                    if (startHeight == null || count == null || startHeight < 0 || count < 0) {
                        errorMessage = "Start height and count must be whole numbers"
                        answer = null
                        return@SubmitButton
                    }
                    val readConnection = connection.value
                    val submission = submissions.submit()
                    isRunning = true
                    errorMessage = null
                    answer = null
                    scope.launch {
                        val result = runCatching { currentSdk.blocks.getBlocks(startHeight, count) }
                        // Read through an SDK the app has since replaced, or for edited
                        // parameters: not what the screen shows
                        if (!submissions.isCurrent(submission) || connection.value != readConnection) {
                            return@launch
                        }
                        result
                            .onSuccess { answer = LoadedBlocks(it, readConnection) }
                            .onFailure { errorMessage = it.message ?: "Read failed" }
                        isRunning = false
                    }
                }
                SectionFooter(
                    "Reads consecutive blocks from the start height. Use a count of 1 for a " +
                        "single block. The node serves 1 to 10 blocks and may return fewer than " +
                        "asked; nothing proves a block left off the end does not exist.",
                )
            }

            val message = errorMessage
            if (message != null) {
                FormSection(title = "Error") {
                    SelectionContainer {
                        Text(
                            message,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.testTag("blocks.errorText"),
                        )
                    }
                }
            } else if (loaded != null) {
                FormSection(title = "Result (${loaded.blocks.blocks.size})") {
                    if (loaded.connection != connection.value) {
                        NetworkChangedNote("blocks.networkChangedText")
                    }
                    if (loaded.blocks.blocks.isEmpty()) {
                        // Nothing came back, so there is nothing a proof covers
                        Text(
                            "No blocks returned",
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.testTag("blocks.emptyText"),
                        )
                    } else {
                        ProvedBadge(proved = loaded.blocks.proved, testTag = "blocks.provedBadge")
                    }
                    loaded.blocks.blocks.forEach { block ->
                        val count = block.transactions.size
                        ListItem(
                            headlineContent = { Text("Block ${block.height}") },
                            supportingContent = {
                                Column {
                                    Text(
                                        BlockFormatting.time(block.timeMs),
                                        style = MaterialTheme.typography.bodySmall,
                                    )
                                    Text(
                                        "$count state transition" + if (count == 1) "" else "s",
                                        style = MaterialTheme.typography.bodySmall,
                                    )
                                }
                            },
                            modifier = Modifier
                                .clickable { openedHeight = block.height }
                                .testTag("blocks.row.${block.height}"),
                        )
                    }
                }
            }
        }
    }
}

/** Blocks with the connection count of the SDK that read them. */
private data class LoadedBlocks(val blocks: PlatformBlockList, val connection: Int)

/**
 * One block's fields and state transitions, and a lookup of one identity's transitions in it
 * (← `PlatformBlockDetailView`).
 *
 * The lookup runs only while [sdk] is still the SDK that read the block ([readConnection] is
 * the current [connection]): another chain's transitions are never shown under this block.
 * Its result answers the identity id it was asked for: editing the field clears it, and a
 * lookup that completes after an edit or an SDK change is dropped.
 */
@Composable
private fun PlatformBlockDetail(
    block: PlatformBlock,
    proved: Boolean,
    networkChanged: Boolean,
    sdk: Sdk?,
    connection: ConnectionCount,
    readConnection: Int,
    modifier: Modifier = Modifier,
) {
    val scope = rememberCoroutineScope()
    var ownerIdText by rememberSaveable(block.height) { mutableStateOf("") }
    var isRunning by remember(block.height) { mutableStateOf(false) }
    var ownerAnswer by remember(block.height) { mutableStateOf<OwnerTransactionsInBlock?>(null) }
    var ownerError by remember(block.height) { mutableStateOf<String?>(null) }
    val submissions = remember(block.height) { Submissions() }

    // A lookup still running would land beside the notice; a result already shown was read
    // through this block's SDK and stays
    LaunchedEffect(networkChanged) {
        if (networkChanged) {
            submissions.invalidate()
            isRunning = false
        }
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .imePadding()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        FormSection(title = "Block") {
            if (networkChanged) NetworkChangedNote("blockDetail.networkChangedText")
            ProvedBadge(proved = proved, testTag = "blockDetail.provedBadge")
            BlockField("Height", "${block.height}")
            BlockField("Time", BlockFormatting.time(block.timeMs))
            BlockField("Hash", block.hash, monospaced = true)
            BlockField("Core chain-locked height", "${block.coreChainLockedHeight}")
            BlockField("App hash", block.appHash, monospaced = true)
            BlockField("Proposer", block.proposerProTxHash, monospaced = true)
        }

        FormSection(title = "State transitions (${block.transactions.size})") {
            if (block.transactions.isEmpty()) {
                Text(
                    "This block holds no state transitions",
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.testTag("blockDetail.emptyText"),
                )
            }
            block.transactions.forEachIndexed { index, transaction ->
                if (index > 0) HorizontalDivider()
                BlockTransactionRow(
                    transaction,
                    Modifier.testTag("blockDetail.txRow.${transaction.hash}"),
                )
            }
        }

        FormSection(title = "An identity's transitions in this block") {
            OutlinedTextField(
                value = ownerIdText,
                onValueChange = {
                    ownerIdText = it
                    // The result shown, and a lookup running, answer the old identity id
                    submissions.invalidate()
                    ownerAnswer = null
                    ownerError = null
                    isRunning = false
                },
                label = { Text("Identity ID (base58)") },
                singleLine = true,
                textStyle = MaterialTheme.typography.bodySmall
                    .copy(fontFamily = FontFamily.Monospace),
                modifier = Modifier.fillMaxWidth().testTag("blockDetail.ownerIdField"),
            )
            SubmitButton(
                text = if (isRunning) "Reading…" else "Find Identity's Transitions",
                isLoading = isRunning,
                enabled = !networkChanged && sdk != null && ownerIdText.isNotBlank(),
                modifier = Modifier.fillMaxWidth().padding(top = 8.dp)
                    .testTag("blockDetail.findOwnerButton"),
            ) {
                // Only through the SDK that read this block, and only while it is the app's SDK
                val currentSdk = sdk
                if (currentSdk == null || connection.value != readConnection) {
                    return@SubmitButton
                }
                val ownerId = ownerIdText.trim()
                val height = block.height
                val submission = submissions.submit()
                isRunning = true
                ownerError = null
                ownerAnswer = null
                scope.launch {
                    val result = runCatching {
                        currentSdk.blocks.getOwnerTransactionsInBlock(height, ownerId)
                    }
                    if (!submissions.isCurrent(submission) || connection.value != readConnection) {
                        return@launch
                    }
                    result
                        .onSuccess { ownerAnswer = it }
                        .onFailure { ownerError = it.message ?: "Read failed" }
                    isRunning = false
                }
            }
            if (networkChanged) NetworkChangedNote("blockDetail.ownerNetworkChangedText")

            val error = ownerError
            val owner = ownerAnswer
            if (error != null) {
                SelectionContainer {
                    Text(
                        error,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("blockDetail.ownerErrorText"),
                    )
                }
            } else if (owner != null) {
                if (owner.transactions.isEmpty()) {
                    // Nothing came back, so there is nothing a proof covers
                    Text(
                        "The node returned no transitions of this identity in block ${owner.height}",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.testTag("blockDetail.ownerEmptyText"),
                    )
                } else {
                    ProvedBadge(
                        proved = owner.proved,
                        provedText = "Each transition proved; the list is not proved complete",
                        testTag = "blockDetail.ownerProvedBadge",
                    )
                }
                owner.transactions.forEach { transaction ->
                    BlockTransactionRow(
                        transaction,
                        Modifier.testTag("blockDetail.ownerTxRow.${transaction.hash}"),
                    )
                }
            }
            SectionFooter(
                "Asks the node for the transitions this identity owns in this block. Each one " +
                    "returned is proved to be in the block with its result, but a node can " +
                    "leave some out: the block's full list above is the complete one. A " +
                    "recipient is not an owner.",
            )
        }
    }
}

/** A state transition of a block with its result (← `BlockTransactionRow`). */
@Composable
internal fun BlockTransactionRow(transaction: PlatformBlockTransaction, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier.fillMaxWidth().padding(vertical = 6.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Row {
            Text(
                transaction.type,
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.weight(1f),
            )
            if (transaction.executed) {
                Text("Executed", style = MaterialTheme.typography.labelSmall, color = ProvedGreen)
            } else {
                Text(
                    "Failed, code ${transaction.code}",
                    style = MaterialTheme.typography.labelSmall,
                    color = UnprovedOrange,
                )
            }
        }
        SelectionContainer {
            Text(
                transaction.hash,
                style = MaterialTheme.typography.labelSmall,
                fontFamily = FontFamily.Monospace,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
        transaction.ownerId?.let { ownerId ->
            SelectionContainer {
                Text(
                    "Owner $ownerId",
                    style = MaterialTheme.typography.labelSmall,
                    fontFamily = FontFamily.Monospace,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
        Text(
            "Gas used ${transaction.gasUsed}",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/** Says the app's SDK was replaced since the blocks shown were read (← `NetworkChangedNote`). */
@Composable
internal fun NetworkChangedNote(testTag: String) {
    Text(
        "Network changed, reload the blocks",
        style = MaterialTheme.typography.bodySmall,
        color = UnprovedOrange,
        modifier = Modifier.padding(vertical = 4.dp).testTag(testTag),
    )
}

/** Whether Rust proved what the section shows (← `ProvedBadge`). */
@Composable
internal fun ProvedBadge(
    proved: Boolean,
    testTag: String,
    provedText: String = "Proved by the block's quorum signature",
    unprovedText: String = "Unproved: the node's word",
) {
    Text(
        if (proved) provedText else unprovedText,
        style = MaterialTheme.typography.bodySmall,
        color = if (proved) ProvedGreen else UnprovedOrange,
        modifier = Modifier.padding(vertical = 4.dp).testTag(testTag),
    )
}

/** A labelled value of a block (← `BlockField`). */
@Composable
internal fun BlockField(
    label: String,
    value: String,
    monospaced: Boolean = false,
    modifier: Modifier = Modifier,
) {
    Column(modifier = modifier.fillMaxWidth().padding(vertical = 4.dp)) {
        Text(
            label,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        SelectionContainer {
            Text(
                value,
                style = if (monospaced) {
                    MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace)
                } else {
                    MaterialTheme.typography.bodyMedium
                },
            )
        }
    }
}

/** A form section's explanatory footer. */
@Composable
internal fun SectionFooter(text: String) {
    Spacer(Modifier.padding(top = 4.dp))
    Text(
        text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

internal object BlockFormatting {
    /** A block time in milliseconds since the Unix epoch, as a local date and time. */
    fun time(ms: Long): String =
        DateFormat.getDateTimeInstance(DateFormat.MEDIUM, DateFormat.MEDIUM).format(Date(ms))
}

internal val ProvedGreen = Color(0xFF2E7D32)
internal val UnprovedOrange = Color(0xFFEF6C00)

/**
 * How many times the app's SDK was replaced while this screen was on screen (Swift
 * `AppState.sdkGeneration`): an answer read at one count is not this network's at another.
 */
internal class ConnectionCount {
    var value by mutableIntStateOf(0)
}

/**
 * A [ConnectionCount] that rises each time [sdk] changes after the first composition, calling
 * [onChange] then.
 */
@Composable
internal fun rememberConnection(sdk: Sdk?, onChange: () -> Unit): ConnectionCount {
    val count = remember { ConnectionCount() }
    var seen by remember { mutableStateOf(false) }
    LaunchedEffect(sdk) {
        if (seen) {
            count.value++
            onChange()
        }
        seen = true
    }
    return count
}

/**
 * The reads a screen started, so an answer is shown only while it still answers the inputs on
 * screen (Swift `InputBoundSubmissions`): each read takes a number, and an edit invalidates
 * every number taken so far.
 */
internal class Submissions {
    private var current by mutableIntStateOf(0)

    /** A number for a read starting now. */
    fun submit(): Int = ++current

    /** Whether [submission] is the last read started and nothing was edited since. */
    fun isCurrent(submission: Int): Boolean = submission == current

    /** The inputs changed: every read started so far no longer answers them. */
    fun invalidate() {
        current++
    }
}
