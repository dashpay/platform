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
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.navigation.NavHostController
import org.dashfoundation.example.navigation.TransitionDetailRoute
import org.dashfoundation.example.services.transitions.StateTransitionDefinitions
import org.dashfoundation.example.services.transitions.TransitionCategory
import org.dashfoundation.example.ui.components.EntityRow

/**
 * The transitions in one category — port of `TransitionCategoryView.swift`:
 * a list of the category's [org.dashfoundation.example.services.transitions.TransitionDefinition]
 * rows, each drilling into [TransitionDetailScreen].
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TransitionCategoryScreen(categoryName: String, navController: NavHostController) {
    val category = remember(categoryName) {
        runCatching { TransitionCategory.valueOf(categoryName) }.getOrNull()
    }
    val definitions = remember(category) {
        category?.let { StateTransitionDefinitions.forCategory(it) } ?: emptyList()
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(category?.displayName ?: "Transitions") },
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
            items(definitions, key = { it.key }) { def ->
                EntityRow(
                    icon = Icons.Filled.SwapVert,
                    title = def.label,
                    subtitle = def.description,
                    onClick = { navController.navigate(TransitionDetailRoute(def.key)) },
                    modifier = Modifier.testTag("transition.${def.key}"),
                )
            }
        }
    }
}
