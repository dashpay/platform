package org.dashfoundation.example.ui.storage

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
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.navigation.NavHostController
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.formatDate
import org.dashfoundation.example.util.toHex
import java.util.Date

/**
 * Generic record detail — port of the per-model detail views in
 * `StorageRecordDetailViews.swift`: every column of the row as a
 * label/value pair. Formatting rules mirror the iOS renderers: blobs as
 * hex (32-byte identifiers additionally as base58), epoch-millis date
 * columns as dates, and everything else verbatim.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun StorageRecordDetailScreen(
    modelName: String,
    rowId: Long,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val model = remember(modelName) { storageModel(modelName) }

    var row by remember { mutableStateOf<StorageRow?>(null) }
    var isLoading by remember { mutableStateOf(true) }
    var errorMessage by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(modelName, rowId) {
        val current = model ?: return@LaunchedEffect
        isLoading = true
        try {
            row = loadStorageRow(container.database, current, rowId)
        } catch (e: Exception) {
            errorMessage = e.message ?: "Failed to load record"
        } finally {
            isLoading = false
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(model?.displayName ?: modelName) },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        when {
            isLoading -> Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentAlignment = Alignment.Center,
            ) {
                CircularProgressIndicator()
            }
            row == null -> Text(
                errorMessage ?: "Record not found.",
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(padding).padding(16.dp),
            )
            else -> Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .verticalScroll(rememberScrollState())
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                FormSection(title = "Fields") {
                    row?.columns?.forEach { (name, value) ->
                        FieldRow(name, value)
                    }
                }
            }
        }
    }
}

/** Column names Room persists as epoch-millis `Date`s (see `Converters.kt`). */
private val DATE_COLUMNS = setOf(
    "createdAt", "updatedAt", "lastUpdated", "lastUpdatedAt", "lastAccessedAt",
    "lastAccessed", "lastSyncedAt", "eventTimestamp", "localCreatedAt",
    "localUpdatedAt", "transferredAt",
)

@Composable
private fun FieldRow(name: String, value: Any?) {
    Column(modifier = Modifier
        .fillMaxWidth()
        .padding(vertical = 6.dp)) {
        Text(
            name,
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        when (value) {
            null -> Text("—", style = MaterialTheme.typography.bodyMedium)
            is ByteArray -> {
                if (value.isEmpty()) {
                    Text("(empty)", style = MaterialTheme.typography.bodyMedium)
                } else {
                    if (value.size == 32) {
                        Text(
                            Base58.encode(value),
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace,
                        )
                    }
                    val hex = if (value.size <= 128) {
                        value.toHex()
                    } else {
                        value.copyOfRange(0, 128).toHex() + "… (${value.size} bytes)"
                    }
                    Text(
                        hex,
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            is Long ->
                if (name in DATE_COLUMNS) {
                    Text(formatDate(Date(value)), style = MaterialTheme.typography.bodyMedium)
                } else {
                    Text("$value", style = MaterialTheme.typography.bodyMedium)
                }
            else -> Text(value.toString(), style = MaterialTheme.typography.bodyMedium)
        }
    }
}
