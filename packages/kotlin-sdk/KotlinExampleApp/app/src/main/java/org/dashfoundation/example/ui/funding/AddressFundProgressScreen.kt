package org.dashfoundation.example.ui.funding

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
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
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
import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.AddressFundProgress
import org.dashfoundation.example.navigation.FundFromAssetLock
import org.dashfoundation.example.services.assetlock.AddressFundFromAssetLockController
import org.dashfoundation.example.services.assetlock.AddressFundFromAssetLockController.Phase
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * Compact pending-address-funding row — port of
 * `PendingPlatformFundFromAssetLockRow` in
 * `PendingPlatformFundFromAssetLocksList.swift`. Icon + phase label + slot
 * fingerprint; tapping opens [AddressFundProgressScreen].
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PendingAssetLockRow(
    controller: AddressFundFromAssetLockController,
    navController: NavHostController,
) {
    val phase by controller.phase.collectAsStateWithLifecycle()
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .testTag("pendingAssetLock.row.${controller.slotRowId}")
            .clickable {
                navController.navigate(
                    AddressFundProgress(
                        controller.walletId.toHex(),
                        controller.platformAccountIndex,
                        controller.recipientHash.toHex(),
                    ),
                )
            },
    ) {
        ListItem(
            leadingContent = { PhaseIcon(phase) },
            headlineContent = { Text("Platform Account #${controller.platformAccountIndex}") },
            supportingContent = {
                Text("→ ${controller.recipientHash.toHex().take(12)}… · ${phaseLabel(phase)}")
            },
        )
    }
}

/**
 * The pending-address-fundings section — the header + rows the
 * IdentitiesHome list embeds. Port of
 * `PendingPlatformFundFromAssetLocksList.swift`: merges two row sources —
 * (1) [controllers], the live in-flight fundings from the coordinator, and
 * (2) [resumableLocks], DB-backed orphan `AssetLockEntity` rows
 * (`fundingTypeRaw == 4`, `statusRaw ∈ [1, 3]`) recovered after a crash
 * between asset-lock broadcast and ST submission.
 *
 * Anti-double-consume gate: if ANY controller is [Phase.InFlight]
 * ([hasActiveFunding]), Resume is suppressed on every orphan row — the
 * in-flight funding might be driving one of these very outpoints (the
 * controller doesn't expose the outpoint it's consuming), so offering Resume
 * would risk a second consume on the same lock. ← Swift's `hasActiveFunding`
 * rationale.
 */
@Composable
fun PendingAssetLocksList(
    controllers: List<AddressFundFromAssetLockController>,
    navController: NavHostController,
    resumableLocks: List<Pair<String, AssetLockEntity>> = emptyList(),
    hasActiveFundingFor: (walletIdHex: String) -> Boolean = { false },
) {
    if (controllers.isEmpty() && resumableLocks.isEmpty()) return
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(
            "PENDING PLATFORM TOP UPS",
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        controllers.forEach { PendingAssetLockRow(it, navController) }
        resumableLocks.forEach { (walletIdHex, lock) ->
            ResumablePlatformFundFromAssetLockRow(
                lock = lock,
                hasActiveFunding = hasActiveFundingFor(walletIdHex),
                onResume = {
                    navController.navigate(
                        FundFromAssetLock(
                            walletIdHex = walletIdHex,
                            resumeOutPointHex = lock.outPointHex,
                        ),
                    )
                },
            )
        }
    }
}

/**
 * Single orphaned-asset-lock row — port of
 * `ResumablePlatformFundFromAssetLockRow` in
 * `PendingPlatformFundFromAssetLocksList.swift`. Shows the outpoint summary
 * (short txid, amount, status) plus a Resume button that opens
 * [FundFromAssetLockScreen] in resume mode. When [hasActiveFunding] the
 * Resume affordance is replaced by a non-interactive "Waiting for
 * InstantSend / ChainLock…" indicator (double-consume guard, at ANY status).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ResumablePlatformFundFromAssetLockRow(
    lock: AssetLockEntity,
    hasActiveFunding: Boolean,
    onResume: () -> Unit,
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .testTag("resumableAssetLock.row.${lock.outPointHex}"),
    ) {
        ListItem(
            headlineContent = { Text("Asset Lock ${lock.shortOutPointDisplay}") },
            supportingContent = {
                Text("${lock.amountDuffs} duffs · ${lock.statusLabel}")
            },
            trailingContent = {
                if (hasActiveFunding) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        CircularProgressIndicator(
                            modifier = Modifier.size(16.dp),
                            strokeWidth = 2.dp,
                        )
                        Text(
                            "Waiting…",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                } else {
                    Button(
                        onClick = onResume,
                        modifier = Modifier.testTag("resumableAssetLock.resume.${lock.outPointHex}"),
                    ) {
                        Icon(
                            Icons.Default.Refresh,
                            contentDescription = null,
                            modifier = Modifier.size(18.dp),
                        )
                        Text("Resume")
                    }
                }
            },
        )
    }
}

/**
 * Live address-funding progress for a slot — port of
 * `AddressFundFromAssetLockProgressView.swift`. Resolves its controller
 * from the coordinator so it is dismissal-safe; renders a phase-driven
 * stepper + terminal banner. The IS/CL elapsed-time sub-steps that the
 * Swift view reads from the asset-lock rows collapse to a single "proving"
 * step here (the funding FFI isn't bridged, so no lock rows are produced).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AddressFundProgressScreen(
    walletIdHex: String,
    platformAccountIndex: Int,
    recipientHashHex: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val walletId = walletIdHex.hexToBytes()
    val recipientHash = recipientHashHex.hexToBytes()
    val controller = container.addressFundCoordinator
        .controller(walletId, platformAccountIndex, recipientHash)

    Scaffold(topBar = { TopAppBar(title = { Text("Platform Top Up") }) }) { padding ->
        if (controller == null) {
            Column(
                modifier = Modifier.fillMaxSize().padding(padding).padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterVertically),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text("Funding finished", style = MaterialTheme.typography.titleMedium)
                Text(
                    "This funding is no longer tracked.",
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
                    "Address funded — ${p.newBalance} credits",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.testTag("addressFundProgress.completed"),
                )
                is Phase.Failed -> Text(
                    p.message,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.testTag("addressFundProgress.failed"),
                )
                else -> Unit
            }
        }
    }
}

private enum class Step(val label: String) {
    Building("Building asset-lock transaction"),
    Proving("Waiting for InstantSend / ChainLock proof"),
    Funding("Funding platform address"),
}

private val Steps = Step.entries

private fun reachedStep(phase: Phase): Int = when (phase) {
    is Phase.Idle -> 0
    is Phase.InFlight -> 2 // in flight covers building → funding
    is Phase.Completed -> 3
    is Phase.Failed -> 2
}

@Composable
private fun StepRow(step: Step, phase: Phase) {
    val reached = reachedStep(phase)
    val index = step.ordinal
    androidx.compose.foundation.layout.Row(
        modifier = Modifier.fillMaxWidth().testTag("addressFundProgress.step.${step.name}"),
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

@Composable
private fun PhaseIcon(phase: Phase) {
    when (phase) {
        is Phase.Completed -> Icon(Icons.Default.CheckCircle, null, tint = MaterialTheme.colorScheme.primary)
        is Phase.Failed -> Icon(Icons.Default.Warning, null, tint = MaterialTheme.colorScheme.error)
        else -> CircularProgressIndicator(modifier = Modifier.size(18.dp), strokeWidth = 2.dp)
    }
}

private fun phaseLabel(phase: Phase): String = when (phase) {
    is Phase.Idle -> "Queued"
    is Phase.InFlight -> "In flight"
    is Phase.Completed -> "Funded"
    is Phase.Failed -> "Failed"
}
