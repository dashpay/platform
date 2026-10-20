package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.LenientJson
import org.dashfoundation.example.util.truncateMiddle

/**
 * Read-only diagnostic over the raw GroveDB path-elements query — port of
 * `GroveDBPathElementsView.swift` (QA test SYS-06), driving the bridged
 * `SystemQueries.groveDbPathElements` (`dash_sdk_system_get_path_elements`).
 *
 * Takes a `path` (JSON array of strings) and `keys` (JSON array of
 * strings) — each string hex bytes (preferred) or plain text — and renders
 * the returned elements (`type` / `key` / `element`) or the decode / FFI
 * error. The "DPNS contract example" preset fills a bounded query (the
 * DPNS system contract under the `DataContractDocuments` root, byte
 * `0x40`); root-level (empty-path) queries currently fail GroveDB proof
 * verification, so the preset stays bounded on purpose.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GroveDBPathElementsScreen(navController: NavHostController) {
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val sdk by appState.sdk.collectAsStateWithLifecycle()

    var pathText by rememberSaveable { mutableStateOf("") }
    var keysText by rememberSaveable { mutableStateOf("") }
    var isRunning by remember { mutableStateOf(false) }
    var didRun by remember { mutableStateOf(false) }
    var elements by remember { mutableStateOf<List<PathElementRow>>(emptyList()) }
    var errorMessage by remember { mutableStateOf<String?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("GroveDB Path Elements") },
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
            FormSection(title = "Query") {
                OutlinedTextField(
                    value = pathText,
                    onValueChange = { pathText = it },
                    label = { Text("Path JSON array") },
                    placeholder = { Text("[] or [\"60\"]") },
                    singleLine = true,
                    enabled = !isRunning,
                    textStyle = MaterialTheme.typography.bodySmall
                        .copy(fontFamily = FontFamily.Monospace),
                    modifier = Modifier.fillMaxWidth().testTag("sysPathElements.pathField"),
                )
                OutlinedTextField(
                    value = keysText,
                    onValueChange = { keysText = it },
                    label = { Text("Keys JSON array") },
                    placeholder = { Text("[\"20\",\"40\",\"10\",\"60\"]") },
                    singleLine = true,
                    enabled = !isRunning,
                    textStyle = MaterialTheme.typography.bodySmall
                        .copy(fontFamily = FontFamily.Monospace),
                    modifier = Modifier.fillMaxWidth().testTag("sysPathElements.keysField"),
                )
                OutlinedButton(
                    onClick = {
                        // Bounded example: the DPNS system contract under the
                        // DataContractDocuments root (RootTree byte 0x40).
                        pathText = "[\"40\"]"
                        keysText =
                            "[\"e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c53155\"]"
                        didRun = false
                        elements = emptyList()
                        errorMessage = null
                    },
                    enabled = !isRunning,
                    modifier = Modifier.testTag("sysPathElements.presetButton"),
                ) { Text("DPNS contract example") }
                Text(
                    "path and keys are JSON arrays of strings — each string hex " +
                        "bytes (preferred) or plain text. Use a bounded (non-empty) " +
                        "path: root-level queries fail GroveDB proof verification.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            SubmitButton(
                text = if (isRunning) "Running…" else "Run",
                isLoading = isRunning,
                enabled = sdk != null && !isRunning,
                modifier = Modifier.fillMaxWidth().testTag("sysPathElements.runButton"),
            ) {
                val currentSdk = sdk ?: return@SubmitButton
                val path = decodeStringArray(pathText)
                if (path == null) {
                    errorMessage = "path must be a JSON array of strings, e.g. [\"60\"]"
                    didRun = true
                    return@SubmitButton
                }
                val keys = decodeStringArray(keysText)
                if (keys == null) {
                    errorMessage = "keys must be a JSON array of strings, e.g. [\"60\"]"
                    didRun = true
                    return@SubmitButton
                }
                isRunning = true
                errorMessage = null
                elements = emptyList()
                scope.launch {
                    try {
                        val json = currentSdk.system.groveDbPathElements(
                            pathJson = pathText.trim(),
                            keysJson = keysText.trim(),
                        )
                        elements = json?.let(::parsePathElements).orEmpty()
                    } catch (e: Exception) {
                        errorMessage = e.message ?: "Query failed"
                    } finally {
                        didRun = true
                        isRunning = false
                    }
                }
            }

            if (didRun) {
                val message = errorMessage
                if (message != null) {
                    FormSection(title = "Error") {
                        Text(
                            message,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.testTag("sysPathElements.errorText"),
                        )
                    }
                } else {
                    FormSection(title = "Elements (${elements.size})") {
                        if (elements.isEmpty()) {
                            Text(
                                "No elements found",
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.testTag("sysPathElements.emptyText"),
                            )
                        } else {
                            elements.forEach { element ->
                                Column(
                                    verticalArrangement = Arrangement.spacedBy(2.dp),
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .padding(vertical = 4.dp)
                                        .testTag("sysPathElements.resultRow.${element.key}"),
                                ) {
                                    Text(
                                        element.type,
                                        style = MaterialTheme.typography.titleSmall,
                                        color = MaterialTheme.colorScheme.primary,
                                    )
                                    Text(
                                        truncateMiddle(element.key, 24, 12),
                                        style = MaterialTheme.typography.bodySmall,
                                        fontFamily = FontFamily.Monospace,
                                    )
                                    Text(
                                        element.element,
                                        style = MaterialTheme.typography.bodySmall,
                                        fontFamily = FontFamily.Monospace,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/** One rendered element row (`type` / `key` / `element`). */
private data class PathElementRow(val type: String, val key: String, val element: String)

/** Validate a JSON array-of-strings field; null when it isn't one. */
private fun decodeStringArray(text: String): List<String>? = try {
    (LenientJson.parseToJsonElement(text.trim()) as? JsonArray)
        ?.map { (it as? JsonPrimitive)?.content ?: return null }
} catch (_: Exception) {
    null
}

/** Parse `[{"key":hex,"element":…,"type":…}]` from the FFI. */
private fun parsePathElements(json: String): List<PathElementRow> = try {
    (LenientJson.parseToJsonElement(json) as? JsonArray)
        ?.mapNotNull { it as? JsonObject }
        ?.map { row ->
            PathElementRow(
                type = (row["type"] as? JsonPrimitive)?.content ?: "unknown",
                key = (row["key"] as? JsonPrimitive)?.content ?: "",
                element = (row["element"] as? JsonPrimitive)?.content ?: "",
            )
        }
        .orEmpty()
} catch (_: Exception) {
    emptyList()
}
