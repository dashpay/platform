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
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.LockOpen
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
import kotlinx.coroutines.flow.map
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
 * `firstSeen` descending. Asset-lock rows swap in the linked
 * `asset_locks.amountDuffs` (the L1 burn) for the ~0 `netAmount` — the
 * credit output is structurally self-owned, so the wallet's net diff
 * reads as a broken zero-value send without the override.
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

    // Display-order txid hex → total locked duffs, from this wallet's
    // asset-lock rows (← TransactionListView.assetLockAmountByTxid).
    // `outPointHex` is `"<display txid>:<vout>"`; one funding tx can carry
    // multiple credit outputs (DIP-0027 allows up to 255), so sum across
    // vouts to show the total DASH burned by that tx.
    val assetLockAmounts by remember(walletIdHex) {
        container.database.assetLockDao().observeByWallet(walletId).map { locks ->
            locks.groupBy { it.outPointHex.substringBefore(':') }
                .mapValues { (_, rows) -> rows.sumOf { it.amountDuffs } }
        }
    }.collectAsStateWithLifecycle(initialValue = emptyMap())

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
                    // asset_locks keys by *display-order* txid; the entity
                    // stores wire order, so flip before the lookup.
                    val lockedDuffs = assetLockAmounts[tx.txid.reversedArray().toHexString()]
                    Card(
                        onClick = {
                            navController.navigate(
                                WalletTransactionDetail(
                                    txidHex,
                                    assetLockAmountDuffs = lockedDuffs
                                        ?: WalletTransactionDetail.AMOUNT_ABSENT,
                                ),
                            )
                        },
                        modifier = Modifier.testTag("transactions.row.$txidHex"),
                        shape = MaterialTheme.shapes.large,
                        colors = CardDefaults.cardColors(
                            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
                        ),
                        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
                    ) {
                        TransactionRow(tx, assetLockAmountDuffs = lockedDuffs)
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalStdlibApi::class)
@Composable
private fun TransactionRow(tx: TransactionEntity, assetLockAmountDuffs: Long? = null) {
    val confirmed = tx.context >= 2
    val incoming = tx.direction == 0
    val amountColor = transactionColor(tx)
    val statusColor = if (confirmed) appStatusColors.success else appStatusColors.warning
    // Asset-lock / asset-unlock override the direction arrows (← typeIcon):
    // the `direction` classifier sees the credit output as self-owned, but
    // the intent is L1↔L2 credit conversion, not a send to myself.
    val icon = when {
        tx.isAssetLock -> Icons.Filled.Lock
        tx.isAssetUnlock -> Icons.Filled.LockOpen
        incoming -> Icons.Filled.ArrowDownward
        else -> Icons.Filled.ArrowUpward
    }

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
                icon,
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
                displayAmount(tx, assetLockAmountDuffs),
                style = MaterialTheme.typography.titleMedium,
                color = amountColor,
            )
            Row(
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                transactionTypeLabel(tx)?.let { label ->
                    Text(
                        label,
                        style = MaterialTheme.typography.labelSmall,
                        color = amountColor,
                        modifier = Modifier
                            .clip(MaterialTheme.shapes.small)
                            .background(amountColor.copy(alpha = 0.12f))
                            .padding(horizontal = 6.dp, vertical = 1.dp),
                    )
                }
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

// `TransactionTypeKind` discriminants (mirror of the Swift enum pinned to
// `transaction_type_to_u8` in rs-platform-wallet-ffi). Only the two kinds
// the UI branches on; 0xFF is the "not yet populated" sentinel and matches
// neither, so no branch fires falsely on pre-feature rows.
private const val KIND_ASSET_LOCK = 6
private const val KIND_ASSET_UNLOCK = 7

/** L1 burn minting L2 credits (← `PersistentTransaction.isAssetLock`). */
internal val TransactionEntity.isAssetLock: Boolean
    get() = transactionTypeKind == KIND_ASSET_LOCK

/** Withdrawal back to L1 (← `PersistentTransaction.isAssetUnlock`). */
internal val TransactionEntity.isAssetUnlock: Boolean
    get() = transactionTypeKind == KIND_ASSET_UNLOCK

/**
 * Chip label for special tx types, `null` for plain sends/receives. The
 * asset-lock/unlock names come from the typed kind; anything else
 * non-Standard falls back to the human-readable `transactionType`.
 */
internal fun transactionTypeLabel(tx: TransactionEntity): String? = when {
    tx.isAssetLock -> "Asset Lock"
    tx.isAssetUnlock -> "Asset Unlock"
    tx.transactionType.isEmpty() || tx.transactionType == "Standard" -> null
    else -> tx.transactionType
}

/**
 * Amount label (← `TransactionRowView.displayAmount`): asset locks show
 * the linked `asset_locks.amountDuffs` — the L1 DASH actually burned to
 * mint platform credits — because `netAmount` is ~0 for them (the credit
 * output is a self-owned address). When the row is a known asset lock but
 * its tracking row is gone (e.g. consumed by identity registration before
 * retention shipped), say so instead of the misleading `+0.00000000 DASH`.
 */
internal fun displayAmount(tx: TransactionEntity, assetLockAmountDuffs: Long?): String = when {
    tx.isAssetLock && assetLockAmountDuffs != null ->
        "-%.8f DASH".format(assetLockAmountDuffs.toDouble() / 100_000_000.0)
    tx.isAssetLock -> "Asset Lock (amount unknown)"
    else -> formattedNetAmount(tx)
}

/** direction: 0=incoming, 1=outgoing, 2=internal, 3=coinJoin (← typeColor). */
internal fun transactionColor(tx: TransactionEntity): Color = when {
    // Purple = credit conversion, a third axis off the send/receive
    // red/green so funding rows stand out while scanning (← iOS .purple).
    tx.isAssetLock || tx.isAssetUnlock -> Color(0xFF6A1B9A)
    tx.direction == 0 -> Color(0xFF2E7D32)
    tx.direction == 1 || tx.direction == 2 -> Color(0xFFC62828)
    tx.direction == 3 -> Color(0xFF1565C0)
    else -> Color.Gray
}
