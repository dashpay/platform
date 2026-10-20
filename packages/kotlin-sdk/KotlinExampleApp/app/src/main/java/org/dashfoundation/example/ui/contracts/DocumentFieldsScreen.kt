package org.dashfoundation.example.ui.contracts

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
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.navigation.NavHostController
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.LenientJson
import org.dashfoundation.example.util.prettyPrintJson

/**
 * Single fetched document as pretty-printed key/value rows — the viewer
 * counterpart of `DocumentFieldsView.swift` (whose iOS role is the
 * create-form builder; document creation needs state-transition FFI not
 * yet exposed by the Kotlin queries layer). System fields (`$`-prefixed)
 * group separately from application data, and nested objects/arrays fall
 * back to a monospace JSON block.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DocumentFieldsScreen(
    documentJson: String,
    navController: NavHostController,
) {
    val document = remember(documentJson) {
        try {
            LenientJson.parseToJsonElement(documentJson) as? JsonObject
        } catch (_: Exception) {
            null
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Document") },
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
            if (document == null) {
                Text(
                    "Could not parse document JSON.",
                    color = MaterialTheme.colorScheme.error,
                )
                return@Column
            }

            val (system, application) = document.entries.partition { it.key.startsWith("$") }

            if (system.isNotEmpty()) {
                FormSection(title = "System Fields") {
                    system.sortedBy { it.key }.forEach { (key, value) ->
                        FieldRow(key, value)
                    }
                }
            }

            FormSection(title = "Fields (${application.size})") {
                if (application.isEmpty()) {
                    Text(
                        "This document has no application data fields.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    application.sortedBy { it.key }.forEach { (key, value) ->
                        FieldRow(key, value)
                    }
                }
            }

            FormSection(title = "Raw JSON") {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .horizontalScroll(rememberScrollState()),
                ) {
                    Text(
                        prettyPrintJson(documentJson),
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                        modifier = Modifier.testTag("documentFields.rawJson"),
                    )
                }
            }
        }
    }
}

@Composable
private fun FieldRow(key: String, value: JsonElement) {
    when (value) {
        is JsonPrimitive -> LabeledContent(
            key,
            if (value is JsonNull) "null" else value.content,
            modifier = Modifier.testTag("documentFields.field.$key"),
        )
        is JsonObject, is JsonArray -> Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 6.dp)
                .testTag("documentFields.field.$key"),
        ) {
            Text(key, style = MaterialTheme.typography.bodyMedium)
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .horizontalScroll(rememberScrollState()),
            ) {
                Text(
                    prettyPrintJson(value.toString()),
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
