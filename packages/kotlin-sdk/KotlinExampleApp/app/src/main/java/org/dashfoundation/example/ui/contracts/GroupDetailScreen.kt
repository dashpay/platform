package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
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
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.IdentityDetail
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.prettyPrintJson
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.truncateMiddle

/**
 * One contract group's definition — port of `GroupDetailView.swift`:
 * required power + member rows resolved against the local identity store
 * (rows whose member id matches a persisted identity link into
 * [IdentityDetail], mirroring the iOS `@Query`-driven `GroupMemberRow`).
 *
 * Beyond the iOS static view, the bridged `Groups.pendingActions` read
 * (`platform_wallet_token_pending_group_actions`) powers an on-demand
 * "Open Proposals" section when the active manager holds a wallet.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GroupDetailScreen(
    contractIdHex: String,
    position: Int,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val scope = androidx.compose.runtime.rememberCoroutineScope()
    val contractId = remember(contractIdHex) { contractIdHex.hexToBytes() }

    val contract by remember(contractIdHex) {
        container.database.dataContractDao().observeById(contractId)
    }.collectAsStateWithLifecycle(initialValue = null)

    // Local identities, for the "in local store" member resolution
    // (← the targeted `@Query` in the iOS `GroupMemberRow`).
    val identities by remember {
        container.database.identityDao().observeAll()
    }.collectAsStateWithLifecycle(initialValue = emptyList())

    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsFlow = remember(manager) {
        manager?.wallets
            ?: kotlinx.coroutines.flow.MutableStateFlow<
                Map<String, org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet>,
                >(emptyMap())
    }
    val wallets by walletsFlow.collectAsStateWithLifecycle()

    var proposalsJson by remember { mutableStateOf<String?>(null) }
    var proposalsError by remember { mutableStateOf<String?>(null) }
    var isLoadingProposals by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Group $position") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        val current = contract ?: return@Scaffold
        val parsed = remember(current.lastUpdated, current.id) { ParsedContract.from(current) }
        val group = parsed?.groups?.get(position.toString())
        val members: Map<String, Int> = remember(group) {
            (group?.objectField("members") ?: JsonObject(emptyMap()))
                .mapValues { (_, v) -> v.jsonPrimitive.content.toIntOrNull() ?: 0 }
        }
        val requiredPower = group?.intField("requiredPower") ?: 0

        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Group $position") {
                LabeledContent("Contract", truncateMiddle(Base58.encode(current.id), 10, 6))
                LabeledContent("Required Power", "$requiredPower")
                LabeledContent("Members", "${members.size}")
            }

            FormSection(title = "Members") {
                if (members.isEmpty()) {
                    Text(
                        "No members declared at this position.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                members.keys.sorted().forEach { memberBase58 ->
                    val power = members.getValue(memberBase58)
                    val memberBytes = Base58.decodeIdentifier(memberBase58)
                    val local = memberBytes?.let { bytes ->
                        identities.firstOrNull { it.identityId.contentEquals(bytes) }
                    }
                    ListItem(
                        headlineContent = {
                            Text(
                                local?.mainDpnsName ?: local?.alias
                                    ?: truncateMiddle(memberBase58, 8, 4),
                            )
                        },
                        supportingContent = {
                            Text(
                                if (local != null) {
                                    truncateMiddle(memberBase58, 8, 4)
                                } else {
                                    "Not in local store"
                                },
                                style = MaterialTheme.typography.bodySmall,
                            )
                        },
                        trailingContent = { Text("Power $power") },
                        modifier = Modifier
                            .let { base ->
                                if (local != null && memberBytes != null) {
                                    base.clickable {
                                        navController.navigate(
                                            IdentityDetail(memberBytes.toHex()),
                                        )
                                    }
                                } else {
                                    base
                                }
                            }
                            .testTag("groupDetail.member.$memberBase58"),
                    )
                }
            }

            FormSection(title = "Open Proposals") {
                Text(
                    "Query active group-action proposals at this position " +
                        "(platform_wallet_token_pending_group_actions).",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                val wallet = wallets.values.firstOrNull()
                SubmitButton(
                    text = "Load Pending Actions",
                    isLoading = isLoadingProposals,
                    enabled = wallet != null,
                    modifier = Modifier.testTag("groupDetail.loadProposals"),
                ) {
                    val currentWallet = wallet ?: return@SubmitButton
                    scope.launch {
                        isLoadingProposals = true
                        proposalsError = null
                        try {
                            val json = currentWallet.groups.pendingActions(
                                tokenContractId = current.id,
                                groupContractPosition = position,
                            )
                            proposalsJson = json?.let { prettyPrintJson(it) } ?: "[]"
                        } catch (e: Exception) {
                            proposalsError = e.message ?: e.toString()
                        } finally {
                            isLoadingProposals = false
                        }
                    }
                }
                if (wallet == null) {
                    Text(
                        "Requires an active wallet (proposal discovery runs on a " +
                            "wallet handle).",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                proposalsError?.let { message ->
                    Text(
                        message,
                        color = MaterialTheme.colorScheme.error,
                        style = MaterialTheme.typography.bodySmall,
                        modifier = Modifier.testTag("groupDetail.proposalsError"),
                    )
                }
                proposalsJson?.let { text ->
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .horizontalScroll(rememberScrollState()),
                    ) {
                        Text(
                            text,
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace,
                            modifier = Modifier.testTag("groupDetail.proposalsJson"),
                        )
                    }
                }
            }
        }
    }
}
