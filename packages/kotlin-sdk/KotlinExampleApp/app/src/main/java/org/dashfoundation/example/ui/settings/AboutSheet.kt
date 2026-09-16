package org.dashfoundation.example.ui.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.AccountCircle
import androidx.compose.material.icons.filled.Description
import androidx.compose.material.icons.filled.Language
import androidx.compose.material.icons.filled.Paid
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.example.BuildConfig
import org.dashfoundation.example.ui.components.LabeledContent

/**
 * About sheet — port of `AboutView` in `OptionsView.swift` (the iOS
 * `.sheet(isPresented: $showingAbout)`), rendered as a Material
 * [ModalBottomSheet]. Version and commit come from [BuildConfig]
 * (`GIT_COMMIT` is stamped by the app's Gradle script at configuration
 * time).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AboutSheet(onDismiss: () -> Unit) {
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        modifier = Modifier.testTag("about.sheet"),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 24.dp)
                .padding(bottom = 32.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                "Dash SDK Example",
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold,
            )
            Text(
                "A demonstration app showcasing the capabilities of the Dash " +
                    "Platform SDK for Android.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            Column(
                modifier = Modifier.fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                FeatureRow(
                    icon = Icons.Default.AccountCircle,
                    title = "Identity Management",
                    description = "Create and manage Dash Platform identities",
                )
                FeatureRow(
                    icon = Icons.Default.Description,
                    title = "Document Storage",
                    description = "Store and retrieve documents on the platform",
                )
                FeatureRow(
                    icon = Icons.Default.Paid,
                    title = "Token Support",
                    description = "Manage tokens and token balances",
                )
                FeatureRow(
                    icon = Icons.Default.Language,
                    title = "Multi-Network",
                    description = "Switch between mainnet, testnet, and devnet",
                )
            }

            Column(modifier = Modifier.fillMaxWidth()) {
                LabeledContent("App Version", BuildConfig.VERSION_NAME)
                LabeledContent("SDK Version", Sdk.version())
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text("Commit", color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Text(
                        BuildConfig.GIT_COMMIT,
                        fontFamily = FontFamily.Monospace,
                        modifier = Modifier.testTag("about.commit"),
                    )
                }
            }
        }
    }
}

@Composable
private fun FeatureRow(icon: ImageVector, title: String, description: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Icon(
            icon,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.primary,
        )
        Column {
            Text(title, style = MaterialTheme.typography.titleSmall)
            Text(
                description,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
