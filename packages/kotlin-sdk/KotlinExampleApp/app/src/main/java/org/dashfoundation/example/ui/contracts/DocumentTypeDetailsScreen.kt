package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.CountDocuments
import org.dashfoundation.example.navigation.Documents
import org.dashfoundation.example.navigation.NewDocument
import org.dashfoundation.example.navigation.SumAverageDocuments
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.hexToBytes

/**
 * Document-type schema drill-in — port of `DocumentTypeDetailsView.swift`:
 * info, settings flags, expandable indices, and property rows, all parsed
 * from the stored contract JSON (iOS reads the same data from the
 * `PersistentDocumentType` rows `DataContractParser` materializes).
 *
 * The "New Document" affordance broadcasts a real create state transition
 * via `CreateDocumentScreen` (port of iOS `CreateDocumentView`); the query
 * actions (browse / count / sum-average) live alongside it.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DocumentTypeDetailsScreen(
    contractIdHex: String,
    typeName: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val contractId = remember(contractIdHex) { contractIdHex.hexToBytes() }

    val contractFlow = remember(contractIdHex) {
        container.database.dataContractDao().observeById(contractId)
    }
    val contract by contractFlow.collectAsStateWithLifecycle(initialValue = null)

    var expandedIndices by rememberSaveable { mutableStateOf(setOf<String>()) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(typeName) },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        val current = contract ?: return@Scaffold
        val schema = remember(current.lastUpdated, typeName) {
            ParsedContract.from(current)?.documentTypes?.get(typeName)
        } ?: return@Scaffold

        val properties = schema.objectField("properties") ?: JsonObject(emptyMap())
        val indices = schema.arrayField("indices")?.mapNotNull { it as? JsonObject }.orEmpty()
        val required = schema.arrayField("required")
            ?.mapNotNull { (it as? JsonPrimitive)?.content }
            ?.toSet()
            .orEmpty()

        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Actions") {
                TextButton(
                    onClick = { navController.navigate(NewDocument(contractIdHex, typeName)) },
                    modifier = Modifier.testTag("documentType.newDocument"),
                ) { Text("New Document") }
            }

            FormSection(title = "Queries") {
                TextButton(
                    onClick = { navController.navigate(Documents(contractIdHex, typeName)) },
                    modifier = Modifier.testTag("documentType.browseDocuments"),
                ) { Text("Browse Documents") }
                TextButton(
                    onClick = { navController.navigate(CountDocuments(contractIdHex, typeName)) },
                    modifier = Modifier.testTag("documentType.countDocuments"),
                ) { Text("Count Documents") }
                TextButton(
                    onClick = {
                        navController.navigate(SumAverageDocuments(contractIdHex, typeName))
                    },
                    modifier = Modifier.testTag("documentType.sumAverageDocuments"),
                ) { Text("Sum / Average Documents") }
            }

            FormSection(title = "Document Type Information") {
                LabeledContent("Name", typeName)
                LabeledContent("Properties", "${properties.size}")
                LabeledContent("Indices", "${indices.size}")
                if (required.isNotEmpty()) {
                    LabeledContent("Required Fields", "${required.size}")
                }
                schema.intField("securityLevel")?.let {
                    LabeledContent("Security Level", "$it")
                }
            }

            FormSection(title = "Document Settings") {
                LabeledContent(
                    "Keep History",
                    if (schema.boolField("documentsKeepHistory") == true) "Yes" else "No",
                )
                LabeledContent(
                    "Mutable",
                    if (schema.boolField("documentsMutable") != false) "Yes" else "No",
                )
                LabeledContent(
                    "Can Be Deleted",
                    if (schema.boolField("canBeDeleted") != false ||
                        schema.boolField("documentsCanBeDeleted") != false
                    ) "Yes" else "No",
                )
                LabeledContent(
                    "Transferable",
                    if ((schema.intField("transferable") ?: 0) > 0) "Yes" else "No",
                )
                LabeledContent(
                    "Trade Mode",
                    if ((schema.intField("tradeMode") ?: 0) > 0) "Yes" else "No",
                )
                val restriction = schema.intField("creationRestrictionMode") ?: 0
                if (restriction > 0) {
                    LabeledContent(
                        "Creation",
                        if (restriction == 1) "Owner Only" else "System Only",
                    )
                }
                if (schema.boolField("requiresIdentityEncryptionBoundedKey") == true) {
                    LabeledContent("Requires Encryption Key", "Yes")
                }
                if (schema.boolField("requiresIdentityDecryptionBoundedKey") == true) {
                    LabeledContent("Requires Decryption Key", "Yes")
                }
            }

            if (indices.isNotEmpty()) {
                FormSection(title = "Indices (${indices.size})") {
                    indices.sortedBy { it.stringField("name").orEmpty() }.forEach { index ->
                        val name = index.stringField("name") ?: "(unnamed)"
                        val isExpanded = name in expandedIndices
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable {
                                    expandedIndices =
                                        if (isExpanded) expandedIndices - name
                                        else expandedIndices + name
                                }
                                .padding(vertical = 6.dp)
                                .testTag("documentType.index.$name"),
                        ) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                            ) {
                                Text(name, style = MaterialTheme.typography.titleSmall)
                                Row {
                                    if (index.boolField("unique") == true) {
                                        Text(
                                            "UNIQUE",
                                            style = MaterialTheme.typography.labelSmall,
                                            color = MaterialTheme.colorScheme.tertiary,
                                        )
                                    }
                                    Icon(
                                        if (isExpanded) Icons.Default.KeyboardArrowUp
                                        else Icons.Default.KeyboardArrowDown,
                                        contentDescription = null,
                                    )
                                }
                            }
                            if (isExpanded) {
                                index.arrayField("properties")
                                    ?.mapNotNull { it as? JsonObject }
                                    ?.forEach { propEntry ->
                                        propEntry.entries.forEach { (field, direction) ->
                                            Text(
                                                "→ $field " +
                                                    "(${(direction as? JsonPrimitive)?.content})",
                                                style = MaterialTheme.typography.bodySmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                modifier = Modifier.padding(start = 8.dp),
                                            )
                                        }
                                    }
                                if (index.boolField("nullSearchable") == true) {
                                    Text(
                                        "Null Searchable",
                                        style = MaterialTheme.typography.labelSmall,
                                        modifier = Modifier.padding(start = 8.dp),
                                    )
                                }
                                if (index["contested"] != null) {
                                    Text(
                                        "Contested",
                                        style = MaterialTheme.typography.labelSmall,
                                        color = MaterialTheme.colorScheme.error,
                                        modifier = Modifier.padding(start = 8.dp),
                                    )
                                }
                            }
                        }
                    }
                }
            }

            if (properties.isNotEmpty()) {
                FormSection(title = "Properties (${properties.size})") {
                    properties.keys.sorted().forEach { propName ->
                        val prop = properties[propName] as? JsonObject ?: JsonObject(emptyMap())
                        PropertyRow(
                            name = propName,
                            property = prop,
                            isRequired = propName in required,
                        )
                    }
                }
            }
        }
    }
}

/** One schema property row (← `PropertyRowView` in DocumentTypeDetailsView.swift). */
@Composable
private fun PropertyRow(name: String, property: JsonObject, isRequired: Boolean) {
    Column(modifier = Modifier
        .fillMaxWidth()
        .padding(vertical = 6.dp)) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(name, style = MaterialTheme.typography.titleSmall)
            Text(
                property.stringField("type") ?: "unknown",
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.primary,
            )
        }
        val attributes = buildList {
            if (isRequired) add("Required")
            property.intField("minLength")?.let { add("Min: $it") }
            property.intField("maxLength")?.let { add("Max: $it") }
            if (property.stringField("pattern") != null) add("Pattern")
            if (property.boolField("byteArray") == true) add("Byte Array")
            property.stringField("contentMediaType")
                ?.substringAfterLast('.')
                ?.let { add(it) }
        }
        if (attributes.isNotEmpty()) {
            Text(
                attributes.joinToString(" · "),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        property.stringField("description")?.let {
            Text(
                it,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 3,
            )
        }
        property.objectField("properties")?.let { subProperties ->
            subProperties.keys.sorted().forEach { subName ->
                val subType = (subProperties[subName] as? JsonObject)?.stringField("type")
                Text(
                    "→ $subName${subType?.let { " ($it)" } ?: ""}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 8.dp),
                )
            }
        }
    }
}
