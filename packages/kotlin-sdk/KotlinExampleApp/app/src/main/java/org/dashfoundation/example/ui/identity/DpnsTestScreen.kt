package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.delay
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.FormSection

/**
 * DPNS availability tester — port of `DPNSTestView.swift`. Debounced
 * `sdk.dpns.checkAvailability` on the entered label, raw JSON shown.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DpnsTestScreen(navController: NavHostController) {
    val appState = LocalAppState.current
    val sdk by appState.sdk.collectAsStateWithLifecycle()

    var label by remember { mutableStateOf("") }
    var result by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(label) {
        result = null
        if (label.isBlank()) return@LaunchedEffect
        delay(300)
        val activeSdk = sdk ?: return@LaunchedEffect
        result = runCatching { activeSdk.dpns.checkAvailability(label.trim()) ?: "null" }
            .getOrElse { "Error: ${it.message}" }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("DPNS Test") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Availability") {
                OutlinedTextField(
                    value = label,
                    onValueChange = { label = it.trim() },
                    label = { Text("Label") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().testTag("dpnsTest.label"),
                )
                result?.let {
                    Text(
                        it,
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                        modifier = Modifier.padding(top = 8.dp).testTag("dpnsTest.result"),
                    )
                }
            }
        }
    }
}
