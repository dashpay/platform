package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.DocumentTypeDetail
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.formatByteCount
import org.dashfoundation.example.util.formatDate
import org.dashfoundation.example.util.formatRelative
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.prettyPrintJson
import org.dashfoundation.example.util.truncateMiddle

/**
 * Saved-contract details — port of `DataContractDetailsView.swift`:
 * configuration flags, contract info, document-type list (drilling into
 * [DocumentTypeDetail]), token count, group rows (drilling into
 * [org.dashfoundation.example.navigation.GroupDetail]), and a raw JSON
 * viewer.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DataContractDetailsScreen(
    contractIdHex: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val contractId = remember(contractIdHex) { contractIdHex.hexToBytes() }

    val contractFlow = remember(contractIdHex) {
        container.database.dataContractDao().observeById(contractId)
    }
    val contract by contractFlow.collectAsStateWithLifecycle(initialValue = null)

    var showRawJson by rememberSaveable { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Contract Details") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        val current = contract ?: return@Scaffold
        val parsed = remember(current.lastUpdated, current.id) { ParsedContract.from(current) }

        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Contract Configuration") {
                LabeledContent("Can Be Deleted", if (current.canBeDeleted) "Yes" else "No")
                LabeledContent("Read Only", if (current.readonly) "Yes" else "No")
                LabeledContent("Keeps History", if (current.keepsHistory) "Yes" else "No")
                LabeledContent(
                    "Documents Keep History (Default)",
                    if (current.documentsKeepHistoryContractDefault) "Yes" else "No",
                )
                LabeledContent(
                    "Documents Mutable (Default)",
                    if (current.documentsMutableContractDefault) "Yes" else "No",
                )
                LabeledContent(
                    "Documents Can Be Deleted (Default)",
                    if (current.documentsCanBeDeletedContractDefault) "Yes" else "No",
                )
                current.schemaDefs?.let { LabeledContent("Schema Definitions", "$it") }
            }

            FormSection(title = "Contract Information") {
                LabeledContent("Name", current.name)
                LabeledContent("ID", truncateMiddle(Base58.encode(current.id), 12, 8))
                current.version?.let { LabeledContent("Version", "$it") }
                current.ownerId?.let {
                    LabeledContent("Owner", truncateMiddle(Base58.encode(it), 12, 8))
                }
                LabeledContent(
                    "JSON Size",
                    formatByteCount(current.serializedContract.size.toLong()),
                )
                current.binarySerialization?.let {
                    LabeledContent("Binary Size", formatByteCount(it.size.toLong()))
                }
                LabeledContent("Created", formatDate(current.createdAt))
                LabeledContent("Last Used", formatRelative(current.lastAccessedAt))
            }

            val documentTypes = parsed?.documentTypes.orEmpty()
            if (documentTypes.isNotEmpty()) {
                FormSection(title = "Document Types (${documentTypes.size})") {
                    documentTypes.keys.sorted().forEach { typeName ->
                        val schema = documentTypes.getValue(typeName)
                        val propertyCount = schema.objectField("properties")?.size ?: 0
                        val indexCount = schema.arrayField("indices")?.size ?: 0
                        ListItem(
                            headlineContent = { Text(typeName) },
                            supportingContent = {
                                Text(
                                    "$propertyCount properties · $indexCount indices",
                                    style = MaterialTheme.typography.bodySmall,
                                )
                            },
                            modifier = Modifier
                                .clickable {
                                    navController.navigate(
                                        DocumentTypeDetail(contractIdHex, typeName),
                                    )
                                }
                                .testTag("contractDetail.docType.$typeName"),
                        )
                    }
                }
            }

            val tokens = parsed?.tokens.orEmpty()
            if (current.hasTokens || tokens.isNotEmpty()) {
                FormSection(title = "Tokens (${tokens.size})") {
                    tokens.keys.sortedBy { it.toIntOrNull() ?: Int.MAX_VALUE }.forEach { position ->
                        val token = tokens.getValue(position)
                        val singular = token.objectField("conventions")
                            ?.objectField("localizations")
                            ?.objectField("en")
                            ?.stringField("singularForm")
                        LabeledContent(
                            "Position $position",
                            singular ?: token.stringField("description") ?: "Token",
                        )
                    }
                }
            }

            val groups = parsed?.groups.orEmpty()
            if (groups.isNotEmpty()) {
                FormSection(title = "Groups (${groups.size})") {
                    groups.keys.sortedBy { it.toIntOrNull() ?: Int.MAX_VALUE }.forEach { position ->
                        val group = groups.getValue(position)
                        val memberCount = group.objectField("members")?.size ?: 0
                        val requiredPower = group.intField("requiredPower") ?: 0
                        ListItem(
                            headlineContent = { Text("Group $position") },
                            supportingContent = {
                                Text(
                                    "$memberCount members · required power $requiredPower",
                                    style = MaterialTheme.typography.bodySmall,
                                )
                            },
                            modifier = Modifier
                                .clickable {
                                    position.toIntOrNull()?.let { pos ->
                                        navController.navigate(
                                            org.dashfoundation.example.navigation.GroupDetail(
                                                contractIdHex,
                                                pos,
                                            ),
                                        )
                                    }
                                }
                                .testTag("contractDetail.group.$position"),
                        )
                    }
                }
            }

            FormSection(title = "Raw Contract JSON") {
                TextButton(
                    onClick = { showRawJson = !showRawJson },
                    modifier = Modifier.testTag("contractDetail.toggleRawJson"),
                ) {
                    Text(if (showRawJson) "Hide JSON" else "Show JSON")
                }
                if (showRawJson) {
                    val pretty = remember(current.lastUpdated, current.id) {
                        prettyPrintJson(current.serializedContract.decodeToString())
                    }
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .heightIn(max = 400.dp)
                            .verticalScroll(rememberScrollState())
                            .horizontalScroll(rememberScrollState()),
                    ) {
                        Text(
                            pretty,
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace,
                            modifier = Modifier.testTag("contractDetail.rawJson"),
                        )
                    }
                }
            }
        }
    }
}
