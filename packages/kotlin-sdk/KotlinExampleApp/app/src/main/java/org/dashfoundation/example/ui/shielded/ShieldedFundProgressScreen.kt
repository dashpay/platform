package org.dashfoundation.example.ui.shielded

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.services.shielded.ShieldedFundFromAssetLockController.Phase
import org.dashfoundation.example.util.hexToBytes

/**
 * Live shielded-funding progress for a slot — port of
 * `ShieldedFundFromAssetLockProgressView.swift`. Resolves its controller
 * from the coordinator (dismissal-safe) and renders a phase stepper +
 * terminal banner. The Orchard note arrives on the next shielded sync pass,
 * so [Phase.Completed] shows the "next sync" note (← Swift).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ShieldedFundProgressScreen(
    walletIdHex: String,
    recipientRaw43Hex: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val walletId = walletIdHex.hexToBytes()
    val recipient = recipientRaw43Hex.hexToBytes()
    val controller = container.shieldedFundCoordinator.controller(walletId, recipient)

    Scaffold(topBar = { TopAppBar(title = { Text("Shielding") }) }) { padding ->
        if (controller == null) {
            Column(
                modifier = Modifier.fillMaxSize().padding(padding).padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterVertically),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text("Shielding finished", style = MaterialTheme.typography.titleMedium)
                Text(
                    "This shield is no longer tracked.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            return@Scaffold
        }

        val phase by controller.phase.collectAsStateWithLifecycle()
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Steps.forEach { step -> StepRow(step, phase) }

            when (val p = phase) {
                is Phase.Completed -> Text(
                    "Shielded — the note arrives on the next shielded sync pass.",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.testTag("shieldedFundProgress.completed"),
                )
                is Phase.Failed -> Text(
                    p.message,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.testTag("shieldedFundProgress.failed"),
                )
                else -> Unit
            }
        }
    }
}

private enum class Step(val label: String) {
    Building("Building asset-lock transaction"),
    Proving("Waiting for InstantSend / ChainLock proof"),
    Shielding("Building Orchard proof + shield"),
}

private val Steps = Step.entries

private fun reachedStep(phase: Phase): Int = when (phase) {
    is Phase.Idle -> 0
    is Phase.InFlight -> 2
    is Phase.Completed -> 3
    is Phase.Failed -> 2
}

@Composable
private fun StepRow(step: Step, phase: Phase) {
    val reached = reachedStep(phase)
    val index = step.ordinal
    Row(
        modifier = Modifier.fillMaxWidth().testTag("shieldedFundProgress.step.${step.name}"),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        when {
            phase is Phase.Failed && index == reached ->
                Icon(Icons.Default.Warning, null, tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(20.dp))
            index < reached ->
                Icon(Icons.Default.CheckCircle, null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(20.dp))
            index == reached && phase !is Phase.Completed ->
                CircularProgressIndicator(modifier = Modifier.size(18.dp), strokeWidth = 2.dp)
            else ->
                Icon(Icons.Default.CheckCircle, null, tint = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.size(20.dp))
        }
        Text(step.label, style = MaterialTheme.typography.bodyMedium)
    }
}
