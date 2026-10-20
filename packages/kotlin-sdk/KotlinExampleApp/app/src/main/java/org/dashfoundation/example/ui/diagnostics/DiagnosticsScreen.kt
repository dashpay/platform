package org.dashfoundation.example.ui.diagnostics

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
import androidx.compose.material3.LinearProgressIndicator
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.contracts.QUERY_REGISTRY

/**
 * Platform diagnostics — port of `DiagnosticsView.swift` ("Run All
 * Queries"): every bridged query from the registry executes against the
 * known-good testnet fixtures, reporting pass/fail + duration per query.
 * Environment sections (network state, SDK version, sync liveness,
 * database counts) fold in the status rows `OptionsView.swift` renders in
 * its Platform section so one screen answers "is anything broken?".
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DiagnosticsScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()

    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    // ── Sync liveness (one-shot poll; Refresh re-runs via reloadKey) ──
    var reloadKey by remember { mutableIntStateOf(0) }
    var spvRunning by remember { mutableStateOf<Boolean?>(null) }
    var addressSyncRunning by remember { mutableStateOf<Boolean?>(null) }
    var identitySyncRunning by remember { mutableStateOf<Boolean?>(null) }
    var shieldedSyncRunning by remember { mutableStateOf<Boolean?>(null) }

    LaunchedEffect(manager, reloadKey) {
        val current = manager
        if (current == null) {
            spvRunning = null
            addressSyncRunning = null
            identitySyncRunning = null
            shieldedSyncRunning = null
            return@LaunchedEffect
        }
        spvRunning = runCatching { current.isSpvRunning() }.getOrNull()
        addressSyncRunning = runCatching { current.isPlatformAddressSyncRunning() }.getOrNull()
        identitySyncRunning = runCatching { current.isIdentitySyncRunning() }.getOrNull()
        shieldedSyncRunning = if (Sdk.hasShielded()) {
            runCatching { current.isShieldedSyncRunning() }.getOrNull()
        } else {
            null
        }
    }

    // ── Database counts ────────────────────────────────────────────────
    val counts = container.database.storageCountsDao()
    val walletCount by counts.countWallets().collectAsStateWithLifecycle(initialValue = 0L)
    val identityCount by counts.countIdentities().collectAsStateWithLifecycle(initialValue = 0L)
    val contractCount by counts.countDataContracts().collectAsStateWithLifecycle(initialValue = 0L)
    val documentCount by counts.countDocuments().collectAsStateWithLifecycle(initialValue = 0L)
    val tokenBalanceCount by counts.countTokenBalances()
        .collectAsStateWithLifecycle(initialValue = 0L)
    val transactionCount by counts.countTransactions()
        .collectAsStateWithLifecycle(initialValue = 0L)
    val platformAddressCount by counts.countPlatformAddresses()
        .collectAsStateWithLifecycle(initialValue = 0L)

    // ── Run-all-queries state ──────────────────────────────────────────
    var isRunning by remember { mutableStateOf(false) }
    var currentQuery by remember { mutableStateOf("") }
    var progress by remember { mutableStateOf(0f) }
    var results by remember { mutableStateOf<List<QueryRunResult>>(emptyList()) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Diagnostics") },
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
            FormSection(title = "Environment") {
                LabeledContent("Network", network.displayName)
                LabeledContent("SDK Initialized", if (sdk != null) "Yes" else "No")
                LabeledContent("SDK Version", Sdk.version())
                LabeledContent(
                    "Shielded Support",
                    if (Sdk.hasShielded()) "Enabled" else "Disabled",
                )
                LabeledContent(
                    "Wallet Manager",
                    if (manager != null) "Active" else "Not active",
                )
            }

            FormSection(title = "Sync States") {
                LabeledContent("SPV Sync", runningLabel(spvRunning))
                LabeledContent("Platform Address Sync", runningLabel(addressSyncRunning))
                LabeledContent("Identity Sync", runningLabel(identitySyncRunning))
                LabeledContent(
                    "Shielded Sync",
                    if (Sdk.hasShielded()) runningLabel(shieldedSyncRunning) else "Unavailable",
                )
                androidx.compose.material3.TextButton(
                    onClick = { reloadKey++ },
                    modifier = Modifier.testTag("diagnostics.refreshSync"),
                ) {
                    Text("Refresh Sync States")
                }
            }

            FormSection(title = "Database") {
                LabeledContent("Wallets", "$walletCount")
                LabeledContent("Transactions", "$transactionCount")
                LabeledContent("Identities", "$identityCount")
                LabeledContent("Contracts", "$contractCount")
                LabeledContent("Documents", "$documentCount")
                LabeledContent("Token Balances", "$tokenBalanceCount")
                LabeledContent("Platform Addresses", "$platformAddressCount")
            }

            FormSection(title = "Run All Queries") {
                Text(
                    "Executes every bridged platform query with test data to " +
                        "verify connectivity and functionality (testnet fixtures).",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                SubmitButton(
                    text = if (isRunning) "Running…" else "Run All Queries",
                    isLoading = isRunning,
                    enabled = sdk != null && !isRunning,
                    modifier = Modifier.testTag("diagnostics.runAllQueries"),
                ) {
                    val currentSdk = sdk ?: return@SubmitButton
                    scope.launch {
                        isRunning = true
                        results = emptyList()
                        progress = 0f
                        val runnable = QUERY_REGISTRY.filter { query ->
                            query.inputs.filter { it.required }
                                .all { query.diagnosticInputs.containsKey(it.name) }
                        }
                        val collected = ArrayList<QueryRunResult>(runnable.size)
                        runnable.forEachIndexed { index, query ->
                            currentQuery = query.label
                            val startedAt = System.currentTimeMillis()
                            val outcome = runCatching {
                                query.execute(currentSdk, query.diagnosticInputs)
                            }
                            collected.add(
                                QueryRunResult(
                                    name = query.name,
                                    label = query.label,
                                    success = outcome.isSuccess,
                                    detail = outcome.exceptionOrNull()?.message
                                        ?: outcome.getOrNull()?.take(120)
                                        ?: "null",
                                    durationMs = System.currentTimeMillis() - startedAt,
                                ),
                            )
                            progress = (index + 1).toFloat() / runnable.size
                            results = collected.toList()
                        }
                        currentQuery = ""
                        isRunning = false
                    }
                }
                if (isRunning) {
                    LinearProgressIndicator(
                        progress = { progress },
                        modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
                    )
                    Text(
                        currentQuery,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            if (results.isNotEmpty()) {
                val passed = results.count { it.success }
                val failed = results.size - passed
                FormSection(title = "Results ($passed/${results.size} passed)") {
                    LabeledContent(
                        "Summary",
                        "$passed passed · $failed failed · ${results.size} total",
                    )
                    HorizontalDivider(Modifier.padding(vertical = 4.dp))
                    results.forEach { result ->
                        LabeledContent(
                            label = (if (result.success) "PASS · " else "FAIL · ") + result.label,
                            value = "${result.durationMs} ms",
                        )
                        if (!result.success) {
                            Text(
                                result.detail,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.error,
                                modifier = Modifier
                                    .padding(bottom = 4.dp)
                                    .testTag("diagnostics.failure.${result.name}"),
                            )
                        }
                    }
                }
            }
        }
    }
}

private data class QueryRunResult(
    val name: String,
    val label: String,
    val success: Boolean,
    val detail: String,
    val durationMs: Long,
)

private fun runningLabel(running: Boolean?): String = when (running) {
    null -> "—"
    true -> "Running"
    false -> "Stopped"
}
