package org.dashfoundation.example.ui.tokens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
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
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.TokenActionPermissions
import org.dashfoundation.example.navigation.TokenDetail
import org.dashfoundation.example.navigation.TokenSearch
import org.dashfoundation.example.services.tokens.TokenAmounts
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.truncateMiddle

/**
 * Tokens home — port of `TokensView.swift` (identity picker + that
 * identity's token balances, rows → the action-permissions screen) plus
 * the network-scoped token list with capability filter chips (the
 * `TokenSearchView` predicate pickers, surfaced here so all locally
 * known tokens are reachable even before any balance exists). Rows in
 * the network list drill into [TokenDetail].
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TokensScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()

    val identitiesFlow = remember(network) {
        container.database.identityDao().observeWalletOwnedByNetwork(network.ffiValue)
    }
    val identities by identitiesFlow.collectAsStateWithLifecycle(initialValue = emptyList())

    var selectedIdentityHex by rememberSaveable { mutableStateOf<String?>(null) }
    val selectedIdentity = identities.firstOrNull { it.identityId.toHex() == selectedIdentityHex }
        ?: identities.firstOrNull()

    val tokensFlow = remember(network) {
        container.database.tokenDao().observeTokensByNetwork(network.ffiValue)
    }
    val tokens by tokensFlow.collectAsStateWithLifecycle(initialValue = emptyList())

    var filter by rememberSaveable { mutableStateOf(TokenCapabilityFilter.ALL.name) }
    val selectedFilter = TokenCapabilityFilter.valueOf(filter)
    val filteredTokens = tokens.filter(selectedFilter.predicate)

    val balancesFlow = remember(selectedIdentity?.identityId?.toHex()) {
        selectedIdentity?.let {
            container.database.tokenDao().observeBalancesByIdentity(it.identityId)
        }
    }
    val balances by (balancesFlow ?: remember {
        kotlinx.coroutines.flow.flowOf(
            emptyList<org.dashfoundation.dashsdk.persistence.entities.TokenBalanceEntity>(),
        )
    }).collectAsStateWithLifecycle(initialValue = emptyList())

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Tokens") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = { navController.navigate(TokenSearch) },
                        modifier = Modifier.testTag("tokens.search"),
                    ) {
                        Icon(Icons.Default.Search, contentDescription = "Token Search")
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
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            // ── Identity + balances (← TokensView) ─────────────────────
            item {
                FormSection(title = "Select Identity") {
                    if (identities.isEmpty()) {
                        Text(
                            "Add identities in the Identities tab to use tokens",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(vertical = 8.dp),
                        )
                    } else {
                        AccessiblePicker(
                            label = "Identity",
                            options = identities,
                            selected = selectedIdentity ?: identities.first(),
                            optionLabel = { it.displayLabel() },
                            testTag = "tokens.identityPicker",
                            modifier = Modifier.padding(vertical = 8.dp),
                        ) { selectedIdentityHex = it.identityId.toHex() }
                    }
                }
            }

            if (selectedIdentity != null) {
                item {
                    Text(
                        "TOKEN BALANCES",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                if (balances.isEmpty()) {
                    item {
                        Text(
                            "No token balances yet",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                } else {
                    items(balances, key = { it.id }) { balance ->
                        val token = tokens.firstOrNull {
                            balance.tokenRef?.contentEquals(it.id) == true
                        }
                        Card(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable(enabled = token != null) {
                                    token ?: return@clickable
                                    navController.navigate(
                                        TokenActionPermissions(
                                            tokenIdHex = token.id.toHex(),
                                            identityIdHex = selectedIdentity.identityId.toHex(),
                                        ),
                                    )
                                }
                                .testTag("tokens.balanceRow.${balance.tokenId}"),
                        ) {
                            ListItem(
                                headlineContent = {
                                    Text(token?.name ?: balance.tokenName ?: "Token")
                                },
                                supportingContent = {
                                    Column {
                                        Text(
                                            "Balance: " + TokenAmounts.format(
                                                balance.balance.value,
                                                balance.tokenDecimals ?: token?.decimals ?: 8,
                                            ) + if (balance.frozen) " (frozen)" else "",
                                            style = MaterialTheme.typography.bodySmall,
                                        )
                                        token?.let {
                                            Text(
                                                it.supplyLine(),
                                                style = MaterialTheme.typography.bodySmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                            )
                                        }
                                    }
                                },
                                trailingContent = {
                                    balance.tokenSymbol?.let {
                                        Text(it, style = MaterialTheme.typography.bodySmall)
                                    }
                                },
                            )
                        }
                    }
                }
            }

            // ── Network token list + capability chips ──────────────────
            item {
                Column {
                    Text(
                        "ALL TOKENS ON ${network.displayName.uppercase()}",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .horizontalScroll(rememberScrollState())
                            .padding(top = 8.dp),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        TokenCapabilityFilter.entries.forEach { candidate ->
                            FilterChip(
                                selected = selectedFilter == candidate,
                                onClick = { filter = candidate.name },
                                label = { Text(candidate.title) },
                                modifier = Modifier.testTag("tokens.filter.${candidate.name}"),
                            )
                        }
                    }
                }
            }

            if (filteredTokens.isEmpty()) {
                item {
                    Text(
                        if (tokens.isEmpty()) {
                            "No tokens yet — fetch a token contract from the Contracts tab."
                        } else {
                            "No tokens match this filter."
                        },
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            } else {
                items(filteredTokens, key = { it.id.toHex() }) { token ->
                    TokenRowCard(token = token) {
                        navController.navigate(TokenDetail(token.id.toHex()))
                    }
                }
            }
        }
    }
}

/**
 * Capability filter chips — mirror of `TokenSearchView.TokenFilter`
 * (each case maps onto one persisted capability column / predicate).
 */
enum class TokenCapabilityFilter(
    val title: String,
    val predicate: (TokenEntity) -> Boolean,
) {
    ALL("All Tokens", { true }),
    MINTABLE("Can Mint", { it.canManuallyMint }),
    BURNABLE("Can Burn", { it.canManuallyBurn }),
    FREEZABLE("Can Freeze", { it.canFreeze }),
    HAS_DISTRIBUTION("Has Distribution", { it.hasDistribution }),
    PAUSED("Paused", { it.isPaused }),
}

/** One token row (name, contract position, supply line) → [onClick]. */
@Composable
fun TokenRowCard(token: TokenEntity, onClick: () -> Unit) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .testTag("tokens.row.${Base58.encode(token.contractId)}.${token.position}"),
    ) {
        ListItem(
            headlineContent = { Text(token.name) },
            supportingContent = {
                Column {
                    Text(
                        "Contract ${truncateMiddle(Base58.encode(token.contractId), 8, 6)} " +
                            "· position ${token.position}",
                        style = MaterialTheme.typography.bodySmall,
                        maxLines = 1,
                    )
                    Text(
                        token.supplyLine(),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            },
            trailingContent = {
                val badges = buildList {
                    if (token.canManuallyMint) add("mint")
                    if (token.canManuallyBurn) add("burn")
                    if (token.canFreeze) add("freeze")
                    if (token.hasDistribution) add("dist")
                    if (token.isPaused) add("paused")
                }
                if (badges.isNotEmpty()) {
                    Text(
                        badges.joinToString(" · "),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            },
        )
    }
}

/** "Max Supply: …" preferred over "Base Supply: …" (← `TokenRow.totalSupplyLine`). */
internal fun TokenEntity.supplyLine(): String =
    maxSupply?.takeIf { it.isNotEmpty() }
        ?.let { "Max Supply: ${TokenAmounts.format(it, decimals)}" }
        ?: "Base Supply: ${TokenAmounts.format(baseSupply, decimals)}"

/** DPNS name → alias → truncated base58 (the app's identity label chain). */
internal fun IdentityEntity.displayLabel(): String =
    mainDpnsName ?: dpnsName ?: alias ?: truncateMiddle(Base58.encode(identityId), 6, 4)
