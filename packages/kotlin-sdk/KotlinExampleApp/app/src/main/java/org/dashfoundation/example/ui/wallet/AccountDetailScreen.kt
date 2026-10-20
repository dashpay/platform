package org.dashfoundation.example.ui.wallet

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.flow.map
import org.dashfoundation.dashsdk.persistence.entities.AccountEntity
import org.dashfoundation.dashsdk.persistence.entities.CoreAddressEntity
import org.dashfoundation.dashsdk.persistence.entities.TxoEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.formatDuffs
import org.dashfoundation.example.util.truncateMiddle

/**
 * Account detail — port of `AccountDetailView.swift`: overview card,
 * balance card, address-pool summary, and the per-pool address lists
 * (External / Internal / Absent / Absent-Hardened) plus this account's
 * UTXO set, all read from Room (`CoreAddressDao` / `TxoDao`).
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalStdlibApi::class)
@Composable
fun AccountDetailScreen(
    walletIdHex: String,
    accountId: Long,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val walletId = remember(walletIdHex) { walletIdHex.hexToByteArray() }

    val account by produceState<AccountEntity?>(initialValue = null, accountId) {
        value = container.database.accountDao().getById(accountId)
    }

    val addresses by remember(accountId) {
        container.database.coreAddressDao().observeByAccount(accountId)
    }.collectAsStateWithLifecycle(initialValue = emptyList())

    // This account's TXOs — walletId scan narrowed to the account pointer
    // (the Kotlin analogue of walking `coreAddresses.flatMap(\.txos)`).
    val txos by remember(walletIdHex, accountId) {
        container.database.txoDao().observeByWallet(walletId)
            .map { rows -> rows.filter { it.accountId == accountId } }
    }.collectAsStateWithLifecycle(initialValue = emptyList())

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(account?.accountTypeName ?: "Account") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        val current = account
        if (current == null) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally,
            ) { CircularProgressIndicator() }
            return@Scaffold
        }

        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Account Information") {
                LabeledContent("Type", current.accountTypeName)
                LabeledContent("Index", "#${current.accountIndex}")
                LabeledContent("Network", network.displayName)
            }

            if (current.accountType in intArrayOf(0, 1, 14)) {
                FormSection(title = "Balance") {
                    LabeledContent("Confirmed", formatDuffs(current.balanceConfirmed))
                    if (current.balanceUnconfirmed > 0) {
                        LabeledContent("Pending", formatDuffs(current.balanceUnconfirmed))
                    }
                    LabeledContent(
                        "Total",
                        formatDuffs(current.balanceConfirmed + current.balanceUnconfirmed),
                    )
                }
            }

            // Pool summary (← poolSummaryCard).
            FormSection(title = "Address Pool") {
                val externalCount = addresses.count { it.poolTypeTag == 0 }
                val internalCount = addresses.count { it.poolTypeTag == 1 }
                if (externalCount > 0) {
                    LabeledContent("Pool Size (External)", externalCount.toString())
                }
                if (internalCount > 0) {
                    LabeledContent("Pool Size (Internal)", internalCount.toString())
                }
                LabeledContent(
                    "Highest Used (External)",
                    if (current.externalHighestUsed >= 0) current.externalHighestUsed.toString() else "—",
                )
                LabeledContent(
                    "Highest Used (Internal)",
                    if (current.internalHighestUsed >= 0) current.internalHighestUsed.toString() else "—",
                )
                LabeledContent("Transactions", distinctTransactionCount(txos).toString())
                LabeledContent("TXOs", txos.size.toString())
            }

            // Per-pool address lists (← addressSections()).
            val pools = listOf(
                0 to "External",
                1 to "Internal",
                2 to "Absent",
                3 to "Absent (Hardened)",
            )
            pools.forEach { (tag, name) ->
                val bucket = addresses.filter { it.poolTypeTag == tag }
                    .sortedBy { it.addressIndex }
                if (bucket.isNotEmpty()) {
                    FormSection(title = "$name Addresses (${bucket.size})") {
                        bucket.forEach { AddressRow(it) }
                    }
                }
            }
            if (addresses.isEmpty()) {
                FormSection(title = "Addresses") {
                    Text(
                        "No addresses have been persisted for this account yet. " +
                            "They land here after the wallet is (re)created via the wallet manager.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(vertical = 8.dp),
                    )
                }
            }

            // UTXO list — unspent first, then spent (history stays whole,
            // mirroring the never-delete TXO model).
            if (txos.isNotEmpty()) {
                FormSection(title = "UTXOs (${txos.size})") {
                    txos.sortedWith(
                        compareBy<TxoEntity> { it.isSpent }.thenByDescending { it.height },
                    ).forEach { UtxoRow(it) }
                }
            }
        }
    }
}

/** Distinct creating/spending txids across the account's TXOs. */
@OptIn(ExperimentalStdlibApi::class)
private fun distinctTransactionCount(txos: List<TxoEntity>): Int {
    val seen = HashSet<String>()
    txos.forEach { txo ->
        txo.txid?.let { seen.add(it.toHexString()) }
        txo.spendingTxid?.let { seen.add(it.toHexString()) }
    }
    return seen.size
}

@Composable
private fun AddressRow(address: CoreAddressEntity) {
    Column(modifier = Modifier.padding(vertical = 6.dp)) {
        Text(
            address.address,
            style = MaterialTheme.typography.bodySmall,
            fontFamily = FontFamily.Monospace,
            maxLines = 1,
        )
        val detail = buildList {
            add("#${address.addressIndex}")
            if (address.isUsed) add("used")
            if (address.balance > 0) add(formatDuffs(address.balance))
        }.joinToString(" • ")
        Text(
            detail,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@OptIn(ExperimentalStdlibApi::class)
@Composable
private fun UtxoRow(txo: TxoEntity) {
    Column(modifier = Modifier.padding(vertical = 6.dp)) {
        Text(
            truncateMiddle(txo.outpoint.toHexString(), head = 12, tail = 8),
            style = MaterialTheme.typography.bodySmall,
            fontFamily = FontFamily.Monospace,
        )
        val detail = buildList {
            add(formatDuffs(txo.amount))
            add(if (txo.isSpent) "spent" else "unspent")
            if (txo.height > 0) add("height ${txo.height}")
            if (txo.isInstantLocked) add("instant-locked")
        }.joinToString(" • ")
        Text(
            detail,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}
