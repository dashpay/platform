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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import org.dashfoundation.dashsdk.persistence.entities.AccountEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.AccountDetail
import org.dashfoundation.example.ui.components.collectEachAsStateWithLifecycle
import org.dashfoundation.example.ui.dashpay.AccountBalance
import org.dashfoundation.example.ui.dashpay.parseAccountBalances
import org.dashfoundation.example.util.formatDuffs

/**
 * Per-wallet account list — port of `AccountListView.swift`. Rows come
 * from the Room `AccountDao` flow (= the SwiftData `@Query`) in the same
 * grouped display order (BIP44 → PlatformPayment → BIP32 → CoinJoin →
 * special purpose). A Core account's balance is Rust's in-memory
 * per-account read (`PlatformWalletManager.accountBalances`, ← Swift
 * `accountBalances(for:)`): nothing persists the Room balance columns,
 * which stay 0. A PlatformPayment account shows no balance line until Rust
 * offers an account-scoped credit total (#6235); a sum of its persisted
 * address rows can lag the managed account.
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
    }.collectEachAsStateWithLifecycle(initialValue = emptyList())
    val coreBalances = rememberCoreAccountBalances(walletId)

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
                balance = accountRowBalance(account, coreBalances),
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

/**
 * Rust's per-account Core balances for [walletId], read again every two
 * seconds while shown (the wallet detail's Core balance has the same
 * cadence). Null while nothing has been read: before the first read,
 * without an active manager, and after a manager change. A failed read
 * keeps the same manager's last snapshot.
 */
@Composable
internal fun rememberCoreAccountBalances(walletId: ByteArray): List<AccountBalance>? {
    val container = LocalAppContainer.current
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    var balances by remember(walletId, manager) { mutableStateOf<List<AccountBalance>?>(null) }
    LaunchedEffect(manager, walletId) {
        val m = manager ?: return@LaunchedEffect
        while (true) {
            try {
                balances = parseAccountBalances(m.accountBalances(walletId))
            } catch (e: CancellationException) {
                // The manager or wallet changed: write nothing for the old one.
                throw e
            } catch (_: Exception) {
                // Keep the last snapshot; the next read tries again.
            }
            delay(2_000)
        }
    }
    return balances
}

/** What an account row shows as its balance. */
internal sealed interface AccountRowBalance {
    /** A Core account (BIP44, BIP32): duffs, from Rust's per-account read. */
    data class Core(val confirmed: Long, val unconfirmed: Long) : AccountRowBalance

    /**
     * A Core account whose balance is not known: nothing read yet, or the
     * read has no row for it (wallet not loaded in Rust). Shown as such,
     * never as a measured 0.
     */
    data object CoreUnknown : AccountRowBalance

    /**
     * A PlatformPayment account: no balance line until Rust offers an
     * account-scoped credit total (#6235).
     */
    data object PlatformNotShown : AccountRowBalance

    /** Any other account: no balance line (special purpose). */
    data object None : AccountRowBalance
}

/**
 * The balance of [account]. A Core account matches Rust's row on type tag,
 * standard tag and index, as `AccountListView` does; with no read yet
 * ([coreBalances] null) or no matching row it is [AccountRowBalance.CoreUnknown].
 */
internal fun accountRowBalance(
    account: AccountEntity,
    coreBalances: List<AccountBalance>?,
): AccountRowBalance = when (account.accountType) {
    0, 1 -> {
        val match = coreBalances?.firstOrNull {
            it.typeTag == account.accountType &&
                it.standardTag == account.standardTag &&
                it.index == account.accountIndex
        }
        if (match == null) {
            AccountRowBalance.CoreUnknown
        } else {
            AccountRowBalance.Core(match.confirmed, match.unconfirmed)
        }
    }
    14 -> AccountRowBalance.PlatformNotShown
    else -> AccountRowBalance.None
}

/** One account row (← `AccountRowView`). */
@Composable
private fun AccountRow(
    account: AccountEntity,
    balance: AccountRowBalance,
    modifier: Modifier = Modifier,
) {
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

        when (balance) {
            is AccountRowBalance.Core -> Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(
                    "Confirmed ${formatDuffs(balance.confirmed)}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                // confirmed + unconfirmed is key-wallet's `spendable()`;
                // immature and locked funds are not in it.
                Text(
                    "Spendable ${formatDuffs(balance.confirmed + balance.unconfirmed)}",
                    style = MaterialTheme.typography.bodySmall,
                )
            }
            AccountRowBalance.CoreUnknown -> Text(
                "Balance not read yet",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            AccountRowBalance.PlatformNotShown -> Unit
            AccountRowBalance.None -> Text(
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
