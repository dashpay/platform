package org.dashfoundation.example.ui.tokens

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.TokenDetail
import org.dashfoundation.example.util.toHex

/**
 * Token search — port of `TokenSearchView.swift`: free-text search over
 * name/description plus the capability filter chips; rows drill into
 * [TokenDetail]. Network-scoped through the same contract join the
 * tokens home uses (the iOS `@Query` is store-wide, but the Android
 * store is shared across networks, so scoping keeps parity with what
 * the user can actually act on).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TokenSearchScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()

    val tokensFlow = remember(network) {
        container.database.tokenDao().observeTokensByNetwork(network.ffiValue)
    }
    val tokens by tokensFlow.collectAsStateWithLifecycle(initialValue = emptyList())

    var searchText by rememberSaveable { mutableStateOf("") }
    var filter by rememberSaveable { mutableStateOf(TokenCapabilityFilter.ALL.name) }
    val selectedFilter = TokenCapabilityFilter.valueOf(filter)

    val filtered = tokens
        .filter(selectedFilter.predicate)
        .filter { token ->
            val query = searchText.trim()
            query.isEmpty() ||
                token.name.contains(query, ignoreCase = true) ||
                (token.tokenDescription ?: "").contains(query, ignoreCase = true)
        }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Token Search") },
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
                .padding(padding),
        ) {
            OutlinedTextField(
                value = searchText,
                onValueChange = { searchText = it },
                label = { Text("Search tokens…") },
                singleLine = true,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 8.dp)
                    .testTag("tokenSearch.field"),
            )

            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .horizontalScroll(rememberScrollState())
                    .padding(horizontal = 16.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                TokenCapabilityFilter.entries.forEach { candidate ->
                    FilterChip(
                        selected = selectedFilter == candidate,
                        onClick = { filter = candidate.name },
                        label = { Text(candidate.title) },
                        modifier = Modifier.testTag("tokenSearch.filter.${candidate.name}"),
                    )
                }
            }

            if (filtered.isEmpty()) {
                Column(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(24.dp),
                    verticalArrangement = Arrangement.spacedBy(
                        8.dp, Alignment.CenterVertically,
                    ),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Text("No tokens found", style = MaterialTheme.typography.titleMedium)
                    Text(
                        "Try adjusting your search or filters",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            } else {
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(16.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    items(filtered, key = { it.id.toHex() }) { token ->
                        TokenRowCard(token = token) {
                            navController.navigate(TokenDetail(token.id.toHex()))
                        }
                    }
                }
            }
        }
    }
}
