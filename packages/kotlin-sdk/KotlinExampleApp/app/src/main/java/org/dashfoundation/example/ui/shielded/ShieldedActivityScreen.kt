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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.LifecycleResumeEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.persistence.entities.ShieldedActivityEntity
import org.dashfoundation.dashsdk.wallet.ShieldedIdentityDebitRecoveryRecord
import org.dashfoundation.dashsdk.wallet.WalletSyncEvent
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.util.hexToBytes

/**
 * Shielded activity timeline for a wallet — port of `ShieldedActivityView.swift`.
 * Reads the Room `shielded_activities` rows via
 * [org.dashfoundation.dashsdk.persistence.dao.ShieldedDao.observeActivityByWallet]:
 * a Pending section (status 0) above a History section (status != 0), each
 * row labeled + signed by its kind / direction.
 *
 * Above them, In-Progress Identity Shields lists the identity shields
 * (Type 21) whose outcome is not known yet, read from Rust's durable records
 * (`shieldedIdentityDebitRecoveryRecords`), so one interrupted by a crash or
 * a killed process is listed again after relaunch (SH-23). They are read on
 * open, on every resume, after any shielded sync result for this wallet, and
 * after Stop Retrying.
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

    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val identities by container.database.identityDao()
        .observeByWallet(walletId)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val syncState by container.shieldedService.state.collectAsStateWithLifecycle()
    val canResume by container.shieldedService.canResume.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()

    // Another wallet's or manager's records never show here: the state is
    // keyed by both. A newer read cancels the one in flight (the effect's
    // key changes), so only the newest read applies.
    var recovery by remember(manager, walletIdHex) {
        mutableStateOf(RecoveryReadState<ShieldedIdentityDebitRecoveryRecord>())
    }
    var readEpoch by remember { mutableIntStateOf(0) }
    var checkStatus by remember { mutableStateOf<CheckNowStatus>(CheckNowStatus.Idle) }
    var isChecking by remember { mutableStateOf(false) }
    var isAbandoning by remember { mutableStateOf(false) }
    var actionError by remember { mutableStateOf<String?>(null) }

    LifecycleResumeEffect(Unit) {
        readEpoch++
        onPauseOrDispose {}
    }
    LaunchedEffect(manager, walletIdHex, readEpoch) {
        val activeManager = manager ?: return@LaunchedEffect
        recovery = try {
            recovery.applySuccess(activeManager.shieldedIdentityDebitRecoveryRecords(walletId))
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            recovery.applyFailure(e.message ?: "unknown error")
        }
    }
    // A finished sync pass is what resends and resolves the records.
    LaunchedEffect(manager, walletIdHex) {
        manager?.syncEvents?.collect { event ->
            if (event is WalletSyncEvent.ShieldedResult && event.walletId.contentEquals(walletId)) {
                readEpoch++
            }
        }
    }

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
            item {
                IdentityShieldRecoverySection(
                    state = recovery,
                    identities = identities,
                    isChecking = isChecking || syncState.isSyncing,
                    canCheck = canResume && manager != null,
                    isAbandoning = isAbandoning,
                    checkStatus = checkStatus,
                    onCheckNow = {
                        val activeManager = manager ?: return@IdentityShieldRecoverySection
                        isChecking = true
                        checkStatus = CheckNowStatus.Checking
                        scope.launch {
                            checkStatus = try {
                                activeManager.syncShieldedNow()
                                CheckNowStatus.Finished
                            } catch (e: CancellationException) {
                                throw e
                            } catch (e: Exception) {
                                CheckNowStatus.Failed(e.message ?: "unknown error")
                            } finally {
                                isChecking = false
                            }
                            // The records stay listed whatever the outcome.
                            readEpoch++
                        }
                    },
                    onStopRetrying = { record ->
                        val activeManager = manager ?: return@IdentityShieldRecoverySection
                        isAbandoning = true
                        scope.launch {
                            try {
                                // Only after the user confirmed the warning.
                                activeManager.abandonShieldedIdentityDebit(
                                    walletId = walletId,
                                    accountIndex = record.accountIndex,
                                    activityId = record.activityId,
                                    acknowledgePossibleExecution = true,
                                )
                                readEpoch++
                            } catch (e: CancellationException) {
                                throw e
                            } catch (e: Exception) {
                                actionError = e.message ?: "Could not stop retrying."
                            } finally {
                                isAbandoning = false
                            }
                        }
                    },
                )
            }
            if (activity.isEmpty() && recovery.isKnownEmpty) {
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

    ErrorAlertDialog(
        message = actionError,
        title = "Could not stop retrying",
        onDismiss = { actionError = null },
    )
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
            supportingContent = { Text(supportingText(row.kindTag, row.status)) },
            trailingContent = { Text(signedAmount(row)) },
        )
    }
}

/** Kind label — ← Swift `ShieldedActivityView` kind map. */
internal fun kindLabel(kindTag: Int): String = when (kindTag) {
    0 -> "Shielded"
    1 -> "Shielded from Asset Lock"
    2 -> "Received"
    3 -> "Sent"
    4 -> "Unshielded"
    5 -> "Withdrawn"
    6 -> "Identity Created"
    8 -> "Shielded from Identity"
    9 -> "Identity Top-Up from Pool"
    KIND_TOKEN_PURCHASE -> "Token Purchase"
    else -> "Shielded Spend"
}

/** `ShieldedActivityKind::TokenPurchase`, a purchase from the shielded pool (type 28). */
internal const val KIND_TOKEN_PURCHASE = 10

/**
 * What a token purchase row says about its tokens, by its stored status:
 * minted only once Confirmed (1). The row is recorded before the purchase
 * is broadcast, so its kind alone says what the buyer intended, not what
 * executed. ← Swift `ShieldedActivityView.tokenPurchaseNote`.
 */
internal fun tokenPurchaseNote(status: Int): String = when (status) {
    1 -> "The tokens were minted to this wallet's shielded token pool, which the app does not show yet."
    2 -> "The purchase did not go through; no tokens were minted."
    else -> "Not confirmed yet: a shielded sync confirms the purchase once it lands. " +
        "The app does not show the shielded token pool yet."
}

/** A row's supporting line: its status, and for a token purchase what that status means. */
internal fun supportingText(kindTag: Int, status: Int): String =
    if (kindTag == KIND_TOKEN_PURCHASE) {
        "${statusLabel(status)}. ${tokenPurchaseNote(status)}"
    } else {
        statusLabel(status)
    }

private fun statusLabel(status: Int): String = when (status) {
    0 -> "Pending"
    1 -> "Confirmed"
    2 -> "Failed"
    else -> "Unknown"
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
