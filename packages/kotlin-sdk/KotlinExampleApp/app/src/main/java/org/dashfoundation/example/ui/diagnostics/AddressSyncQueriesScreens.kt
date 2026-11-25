package org.dashfoundation.example.ui.diagnostics

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
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
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.queries.AddressLeafBoundary
import org.dashfoundation.dashsdk.queries.AddressTreeElement
import org.dashfoundation.dashsdk.queries.CompactedAddressChange
import org.dashfoundation.dashsdk.queries.CreditOperation
import org.dashfoundation.dashsdk.queries.addressSync
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.AddressBranchStateQuery
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.formatCredits
import org.dashfoundation.example.util.hexToBytes

// The address-sync queries of the catalog (ADDR-11) — ports of `GetRecentBalanceChangesView`,
// `GetCompactedBalanceChangesView`, `GetTrunkStateView` and `GetBranchStateView` in
// `AddressQueriesView.swift`. Results are raw and network-wide (no wallet filter). Each screen
// parses only the numbers and hex it is given; what a query accepts (a branch depth, a
// checkpoint) is decided by Rust and the server, whose error is shown.

/** Recent address balance changes from a start height (← `GetRecentBalanceChangesView`). */
@Composable
fun RecentAddressBalanceChangesScreen(navController: NavHostController) {
    var startHeight by rememberSaveable { mutableStateOf("0") }
    QueryScaffold("Get Recent Balance Changes", navController) { sdk, execute ->
        FormSection(title = "Start Block Height") {
            NumberField(startHeight, { startHeight = it }, "Start block height", "addressQueries.recent.startHeight")
            Hint("Enter 0 to get all recent balance changes (one server page).")
        }
        SubmitButton(
            text = "Fetch",
            isLoading = false,
            enabled = sdk != null && startHeight.toLongOrNull() != null,
            modifier = Modifier.fillMaxWidth().testTag("addressQueries.recent.submit"),
        ) {
            val height = startHeight.toLongOrNull() ?: return@SubmitButton
            execute {
                val blocks = it.addressSync.recentBalanceChanges(height)
                val shown: @Composable () -> Unit = {
                    if (blocks.isEmpty()) {
                        Hint("No balance changes found from the specified block height.")
                    }
                    blocks.forEach { block ->
                        LabeledContent("Block ${block.blockHeight}", "${block.changes.size} change(s)")
                        block.changes.forEach { change ->
                            Mono(change.addressHex)
                            Text(
                                when (change.operation) {
                                    CreditOperation.SET -> "Set to ${change.credits} credits"
                                    CreditOperation.ADD -> "Add ${change.credits} credits"
                                } + " (${formatCredits(change.credits)})",
                                style = MaterialTheme.typography.bodySmall,
                            )
                        }
                        HorizontalDivider()
                    }
                }
                shown
            }
        }
    }
}

/** Compacted address balance changes from a start height (← `GetCompactedBalanceChangesView`). */
@Composable
fun CompactedAddressBalanceChangesScreen(navController: NavHostController) {
    var startHeight by rememberSaveable { mutableStateOf("0") }
    QueryScaffold("Get Compacted Balance Changes", navController) { sdk, execute ->
        FormSection(title = "Start Block Height") {
            NumberField(startHeight, { startHeight = it }, "Start block height", "addressQueries.compacted.startHeight")
            Hint("Enter 0 to get all compacted balance changes (at most one server page).")
        }
        SubmitButton(
            text = "Fetch",
            isLoading = false,
            enabled = sdk != null && startHeight.toLongOrNull() != null,
            modifier = Modifier.fillMaxWidth().testTag("addressQueries.compacted.submit"),
        ) {
            val height = startHeight.toLongOrNull() ?: return@SubmitButton
            execute {
                val ranges = it.addressSync.compactedBalanceChanges(height)
                val shown: @Composable () -> Unit = {
                    if (ranges.isEmpty()) {
                        Hint("No compacted balance changes found from the specified block height.")
                    }
                    ranges.forEach { range ->
                        LabeledContent(
                            "Block ${range.startBlockHeight} - ${range.endBlockHeight}",
                            "${range.changes.size} address(es)",
                        )
                        range.changes.forEach { change ->
                            Mono(change.addressHex)
                            when (val op = change.operation) {
                                is CompactedAddressChange.Operation.SetCredits ->
                                    Hint("Set to ${op.credits} credits (${formatCredits(op.credits)})")
                                is CompactedAddressChange.Operation.AddToCredits -> {
                                    Hint("${op.entries.size} Add Operation(s)")
                                    op.entries.forEach { entry ->
                                        Hint("  Block ${entry.blockHeight}: +${entry.credits} credits")
                                    }
                                }
                            }
                        }
                        HorizontalDivider()
                    }
                }
                shown
            }
        }
    }
}

/**
 * The address tree's trunk state (← `GetTrunkStateView`). Each leaf boundary offers **Query
 * this branch**, which opens [AddressBranchStateScreen] with the key, hash and checkpoint
 * filled in and the depth Rust recommends for the boundary's estimated count.
 */
@Composable
fun AddressTrunkStateScreen(navController: NavHostController) {
    val scope = rememberCoroutineScope()
    QueryScaffold("Get Trunk State", navController) { sdk, execute ->
        Hint("This is a low-level API used for privacy-preserving address synchronization.")
        SubmitButton(
            text = "Fetch Trunk State",
            isLoading = false,
            enabled = sdk != null,
            modifier = Modifier.fillMaxWidth().testTag("addressQueries.trunk.submit"),
        ) {
            execute { current ->
                val state = current.addressSync.trunkState()
                val shown: @Composable () -> Unit = {
                    LabeledContent("Checkpoint Height", "${state.checkpointHeight}")
                    TreeResult(state.elements, state.leafBoundaries) { index, boundary ->
                        TextButton(
                            onClick = {
                                scope.launch {
                                    // The depth is Rust's recommendation; null leaves the field empty.
                                    val depth = runCatching {
                                        current.addressSync.branchQueryDepth(boundary.estimatedCount)
                                    }.getOrNull()
                                    navController.navigate(
                                        AddressBranchStateQuery(
                                            keyHex = boundary.keyHex,
                                            expectedHashHex = boundary.hashHex,
                                            checkpointHeight = "${state.checkpointHeight}",
                                            depth = depth?.takeIf { it > 0 }?.toString() ?: "",
                                        ),
                                    )
                                }
                            },
                            modifier = Modifier.testTag("addressQueries.trunk.queryBranch.$index"),
                        ) { Text("Query this branch") }
                    }
                }
                shown
            }
        }
    }
}

/**
 * One branch of the address tree (← `GetBranchStateView`). Every field stays editable; the
 * depth field takes any whole number, and a depth the platform version does not allow shows
 * the query's error instead of being blocked here.
 */
@Composable
fun AddressBranchStateScreen(
    initial: AddressBranchStateQuery,
    navController: NavHostController,
) {
    var keyHex by rememberSaveable { mutableStateOf(initial.keyHex) }
    var depth by rememberSaveable { mutableStateOf(initial.depth.ifEmpty { "6" }) }
    var hashHex by rememberSaveable { mutableStateOf(initial.expectedHashHex) }
    var checkpoint by rememberSaveable { mutableStateOf(initial.checkpointHeight) }
    QueryScaffold("Get Branch State", navController) { sdk, execute ->
        Hint("This is a low-level API. Parameters come from trunk state leaf boundaries.")
        FormSection(title = "Parameters") {
            TextInput(keyHex, { keyHex = it.trim() }, "Key (hex)", "addressQueries.branch.key")
            NumberField(depth, { depth = it }, "Depth", "addressQueries.branch.depth")
            TextInput(hashHex, { hashHex = it.trim() }, "Expected Hash (hex, 64 chars)", "addressQueries.branch.hash")
            NumberField(checkpoint, { checkpoint = it }, "Checkpoint Height", "addressQueries.branch.checkpoint")
        }
        val key = keyHex.hexOrNull()
        val hash = hashHex.hexOrNull()?.takeIf { it.size == 32 }
        val depthValue = depth.toIntOrNull()
        val checkpointValue = checkpoint.toLongOrNull()
        SubmitButton(
            text = "Fetch Branch State",
            isLoading = false,
            enabled = sdk != null && key != null && key.isNotEmpty() && hash != null &&
                depthValue != null && checkpointValue != null,
            modifier = Modifier.fillMaxWidth().testTag("addressQueries.branch.submit"),
        ) {
            if (key == null || hash == null || depthValue == null || checkpointValue == null) return@SubmitButton
            execute {
                val state = it.addressSync.branchState(key, depthValue, hash, checkpointValue)
                val shown: @Composable () -> Unit = { TreeResult(state.elements, state.leafBoundaries, null) }
                shown
            }
        }
    }
}

// ── Shared pieces ─────────────────────────────────────────────────────────

/**
 * The scaffold of a query screen: a back arrow, a scrolling form, and the latest result or
 * error below it. [content] gets the SDK and an `execute` that runs a query and shows the
 * composable it returns (or the error), dropping a late answer from an older run.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun QueryScaffold(
    title: String,
    navController: NavHostController,
    content: @Composable ColumnScope.(Sdk?, (suspend (Sdk) -> @Composable () -> Unit) -> Unit) -> Unit,
) {
    val appState = LocalAppState.current
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    var result by remember { mutableStateOf<(@Composable () -> Unit)?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var loading by remember { mutableStateOf(false) }
    var generation by remember { mutableStateOf(0) }

    val execute: (suspend (Sdk) -> @Composable () -> Unit) -> Unit = { query ->
        val current = sdk
        if (current != null) {
            val mine = ++generation
            loading = true
            error = null
            scope.launch {
                try {
                    val shown = query(current)
                    if (mine == generation) result = shown
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    if (mine == generation) {
                        result = null
                        error = (e as? DashSdkError)?.userMessage ?: e.message ?: "Query failed."
                    }
                } finally {
                    if (mine == generation) loading = false
                }
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(title) },
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
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            content(sdk, execute)
            if (loading) Hint("Loading…")
            error?.let {
                Text(
                    it,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.testTag("addressQueries.error"),
                )
            }
            result?.let { shown ->
                FormSection(title = "Result") {
                    Column(
                        verticalArrangement = Arrangement.spacedBy(4.dp),
                        modifier = Modifier.testTag("addressQueries.result"),
                    ) { shown() }
                }
            }
        }
    }
}

@Composable
private fun TreeResult(
    elements: List<AddressTreeElement>,
    leaves: List<AddressLeafBoundary>,
    leafAction: (@Composable (Int, AddressLeafBoundary) -> Unit)?,
) {
    LabeledContent("Elements", "${elements.size}")
    LabeledContent("Leaf Boundaries", "${leaves.size}")
    LabeledContent("Total Balance", formatCredits(elements.sumOf { it.balance }))
    elements.forEach { element ->
        Mono("Key: ${element.keyHex}")
        Hint("${element.balance} credits, nonce ${element.nonce}")
    }
    if (leaves.isNotEmpty()) HorizontalDivider()
    leaves.forEachIndexed { index, boundary ->
        Mono("Key: ${boundary.keyHex}")
        Mono("Hash: ${boundary.hashHex}")
        Hint("Est. count: ${boundary.estimatedCount}")
        leafAction?.invoke(index, boundary)
    }
}

@Composable
private fun Hint(text: String) {
    Text(text, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
}

@Composable
private fun Mono(text: String) {
    SelectionContainer {
        Text(text, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace)
    }
}

@Composable
private fun NumberField(value: String, onChange: (String) -> Unit, label: String, tag: String) {
    OutlinedTextField(
        value = value,
        onValueChange = { onChange(it.filter(Char::isDigit)) },
        label = { Text(label) },
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
        modifier = Modifier.fillMaxWidth().testTag(tag),
    )
}

@Composable
private fun TextInput(value: String, onChange: (String) -> Unit, label: String, tag: String) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label) },
        singleLine = true,
        modifier = Modifier.fillMaxWidth().testTag(tag),
    )
}

private fun String.hexOrNull(): ByteArray? =
    if (length % 2 == 0 && all { it.isDigit() || it.lowercaseChar() in 'a'..'f' }) hexToBytes() else null
