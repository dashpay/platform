package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
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
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.AddressQueries
import org.dashfoundation.example.navigation.Diagnostics
import org.dashfoundation.example.navigation.GroveDbPathElements
import org.dashfoundation.example.navigation.QueryDetail
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.prettyPrintJson

/**
 * Platform-queries explorer — the Kotlin counterpart of
 * `QueryDetailView.swift` (a dynamic parameter form + execute + JSON
 * result) with a small list screen standing in for the iOS
 * `PlatformQueriesView` entry point. The registry ([QUERY_REGISTRY] in
 * `QueryRegistry.kt`) covers the queries the Kotlin SDK's read surface
 * exposes today; the iOS view routes ~40 more (contested resources,
 * epochs, groups, tokens, …) that wait on their JNI exports. The two iOS
 * categories with dedicated destinations map the same way here:
 * Addresses → [AddressQueries], Diagnostics → [Diagnostics].
 */

/** Grouped list of runnable queries (← the `PlatformQueriesView` role). */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun QueriesListScreen(navController: NavHostController) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Platform Queries") },
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
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            QUERY_REGISTRY.forEach { query ->
                ListItem(
                    headlineContent = { Text(query.label) },
                    supportingContent = {
                        Text(query.description, style = MaterialTheme.typography.bodySmall)
                    },
                    modifier = Modifier
                        .clickable {
                            navController.navigate(QueryDetail(query.name))
                        }
                        .testTag("queries.row.${query.name}"),
                )
            }

            // Categories with dedicated destinations, mirroring the iOS
            // list (System → GroveDBPathElementsView, Addresses →
            // AddressQueriesView, Diagnostics → DiagnosticsView's
            // "Run All Queries").
            ListItem(
                headlineContent = { Text("GroveDB Path Elements") },
                supportingContent = {
                    Text(
                        "Raw GroveDB path-elements query (SYS-06)",
                        style = MaterialTheme.typography.bodySmall,
                    )
                },
                modifier = Modifier
                    .clickable { navController.navigate(GroveDbPathElements) }
                    .testTag("queries.row.groveDbPathElements"),
            )
            ListItem(
                headlineContent = { Text("Address Queries") },
                supportingContent = {
                    Text(
                        "Platform address balance and nonce queries",
                        style = MaterialTheme.typography.bodySmall,
                    )
                },
                modifier = Modifier
                    .clickable { navController.navigate(AddressQueries) }
                    .testTag("queries.row.addressQueries"),
            )
            ListItem(
                headlineContent = { Text("Run All Queries") },
                supportingContent = {
                    Text(
                        "Execute all bridged queries with test data to verify connectivity",
                        style = MaterialTheme.typography.bodySmall,
                    )
                },
                modifier = Modifier
                    .clickable { navController.navigate(Diagnostics) }
                    .testTag("queries.row.runAllQueries"),
            )
        }
    }
}

/** One query's parameter form + execute + result (← `QueryDetailView.swift`). */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun QueryDetailScreen(
    queryName: String,
    navController: NavHostController,
) {
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val sdk by appState.sdk.collectAsStateWithLifecycle()

    val query = remember(queryName) { QUERY_REGISTRY.firstOrNull { it.name == queryName } }

    val inputs = remember(queryName) { mutableStateMapOf<String, String>() }
    var isLoading by remember { mutableStateOf(false) }
    var result by remember { mutableStateOf<String?>(null) }
    var errorMessage by remember { mutableStateOf<String?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(query?.label ?: queryName) },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        val current = query ?: run {
            Text(
                "Unknown query: $queryName",
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(padding).padding(16.dp),
            )
            return@Scaffold
        }

        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Description") {
                Text(current.description, style = MaterialTheme.typography.bodyMedium)
            }

            if (current.inputs.isNotEmpty()) {
                FormSection(title = "Parameters") {
                    current.inputs.forEach { input ->
                        OutlinedTextField(
                            value = inputs[input.name] ?: "",
                            onValueChange = { inputs[input.name] = it },
                            label = {
                                Text(input.label + if (input.required) " *" else "")
                            },
                            placeholder = { Text(input.placeholder) },
                            enabled = !isLoading,
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(vertical = 4.dp)
                                .testTag("queryDetail.input.${input.name}"),
                        )
                    }
                }
            }

            val requiredFilled = current.inputs
                .filter { it.required }
                .all { !inputs[it.name].isNullOrBlank() }

            SubmitButton(
                text = "Execute Query",
                isLoading = isLoading,
                enabled = requiredFilled,
                modifier = Modifier.testTag("queryDetail.executeButton"),
            ) {
                val currentSdk = sdk
                if (currentSdk == null) {
                    errorMessage = "SDK not initialized"
                    return@SubmitButton
                }
                scope.launch {
                    isLoading = true
                    errorMessage = null
                    result = null
                    try {
                        val response = current.execute(
                            currentSdk,
                            current.inputs.associate { it.name to (inputs[it.name] ?: "").trim() },
                        )
                        result = response?.let { prettyPrintJson(it) } ?: "null"
                    } catch (e: Exception) {
                        errorMessage = e.message ?: "Query failed"
                    } finally {
                        isLoading = false
                    }
                }
            }

            errorMessage?.let { message ->
                FormSection(title = "Error") {
                    Text(
                        message,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("queryDetail.errorText"),
                    )
                }
            }

            result?.let { text ->
                FormSection(title = "Result") {
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .horizontalScroll(rememberScrollState()),
                    ) {
                        Text(
                            text,
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace,
                            modifier = Modifier.testTag("queryDetail.resultText"),
                        )
                    }
                }
            }
        }
    }
}
