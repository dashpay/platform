package org.dashfoundation.example.ui.shielded

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.dashsdk.persistence.entities.ShieldedActivityEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.util.hexToBytes

/**
 * Shielded activity timeline for a wallet — port of `ShieldedActivityView.swift`.
 * Reads the Room `shielded_activities` rows via
 * [org.dashfoundation.dashsdk.persistence.dao.ShieldedDao.observeActivityByWallet]:
 * a Pending section (status 0) above a History section (status != 0), each
 * row labeled + signed by its kind / direction.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ShieldedActivityScreen(walletIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val walletId = remember(walletIdHex) { walletIdHex.hexToBytes() }

    val activity by container.database.shieldedDao()
        .observeActivityByWallet(walletId)
        .collectAsStateWithLifecycle(initialValue = emptyList())

    val pending = remember(activity) { activity.filter { it.status == 0 } }
    val history = remember(activity) { activity.filter { it.status != 0 } }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Shielded Activity") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            if (activity.isEmpty()) {
                item {
                    Text(
                        "No shielded activity.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            if (pending.isNotEmpty()) {
                item { SectionHeader("PENDING") }
                items(pending, key = { it.entryId.joinToString("") { b -> "%02x".format(b) } + "-p" }) {
                    ActivityRow(it)
                }
            }
            if (history.isNotEmpty()) {
                item { SectionHeader("HISTORY") }
                items(history, key = { it.entryId.joinToString("") { b -> "%02x".format(b) } + "-h" }) {
                    ActivityRow(it)
                }
            }
        }
    }
}

@Composable
private fun SectionHeader(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.labelMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ActivityRow(row: ShieldedActivityEntity) {
    Card(modifier = Modifier.testTag("shieldedActivity.row")) {
        ListItem(
            headlineContent = { Text(kindLabel(row.kindTag)) },
            supportingContent = { Text(statusLabel(row.status)) },
            trailingContent = { Text(signedAmount(row)) },
        )
    }
}

/** Kind label — ← Swift `ShieldedActivityView` kind map. */
private fun kindLabel(kindTag: Int): String = when (kindTag) {
    0 -> "Shielded"
    1 -> "Shielded from Asset Lock"
    2 -> "Received"
    3 -> "Sent"
    4 -> "Unshielded"
    5 -> "Withdrawn"
    6 -> "Identity Created"
    else -> "Shielded Spend"
}

private fun statusLabel(status: Int): String = when (status) {
    0 -> "Pending"
    1 -> "Confirmed"
    else -> "Failed"
}

/** 1 DASH = 1e11 credits; signed by direction (0 In +, 1 Out −, 2 Self). */
private fun signedAmount(row: ShieldedActivityEntity): String {
    val dash = row.amount.toDouble() / 1e11
    val sign = when (row.direction) {
        0 -> "+"
        1 -> "−"
        else -> ""
    }
    return "%s%.4f DASH".format(sign, dash)
}
