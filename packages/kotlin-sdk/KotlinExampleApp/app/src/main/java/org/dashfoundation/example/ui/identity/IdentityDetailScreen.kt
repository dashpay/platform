package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.Friends
import org.dashfoundation.example.navigation.KeysList
import org.dashfoundation.example.navigation.RegisterName
import org.dashfoundation.example.navigation.SelectMainName
import org.dashfoundation.example.navigation.TopUpIdentity
import org.dashfoundation.example.navigation.TransferCredits
import org.dashfoundation.example.navigation.WithdrawCredits
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.hexToBytes

/**
 * One identity's detail — port of `IdentityDetailView.swift`: identity info,
 * balance + credit actions, DPNS names (with Register / Select-Main entries),
 * DashPay (Friends entry), and the keys summary (View All Keys). The credit
 * actions (Top Up / Transfer / Withdraw) route to their B-M6 credits screens
 * — Transfer / Withdraw run the bridged FFI directly; Top Up is form-wired
 * pending the funding-input accessor.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun IdentityDetailScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }
    val idBase58 = identityIdHex // publicKeyDao keys on the Swift storage id string

    val identity by container.database.identityDao()
        .observeByIdentityId(idBytes)
        .collectAsStateWithLifecycle(initialValue = null)
    val dpnsNames by container.database.dpnsNameDao()
        .observeByIdentity(idBytes)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val keys by container.database.publicKeyDao()
        .observeByIdentityId(idBase58)
        .collectAsStateWithLifecycle(initialValue = emptyList())

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Identity") },
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
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Identity") {
                LabeledContent("Name", identity?.mainDpnsName ?: identity?.alias ?: "—")
                LabeledContent("Type", identity?.identityType ?: "User")
                Text(
                    identityIdHex,
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.padding(top = 4.dp).testTag("identityDetail.idHex"),
                )
                if (identity?.isLocal == false) {
                    Text(
                        "Loaded (read-only)",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            FormSection(title = "Balance") {
                LabeledContent("Credits", "${identity?.balance ?: 0}")
                HorizontalDivider(Modifier.padding(vertical = 8.dp))
                ListItem(
                    headlineContent = { Text("Top Up") },
                    modifier = Modifier
                        .clickable { navController.navigate(TopUpIdentity(identityIdHex)) }
                        .testTag("identityDetail.topUp"),
                )
                ListItem(
                    headlineContent = { Text("Transfer") },
                    modifier = Modifier
                        .clickable { navController.navigate(TransferCredits(identityIdHex)) }
                        .testTag("identityDetail.transfer"),
                )
                ListItem(
                    headlineContent = { Text("Withdraw") },
                    modifier = Modifier
                        .clickable { navController.navigate(WithdrawCredits(identityIdHex)) }
                        .testTag("identityDetail.withdraw"),
                )
            }

            FormSection(title = "DPNS Names") {
                if (dpnsNames.isEmpty()) {
                    Text(
                        "No names registered.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    dpnsNames.forEach { name ->
                        val isMain = identity?.mainDpnsName == name.label
                        LabeledContent(
                            label = if (isMain) "${name.label} ★" else name.label,
                            value = name.parentDomainName,
                        )
                    }
                }
                HorizontalDivider(Modifier.padding(vertical = 8.dp))
                ListItem(
                    headlineContent = { Text("Register Name") },
                    modifier = Modifier
                        .clickable { navController.navigate(RegisterName(identityIdHex)) }
                        .testTag("identityDetail.registerName"),
                )
                ListItem(
                    headlineContent = { Text("Select Main Name") },
                    modifier = Modifier
                        .clickable { navController.navigate(SelectMainName(identityIdHex)) }
                        .testTag("identityDetail.selectMainName"),
                )
            }

            FormSection(title = "DashPay") {
                ListItem(
                    headlineContent = { Text("Friends") },
                    modifier = Modifier
                        .clickable { navController.navigate(Friends(identityIdHex)) }
                        .testTag("identityDetail.friends"),
                )
            }

            FormSection(title = "Keys") {
                LabeledContent("Public keys", "${keys.size}")
                ListItem(
                    headlineContent = { Text("View All Keys") },
                    modifier = Modifier
                        .clickable { navController.navigate(KeysList(identityIdHex)) }
                        .testTag("identityDetail.viewKeys"),
                )
            }
        }
    }
}
