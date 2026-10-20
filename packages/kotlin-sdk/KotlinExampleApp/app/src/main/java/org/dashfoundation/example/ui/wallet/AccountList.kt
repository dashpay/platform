package org.dashfoundation.example.ui.wallet

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.dashsdk.persistence.entities.AccountEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.AccountDetail
import org.dashfoundation.example.util.formatDuffs

/**
 * Per-wallet account list — port of `AccountListView.swift`. Rows come
 * from the Room `AccountDao` flow (= the SwiftData `@Query`) in the same
 * grouped display order (BIP44 → PlatformPayment → BIP32 → CoinJoin →
 * special purpose). Balances render the persisted per-account columns —
 * the per-account in-memory FFI read (`accountBalances(for:)`) isn't
 * bridged in the Kotlin SDK yet; the persisted values are refreshed by the
 * same Rust persister callbacks.
 */
@Composable
internal fun AccountListSection(
    walletIdHex: String,
    walletId: ByteArray,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val accounts by remember(walletIdHex) {
        container.database.accountDao().observeByWallet(walletId)
    }.collectAsStateWithLifecycle(initialValue = emptyList())

    if (accounts.isEmpty()) {
        Text(
            "No accounts yet — accounts are created automatically when the wallet syncs.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(vertical = 8.dp),
        )
        return
    }

    val ordered = remember(accounts) { accounts.sortedWith(compareBy(::accountSortKey)) }
    Column {
        ordered.forEachIndexed { index, account ->
            AccountRow(
                account = account,
                modifier = Modifier
                    .clickable {
                        navController.navigate(AccountDetail(walletIdHex, account.id))
                    }
                    .testTag("accountList.row.${account.accountTypeName}.${account.accountIndex}"),
            )
            if (index < ordered.lastIndex) HorizontalDivider()
        }
    }
}

/**
 * Sort key mirroring `AccountListView.sortKey(for:)`: BIP44 leads,
 * PlatformPayment second, BIP32 third, CoinJoin after, everything else in
 * tag order.
 */
private fun accountSortKey(account: AccountEntity): String {
    val group = when (account.accountType) {
        0 -> if (account.standardTag == 0) 0 else 2
        14 -> 1
        1 -> 3
        else -> 4
    }
    return "%d-%03d-%d-%05d".format(
        group,
        account.accountType,
        account.standardTag,
        account.accountIndex,
    )
}

/** One account row (← `AccountRowView`). */
@Composable
private fun AccountRow(account: AccountEntity, modifier: Modifier = Modifier) {
    val showsBalance = account.accountType in intArrayOf(0, 1, 14)
    Column(
        modifier = modifier
            .fillMaxWidth()
            .padding(vertical = 10.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(accountLabel(account), style = MaterialTheme.typography.titleSmall)
            Text(
                account.accountTypeName,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }

        if (showsBalance) {
            val total = account.balanceConfirmed + account.balanceUnconfirmed
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(
                    "Confirmed ${formatDuffs(account.balanceConfirmed)}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    "Total ${formatDuffs(total)}",
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        } else {
            Text(
                "Special Purpose Account",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }

        // Pool summary (← "X receive / Y change" footer).
        val receive = (account.externalHighestUsed + 1).coerceAtLeast(0)
        val change = (account.internalHighestUsed + 1).coerceAtLeast(0)
        if (receive > 0 || change > 0) {
            Text(
                buildList {
                    if (receive > 0) add("$receive receive")
                    if (change > 0) add("$change change")
                }.joinToString(" • "),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

/** "BIP44 #0" style label (← `AccountRowView.label`). */
internal fun accountLabel(account: AccountEntity): String = when (account.accountType) {
    0, 1, 14 -> "${account.accountTypeName} #${account.accountIndex}"
    else -> account.accountTypeName
}
