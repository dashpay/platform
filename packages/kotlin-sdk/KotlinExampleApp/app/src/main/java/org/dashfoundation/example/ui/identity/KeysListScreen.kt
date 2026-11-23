package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
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
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.AddIdentityKey
import org.dashfoundation.example.navigation.KeyDetail
import org.dashfoundation.example.navigation.UpdateKeyLimits
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.KeyLimitsFormatting
import org.dashfoundation.example.util.hexToBytes

/**
 * All public keys of an identity — port of `KeysListView.swift`. Rows from
 * the Room [PublicKeyDao]; tapping opens [KeyDetailScreen]. The toolbar's
 * Add action opens [AddIdentityKeyScreen] (← the `AddIdentityKeyView`
 * sheet); disabling lives on [KeyDetailScreen], gated by
 * [org.dashfoundation.example.services.KeyDisableGate].
 *
 * A key with a budget or an expiry shows them on its row (what is left of the budget only
 * next to the total it was read for, from one remaining-budgets read for every budgeted
 * key), and a **Limits** action (`keyLimits.update.swipe.<keyId>`, Swift's leading-edge
 * swipe in `KeysListView.swift`) that opens [UpdateKeyLimitsScreen].
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun KeysListScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    // Identity-creation persists public keys keyed by the Base58 identity id
    // (ID-08 path), while other rows may be keyed by hex. Mirror
    // IdentityDetailScreen / KeyDetailScreen's dual lookup (Base58-first, hex
    // fallback) so the list isn't empty for Base58-keyed identities.
    val idBase58 = remember(identityIdHex) { Base58.encode(identityIdHex.hexToBytes()) }
    val keysBase58 by container.database.publicKeyDao()
        .observeByIdentityId(idBase58)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val keysHex by container.database.publicKeyDao()
        .observeByIdentityId(identityIdHex)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val keys = if (keysBase58.isNotEmpty()) keysBase58 else keysHex

    val appState = LocalAppState.current
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    // Each reading is kept with the total it was read for; a raise changes the key and reads again.
    val budgetTotals = keys.mapNotNull { key -> key.totalBudget?.let { key.keyId to it.toULong() } }
    var remainingBudgets by remember { mutableStateOf<Map<Int, Pair<ULong, ULong>>>(emptyMap()) }
    LaunchedEffect(budgetTotals, sdk) {
        val activeSdk = sdk
        remainingBudgets = emptyMap()
        if (budgetTotals.isEmpty() || activeSdk == null) return@LaunchedEffect
        try {
            val read = KeyLimitsFormatting.remainingBudgets(
                activeSdk.identities.fetchKeysRemainingBudgets(idBase58, budgetTotals.map { it.first }),
            )
            remainingBudgets = budgetTotals.mapNotNull { (keyId, total) ->
                read[keyId]?.let { keyId to (total to it) }
            }.toMap()
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            // The rows show the total alone.
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Identity Keys") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = { navController.navigate(AddIdentityKey(identityIdHex)) },
                        modifier = Modifier.testTag("keysList.addKey"),
                    ) {
                        Icon(Icons.Default.Add, contentDescription = "Add Key")
                    }
                },
            )
        },
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            if (keys.isEmpty()) {
                item {
                    FormSection {
                        Text(
                            "No keys recorded for this identity.",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
            items(keys, key = { it.keyId }) { key ->
                Card(
                    modifier = Modifier
                        .testTag("keysList.row.${key.keyId}")
                        .clickable { navController.navigate(KeyDetail(identityIdHex, key.keyId)) },
                ) {
                    ListItem(
                        headlineContent = { Text("Key #${key.keyId} · ${key.purpose}") },
                        supportingContent = {
                            Column {
                                Text(
                                    buildString {
                                        append(key.securityLevel)
                                        append(" · ")
                                        append(key.keyType)
                                        if (key.disabledAt != null) append(" · disabled")
                                    },
                                )
                                key.totalBudget?.toULong()?.let { total ->
                                    val remaining = remainingBudgets[key.keyId]
                                        ?.takeIf { it.first == total }?.second
                                    Text(
                                        KeyLimitsFormatting.budget(total, remaining),
                                        style = MaterialTheme.typography.bodySmall,
                                        color = if (remaining == 0uL) {
                                            MaterialTheme.colorScheme.error
                                        } else {
                                            MaterialTheme.colorScheme.onSurfaceVariant
                                        },
                                    )
                                }
                                key.expiresAt?.toULong()?.let { expiresAt ->
                                    val now = System.currentTimeMillis()
                                    Text(
                                        KeyLimitsFormatting.expiryRelative(expiresAt, now),
                                        style = MaterialTheme.typography.bodySmall,
                                        color = if (KeyLimitsFormatting.isExpired(expiresAt, now)) {
                                            MaterialTheme.colorScheme.error
                                        } else {
                                            MaterialTheme.colorScheme.onSurfaceVariant
                                        },
                                    )
                                }
                            }
                        },
                        trailingContent = if (key.hasLimits) {
                            {
                                TextButton(
                                    onClick = {
                                        navController.navigate(UpdateKeyLimits(identityIdHex, key.keyId))
                                    },
                                    modifier = Modifier.testTag("keyLimits.update.swipe.${key.keyId}"),
                                ) { Text("Limits") }
                            }
                        } else {
                            null
                        },
                    )
                }
            }
        }
    }
}
