package org.dashfoundation.example.ui.wallet

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.ArrowDownward
import androidx.compose.material.icons.filled.ArrowUpward
import androidx.compose.material.icons.filled.History
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.flatMapLatest
import kotlinx.coroutines.flow.flowOf
import org.dashfoundation.dashsdk.persistence.entities.TransactionEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.WalletTransactionDetail
import org.dashfoundation.example.ui.theme.appStatusColors
import org.dashfoundation.example.util.formatRelative
import java.util.Date

/**
 * Per-wallet transaction timeline — port of `TransactionListView.swift`:
 * the wallet's TXO set (denormalized `walletId` scan) resolved to the
 * distinct creating-or-spending transactions, mempool rows first then
 * `firstSeen` descending. The iOS asset-lock amount override is skipped —
 * asset-lock rows aren't populated on Android yet.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalCoroutinesApi::class, ExperimentalStdlibApi::class)
@Composable
fun TransactionListScreen(
    walletIdHex: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val walletId = remember(walletIdHex) { walletIdHex.hexToByteArray() }

    val transactions by remember(walletIdHex) {
        container.database.txoDao().observeByWallet(walletId)
            .flatMapLatest { txos ->
                val txids = LinkedHashSet<List<Byte>>()
                txos.forEach { txo ->
                    txo.txid?.let { txids.add(it.toList()) }
                    txo.spendingTxid?.let { txids.add(it.toList()) }
                }
                if (txids.isEmpty()) {
                    flowOf(emptyList())
                } else {
                    container.database.transactionDao()
                        .observeByTxids(txids.map { it.toByteArray() })
                }
            }
    }.collectAsStateWithLifecycle(initialValue = emptyList())

    // Mempool (context == 0) first, then newest first (← the iOS sort).
    val sorted = remember(transactions) {
        transactions.sortedWith(
            compareBy<TransactionEntity> { it.context != 0 }.thenByDescending { it.firstSeen },
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Transactions") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        if (sorted.isEmpty()) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp, Alignment.CenterVertically),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Box(
                    modifier = Modifier
                        .size(72.dp)
                        .clip(CircleShape)
                        .background(MaterialTheme.colorScheme.primaryContainer),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        Icons.Filled.History,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onPrimaryContainer,
                        modifier = Modifier.size(36.dp),
                    )
                }
                Text("No transactions found.", style = MaterialTheme.typography.titleLarge)
                Text(
                    "Transactions will appear here once you send or receive Dash.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        } else {
            LazyColumn(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentPadding = PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                items(sorted, key = { it.txid.toHexString() }) { tx ->
                    val txidHex = tx.txid.toHexString()
                    Card(
                        onClick = { navController.navigate(WalletTransactionDetail(txidHex)) },
                        modifier = Modifier.testTag("transactions.row.$txidHex"),
                        shape = MaterialTheme.shapes.large,
                        colors = CardDefaults.cardColors(
                            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
                        ),
                        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
                    ) {
                        TransactionRow(tx)
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalStdlibApi::class)
@Composable
private fun TransactionRow(tx: TransactionEntity) {
    val confirmed = tx.context >= 2
    val incoming = tx.direction == 0
    val amountColor = transactionColor(tx)
    val statusColor = if (confirmed) appStatusColors.success else appStatusColors.warning

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(14.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Box(
            modifier = Modifier
                .size(40.dp)
                .clip(CircleShape)
                .background(MaterialTheme.colorScheme.primaryContainer),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                if (incoming) Icons.Filled.ArrowDownward else Icons.Filled.ArrowUpward,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onPrimaryContainer,
                modifier = Modifier.size(20.dp),
            )
        }
        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            Text(
                formattedNetAmount(tx),
                style = MaterialTheme.typography.titleMedium,
                color = amountColor,
            )
            Row(
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    if (confirmed) "Confirmed" else "Pending",
                    style = MaterialTheme.typography.labelMedium,
                    color = statusColor,
                )
                if (tx.firstSeen > 0) {
                    Text(
                        "·",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Text(
                        formatRelative(Date(tx.firstSeen * 1000)),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

/** Signed net amount — counterpart of `PersistentTransaction.formattedAmount`. */
internal fun formattedNetAmount(tx: TransactionEntity): String {
    val dash = tx.netAmount.toDouble() / 100_000_000.0
    val sign = if (tx.netAmount > 0) "+" else ""
    return "$sign%.8f DASH".format(dash)
}

/** direction: 0=incoming, 1=outgoing, 2=internal, 3=coinJoin (← typeColor). */
internal fun transactionColor(tx: TransactionEntity): Color = when (tx.direction) {
    0 -> Color(0xFF2E7D32)
    1, 2 -> Color(0xFFC62828)
    3 -> Color(0xFF1565C0)
    else -> Color.Gray
}
