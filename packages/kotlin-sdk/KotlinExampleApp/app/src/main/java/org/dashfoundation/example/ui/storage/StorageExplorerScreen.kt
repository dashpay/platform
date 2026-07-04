package org.dashfoundation.example.ui.storage

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.StorageModelList

/**
 * Storage Explorer — port of `StorageExplorerView.swift`: one row per
 * persisted model with a live record count, drilling into the generic
 * model list. Counts come from [org.dashfoundation.dashsdk.persistence.dao.StorageCountsDao]
 * and are whole-table (the iOS variant re-filters counts to the active
 * network; the drilled-in lists here apply the network scope instead).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun StorageExplorerScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Storage Explorer") },
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
            contentPadding = PaddingValues(vertical = 8.dp),
        ) {
            item {
                ListItem(
                    headlineContent = { Text("Network") },
                    trailingContent = {
                        Text(
                            network.displayName,
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    },
                )
                HorizontalDivider()
            }
            items(STORAGE_MODELS, key = { it.name }) { model ->
                val countFlow = remember(model.name) {
                    model.countFlow(container.database.storageCountsDao())
                }
                val count by countFlow.collectAsStateWithLifecycle(initialValue = 0L)
                ListItem(
                    headlineContent = { Text(model.displayName) },
                    trailingContent = {
                        Text(
                            "$count",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    },
                    modifier = Modifier
                        .clickable {
                            navController.navigate(StorageModelList(model.name))
                        }
                        .testTag("storage.model.${model.name}"),
                )
            }
        }
    }
}
