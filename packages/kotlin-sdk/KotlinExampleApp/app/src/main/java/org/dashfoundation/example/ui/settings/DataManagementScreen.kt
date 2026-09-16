package org.dashfoundation.example.ui.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.services.DataManager
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent

/**
 * Data management — port of `DataManagementView` (in OptionsView.swift):
 * per-category row counts with confirm-gated clear actions, plus the
 * destructive clear-all.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DataManagementScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val dataManager = remember { DataManager(container.database) }
    val scope = rememberCoroutineScope()

    var pendingClear by remember { mutableStateOf<DataManager.Category?>(null) }
    var pendingClearAll by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            androidx.compose.material3.TopAppBar(
                title = { Text("Data Management") },
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
            FormSection(title = "Stored Data") {
                DataManager.Category.entries.forEach { category ->
                    val count by dataManager.count(category)
                        .collectAsStateWithLifecycle(initialValue = 0L)
                    androidx.compose.foundation.layout.Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Column(Modifier.padding(vertical = 6.dp)) {
                            Text(category.displayName)
                            Text(
                                "$count records",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        TextButton(
                            onClick = { pendingClear = category },
                            enabled = count > 0,
                        ) {
                            Text("Clear", color = MaterialTheme.colorScheme.error)
                        }
                    }
                }
            }

            Button(
                onClick = { pendingClearAll = true },
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.error,
                ),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text("Clear All Data")
            }
        }
    }

    pendingClear?.let { category ->
        AlertDialog(
            onDismissRequest = { pendingClear = null },
            title = { Text("Clear ${category.displayName}?") },
            text = { Text("This permanently deletes the locally stored records. Data on the network is unaffected.") },
            confirmButton = {
                TextButton(onClick = {
                    scope.launch { dataManager.clear(category) }
                    pendingClear = null
                }) { Text("Clear", color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = {
                TextButton(onClick = { pendingClear = null }) { Text("Cancel") }
            },
        )
    }

    if (pendingClearAll) {
        AlertDialog(
            onDismissRequest = { pendingClearAll = false },
            title = { Text("Clear ALL local data?") },
            text = { Text("Every locally stored wallet, identity, contract, and token record will be deleted. Mnemonics in secure storage are kept.") },
            confirmButton = {
                TextButton(onClick = {
                    scope.launch { dataManager.clearAll() }
                    pendingClearAll = false
                }) { Text("Clear Everything", color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = {
                TextButton(onClick = { pendingClearAll = false }) { Text("Cancel") }
            },
        )
    }
}
