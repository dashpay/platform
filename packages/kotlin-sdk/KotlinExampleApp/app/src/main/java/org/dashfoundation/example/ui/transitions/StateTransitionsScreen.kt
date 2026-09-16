package org.dashfoundation.example.ui.transitions

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.SwapVert
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.navigation.NavHostController
import org.dashfoundation.example.navigation.TransitionCategoryRoute
import org.dashfoundation.example.services.transitions.TransitionCategory
import org.dashfoundation.example.ui.components.EntityRow

/**
 * Root of the state-transition catalog — port of `StateTransitionsView.swift`
 * / `PlatformStateTransitionsView.swift`: the six [TransitionCategory] rows,
 * each drilling into [TransitionCategoryScreen].
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun StateTransitionsScreen(navController: NavHostController) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("State Transitions") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            items(TransitionCategory.entries, key = { it.name }) { category ->
                EntityRow(
                    icon = Icons.Filled.SwapVert,
                    title = category.displayName,
                    subtitle = category.description,
                    onClick = { navController.navigate(TransitionCategoryRoute(category.name)) },
                    modifier = Modifier.testTag("transitions.category.${category.name}"),
                )
            }
        }
    }
}
