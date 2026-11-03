package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.dashfoundation.dashsdk.queries.DocumentPropertyConstraint
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
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
 *
 * The "Property Constraints" section (protocol version 14) lists the rules
 * Rust reads from the contract's stored platform serialization
 * (`sdk.contracts.propertyConstraints`), re-read when the SDK learns the
 * network's protocol version: the rules are read at that version, and none
 * exist below 14.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DocumentTypeDetailsScreen(
    contractIdHex: String,
    typeName: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val protocolVersion by appState.platformProtocolVersion.collectAsStateWithLifecycle()
    val contractId = remember(contractIdHex) { contractIdHex.hexToBytes() }

    val contractFlow = remember(contractIdHex) {
        container.database.dataContractDao().observeById(contractId)
    }
    val contract by contractFlow.collectAsStateWithLifecycle(initialValue = null)

    var expandedIndices by rememberSaveable { mutableStateOf(setOf<String>()) }
    var constraintsSection by remember(contractIdHex, typeName) {
        mutableStateOf<PropertyConstraintsSection>(PropertyConstraintsSection.Hidden)
    }

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

        val contractConfig = remember(current.lastUpdated) {
            ParsedContract.from(current)?.root?.objectField("config")
        }
        val capabilities = documentTypeCapabilities(schema, contractConfig)

        LaunchedEffect(sdk, protocolVersion, current.lastUpdated, typeName) {
            val activeSdk = sdk
            val read: (suspend (ByteArray) -> List<DocumentPropertyConstraint>)? =
                if (activeSdk == null) {
                    null
                } else {
                    { bytes -> activeSdk.contracts.propertyConstraints(bytes, typeName) }
                }
            constraintsSection =
                loadPropertyConstraintsSection(schema, current.binarySerialization, read)
        }

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
                if (schema.boolField("indexOnly") == true) {
                    LabeledContent("Index Only", "Yes (entries are the documents)")
                }
                LabeledContent(
                    "Keep History",
                    if (schema.boolField("documentsKeepHistory") == true) "Yes" else "No",
                )
                LabeledContent(
                    "Mutable",
                    if (capabilities.documentsMutable) "Yes" else "No",
                )
                val immutability = documentTypeImmutability(schema)
                if (!immutability.isEmpty) {
                    LabeledContent(
                        "Immutable Properties",
                        immutability.immutable.sorted().joinToString(", "),
                    )
                    if (immutability.allowSetting.isNotEmpty()) {
                        LabeledContent(
                            "Settable Once While Absent",
                            immutability.allowSetting.sorted().joinToString(", "),
                        )
                    }
                }
                LabeledContent(
                    "Can Be Deleted",
                    if (capabilities.canBeDeleted) "Yes" else "No",
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

            PropertyConstraintsFormSection(constraintsSection)

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
                                    if (index.boolField("preallocated") == true) {
                                        Text(
                                            "PREALLOCATED",
                                            style = MaterialTheme.typography.labelSmall,
                                            color = MaterialTheme.colorScheme.secondary,
                                            modifier = Modifier.padding(start = 4.dp),
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
                                indexTerminal(
                                    index,
                                    indexOnly = schema.boolField("indexOnly") == true,
                                )?.let { terminal ->
                                    Text(
                                        "Terminal: $terminal",
                                        style = MaterialTheme.typography.labelSmall,
                                        color = MaterialTheme.colorScheme.secondary,
                                        modifier = Modifier.padding(start = 8.dp),
                                    )
                                }
                                val axes = indexAxisDescriptors(index)
                                if (axes.isNotEmpty()) {
                                    Text(
                                        axes.joinToString(" · "),
                                        style = MaterialTheme.typography.labelSmall,
                                        color = MaterialTheme.colorScheme.primary,
                                        modifier = Modifier.padding(start = 8.dp),
                                    )
                                }
                                index.objectField("timeRange")?.let { timeRange ->
                                    val range = timeRange.longField("range")
                                    val step = timeRange.longField("step")
                                    if (range != null && step != null) {
                                        Text(
                                            "Time Range: ${range}s windows every ${step}s",
                                            style = MaterialTheme.typography.labelSmall,
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

/**
 * The protocol-version-14 `propertyConstraints` rules: named conditions every
 * created or replaced document must meet, checked in name order. Rust reads
 * them from the stored contract; this section only shows what it reports
 * (← `propertyConstraintsSection` in DocumentTypeDetailsView.swift).
 */
@Composable
private fun PropertyConstraintsFormSection(section: PropertyConstraintsSection) {
    when (section) {
        PropertyConstraintsSection.Hidden -> Unit

        is PropertyConstraintsSection.Rules -> FormSection(
            title = "Property Constraints (${section.rules.size})",
            modifier = Modifier.testTag("documentType.propertyConstraints"),
        ) {
            section.rules.forEach { rule -> PropertyConstraintRow(rule) }
            Text(
                "Every created or replaced document must meet each rule, checked in name " +
                    "order. A document breaking one is refused (error 10422) and the fee " +
                    "is still charged.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }

        PropertyConstraintsSection.NotEnforced -> FormSection(
            title = "Property Constraints",
            modifier = Modifier.testTag("documentType.propertyConstraints"),
        ) {
            Text(
                "The schema declares propertyConstraints, but the network's protocol " +
                    "version does not enforce them (they take effect at protocol version 14).",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }

        is PropertyConstraintsSection.Unavailable -> FormSection(
            title = "Property Constraints",
            modifier = Modifier.testTag("documentType.propertyConstraints"),
        ) {
            Text(
                section.reason,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
            )
        }
    }
}

/**
 * One `propertyConstraints` rule: its name, the rule as declared, what it
 * reads, the system times and heights and the totals it reads, and whether an owner change
 * is judged against it too
 * (← `PropertyConstraintRowView` in DocumentTypeDetailsView.swift).
 */
@Composable
private fun PropertyConstraintRow(rule: DocumentPropertyConstraint) {
    val prettyRule = remember(rule) { rule.prettyRuleJson }
    val reads = remember(rule) { propertyConstraintReadsText(rule) }
    val systemReads = remember(rule) { propertyConstraintSystemReadsText(rule) }
    val totals = remember(rule) { propertyConstraintTotalsText(rule) }
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 6.dp)
            .testTag("documentType.propertyConstraint.${rule.name}"),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(rule.name, style = MaterialTheme.typography.titleSmall)
            if (rule.readsOwner) {
                Text(
                    "\$ownerId",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.tertiary,
                )
            }
        }
        Text(
            prettyRule,
            style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
            softWrap = false,
            modifier = Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState())
                .padding(vertical = 4.dp),
        )
        if (reads.isNotEmpty()) {
            Text(
                "Reads: $reads",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        if (systemReads != null) {
            Text(
                systemReads,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.tertiary,
                modifier = Modifier.testTag("documentType.propertyConstraint.${rule.name}.readsSystem"),
            )
            Text(
                PROPERTY_CONSTRAINT_SYSTEM_READS_NOTE,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.tertiary,
            )
        }
        if (totals != null) {
            Text(
                totals,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.tertiary,
                modifier = Modifier.testTag("documentType.propertyConstraint.${rule.name}.readsTotals"),
            )
            Text(
                PROPERTY_CONSTRAINT_TOTALS_NOTE,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.tertiary,
            )
        }
        if (rule.readsOwner) {
            Text(
                "Reads \$ownerId, the document's owner: transfers and purchases are judged " +
                    "against this rule too.",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.tertiary,
            )
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
            documentTypedArray(name, property)?.let { add(it.summary) }
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
                val subProperty = subProperties[subName] as? JsonObject
                // A nested typed array reads as its element kind and bounds.
                val subType = subProperty?.let { documentTypedArray(subName, it)?.summary }
                    ?: subProperty?.stringField("type")
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
