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
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.ContestDetail
import org.dashfoundation.example.navigation.DashPayHome
import org.dashfoundation.example.navigation.DpnsMarketplace
import org.dashfoundation.example.navigation.KeysList
import org.dashfoundation.example.navigation.RegisterName
import org.dashfoundation.example.navigation.SelectMainName
import org.dashfoundation.example.navigation.TopUpIdentity
import org.dashfoundation.example.navigation.TopUpIdentityFromCore
import org.dashfoundation.example.navigation.TransferCredits
import org.dashfoundation.example.navigation.TransferToAddress
import org.dashfoundation.example.navigation.WithdrawCredits
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity

internal sealed interface ContestedNamesOwnership {
    data class WalletOwned(val walletId: ByteArray) : ContestedNamesOwnership
    data object External : ContestedNamesOwnership
}

/** Durable ownership comes from the wallet FK; `isLocal` is presentation metadata only. */
internal fun contestedNamesOwnership(identity: IdentityEntity): ContestedNamesOwnership =
    identity.walletId?.let { ContestedNamesOwnership.WalletOwned(it.copyOf()) }
        ?: ContestedNamesOwnership.External

private sealed interface ContestedNamesState {
    data object Loading : ContestedNamesState
    data class Data(val labels: List<String>) : ContestedNamesState
    data class Error(val message: String) : ContestedNamesState
    data class Unavailable(val message: String) : ContestedNamesState
}

/**
 * One identity's detail — port of `IdentityDetailView.swift`: identity info,
 * balance + credit actions, DPNS names (settled rows plus contested-name
 * rows linking into [ContestDetailScreen], with Register / Select-Main
 * entries), DashPay (opens the DashPay tab), and the keys summary
 * (View All Keys).
 *
 * Contested-name rows come from one shared Rust by-identity sync followed by
 * its full-replacement managed-identity cache. There is no local-label probe
 * limit, so locally unknown names and identities with more than eight active
 * contests are complete.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun IdentityDetailScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }
    // The persister keys `public_keys.identityId` by base58; older callers
    // passed the hex nav-arg through — observe base58 and fall back to hex
    // so both encodings resolve (matches KeyDetailScreen's resolution).
    val idBase58 = remember(idBytes) { Base58.encode(idBytes) }

    val identity by container.database.identityDao()
        .observeByIdentityId(idBytes)
        .collectAsStateWithLifecycle(initialValue = null)
    val dpnsNames by container.database.dpnsNameDao()
        .observeByIdentity(idBytes)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val keysBase58 by container.database.publicKeyDao()
        .observeByIdentityId(idBase58)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val keysHex by container.database.publicKeyDao()
        .observeByIdentityId(identityIdHex)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val keys = if (keysBase58.isNotEmpty()) keysBase58 else keysHex

    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    var contestedNamesState by remember { mutableStateOf<ContestedNamesState>(ContestedNamesState.Loading) }
    var contestedNamesRetry by remember { mutableStateOf(0) }

    // Pull the identity's on-chain balance (and revision) and persist it —
    // port of `IdentityDetailView`'s toolbar refresh button. A credit
    // transfer credits the *recipient* identity, but nothing on this device
    // observes that until we re-fetch, so a loaded/received-into identity
    // otherwise shows a stale balance (e.g. ID-14 A→B). `updateBalance` is a
    // targeted UPDATE, so isLocal / alias / keys are preserved.
    val scope = rememberCoroutineScope()
    var isRefreshing by remember { mutableStateOf(false) }
    var refreshError by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(manager, identity, idBytes, contestedNamesRetry) {
        val currentManager = manager
        val currentIdentity = identity
        if (currentIdentity == null) {
            contestedNamesState = ContestedNamesState.Loading
            return@LaunchedEffect
        }
        if (currentManager == null) {
            contestedNamesState = ContestedNamesState.Unavailable(
                "The wallet manager is unavailable on the selected network.",
            )
            return@LaunchedEffect
        }
        val ownership = contestedNamesOwnership(currentIdentity)
        val walletId = when (ownership) {
            is ContestedNamesOwnership.WalletOwned -> ownership.walletId
            ContestedNamesOwnership.External -> {
                contestedNamesState = ContestedNamesState.Unavailable(
                    "Contested names are unavailable for an identity not owned by a local wallet.",
                )
                return@LaunchedEffect
            }
        }
        val wallet = currentManager.wallet(forWalletId = walletId)
        if (wallet == null) {
            contestedNamesState = ContestedNamesState.Unavailable(
                "This identity's wallet is not active on the selected network.",
            )
            return@LaunchedEffect
        }
        contestedNamesState = ContestedNamesState.Loading
        contestedNamesState = try {
            ContestedNamesState.Data(
                currentManager.identityRegistration.contestedDpnsNames(
                    walletHandle = wallet.handle,
                    identityId = idBytes,
                ).labels,
            )
        } catch (e: Exception) {
            ContestedNamesState.Error(e.message ?: "Failed to load contested names")
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Identity") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = {
                            val activeSdk = sdk ?: return@IconButton
                            isRefreshing = true
                            scope.launch {
                                try {
                                    val json = activeSdk.identities.fetch(identityIdHex)
                                    if (json != null) {
                                        val balance = runCatching {
                                            Json.parseToJsonElement(json).jsonObject["balance"]
                                                ?.jsonPrimitive?.longOrNull ?: 0L
                                        }.getOrDefault(0L)
                                        container.database.identityDao()
                                            .updateBalance(idBytes, balance, System.currentTimeMillis())
                                    }
                                } catch (e: Exception) {
                                    // A manual refresh failing (DAPI/network
                                    // outage, or a Room write error) must not
                                    // escape this rememberCoroutineScope launch
                                    // and tear down the screen — surface it and
                                    // keep the UI usable, like the other
                                    // SDK-backed identity actions.
                                    refreshError = e.message ?: "Failed to refresh identity"
                                } finally {
                                    isRefreshing = false
                                }
                            }
                        },
                        enabled = sdk != null && !isRefreshing,
                        modifier = Modifier.testTag("identityDetail.refresh"),
                    ) {
                        if (isRefreshing) {
                            CircularProgressIndicator(modifier = Modifier.padding(4.dp))
                        } else {
                            Icon(Icons.Filled.Refresh, contentDescription = "Refresh")
                        }
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
                    headlineContent = { Text("Top Up from Core") },
                    supportingContent = { Text("Build a new Core asset lock") },
                    modifier = Modifier
                        .clickable { navController.navigate(TopUpIdentityFromCore(identityIdHex)) }
                        .testTag("identityDetail.topUpFromCore"),
                )
                ListItem(
                    headlineContent = { Text("Top Up from Platform addresses") },
                    supportingContent = { Text("Spend funded Platform-payment addresses") },
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
                    headlineContent = { Text("Transfer to Platform Address") },
                    supportingContent = { Text("Send credits to a DIP-17 address") },
                    modifier = Modifier
                        .clickable { navController.navigate(TransferToAddress(identityIdHex)) }
                        .testTag("identityDetail.transferToAddress"),
                )
                ListItem(
                    headlineContent = { Text("Withdraw") },
                    modifier = Modifier
                        .clickable { navController.navigate(WithdrawCredits(identityIdHex)) }
                        .testTag("identityDetail.withdraw"),
                )
            }

            FormSection(title = "DPNS Names") {
                dpnsNames.forEach { name ->
                    val isMain = identity?.mainDpnsName == name.label
                    LabeledContent(
                        label = if (isMain) "${name.label} ★" else name.label,
                        value = name.parentDomainName,
                    )
                }
                when (val contested = contestedNamesState) {
                    ContestedNamesState.Loading -> {
                        CircularProgressIndicator(
                            modifier = Modifier.padding(8.dp).testTag("identityDetail.contested.loading"),
                        )
                    }
                    is ContestedNamesState.Data -> {
                        if (dpnsNames.isEmpty() && contested.labels.isEmpty()) {
                            Text(
                                "No names registered.",
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        // Contested labels — ⚑ rows drilling into the live
                        // contest (← IdentityDetailView's Contested rows).
                        contested.labels.forEach { label ->
                            ListItem(
                                headlineContent = { Text(label) },
                                supportingContent = {
                                    Text(
                                        "Contested — voting in progress",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.tertiary,
                                    )
                                },
                                modifier = Modifier
                                    .clickable {
                                        navController.navigate(
                                            ContestDetail(label, identityIdHex),
                                        )
                                    }
                                    .testTag("identityDetail.contested.$label"),
                            )
                        }
                    }
                    is ContestedNamesState.Error,
                    is ContestedNamesState.Unavailable,
                    -> {
                        val message = when (contested) {
                            is ContestedNamesState.Error -> contested.message
                            is ContestedNamesState.Unavailable -> contested.message
                            else -> error("unreachable contested-name state")
                        }
                        Text(
                            message,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.testTag("identityDetail.contested.error"),
                        )
                        TextButton(
                            onClick = { contestedNamesRetry += 1 },
                            modifier = Modifier.testTag("identityDetail.contested.retry"),
                        ) {
                            Text("Retry")
                        }
                    }
                }
                HorizontalDivider(Modifier.padding(vertical = 8.dp))
                ListItem(
                    headlineContent = { Text("DPNS Marketplace") },
                    supportingContent = { Text("Browse, buy, price, transfer, and view history") },
                    modifier = Modifier
                        .clickable { navController.navigate(DpnsMarketplace(identityIdHex)) }
                        .testTag("identityDetail.dpnsMarketplace"),
                )
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
                    headlineContent = { Text("DashPay") },
                    modifier = Modifier
                        .clickable { navController.navigate(DashPayHome) }
                        .testTag("identityDetail.dashpay"),
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

    ErrorAlertDialog(message = refreshError, onDismiss = { refreshError = null })
}
