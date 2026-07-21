package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.clickable
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
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.IdentityDetail
import org.dashfoundation.example.services.IdentityRegistrationController
import org.dashfoundation.example.services.IdentityRegistrationController.Phase
import org.dashfoundation.example.ui.wallet.toHexString

/**
 * Compact pending-registration row — port of `PendingRegistrationRow` in
 * `PendingRegistrationsList.swift`. Icon + phase label + slot fingerprint;
 * tapping opens [RegistrationProgressScreen]. The dismissal swipe (failed /
 * synced-unconfirmed) is a follow-up UX affordance.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PendingRegistrationRow(
    controller: IdentityRegistrationController,
    navController: NavHostController,
) {
    val phase by controller.phase.collectAsStateWithLifecycle()
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .testTag("pendingRegistration.row.${controller.slotRowId}")
            .clickable {
                navController.navigate(
                    org.dashfoundation.example.navigation.RegistrationProgress(
                        controller.walletId.toHexString(),
                        controller.identityIndex,
                    ),
                )
            },
    ) {
        ListItem(
            leadingContent = { PhaseIcon(phase) },
            headlineContent = { Text("Identity #${controller.identityIndex}") },
            supportingContent = { Text(phaseLabel(phase)) },
        )
    }
}

/**
 * Live registration progress for a slot — port of
 * `RegistrationProgressView.swift`. Resolves its controller from the
 * coordinator by `(walletId, identityIndex)` so it is dismissal-safe (the
 * controller outlives this screen); renders a phase stepper and the terminal
 * banner. The five funding steps collapse to a phase-driven stepper here —
 * the asset-lock IS/CL elapsed-time sub-steps land with the asset-lock
 * funding path (deferred).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RegistrationProgressScreen(
    walletIdHex: String,
    identityIndex: Int,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val walletId = walletIdHex.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
    val controller = container.registrationCoordinator.controller(walletId, identityIndex)

    Scaffold(
        topBar = { TopAppBar(title = { Text("Registration") }) },
    ) { padding ->
        if (controller == null) {
            Column(
                modifier = Modifier.fillMaxSize().padding(padding).padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterVertically),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text("Registration finished", style = MaterialTheme.typography.titleMedium)
                Text(
                    "This registration is no longer tracked.",
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
                is Phase.Completed -> {
                    Text(
                        "Identity created",
                        style = MaterialTheme.typography.titleMedium,
                        modifier = Modifier.testTag("registrationProgress.completed"),
                    )
                    Text(
                        p.identityId.toHexString().take(24) + "…",
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                    )
                    TextButton(onClick = {
                        navController.navigate(IdentityDetail(p.identityId.toHexString()))
                    }) { Text("View Identity") }
                }
                is Phase.Failed -> Text(
                    p.message,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.testTag("registrationProgress.failed"),
                )
                is Phase.Unconfirmed -> Text(
                    "Confirmation pending — the identity may already be live. " + p.message,
                    color = Color(0xFFB26A00),
                    modifier = Modifier.testTag("registrationProgress.unconfirmed"),
                )
                else -> Unit
            }
        }
    }
}

private enum class Step(val label: String) {
    PreparingKeys("Preparing keys"),
    Building("Building funding transaction"),
    Broadcasting("Broadcasting"),
    Confirming("Waiting for confirmation"),
    Registering("Registering identity"),
}

private val Steps = Step.entries

/** Ordinal reached by the current phase — everything below is done, current is active. */
private fun reachedStep(phase: Phase): Int = when (phase) {
    is Phase.Idle, is Phase.PreparingKeys -> 0
    is Phase.InFlight -> 4 // in flight covers building → registering
    is Phase.Completed -> 5
    is Phase.Failed -> 4
    is Phase.Unconfirmed -> 3
}

@Composable
private fun StepRow(step: Step, phase: Phase) {
    val reached = reachedStep(phase)
    val index = step.ordinal
    Row(
        modifier = Modifier.fillMaxWidth().testTag("registrationProgress.step.${step.name}"),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        when {
            phase is Phase.Failed && index == reached ->
                Icon(Icons.Default.Warning, contentDescription = null, tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(20.dp))
            index < reached ->
                Icon(Icons.Default.CheckCircle, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(20.dp))
            index == reached && phase !is Phase.Completed ->
                CircularProgressIndicator(modifier = Modifier.size(18.dp), strokeWidth = 2.dp)
            else ->
                Icon(Icons.Default.Refresh, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.size(20.dp))
        }
        Text(step.label, style = MaterialTheme.typography.bodyMedium)
    }
}

@Composable
private fun PhaseIcon(phase: Phase) {
    when (phase) {
        is Phase.Completed -> Icon(Icons.Default.CheckCircle, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
        is Phase.Failed -> Icon(Icons.Default.Warning, contentDescription = null, tint = MaterialTheme.colorScheme.error)
        is Phase.Unconfirmed -> Icon(Icons.Default.Info, contentDescription = null, tint = Color(0xFFB26A00))
        else -> CircularProgressIndicator(modifier = Modifier.size(18.dp), strokeWidth = 2.dp)
    }
}

private fun phaseLabel(phase: Phase): String = when (phase) {
    is Phase.Idle -> "Queued"
    is Phase.PreparingKeys -> "Preparing keys"
    is Phase.InFlight -> "In flight"
    is Phase.Completed -> "Registered"
    is Phase.Failed -> "Failed"
    is Phase.Unconfirmed -> "Confirmation pending"
}
