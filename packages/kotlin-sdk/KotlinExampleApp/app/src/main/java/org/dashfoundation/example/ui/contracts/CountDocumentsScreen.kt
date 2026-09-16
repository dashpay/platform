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
import org.dashfoundation.example.navigation.CountDocuments
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.truncateMiddle

/**
 * Count-documents aggregate form — port of `CountDocumentsView.swift`.
 * Contract + document type arrive preselected from
 * [org.dashfoundation.example.ui.contracts.DocumentTypeDetailsScreen]
 * (iOS repeats the contract/type pickers because its entry point is a
 * global queries screen). Grouped results follow the Swift shape: the
 * empty-string key is the aggregate total, other keys are per-group rows.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CountDocumentsScreen(
    route: CountDocuments,
    navController: NavHostController,
) {
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val sdk by appState.sdk.collectAsStateWithLifecycle()

    val contractIdBase58 = remember(route.contractIdHex) {
        Base58.encode(route.contractIdHex.hexToBytes())
    }

    var whereJson by rememberSaveable { mutableStateOf("") }
    var groupByJson by rememberSaveable { mutableStateOf("") }
    var isRunning by remember { mutableStateOf(false) }
    var didRun by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var result by remember { mutableStateOf<AggregateResult?>(null) }

    fun run() {
        val currentSdk = sdk
        if (currentSdk == null) {
            errorMessage = "SDK not initialized"
            return
        }
        scope.launch {
            isRunning = true
            errorMessage = null
            result = null
            try {
                val response = currentSdk.contracts.fetch(contractIdBase58).use { contract ->
                    currentSdk.documents.count(
                        contract = contract,
                        documentType = route.typeName,
                        whereJson = whereJson.trim().takeIf { it.isNotEmpty() },
                        groupByJson = groupByJson.trim().takeIf { it.isNotEmpty() },
                    )
                }
                result = parseAggregateResult(response)
            } catch (e: Exception) {
                errorMessage = e.message ?: "Count failed"
            } finally {
                isRunning = false
                didRun = true
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Count Documents") },
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

            FormSection(title = "Filters") {
                OutlinedTextField(
                    value = whereJson,
                    onValueChange = { whereJson = it },
                    label = { Text("Where (JSON, optional)") },
                    placeholder = {
                        Text("[{\"field\":\"amount\",\"operator\":\">\",\"value\":0}]")
                    },
                    enabled = !isRunning,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("countDocuments.whereField"),
                )
                OutlinedTextField(
                    value = groupByJson,
                    onValueChange = { groupByJson = it },
                    label = { Text("Group By (JSON, optional)") },
                    placeholder = { Text("[\"category\"]") },
                    enabled = !isRunning,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 8.dp)
                        .testTag("countDocuments.groupByField"),
                )
                Text(
                    "Where is a JSON array of {field, operator, value}; group by is a " +
                        "JSON array of field names. Counting requires a countable index.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 4.dp),
                )
            }

            SubmitButton(
                text = "Run Count",
                isLoading = isRunning,
                modifier = Modifier.testTag("countDocuments.runButton"),
            ) { run() }

            errorMessage?.let { message ->
                FormSection(title = "Error") {
                    Text(
                        message,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("countDocuments.errorText"),
                    )
                }
            }

            result?.let { counts ->
                FormSection(title = "Total") {
                    LabeledContent(
                        "Count",
                        counts.total?.toString() ?: "—",
                        modifier = Modifier.testTag("countDocuments.totalCount"),
                    )
                }
                if (counts.groups.isNotEmpty()) {
                    FormSection(title = "Per-group counts") {
                        counts.groups.entries.sortedBy { it.key }.forEach { (key, value) ->
                            LabeledContent(
                                key,
                                value,
                                modifier = Modifier.testTag("countDocuments.groupRow.$key"),
                            )
                        }
                    }
                }
            }
            if (didRun && errorMessage == null && result == null) {
                Text(
                    "The query returned no result.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

/**
 * Aggregate result shared by count and sum: an optional overall total plus
 * per-group values (Swift's `DocumentCountResult` / `DocumentSumResult`
 * shape — the empty-string key carries the total).
 */
internal data class AggregateResult(
    val total: String?,
    val groups: Map<String, String>,
)

/**
 * Parse a count/sum result JSON: a bare number is the ungrouped total; an
 * object maps group keys to numbers, with `""` (or `total`) as the total.
 * The FFI wraps that map in a single-key envelope — `{"counts": {...}}` /
 * `{"sums": {...}}` per `dash_sdk_document_count`/`_sum`'s contract — so
 * unwrap it before reading entries.
 */
internal fun parseAggregateResult(response: String?): AggregateResult? {
    if (response.isNullOrBlank()) return null
    var element = LenientJson.parseToJsonElement(response)
    (element as? JsonObject)?.takeIf { it.size == 1 }?.let { root ->
        (root["counts"] ?: root["sums"])?.let { if (it is JsonObject) element = it }
    }
    return when (val el = element) {
        is JsonPrimitive -> AggregateResult(total = el.content, groups = emptyMap())
        is JsonObject -> {
            val entries = el.entries.mapNotNull { (key, value) ->
                (value as? JsonPrimitive)?.content?.let { key to it }
            }.toMap()
            AggregateResult(
                total = entries[""] ?: entries["total"]
                    ?: entries.values.singleOrNull().takeIf { entries.size <= 1 },
                groups = entries.filterKeys { it.isNotEmpty() && it != "total" },
            )
        }
        else -> null
    }
}
