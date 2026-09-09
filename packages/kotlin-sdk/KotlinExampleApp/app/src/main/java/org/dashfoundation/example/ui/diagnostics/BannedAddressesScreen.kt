package org.dashfoundation.example.ui.diagnostics

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.json.JSONArray

/**
 * Read-only DAPI address ban list — port of `BannedAddressesView.swift`.
 * Renders the snapshot from `PlatformWalletManager.addressBanInfo()`
 * (`platform_wallet_manager_address_ban_info`), a JSON array of
 * `{"address","banned","banCount","bannedUntilMs","reason"}` entries.
 *
 * The empty-state semantics match iOS: an empty (or null) list legitimately
 * means either "no DAPI addresses banned" or "address pool not yet seeded".
 * Refresh re-reads the current SDK session.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BannedAddressesScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    var reloadKey by remember { mutableIntStateOf(0) }
    var entries by remember { mutableStateOf<List<BannedAddressEntry>>(emptyList()) }
    var errorMessage by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(manager, reloadKey) {
        val current = manager
        if (current == null) {
            entries = emptyList()
            errorMessage = null
            return@LaunchedEffect
        }
        val outcome = withContext(Dispatchers.IO) {
            runCatching { current.addressBanInfo() }
        }
        outcome.fold(
            onSuccess = { json ->
                entries = json?.let { parseBanInfo(it) } ?: emptyList()
                errorMessage = null
            },
            onFailure = {
                entries = emptyList()
                errorMessage = it.message ?: "Failed to read ban list."
            },
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Banned Addresses") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = { reloadKey++ },
                        modifier = Modifier.testTag("bannedAddresses.refresh"),
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
            FormSection(title = "Addresses (${entries.size})") {
                if (entries.isEmpty()) {
                    Text(
                        "No banned addresses.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.testTag("bannedAddresses.empty"),
                    )
                    Text(
                        "This list reflects the current SDK session. An empty list " +
                            "can mean either that no DAPI addresses have been banned, " +
                            "or that the address pool has not yet been seeded.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(top = 8.dp),
                    )
                } else {
                    entries.forEachIndexed { index, entry ->
                        if (index > 0) HorizontalDivider(Modifier.padding(vertical = 8.dp))
                        Text(
                            entry.address,
                            style = MaterialTheme.typography.bodyMedium,
                            fontFamily = FontFamily.Monospace,
                            modifier = Modifier.testTag("bannedAddresses.entry.$index"),
                        )
                        Text(
                            buildString {
                                append(if (entry.banned) "Banned" else "Active")
                                if (entry.banCount > 0) append(" · ${entry.banCount} ban(s)")
                                if (entry.bannedUntilMs > 0) append(" · until ${entry.bannedUntilMs}")
                            },
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        entry.reason?.takeIf { it.isNotBlank() }?.let { reason ->
                            Text(
                                reason,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }

            errorMessage?.let { message ->
                FormSection(title = "Error") {
                    Text(
                        message,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("bannedAddresses.error"),
                    )
                }
            }

            FormSection(title = "Session") {
                Text(
                    if (manager == null) {
                        "Wallet manager not active — no SDK session to inspect."
                    } else {
                        "Wallet manager active on ${manager?.network?.displayName}."
                    },
                    style = MaterialTheme.typography.bodyMedium,
                )
            }
        }
    }
}

private data class BannedAddressEntry(
    val address: String,
    val banned: Boolean,
    val banCount: Long,
    val bannedUntilMs: Long,
    val reason: String?,
)

/**
 * Parse the ban-info JSON array. Tolerates absent fields so a schema tweak on
 * the Rust side degrades to a partial render rather than an empty list.
 */
private fun parseBanInfo(json: String): List<BannedAddressEntry> = runCatching {
    val array = JSONArray(json)
    (0 until array.length()).map { i ->
        val obj = array.getJSONObject(i)
        BannedAddressEntry(
            address = obj.optString("address", "—"),
            banned = obj.optBoolean("banned", false),
            banCount = obj.optLong("banCount", 0L),
            bannedUntilMs = obj.optLong("bannedUntilMs", 0L),
            reason = obj.optString("reason").takeIf { it.isNotBlank() },
        )
    }
}.getOrElse { emptyList() }
