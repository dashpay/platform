package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
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
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.DocumentActions
import org.dashfoundation.example.navigation.DocumentFields
import org.dashfoundation.example.navigation.DocumentWithPrice
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.truncateMiddle

/**
 * Document query builder + result list — the read/query role of
 * `DocumentsView.swift`. Each result row drills into the document viewer, a
 * price/purchase screen ("Price…"), and the replace/delete/transfer actions
 * ("Actions…" → [DocumentActionsScreen]). Where/orderBy syntax matches the
 * JSON strings iOS passes to `sdk.documentList` (see the keyword search in
 * ContractsTabView.swift:684-707).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DocumentsScreen(
    contractIdHex: String,
    typeName: String,
    navController: NavHostController,
) {
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val sdk by appState.sdk.collectAsStateWithLifecycle()

    val contractIdBase58 = remember(contractIdHex) {
        Base58.encode(contractIdHex.hexToBytes())
    }

    var whereJson by rememberSaveable { mutableStateOf("") }
    var orderByJson by rememberSaveable { mutableStateOf("") }
    var limitText by rememberSaveable { mutableStateOf("25") }
    var isRunning by remember { mutableStateOf(false) }
    var didRun by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var results by remember { mutableStateOf<List<JsonObject>>(emptyList()) }

    fun run() {
        val currentSdk = sdk
        if (currentSdk == null) {
            errorMessage = "SDK not initialized"
            return
        }
        scope.launch {
            isRunning = true
            errorMessage = null
            try {
                val response = currentSdk.contracts.fetch(contractIdBase58).use { contract ->
                    currentSdk.documents.search(
                        contract = contract,
                        documentType = typeName,
                        whereJson = whereJson.trim().takeIf { it.isNotEmpty() },
                        orderByJson = orderByJson.trim().takeIf { it.isNotEmpty() },
                        limit = limitText.trim().toIntOrNull() ?: 0,
                    )
                }
                results = parseDocuments(response)
                didRun = true
            } catch (e: Exception) {
                errorMessage = e.message ?: "Query failed"
                results = emptyList()
                didRun = true
            } finally {
                isRunning = false
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Documents: $typeName") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            item {
                FormSection(title = "Query") {
                    OutlinedTextField(
                        value = whereJson,
                        onValueChange = { whereJson = it },
                        label = { Text("Where (JSON, optional)") },
                        placeholder = {
                            Text("[{\"field\":\"normalizedLabel\",\"operator\":\"startsWith\",\"value\":\"a\"}]")
                        },
                        enabled = !isRunning,
                        modifier = Modifier
                            .fillMaxWidth()
                            .testTag("documents.whereField"),
                    )
                    OutlinedTextField(
                        value = orderByJson,
                        onValueChange = { orderByJson = it },
                        label = { Text("Order By (JSON, optional)") },
                        placeholder = {
                            Text("[{\"field\":\"normalizedLabel\",\"ascending\":true}]")
                        },
                        enabled = !isRunning,
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(top = 8.dp)
                            .testTag("documents.orderByField"),
                    )
                    OutlinedTextField(
                        value = limitText,
                        onValueChange = { limitText = it.filter { c -> c.isDigit() } },
                        label = { Text("Limit") },
                        singleLine = true,
                        enabled = !isRunning,
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(top = 8.dp)
                            .testTag("documents.limitField"),
                    )
                    SubmitButton(
                        text = "Run Query",
                        isLoading = isRunning,
                        modifier = Modifier
                            .padding(top = 8.dp)
                            .testTag("documents.runButton"),
                    ) { run() }
                }
            }

            errorMessage?.let { message ->
                item {
                    FormSection(title = "Error") {
                        Text(
                            message,
                            color = MaterialTheme.colorScheme.error,
                            style = MaterialTheme.typography.bodyMedium,
                            modifier = Modifier.testTag("documents.errorText"),
                        )
                    }
                }
            }

            if (didRun && errorMessage == null) {
                item {
                    Text(
                        "Results (${results.size})",
                        style = MaterialTheme.typography.titleMedium,
                        modifier = Modifier.testTag("documents.resultCount"),
                    )
                }
                if (results.isEmpty()) {
                    item {
                        Text(
                            "No documents matched the query.",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                items(results) { document ->
                    val id = document.stringField("\$id") ?: document.stringField("id") ?: ""
                    val owner = document.stringField("\$ownerId")
                        ?: document.stringField("ownerId") ?: ""
                    Card(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable {
                                navController.navigate(DocumentFields(document.toString()))
                            }
                            .testTag("documents.row.$id"),
                    ) {
                        ListItem(
                            headlineContent = {
                                Text(
                                    documentHeadline(document, typeName),
                                    maxLines = 1,
                                )
                            },
                            supportingContent = {
                                Column {
                                    if (id.isNotEmpty()) {
                                        Text(
                                            truncateMiddle(id),
                                            style = MaterialTheme.typography.bodySmall,
                                            fontFamily = FontFamily.Monospace,
                                        )
                                    }
                                    if (owner.isNotEmpty()) {
                                        Text(
                                            "owner ${truncateMiddle(owner)}",
                                            style = MaterialTheme.typography.bodySmall,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                }
                            },
                            // Price / purchase drill-in (← the Set Price… /
                            // Purchase… context actions on iOS document rows)
                            // plus the replace/delete/transfer actions menu.
                            trailingContent = {
                                if (id.isNotEmpty()) {
                                    Column {
                                        TextButton(
                                            onClick = {
                                                navController.navigate(
                                                    DocumentWithPrice(
                                                        contractIdHex = contractIdHex,
                                                        typeName = typeName,
                                                        documentId = id,
                                                    ),
                                                )
                                            },
                                            modifier = Modifier.testTag("documents.price.$id"),
                                        ) { Text("Price…") }
                                        TextButton(
                                            onClick = {
                                                navController.navigate(
                                                    DocumentActions(
                                                        contractIdHex = contractIdHex,
                                                        typeName = typeName,
                                                        documentId = id,
                                                    ),
                                                )
                                            },
                                            modifier = Modifier.testTag("documents.actions.$id"),
                                        ) { Text("Actions…") }
                                    }
                                }
                            },
                        )
                    }
                }
            }
        }
    }
}

/**
 * Parse the search response into document objects. The FFI returns either
 * a bare JSON array of documents or an object wrapping one under
 * `documents` (the shape iOS reads in ContractsTabView.swift:738).
 */
private fun parseDocuments(response: String?): List<JsonObject> {
    if (response.isNullOrBlank()) return emptyList()
    val element = LenientJson.parseToJsonElement(response)
    val array = when (element) {
        is JsonArray -> element
        is JsonObject -> element["documents"] as? JsonArray ?: return emptyList()
        else -> return emptyList()
    }
    return array.mapNotNull { it as? JsonObject }
}

/**
 * Preferred-field headline heuristic (← `PersistentDocument.displayTitle`):
 * first of label/name/title/message, falling back to the type name.
 */
private fun documentHeadline(document: JsonObject, typeName: String): String {
    for (key in listOf("label", "name", "title", "message", "normalizedLabel")) {
        (document[key] as? JsonPrimitive)?.content?.takeIf { it.isNotBlank() }?.let { return it }
    }
    return typeName
}
