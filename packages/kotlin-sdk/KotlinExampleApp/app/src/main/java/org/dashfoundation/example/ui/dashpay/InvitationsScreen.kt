package org.dashfoundation.example.ui.dashpay

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.flow.MutableStateFlow
import org.dashfoundation.dashsdk.persistence.entities.InvitationEntity
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.util.formatDuffs
import org.dashfoundation.example.util.toHex
import java.text.DateFormat
import java.util.Date

/**
 * "Sent invitations" list — port of `InvitationsView.swift`. Room is the UI
 * source of truth (`InvitationDao.observeAll`, newest first), filtered to
 * invitations whose wallet is loaded on the active manager — multi-wallet
 * aware, and each row reclaims via its OWN wallet. Status badges mirror the
 * iOS colors: Created (primary), Claimed (green-ish tertiary), Reclaimed
 * (orange-ish secondary), Unknown (gray).
 *
 * The create entry ("+", `dashpay.invitations.create`) needs a loaded wallet
 * but NOT an active identity — a voucher is pure funding. Reclaim
 * (`dashpay.invitations.reclaim`) is offered only on `Created` rows.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun InvitationsScreen() {
    val container = LocalAppContainer.current
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by remember(manager) {
        manager?.wallets ?: MutableStateFlow(emptyMap<String, ManagedPlatformWallet>())
    }.collectAsStateWithLifecycle()

    val allInvitations by remember {
        container.database.invitationDao().observeAll()
    }.collectAsStateWithLifecycle(emptyList())
    // Only rows whose wallet is loaded — a foreign-network or unloaded
    // wallet's row can neither be shared again nor reclaimed here.
    val invitations = remember(allInvitations, walletsMap) {
        allInvitations.filter { walletsMap.containsKey(it.walletId.toHex()) }
    }

    var showCreateSheet by remember { mutableStateOf(false) }
    var reclaimTarget by remember { mutableStateOf<InvitationEntity?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Sent Invitations") },
                actions = {
                    IconButton(
                        onClick = { showCreateSheet = true },
                        enabled = walletsMap.isNotEmpty(),
                        modifier = Modifier.testTag("dashpay.invitations.create"),
                    ) {
                        Icon(Icons.Default.Add, contentDescription = "Create invitation")
                    }
                },
            )
        },
    ) { padding ->
        if (invitations.isEmpty()) {
            Column(
                modifier = Modifier.fillMaxSize().padding(padding).padding(24.dp)
                    .testTag("dashpay.invitations.list"),
                verticalArrangement = Arrangement.spacedBy(8.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text("No invitations yet", style = MaterialTheme.typography.titleMedium)
                Text(
                    "Create an invitation to fund a friend's new identity — " +
                        "no Dash required on their side.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        } else {
            LazyColumn(
                modifier = Modifier.fillMaxSize().padding(padding)
                    .testTag("dashpay.invitations.list"),
            ) {
                items(invitations, key = { it.outPointHex }) { invitation ->
                    InvitationRow(
                        invitation = invitation,
                        onReclaim = { reclaimTarget = invitation },
                    )
                }
            }
        }
    }

    if (showCreateSheet) {
        ModalBottomSheet(onDismissRequest = { showCreateSheet = false }) {
            CreateInvitationSheet(onClose = { showCreateSheet = false })
        }
    }
    reclaimTarget?.let { invitation ->
        val wallet = walletsMap[invitation.walletId.toHex()]
        if (wallet != null) {
            ModalBottomSheet(onDismissRequest = { reclaimTarget = null }) {
                ReclaimInvitationSheet(
                    invitation = invitation,
                    wallet = wallet,
                    onClose = { reclaimTarget = null },
                )
            }
        }
    }
}

@Composable
private fun InvitationRow(
    invitation: InvitationEntity,
    onReclaim: () -> Unit,
) {
    val (statusLabel, statusColor) = when (invitation.statusRaw) {
        0 -> "Created" to MaterialTheme.colorScheme.primary
        1 -> "Claimed" to MaterialTheme.colorScheme.tertiary
        2 -> "Reclaimed" to MaterialTheme.colorScheme.secondary
        else -> "Unknown" to MaterialTheme.colorScheme.onSurfaceVariant
    }
    val shortOutpoint = invitation.outPointHex.take(8) + "…" +
        invitation.outPointHex.substringAfterLast(':').let { ":$it" }
    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .let { base ->
                if (invitation.statusRaw == 0) {
                    base.clickable(onClick = onReclaim)
                        .testTag("dashpay.invitations.reclaim")
                } else {
                    base
                }
            },
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(
                    "${formatDuffs(invitation.amountDuffs)} DASH",
                    style = MaterialTheme.typography.bodyLarge,
                )
                Text(
                    shortOutpoint,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                if (invitation.hasInviter) {
                    Text(
                        "Contact request on claim",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Text(
                    DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT)
                        .format(Date(invitation.createdAtSecs * 1000L)),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Column(horizontalAlignment = Alignment.End) {
                Text(statusLabel, style = MaterialTheme.typography.labelLarge, color = statusColor)
                if (invitation.statusRaw == 0) {
                    Text(
                        "Tap to reclaim",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}
