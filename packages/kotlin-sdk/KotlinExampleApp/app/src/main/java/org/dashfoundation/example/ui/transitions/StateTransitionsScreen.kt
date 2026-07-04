package org.dashfoundation.example.ui.transitions

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
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
        ) {
            items(TransitionCategory.entries, key = { it.name }) { category ->
                Card(
                    modifier = Modifier
                        .padding(vertical = 4.dp)
                        .testTag("transitions.category.${category.name}")
                        .clickable { navController.navigate(TransitionCategoryRoute(category.name)) },
                ) {
                    ListItem(
                        headlineContent = { Text(category.displayName) },
                        supportingContent = { Text(category.description) },
                    )
                }
            }
        }
    }
}
