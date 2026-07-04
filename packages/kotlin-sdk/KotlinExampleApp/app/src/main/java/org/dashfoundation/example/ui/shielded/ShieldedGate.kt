package org.dashfoundation.example.ui.shielded

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.navigation.NavHostController
import org.dashfoundation.dashsdk.Sdk

/**
 * Gate a shielded screen on [Sdk.hasShielded]. When the native library was
 * built without Orchard support, renders an explanatory placeholder instead
 * of the shielded [content] — the Kotlin analog of the Swift shielded views
 * being reachable only when the wallet has a bound shielded sub-wallet.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ShieldedGate(
    navController: NavHostController,
    content: @Composable () -> Unit,
) {
    val hasShielded by produceState(initialValue = true) {
        value = runCatching { Sdk.hasShielded() }.getOrDefault(false)
    }
    if (hasShielded) {
        content()
    } else {
        Scaffold(
            topBar = {
                TopAppBar(
                    title = { Text("Shielded") },
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
                    .padding(24.dp)
                    .testTag("shielded.unavailable"),
                verticalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterVertically),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text("Shielded Not Available", style = MaterialTheme.typography.titleMedium)
                Text(
                    "This build of the native library was compiled without Orchard " +
                        "shielded support.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
