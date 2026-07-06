package org.dashfoundation.example.ui.dashpay

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
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
 * DashPay tab root — placeholder for the port of `DashPayTabView.swift`.
 *
 * The DASHPAY tab is first-class as of this milestone; the contacts /
 * requests / profile hub replaces this empty state in the next slice. The
 * [navController] is accepted now so the hub's child navigation attaches
 * without a route re-registration.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DashPayTabScreen(navController: NavHostController) {
    Scaffold(
        topBar = { TopAppBar(title = { Text("DashPay") }) },
    ) { padding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .testTag("dashpay.tab.root"),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                "DashPay contacts arrive in the next milestone.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
