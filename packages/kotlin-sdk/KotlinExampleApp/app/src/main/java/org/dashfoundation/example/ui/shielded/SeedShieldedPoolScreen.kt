package org.dashfoundation.example.ui.shielded

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.Dispatchers
import org.dashfoundation.dashsdk.funding.ShieldedProver
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.hexToBytes

/**
 * Seed the shielded (Orchard) note pool — port of `SeedShieldedPoolView.swift`.
 * Gated on [org.dashfoundation.dashsdk.Sdk.hasShielded] by [ShieldedGate].
 *
 * Submit is wired to the real pool-seeding FFI
 * (`platform_wallet_manager_shielded_seed_pool_notes` via
 * [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.seedShieldedPoolNotes]):
 * it builds real + zero-value filler notes in batches of 6 actions, one ~30s
 * Halo 2 proof per batch, driving a per-batch progress callback. The simple
 * idle → in-flight → completed/failed phase machine mirrors Swift; the sheet
 * cannot be dismissed while a seed is in flight.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SeedShieldedPoolScreen(walletIdHex: String, navController: NavHostController) {
    ShieldedGate(navController) {
        val container = LocalAppContainer.current
        val walletId = remember(walletIdHex) { walletIdHex.hexToBytes() }
        val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
        val scope = rememberCoroutineScope()

        var targetText by rememberSaveable { mutableStateOf("250") }
        var phase by remember { mutableStateOf<SeedPhase>(SeedPhase.Idle) }
        var progress by remember { mutableStateOf<SeedProgress?>(null) }

        // Warm the prover on entry (the first batch pays the ~30s build otherwise).
        produceState(initialValue = Unit) {
            runCatching { ShieldedProver.warmUp() }
            value = Unit
        }

        val target = targetText.toLongOrNull()
        val batches = target?.let { (it + 5) / 6 } // ceil(target / MAX_ACTIONS_PER_BATCH)
        val inFlight = phase is SeedPhase.InFlight
        val canSubmit = manager != null && target != null && target > 0 && !inFlight

        Scaffold(
            topBar = {
                androidx.compose.material3.TopAppBar(
                    title = { Text("Seed Shielded Pool") },
                    navigationIcon = {
                        IconButton(
                            onClick = { navController.popBackStack() },
                            enabled = !inFlight,
                        ) {
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
                    .imePadding()
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                FormSection(title = "Target Pool Size") {
                    Text(
                        "Builds real + zero-value filler notes in batches of 6 " +
                            "actions so future shielded sends have change available.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    OutlinedTextField(
                        value = targetText,
                        onValueChange = { targetText = it.filter(Char::isDigit) },
                        label = { Text("Target notes") },
                        singleLine = true,
                        enabled = !inFlight,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        modifier = Modifier.fillMaxWidth().testTag("seedShieldedPool.target"),
                    )
                    if (batches != null) {
                        Text(
                            "≈ $batches batch${if (batches == 1L) "" else "es"}",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }

                when (val p = phase) {
                    is SeedPhase.InFlight -> FormSection(title = "Seeding") {
                        val prog = progress
                        Text(
                            if (prog != null) {
                                "${prog.batchIndex}/~${prog.batchesTotalEstimate} batches · " +
                                    "${prog.poolNotesNow}/${prog.target} notes"
                            } else {
                                "Building proof for the first batch (~30s)…"
                            },
                            style = MaterialTheme.typography.bodyMedium,
                            modifier = Modifier.testTag("seedShieldedPool.progress"),
                        )
                        if (prog != null && prog.target > 0) {
                            LinearProgressIndicator(
                                progress = {
                                    (prog.poolNotesNow.toFloat() / prog.target.toFloat())
                                        .coerceIn(0f, 1f)
                                },
                                modifier = Modifier.fillMaxWidth(),
                            )
                        }
                        Text(
                            "Keep the app in the foreground — each batch runs a ~30s proof.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    is SeedPhase.Completed -> FormSection(title = "Done") {
                        Icon(Icons.Default.CheckCircle, null, tint = MaterialTheme.colorScheme.primary)
                        Text(
                            "Pool seeded — the notes are usable after the next shielded sync.",
                            style = MaterialTheme.typography.bodyMedium,
                            modifier = Modifier.testTag("seedShieldedPool.completed"),
                        )
                    }
                    is SeedPhase.Failed -> FormSection(title = "Failed") {
                        Text(
                            p.message,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.testTag("seedShieldedPool.failed"),
                        )
                    }
                    is SeedPhase.Idle -> Unit
                }

                SubmitButton(
                    text = "Seed Pool",
                    isLoading = inFlight,
                    enabled = canSubmit,
                    modifier = Modifier.fillMaxWidth().testTag("seedShieldedPool.submit"),
                ) {
                    val m = manager ?: return@SubmitButton
                    val targetTotal = target ?: return@SubmitButton
                    phase = SeedPhase.InFlight
                    progress = null
                    scope.launch {
                        try {
                            m.seedShieldedPoolNotes(
                                walletId = walletId,
                                targetTotalNotes = targetTotal,
                            ) { batchIndex, batchesTotalEstimate, poolNotesNow, tgt ->
                                // Fires on a worker thread; hop onto the main
                                // dispatcher to update Compose state.
                                scope.launch {
                                    withContext(Dispatchers.Main) {
                                        progress = SeedProgress(
                                            batchIndex, batchesTotalEstimate, poolNotesNow, tgt,
                                        )
                                    }
                                }
                            }
                            phase = SeedPhase.Completed
                        } catch (e: Exception) {
                            phase = SeedPhase.Failed(e.message ?: "Seeding failed")
                        }
                    }
                }
            }
        }
    }
}

/** Simple phase machine mirroring the Swift `SeedShieldedPoolView` states. */
private sealed interface SeedPhase {
    data object Idle : SeedPhase
    data object InFlight : SeedPhase
    data object Completed : SeedPhase
    data class Failed(val message: String) : SeedPhase
}

/** One progress tick from the seed-pool FFI callback. */
private data class SeedProgress(
    val batchIndex: Long,
    val batchesTotalEstimate: Long,
    val poolNotesNow: Long,
    val target: Long,
)
