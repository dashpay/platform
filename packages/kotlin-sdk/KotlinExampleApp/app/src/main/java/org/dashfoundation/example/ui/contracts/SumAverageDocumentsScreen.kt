package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
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
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.SumAverageDocuments
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.truncateMiddle
import java.util.Locale

/**
 * Sum/average aggregate form — port of `SumAverageDocumentsView.swift`.
 * Contract + document type arrive preselected from the document-type
 * screen. Averages display as `sum / count` to four decimals, with the
 * raw `n=count, Σ=sum` pair alongside, matching the iOS renderer.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SumAverageDocumentsScreen(
    route: SumAverageDocuments,
    navController: NavHostController,
) {
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val sdk by appState.sdk.collectAsStateWithLifecycle()

    val contractIdBase58 = remember(route.contractIdHex) {
        Base58.encode(route.contractIdHex.hexToBytes())
    }

    var operation by rememberSaveable { mutableStateOf(Operation.SUM) }
    var property by rememberSaveable { mutableStateOf("") }
    var whereJson by rememberSaveable { mutableStateOf("") }
    var groupByJson by rememberSaveable { mutableStateOf("") }
    var isRunning by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var sumResult by remember { mutableStateOf<AggregateResult?>(null) }
    var averageResult by remember { mutableStateOf<AverageResult?>(null) }

    fun run() {
        val currentSdk = sdk
        if (currentSdk == null) {
            errorMessage = "SDK not initialized"
            return
        }
        scope.launch {
            isRunning = true
            errorMessage = null
            sumResult = null
            averageResult = null
            try {
                val response = currentSdk.contracts.fetch(contractIdBase58).use { contract ->
                    when (operation) {
                        Operation.SUM -> currentSdk.documents.sum(
                            contract = contract,
                            documentType = route.typeName,
                            property = property.trim(),
                            whereJson = whereJson.trim().takeIf { it.isNotEmpty() },
                            groupByJson = groupByJson.trim().takeIf { it.isNotEmpty() },
                        )
                        Operation.AVERAGE -> currentSdk.documents.average(
                            contract = contract,
                            documentType = route.typeName,
                            property = property.trim(),
                            whereJson = whereJson.trim().takeIf { it.isNotEmpty() },
                            groupByJson = groupByJson.trim().takeIf { it.isNotEmpty() },
                        )
                    }
                }
                when (operation) {
                    Operation.SUM -> sumResult = parseAggregateResult(response)
                    Operation.AVERAGE -> averageResult = parseAverageResult(response)
                }
            } catch (e: Exception) {
                errorMessage = e.message ?: "${operation.label} failed"
            } finally {
                isRunning = false
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Sum / Average") },
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
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Document") {
                LabeledContent("Contract", truncateMiddle(contractIdBase58, 12, 8))
                LabeledContent("Document Type", route.typeName)
            }

            FormSection(title = "Aggregation") {
                AccessiblePicker(
                    label = "Operation",
                    options = Operation.entries,
                    selected = operation,
                    optionLabel = { it.label },
                    testTag = "sumAverageDocuments.opPicker",
                ) { selected ->
                    operation = selected
                    sumResult = null
                    averageResult = null
                    errorMessage = null
                }
                OutlinedTextField(
                    value = property,
                    onValueChange = { property = it },
                    label = { Text("Numeric property") },
                    placeholder = { Text("amount") },
                    singleLine = true,
                    enabled = !isRunning,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 8.dp)
                        .testTag("sumAverageDocuments.sumPropertyField"),
                )
            }

            FormSection(title = "Filters") {
                OutlinedTextField(
                    value = whereJson,
                    onValueChange = { whereJson = it },
                    label = { Text("Where (JSON, optional)") },
                    enabled = !isRunning,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("sumAverageDocuments.whereField"),
                )
                OutlinedTextField(
                    value = groupByJson,
                    onValueChange = { groupByJson = it },
                    label = { Text("Group By (JSON, optional)") },
                    enabled = !isRunning,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 8.dp)
                        .testTag("sumAverageDocuments.groupByField"),
                )
                Text(
                    "Summing/averaging requires a summable index on the property.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 4.dp),
                )
            }

            SubmitButton(
                text = "Run ${operation.label}",
                isLoading = isRunning,
                enabled = property.isNotBlank(),
                modifier = Modifier.testTag("sumAverageDocuments.runButton"),
            ) { run() }

            errorMessage?.let { message ->
                FormSection(title = "Error") {
                    Text(
                        message,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("sumAverageDocuments.errorText"),
                    )
                }
            }

            sumResult?.let { sums ->
                FormSection(title = "Total") {
                    LabeledContent(
                        "Sum",
                        sums.total ?: "—",
                        modifier = Modifier.testTag("sumAverageDocuments.total"),
                    )
                }
                if (sums.groups.isNotEmpty()) {
                    FormSection(title = "Per-group sums") {
                        sums.groups.entries.sortedBy { it.key }.forEach { (key, value) ->
                            LabeledContent(
                                key,
                                value,
                                modifier = Modifier.testTag("sumAverageDocuments.groupRow.$key"),
                            )
                        }
                    }
                }
            }

            averageResult?.let { averages ->
                FormSection(title = "Total") {
                    LabeledContent(
                        "Average",
                        averages.total?.displayAverage() ?: "—",
                        modifier = Modifier.testTag("sumAverageDocuments.average"),
                    )
                    averages.total?.let { entry ->
                        LabeledContent("Count", "${entry.count}")
                        LabeledContent("Sum", "${entry.sum}")
                    }
                }
                if (averages.groups.isNotEmpty()) {
                    FormSection(title = "Per-group averages") {
                        averages.groups.entries.sortedBy { it.key }.forEach { (key, entry) ->
                            LabeledContent(
                                key,
                                "${entry.displayAverage()} (n=${entry.count}, Σ=${entry.sum})",
                                modifier = Modifier.testTag("sumAverageDocuments.groupRow.$key"),
                            )
                        }
                    }
                }
            }
        }
    }
}

private enum class Operation(val label: String) {
    SUM("Sum"),
    AVERAGE("Average"),
}

/** One average entry — the raw `(count, sum)` pair the FFI returns. */
internal data class AverageEntry(val count: Long, val sum: Long) {
    fun displayAverage(): String =
        if (count == 0L) "—"
        else String.format(Locale.US, "%.4f", sum.toDouble() / count.toDouble())
}

internal data class AverageResult(
    val total: AverageEntry?,
    val groups: Map<String, AverageEntry>,
)

/**
 * Parse the average result: group key → `{count, sum}` objects, with the
 * empty-string key as the overall entry; a bare `{count, sum}` object is
 * an ungrouped total. The FFI wraps the map in an `{"averages": {...}}`
 * envelope (see `dash_sdk_document_average`'s contract) — unwrap it first.
 */
internal fun parseAverageResult(response: String?): AverageResult? {
    if (response.isNullOrBlank()) return null
    var root = LenientJson.parseToJsonElement(response) as? JsonObject ?: return null
    if (root.size == 1) (root["averages"] as? JsonObject)?.let { root = it }

    fun entryOf(obj: JsonObject): AverageEntry? {
        val count = (obj["count"] as? JsonPrimitive)?.content?.toLongOrNull() ?: return null
        val sum = (obj["sum"] as? JsonPrimitive)?.content?.toLongOrNull() ?: return null
        return AverageEntry(count, sum)
    }

    entryOf(root)?.let { return AverageResult(total = it, groups = emptyMap()) }

    val entries = root.entries.mapNotNull { (key, value) ->
        (value as? JsonObject)?.let { obj -> entryOf(obj)?.let { key to it } }
    }.toMap()
    if (entries.isEmpty()) return null
    return AverageResult(
        total = entries[""] ?: entries["total"],
        groups = entries.filterKeys { it.isNotEmpty() && it != "total" },
    )
}
