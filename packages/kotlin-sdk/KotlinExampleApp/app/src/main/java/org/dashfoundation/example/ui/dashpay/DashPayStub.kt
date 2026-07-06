package org.dashfoundation.example.ui.dashpay

import androidx.compose.foundation.layout.Box
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.navigation.NavHostController

/**
 * Shared placeholder scaffold for the DashPay screens still owned by K3
 * slice C (ContactDetail / Profile / Ignored / Hidden). Gives each a titled
 * bar + back button + a root testTag so navigation from the hub / Contacts
 * works end-to-end before those screens land.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun DashPayStubScaffold(
    title: String,
    rootTestTag: String,
    navController: NavHostController,
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(title) },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Box(
            modifier = Modifier.fillMaxSize().padding(padding).testTag(rootTestTag),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                "Coming in the next milestone.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
