package org.dashfoundation.example.ui.wallet

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.ArrowDownward
import androidx.compose.material.icons.filled.ArrowUpward
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.security.BiometricGate
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.FundFromAssetLock
import org.dashfoundation.example.navigation.SeedShieldedPool
import org.dashfoundation.example.navigation.SendTransaction
import org.dashfoundation.example.navigation.ShieldedActivity
import org.dashfoundation.example.navigation.ShieldedFund
import org.dashfoundation.example.navigation.TransferPlatformAddress
import org.dashfoundation.example.navigation.WalletTransactions
import org.dashfoundation.example.navigation.WithdrawPlatformAddress
import androidx.compose.ui.text.font.FontWeight
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.theme.appStatusColors
import org.dashfoundation.example.util.formatCredits
import org.dashfoundation.example.util.formatDate
import org.dashfoundation.example.util.formatDuffs
import java.util.Date

/**
 * Wallet detail — full port of `WalletDetailView.swift`: network chip,
 * balance card (Core / Platform / Shielded rows), Send/Receive actions,
 * transactions link with count badge, accounts section, and the
 * WalletInfoView responsibilities that survive on Android (overview
 * fields, biometric-gated View Seed Phrase, Verify Identity Keys).
 *
 * Core balance reads `ManagedPlatformWallet.balance()` (Rust in-memory
 * state) on a 2 s poll while visible; Platform / Shielded balances are
 * live Room flows, matching the SwiftData `@Query` sums.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalStdlibApi::class)
@Composable
fun WalletDetailScreen(
    walletIdHex: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val walletId = remember(walletIdHex) { walletIdHex.hexToByteArray() }
    val network by appState.currentNetwork.collectAsStateWithLifecycle()

    val walletFlow = remember(walletIdHex) {
        container.database.walletDao().observeByWalletId(walletId)
    }
    val wallet by walletFlow.collectAsStateWithLifecycle(initialValue = null)

    // The live Rust-side wallet handle (for the lock-free balance read).
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by remember(manager) {
        manager?.wallets ?: MutableStateFlow(emptyMap<String, ManagedPlatformWallet>())
    }.collectAsStateWithLifecycle()
    val managed = walletsMap[walletIdHex]

    var coreBalance by remember { mutableStateOf<ManagedPlatformWallet.Balance?>(null) }
    LaunchedEffect(managed) {
        while (managed != null && !managed.isClosed) {
            coreBalance = runCatching { managed.balance() }.getOrNull()
            delay(2_000)
        }
    }

    // Platform credits — BLAST-synced address balances (BalanceCardView's
    // PersistentPlatformAddress sum).
    val platformBalance by remember(walletIdHex) {
        container.database.platformAddressDao().observeByWallet(walletId)
            .map { rows -> rows.sumOf { it.balance } }
    }.collectAsStateWithLifecycle(initialValue = 0L)

    // Shielded credits — unspent-note sum, only when compiled in.
    val hasShielded = remember { Sdk.hasShielded() }
    val shieldedBalance by remember(walletIdHex) {
        if (hasShielded) {
            container.database.shieldedDao().observeUnspentNotesByWallet(walletId)
                .map { notes -> notes.sumOf { it.value } }
        } else {
            MutableStateFlow(0L)
        }
    }.collectAsStateWithLifecycle(initialValue = 0L)

    // Distinct on-chain transactions this wallet participates in — union
    // of every TXO's creating and spending tx (WalletDetailView's
    // transactionCount).
    val transactionCount by remember(walletIdHex) {
        container.database.txoDao().observeByWallet(walletId).map { txos ->
            val seen = HashSet<String>()
            txos.forEach { txo ->
                txo.txid?.let { seen.add(it.toHexString()) }
                txo.spendingTxid?.let { seen.add(it.toHexString()) }
            }
            seen.size
        }
    }.collectAsStateWithLifecycle(initialValue = 0)

    var showReceiveSheet by remember { mutableStateOf(false) }
    var showKeyHealthSheet by remember { mutableStateOf(false) }
    var revealedMnemonic by remember { mutableStateOf<String?>(null) }
    var isAuthorizingSeed by remember { mutableStateOf(false) }
    var showDeleteConfirmation by remember { mutableStateOf(false) }
    var isDeleting by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(wallet?.name?.takeIf { it.isNotBlank() } ?: "Wallet") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        val current = wallet
        if (current == null) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(24.dp),
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
            // Prominent balance header + network caption (← BalanceCardView hero).
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 8.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Text(
                    "BALANCE",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.primary,
                )
                Text(
                    formatDuffs(coreBalance?.confirmed ?: 0),
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.Bold,
                )
                val incoming = coreBalance?.unconfirmed ?: 0
                if (incoming > 0) {
                    Text(
                        "+${formatDuffs(incoming)} incoming",
                        style = MaterialTheme.typography.bodyMedium,
                        color = appStatusColors.success,
                    )
                }
                Text(
                    "Network: ${network.displayName}",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            // Balance breakdown (← BalanceCardView): Core / Platform / Shielded,
            // each shown in DASH. The Platform row's actions (Top Up from Core,
            // Transfer, Withdraw) live in its trailing overflow menu, iOS-style —
            // no separate "Platform Credits" section.
            FormSection(title = "Balances") {
                val core = coreBalance
                // Empty-wallet placeholder (← WalletDetailView.swift:1085-1094
                // allZero branch): a wallet with zero Core + Platform +
                // Shielded shows the single label instead of three zero rows.
                // Includes immature (a row this screen renders that the
                // Swift surface doesn't) so mined-but-unmatured funds are
                // never hidden behind the placeholder.
                val allZero = (core?.confirmed ?: 0) == 0L &&
                    (core?.unconfirmed ?: 0) == 0L &&
                    (core?.immature ?: 0) == 0L &&
                    platformBalance == 0L &&
                    shieldedBalance == 0L
                if (allZero) {
                    Text(
                        "Empty Wallet",
                        style = MaterialTheme.typography.headlineMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.testTag("walletDetail.emptyWallet"),
                    )
                    return@FormSection
                }
                LabeledContent("Core Balance", formatDuffs(core?.confirmed ?: 0))
                if ((core?.unconfirmed ?: 0) > 0) {
                    LabeledContent("Incoming", "+${formatDuffs(core!!.unconfirmed)}")
                }
                if ((core?.immature ?: 0) > 0) {
                    LabeledContent("Immature", formatDuffs(core!!.immature))
                }
                PlatformBalanceRow(
                    value = formatCredits(platformBalance),
                    onTopUp = { navController.navigate(FundFromAssetLock(walletIdHex)) },
                    onTransfer = { navController.navigate(TransferPlatformAddress(walletIdHex)) },
                    onWithdraw = { navController.navigate(WithdrawPlatformAddress(walletIdHex)) },
                )
                if (hasShielded) {
                    ShieldedBalanceRow(
                        value = formatCredits(shieldedBalance),
                        onSeedPool = { navController.navigate(SeedShieldedPool(walletIdHex)) },
                        onFund = { navController.navigate(ShieldedFund(walletIdHex)) },
                        onActivity = { navController.navigate(ShieldedActivity(walletIdHex)) },
                    )
                }
            }

            // Action buttons (← the Send / Receive strip).
            Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                Button(
                    onClick = { navController.navigate(SendTransaction(walletIdHex)) },
                    modifier = Modifier
                        .weight(1f)
                        .testTag("walletDetail.sendButton"),
                ) {
                    Icon(Icons.Default.ArrowUpward, contentDescription = null)
                    Spacer(Modifier.width(8.dp))
                    Text("Send")
                }
                OutlinedButton(
                    onClick = { showReceiveSheet = true },
                    modifier = Modifier
                        .weight(1f)
                        .testTag("walletDetail.receiveButton"),
                ) {
                    Icon(Icons.Default.ArrowDownward, contentDescription = null)
                    Spacer(Modifier.width(8.dp))
                    Text("Receive")
                }
            }

            // Transactions (← "View All Transactions" row + count badge).
            FormSection(title = "Transactions") {
                TextButton(
                    onClick = { navController.navigate(WalletTransactions(walletIdHex)) },
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("walletDetail.transactionsLink"),
                ) {
                    Icon(Icons.AutoMirrored.Filled.List, contentDescription = null)
                    Spacer(Modifier.width(8.dp))
                    Text("View All Transactions")
                    if (transactionCount > 0) {
                        Spacer(Modifier.width(4.dp))
                        Text("($transactionCount)")
                    }
                }
            }

            // Accounts (← the Accounts header + AccountListView).
            FormSection(title = "Accounts") {
                AccountListSection(
                    walletIdHex = walletIdHex,
                    walletId = walletId,
                    navController = navController,
                )
            }

            // Overview (← WalletInfoView's Information section).
            FormSection(title = "Overview") {
                LabeledContent("Wallet ID", walletIdHex.take(16) + "…")
                LabeledContent("Birth height", current.birthHeight.toString())
                LabeledContent("Synced height", current.syncedHeight.toString())
                LabeledContent(
                    "Last synced",
                    if (current.lastSynced == 0L) "Never" else formatDate(Date(current.lastSynced * 1000)),
                )
                LabeledContent("Imported", if (current.isImported) "Yes" else "No")
                LabeledContent("Created", formatDate(current.createdAt))
            }

            // Security (← WalletInfoView's View Seed Phrase + Verify
            // Identity Keys sections).
            FormSection(title = "Security") {
                TextButton(
                    onClick = {
                        if (isAuthorizingSeed) return@TextButton
                        scope.launch {
                            isAuthorizingSeed = true
                            when (
                                container.biometricGate.authenticate(
                                    title = "View Seed Phrase",
                                    subtitle = "Reveal your wallet's recovery phrase.",
                                )
                            ) {
                                BiometricGate.AuthOutcome.AUTHORIZED -> {
                                    val phrase = runCatching {
                                        container.walletStorage.retrieveMnemonic(walletId)
                                    }.getOrNull()
                                    if (phrase.isNullOrBlank()) {
                                        error = "This wallet's recovery phrase isn't stored on this device."
                                    } else {
                                        revealedMnemonic = phrase
                                    }
                                }

                                BiometricGate.AuthOutcome.DENIED -> Unit

                                BiometricGate.AuthOutcome.UNAVAILABLE ->
                                    error = "Authentication is unavailable on this device."

                                BiometricGate.AuthOutcome.FAILED ->
                                    error = "Authorization failed."
                            }
                            isAuthorizingSeed = false
                        }
                    },
                    enabled = !isAuthorizingSeed,
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("walletInfo.viewSeedPhraseButton"),
                ) {
                    if (isAuthorizingSeed) {
                        CircularProgressIndicator(modifier = Modifier.padding(end = 8.dp))
                    }
                    Text("View Seed Phrase")
                }

                TextButton(
                    onClick = { showKeyHealthSheet = true },
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("walletInfo.verifyIdentityKeysButton"),
                ) { Text("Verify Identity Keys") }
            }

            // Danger zone (← WalletInfoView's destructive "Delete Wallet"
            // section, kept last so the destructive action stays at the
            // bottom). Fully wipes this wallet's Rust / Room / Keystore
            // footprint (CORE-17).
            FormSection(title = "Danger Zone") {
                Button(
                    onClick = { showDeleteConfirmation = true },
                    enabled = !isDeleting,
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.error,
                        contentColor = MaterialTheme.colorScheme.onError,
                    ),
                    modifier = Modifier
                        .fillMaxWidth()
                        .testTag("walletDetail.deleteWalletButton"),
                ) {
                    if (isDeleting) {
                        CircularProgressIndicator(
                            modifier = Modifier.padding(end = 8.dp),
                            color = MaterialTheme.colorScheme.onError,
                        )
                    } else {
                        Icon(Icons.Default.Delete, contentDescription = null)
                        Spacer(Modifier.width(8.dp))
                    }
                    Text("Delete Wallet")
                }
            }
        }
    }

    if (showReceiveSheet) {
        ReceiveAddressSheet(
            walletId = walletId,
            onDismiss = { showReceiveSheet = false },
        )
    }

    if (showKeyHealthSheet) {
        WalletKeyHealthSheet(
            walletId = walletId,
            onDismiss = { showKeyHealthSheet = false },
        )
    }

    revealedMnemonic?.let { phrase ->
        SeedPhraseRevealSheet(
            mnemonic = phrase,
            onDismiss = { revealedMnemonic = null },
        )
    }

    // Delete-wallet confirmation (← WalletInfoView's destructive alert).
    // On confirm: the manager's removeWallet handles the full wipe (Rust
    // unregister → Keystore key sweep → Room cascade → mnemonic delete),
    // then we pop back to the Wallets list.
    if (showDeleteConfirmation) {
        AlertDialog(
            onDismissRequest = { if (!isDeleting) showDeleteConfirmation = false },
            title = { Text("Delete Wallet") },
            text = {
                Text(
                    "This permanently removes the wallet, its mnemonic, and all " +
                        "its identities/addresses from this device. This cannot be " +
                        "undone unless you have backed up your recovery phrase.",
                )
            },
            confirmButton = {
                TextButton(
                    enabled = !isDeleting,
                    onClick = {
                        val mgr = manager
                        if (mgr == null) {
                            error = "Wallet manager is not available."
                            showDeleteConfirmation = false
                            return@TextButton
                        }
                        scope.launch {
                            isDeleting = true
                            val result = runCatching { mgr.removeWallet(walletId) }
                            isDeleting = false
                            showDeleteConfirmation = false
                            result.fold(
                                onSuccess = { navController.popBackStack() },
                                onFailure = { error = "Failed to delete wallet: ${it.message}" },
                            )
                        }
                    },
                    colors = ButtonDefaults.textButtonColors(
                        contentColor = MaterialTheme.colorScheme.error,
                    ),
                    modifier = Modifier.testTag("walletDetail.deleteWalletConfirm"),
                ) { Text("Delete") }
            },
            dismissButton = {
                TextButton(
                    enabled = !isDeleting,
                    onClick = { showDeleteConfirmation = false },
                ) { Text("Cancel") }
            },
        )
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

/**
 * Platform Balance row — the DASH value with a trailing overflow menu:
 * Top Up (Core→Platform), Transfer, Withdraw to Core. Mirrors the iOS
 * `BalanceCardView` Platform row and replaces the old "Platform Credits"
 * section; the transfer/withdraw testTags carry over for UAT parity.
 */
@Composable
private fun PlatformBalanceRow(
    value: String,
    onTopUp: () -> Unit,
    onTransfer: () -> Unit,
    onWithdraw: () -> Unit,
) {
    var menuOpen by remember { mutableStateOf(false) }
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 7.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            "Platform Balance",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Text(
            value,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.End,
            modifier = Modifier
                .weight(1f)
                .padding(end = 4.dp),
        )
        Box {
            IconButton(
                onClick = { menuOpen = true },
                modifier = Modifier
                    .size(28.dp)
                    .testTag("walletDetail.platformBalanceMenu"),
            ) {
                Icon(
                    Icons.Default.MoreVert,
                    contentDescription = "Platform Balance actions",
                    tint = MaterialTheme.colorScheme.primary,
                )
            }
            DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                DropdownMenuItem(
                    text = { Text("Top Up from Core") },
                    onClick = {
                        menuOpen = false
                        onTopUp()
                    },
                    modifier = Modifier.testTag("walletDetail.topUpPlatformButton"),
                )
                DropdownMenuItem(
                    text = { Text("Transfer") },
                    onClick = {
                        menuOpen = false
                        onTransfer()
                    },
                    modifier = Modifier.testTag("walletDetail.transferPlatformButton"),
                )
                DropdownMenuItem(
                    text = { Text("Withdraw to Core") },
                    onClick = {
                        menuOpen = false
                        onWithdraw()
                    },
                    modifier = Modifier.testTag("walletDetail.withdrawPlatformButton"),
                )
            }
        }
    }
}

/**
 * Shielded Balance row — the DASH value with a trailing overflow menu:
 * Fund Shielded (Core → pool), Seed Pool Notes, Shielded Activity. Wires up
 * the Orchard shielded screens (← iOS `BalanceCardView` Shielded row and the
 * SeedShieldedPool / ShieldedActivity affordances). Only rendered on a
 * shielded build.
 */
@Composable
private fun ShieldedBalanceRow(
    value: String,
    onSeedPool: () -> Unit,
    onFund: () -> Unit,
    onActivity: () -> Unit,
) {
    var menuOpen by remember { mutableStateOf(false) }
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 7.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            "Shielded Balance",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Text(
            value,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.End,
            modifier = Modifier
                .weight(1f)
                .padding(end = 4.dp),
        )
        Box {
            IconButton(
                onClick = { menuOpen = true },
                modifier = Modifier
                    .size(28.dp)
                    .testTag("walletDetail.shieldedBalanceMenu"),
            ) {
                Icon(
                    Icons.Default.MoreVert,
                    contentDescription = "Shielded Balance actions",
                    tint = MaterialTheme.colorScheme.primary,
                )
            }
            DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                DropdownMenuItem(
                    text = { Text("Fund Shielded") },
                    onClick = {
                        menuOpen = false
                        onFund()
                    },
                    modifier = Modifier.testTag("walletDetail.fundShieldedButton"),
                )
                DropdownMenuItem(
                    text = { Text("Seed Pool Notes") },
                    onClick = {
                        menuOpen = false
                        onSeedPool()
                    },
                    modifier = Modifier.testTag("walletDetail.seedShieldedPoolButton"),
                )
                DropdownMenuItem(
                    text = { Text("Shielded Activity") },
                    onClick = {
                        menuOpen = false
                        onActivity()
                    },
                    modifier = Modifier.testTag("walletDetail.shieldedActivityButton"),
                )
            }
        }
    }
}
