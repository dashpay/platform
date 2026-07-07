package org.dashfoundation.example.ui.wallet

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
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
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.formatDate
import org.dashfoundation.example.util.formatDuffs
import org.dashfoundation.example.util.toHex
import java.util.Date

/**
 * One transaction's fields — port of `TransactionDetailView.swift`:
 * type/amount header, status/date/height/fee rows, and tap-to-copy
 * transaction id + block hash.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalStdlibApi::class)
@Composable
fun TransactionDetailScreen(
    txidHex: String,
    navController: NavHostController,
    /**
     * Locked duffs for asset-lock txs, passed from the list row (← the iOS
     * view's `assetLockAmountDuffs`) — `netAmount` is ~0 for these, so the
     * header substitutes the actual L1 burn. `null` for other tx types or
     * when the tracking row is gone.
     */
    assetLockAmountDuffs: Long? = null,
) {
    val container = LocalAppContainer.current
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val snackbar = remember { SnackbarHostState() }
    val txid = remember(txidHex) { txidHex.hexToByteArray() }

    val transaction by remember(txidHex) {
        container.database.transactionDao().observeByTxid(txid)
    }.collectAsStateWithLifecycle(initialValue = null)

    fun copy(text: String) {
        val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.setPrimaryClip(ClipData.newPlainText("Transaction", text))
        scope.launch { snackbar.showSnackbar("Copied to clipboard") }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Transaction Details") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        val tx = transaction
        if (tx == null) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally,
            ) { CircularProgressIndicator() }
            return@Scaffold
        }

        val confirmed = tx.context >= 2
        val typeDescription = when {
            tx.isAssetLock -> "Asset Lock"
            tx.isAssetUnlock -> "Asset Unlock"
            tx.netAmount > 0 -> "Received"
            tx.netAmount < 0 -> "Sent"
            else -> "Self-Transfer"
        }

        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            // Header (← the icon + type + amount stack).
            Column(
                modifier = Modifier.fillMaxWidth(),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Text(
                    typeDescription,
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    displayAmount(tx, assetLockAmountDuffs),
                    style = MaterialTheme.typography.headlineSmall,
                    color = transactionColor(tx),
                )
            }

            FormSection(title = "Details") {
                LabeledContent("Status", if (confirmed) "Confirmed" else "Pending")
                LabeledContent(
                    "Date",
                    if (tx.firstSeen > 0) formatDate(Date(tx.firstSeen * 1000)) else "—",
                )
                if (tx.blockHeight != 0) {
                    LabeledContent("Block Height", tx.blockHeight.toString())
                }
                // netAmount is ~0 for asset locks (self-owned credit
                // output), yet the wallet did pay the fee — keep the row.
                tx.fee?.takeIf { tx.netAmount < 0 || tx.isAssetLock }?.let { fee ->
                    LabeledContent("Network Fee", formatDuffs(fee))
                }
                LabeledContent("Type", tx.transactionType)
            }

            FormSection(title = "Transaction ID") {
                Text(
                    txidHex,
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable { copy(txidHex) }
                        .padding(vertical = 8.dp),
                )
            }

            tx.blockHash?.takeIf { it.isNotEmpty() }?.let { hash ->
                val hashHex = hash.toHex()
                FormSection(title = "Block Hash") {
                    Text(
                        hashHex,
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { copy(hashHex) }
                            .padding(vertical = 8.dp),
                    )
                }
            }
        }
    }
}
