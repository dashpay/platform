package org.dashfoundation.example.ui.storage

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
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
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.StorageRecordDetail
import org.dashfoundation.example.ui.components.ErrorAlertDialog

/**
 * Generic table browser — port of the per-model list views in
 * `StorageModelListViews.swift`. One LazyColumn renders any table through
 * the [StorageModel] registry's headline/subtitle renderers; rows drill
 * into the generic record detail. Network-scoped tables filter to the
 * active network like their iOS counterparts.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun StorageModelListScreen(
    modelName: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()

    val model = remember(modelName) { storageModel(modelName) }

    var rows by remember { mutableStateOf<List<StorageRow>>(emptyList()) }
    var isLoading by remember { mutableStateOf(true) }
    var errorMessage by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(modelName, network) {
        val current = model ?: return@LaunchedEffect
        isLoading = true
        try {
            rows = loadStorageRows(container.database, current, network)
        } catch (e: Exception) {
            errorMessage = e.message ?: "Failed to load records"
            rows = emptyList()
        } finally {
            isLoading = false
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("${model?.displayName ?: modelName} (${rows.size})") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        val current = model
        if (current == null) {
            Text(
                "Unknown model: $modelName",
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(padding).padding(16.dp),
            )
            return@Scaffold
        }

        when {
            isLoading -> Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentAlignment = Alignment.Center,
            ) {
                CircularProgressIndicator()
            }
            rows.isEmpty() -> Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterVertically),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text("No Records", style = MaterialTheme.typography.titleMedium)
                Text(
                    "Nothing stored in ${current.displayName} on ${network.displayName} yet.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            else -> LazyColumn(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentPadding = PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                itemsIndexed(rows, key = { _, row -> row.rowId }) { index, row ->
                    Card(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable {
                                navController.navigate(
                                    StorageRecordDetail(current.name, row.rowId),
                                )
                            }
                            .testTag("storage.${current.name}.row.$index"),
                    ) {
                        ListItem(
                            headlineContent = { Text(current.headline(row), maxLines = 1) },
                            supportingContent = {
                                current.subtitle(row)?.let {
                                    Text(
                                        it,
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        maxLines = 2,
                                    )
                                }
                            },
                        )
                    }
                }
            }
        }
    }

    ErrorAlertDialog(message = errorMessage, onDismiss = { errorMessage = null })
}
