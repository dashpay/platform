package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Person
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.flowOf
import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.CreateIdentity
import org.dashfoundation.example.navigation.IdentityDetail
import org.dashfoundation.example.navigation.LoadIdentity
import org.dashfoundation.example.navigation.SearchWalletsForIdentities
import org.dashfoundation.example.navigation.StateTransitions
import org.dashfoundation.example.ui.components.EntityRow
import org.dashfoundation.example.ui.components.SectionHeader
import org.dashfoundation.example.ui.funding.PendingAssetLocksList
import org.dashfoundation.example.ui.wallet.toHexString
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * Identities tab root — port of `IdentitiesContentView.swift`: a
 * pending-registrations section (live controllers from the coordinator), the
 * network-scoped identity list (Room), and a toolbar menu for Create / Load /
 * Search. The resumable-asset-lock section is deferred with the asset-lock
 * funding path (see B-M3 deferrals).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun IdentitiesHomeScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val coordinator = container.registrationCoordinator
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    val identitiesFlow = remember(network) {
        container.database.identityDao().observeByNetwork(network.ffiValue)
    }
    val identities by identitiesFlow.collectAsStateWithLifecycle(initialValue = emptyList())
    val controllers by coordinator.controllers.collectAsStateWithLifecycle()
    val addressFundControllers by container.addressFundCoordinator.controllers
        .collectAsStateWithLifecycle()
    var menuExpanded by remember { mutableStateOf(false) }

    // DB-backed resumable orphan asset locks (ADDR-03), merged into the
    // "Pending Platform Top Ups" surface below alongside the in-flight
    // controllers. The Identities tab is network-scoped (not wallet-scoped),
    // so we observe resumable address-topup locks across EVERY loaded wallet
    // and tag each with its owning wallet id for the Resume navigation.
    // ← the DB-backed orphan half of `PendingPlatformFundFromAssetLocksList.swift`.
    val loadedWallets by (manager?.wallets
        ?: MutableStateFlow(emptyMap())).collectAsStateWithLifecycle()
    val walletIdHexes = loadedWallets.keys.sorted()
    val resumableLocks by remember(walletIdHexes) {
        if (walletIdHexes.isEmpty()) {
            flowOf(emptyList<Pair<String, AssetLockEntity>>())
        } else {
            val flows = walletIdHexes.map { hex ->
                val walletId = hex.hexToBytes()
                container.database.assetLockDao().observeResumableAddressTopUps(walletId)
            }
            combine(flows) { arrays ->
                walletIdHexes.zip(arrays.toList()).flatMap { (hex, locks) ->
                    locks.map { hex to it }
                }
            }
        }
    }.collectAsStateWithLifecycle(initialValue = emptyList())

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Identities") },
                actions = {
                    IconButton(
                        onClick = { menuExpanded = true },
                        modifier = Modifier.testTag("identities.addMenu"),
                    ) {
                        Icon(Icons.Default.Add, contentDescription = "Identity actions")
                    }
                    DropdownMenu(expanded = menuExpanded, onDismissRequest = { menuExpanded = false }) {
                        DropdownMenuItem(
                            text = { Text("Create Identity") },
                            modifier = Modifier.testTag("identities.createIdentity"),
                            onClick = {
                                menuExpanded = false
                                navController.navigate(CreateIdentity)
                            },
                        )
                        DropdownMenuItem(
                            text = { Text("Load Identity") },
                            modifier = Modifier.testTag("identities.loadIdentity"),
                            onClick = {
                                menuExpanded = false
                                navController.navigate(LoadIdentity)
                            },
                        )
                        DropdownMenuItem(
                            text = { Text("Search Wallets for Identities") },
                            modifier = Modifier.testTag("identities.searchWallets"),
                            onClick = {
                                menuExpanded = false
                                navController.navigate(SearchWalletsForIdentities)
                            },
                        )
                        DropdownMenuItem(
                            text = { Text("State Transitions") },
                            modifier = Modifier.testTag("identities.stateTransitions"),
                            onClick = {
                                menuExpanded = false
                                navController.navigate(StateTransitions)
                            },
                        )
                    }
                },
            )
        },
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            val active = coordinator.activeControllers()
            if (active.isNotEmpty()) {
                item { SectionHeader("Pending Registrations") }
                items(active, key = { it.slotRowId }) { controller ->
                    PendingRegistrationRow(controller, navController)
                }
            }

            val activeFundings = container.addressFundCoordinator.activeControllers()
            if (activeFundings.isNotEmpty() || resumableLocks.isNotEmpty()) {
                item {
                    PendingAssetLocksList(
                        controllers = activeFundings,
                        navController = navController,
                        resumableLocks = resumableLocks,
                        // Per-wallet double-consume guard: an in-flight funding on
                        // a wallet might be driving one of that wallet's orphan
                        // outpoints, so suppress Resume on every orphan of a wallet
                        // that has any InFlight controller. ← Swift hasActiveFunding.
                        hasActiveFundingFor = { walletIdHex ->
                            activeFundings.any {
                                it.walletId.toHex() == walletIdHex && it.phase.value.isActive
                            }
                        },
                    )
                }
            }

            if (identities.isEmpty() && active.isEmpty()) {
                item {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(top = 48.dp, start = 24.dp, end = 24.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        Box(
                            modifier = Modifier
                                .size(72.dp)
                                .clip(CircleShape)
                                .background(MaterialTheme.colorScheme.primaryContainer),
                            contentAlignment = Alignment.Center,
                        ) {
                            Icon(
                                Icons.Filled.Person,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.onPrimaryContainer,
                                modifier = Modifier.size(36.dp),
                            )
                        }
                        Text("No Identities", style = MaterialTheme.typography.titleLarge)
                        Text(
                            "Register or load an identity on ${network.displayName}.",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            } else {
                if (identities.isNotEmpty()) {
                    item { SectionHeader("Identities") }
                }
                items(identities, key = { it.identityId.toHexString() }) { identity ->
                    EntityRow(
                        icon = Icons.Filled.Person,
                        title = identity.mainDpnsName
                            ?: identity.alias
                            ?: identity.identityId.toHexString().take(16),
                        subtitle = if (identity.walletId != null) {
                            "${identity.balance} credits"
                        } else {
                            "${identity.balance} credits · loaded"
                        },
                        onClick = {
                            navController.navigate(IdentityDetail(identity.identityId.toHexString()))
                        },
                        modifier = Modifier.testTag(
                            "identities.row.${identity.identityId.toHexString()}",
                        ),
                    )
                }
            }
        }
    }
}
