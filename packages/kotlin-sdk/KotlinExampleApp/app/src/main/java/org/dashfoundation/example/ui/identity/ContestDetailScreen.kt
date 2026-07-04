package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.navigation.NavHostController
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.truncateMiddle

/**
 * DPNS contest detail — port of `ContestDetailView.swift`. The iOS view
 * loads live contest state via
 * `ManagedPlatformWallet.fetchContestVoteState`
 * (`platform_wallet_fetch_contest_vote_state`) and casts votes via
 * `dash_sdk_contested_resource_cast_vote`; neither is bridged into
 * the JNI shim yet, so this port renders the static contest shape (the
 * same `(contract, document type, index, index values)` tuple the iOS
 * read path uses) and the About explainer, while Refresh surfaces the
 * named-missing-export dialog.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ContestDetailScreen(
    contestName: String,
    identityIdHex: String,
    navController: NavHostController,
) {
    var notBridged by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Contest Details") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = { notBridged = true },
                        modifier = Modifier.testTag("contestDetail.refresh"),
                    ) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Contest") {
                LabeledContent("Name", "$contestName.dash")
                LabeledContent("Document Type", DPNS_DOCUMENT_TYPE)
                LabeledContent("Index", DPNS_INDEX_NAME)
                LabeledContent(
                    "Contract",
                    truncateMiddle(DPNS_CONTRACT_ID_BASE58, 10, 6),
                )
                if (identityIdHex.isNotEmpty()) {
                    LabeledContent(
                        "Viewing As",
                        truncateMiddle(identityIdHex, 10, 6),
                    )
                }
            }

            FormSection(title = "Contenders") {
                Text(
                    "Live contender tallies require the contest vote-state " +
                        "read — tap Refresh for details.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.testTag("contestDetail.contendersPlaceholder"),
                )
            }

            FormSection(title = "About Contested Names") {
                Text(
                    "When multiple identities want the same DPNS username, " +
                        "masternodes vote to decide the winner. The identity with " +
                        "the most votes will be awarded the name when voting ends.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            FormSection(title = "Note") {
                Text(
                    "Pending JNI exports: platform_wallet_fetch_contest_vote_state " +
                        "(read), dash_sdk_contested_resource_cast_vote (cast).",
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }

    if (notBridged) {
        AlertDialog(
            onDismissRequest = { notBridged = false },
            title = { Text("Contest State Not Available Yet") },
            text = {
                Text(
                    "Loading contest contenders and vote tallies requires the " +
                        "`platform_wallet_fetch_contest_vote_state` FFI " +
                        "(platform-wallet-ffi) to be bridged into the JNI shim; " +
                        "vote casting additionally needs " +
                        "`dash_sdk_contested_resource_cast_vote`. The " +
                        "screen shape and navigation are wired for when they land.",
                )
            },
            confirmButton = { TextButton(onClick = { notBridged = false }) { Text("OK") } },
        )
    }
}

/**
 * DPNS contest poll shape — the constants `ContestDetailView.swift`
 * hard-codes for the `(contract, document type, index)` tuple.
 */
private const val DPNS_CONTRACT_ID_BASE58 = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
private const val DPNS_DOCUMENT_TYPE = "domain"
private const val DPNS_INDEX_NAME = "parentNameAndLabel"
