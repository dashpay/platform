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
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.persistence.entities.ShieldedActivityEntity
import org.dashfoundation.dashsdk.wallet.ShieldedIdentityDebitRecoveryRecord
import org.dashfoundation.dashsdk.wallet.WalletSyncEvent
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.tokens.displayLabel
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

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
    // Projected to plain values: Room entities compare by id only, so a
    // renamed identity would not recompose.
    val identityLabels by remember(walletIdHex) {
        container.database.identityDao().observeByWallet(walletId)
            .map { rows -> rows.associate { it.identityId.toHex() to it.displayLabel() } }
    }.collectAsStateWithLifecycle(initialValue = emptyMap())
    val syncState by container.shieldedService.state.collectAsStateWithLifecycle()
    val canResume by container.shieldedService.canResume.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    // A build without shielded support has no sync export to call.
    val hasShielded = remember { Sdk.hasShielded() }

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
        // Without shielded support there is no export to read the records
        // with; data an earlier build saved may still hold some, so they are
        // unreadable here, not known to be empty.
        if (!hasShielded) {
            recovery = recovery.applyFailure("This build does not support shielded recovery.")
            return@LaunchedEffect
        }
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
                    identityLabels = identityLabels,
                    isChecking = isChecking || syncState.isSyncing,
                    canCheck = hasShielded && canResume && manager != null,
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
            supportingContent = {
                // Kotlin has no detail sheet, so the fee Swift shows there
                // (`ShieldedActivityDetailView`'s "Fee") goes on the row: a
                // private document payment (type 29) moves no value and
                // costs only its fee.
                val fee = activityFeeText(row.hasFee, row.fee)
                Text(
                    listOfNotNull(supportingText(row.kindTag, row.status), fee)
                        .joinToString("\n"),
                )
            },
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
    KIND_TOKEN_TRANSFER -> "Token Transfer"
    KIND_TOKEN_UNSHIELD -> "Token Unshield"
    KIND_PRIVATE_DOCUMENT_CREATE -> "Private Document Entry"
    KIND_PRIVATE_CONTACT_REQUEST -> "Private Contact Request"
    else -> "Shielded Spend"
}

/** `ShieldedActivityKind::TokenPurchase`, a purchase from the shielded pool (type 28). */
internal const val KIND_TOKEN_PURCHASE = 10

/** `ShieldedActivityKind::TokenShieldedTransfer`, a send within a token's shielded pool (type 26). */
internal const val KIND_TOKEN_TRANSFER = 11

/** `ShieldedActivityKind::TokenUnshield`, tokens moved from a token's pool to an identity (type 27). */
internal const val KIND_TOKEN_UNSHIELD = 12

/**
 * `ShieldedActivityKind::PrivateDocumentCreate`, an entry appended to a private document
 * type's store and paid for from the shielded pool (type 29). Its amount is its fee.
 */
internal const val KIND_PRIVATE_DOCUMENT_CREATE = 13

/**
 * `ShieldedActivityKind::PrivateContactRequest`, a DashPay private contact request or the
 * private reply that accepts one (type 29). Its amount is its fee.
 */
internal const val KIND_PRIVATE_CONTACT_REQUEST = 14

/**
 * The kinds whose `amount` is a token count in the token's smallest unit
 * rather than credits: the tokens sent (11) or unshielded (12). Their fee is
 * still credits. ← Swift `ShieldedActivityKindDisplay.amountIsTokens`.
 */
internal fun amountIsTokens(kindTag: Int): Boolean =
    kindTag == KIND_TOKEN_TRANSFER || kindTag == KIND_TOKEN_UNSHIELD

/**
 * A stored activity row's raw `amount` with its unit, unsigned: token units
 * for the kinds [amountIsTokens] names, credits otherwise (a token
 * purchase's price included). For views that show the stored number as is,
 * such as the Storage Explorer. ← Swift `ShieldedActivityKindDisplay.rawAmountText`.
 */
internal fun storedActivityAmountText(kindTag: Int, amount: Long): String =
    if (amountIsTokens(kindTag)) "${amount.toULong()} token units" else "${amount.toULong()} credits"

/**
 * What a token purchase row says about its tokens, by its stored status:
 * minted only once Confirmed (1). The row is recorded before the purchase
 * is broadcast, so its kind alone says what the buyer intended, not what
 * executed. ← Swift `ShieldedActivityView.tokenPurchaseNote`.
 */
internal fun tokenPurchaseNote(status: Int): String = when (status) {
    1 -> "The tokens were minted to this wallet's shielded token pool. Wallet > Shielded Tokens " +
        "shows them after a shielded sync."
    2 -> "The purchase did not go through; no tokens were minted."
    else -> "Not confirmed yet: a shielded sync confirms the purchase once it lands. " +
        "Wallet > Shielded Tokens shows the bought tokens after that sync."
}

/**
 * What a token transfer row says about its tokens, by its stored status:
 * delivered only once Confirmed. ← Swift `ShieldedActivityView.tokenTransferNote`.
 */
internal fun tokenTransferNote(status: Int): String = when (status) {
    1 -> "The tokens were sent to the recipient in the token's shielded pool, and any change " +
        "came back to this wallet."
    2 -> "The transfer did not go through; no tokens left this wallet."
    else -> "Not confirmed yet: a shielded sync confirms the transfer once it lands. Until then " +
        "Wallet > Shielded Tokens shows the notes it spends as pending."
}

/**
 * What a token unshield row says about its tokens, by its stored status:
 * credited only once Confirmed. ← Swift `ShieldedActivityView.tokenUnshieldNote`.
 */
internal fun tokenUnshieldNote(status: Int): String = when (status) {
    1 -> "The tokens were added to the identity's token balance, and any change came back to " +
        "this wallet's shielded token pool."
    2 -> "The unshield did not go through; no tokens left this wallet."
    else -> "Not confirmed yet: a shielded sync confirms the unshield once it lands. Until then " +
        "Wallet > Shielded Tokens shows the notes it spends as pending."
}

/**
 * What a private document entry or private contact request row (kinds 13 and 14) says about
 * its amount, by its stored status: Rust records the fee as the amount, as nothing else
 * leaves the shielded pool. ← Swift `ShieldedActivityView.feeOnlySpendNote`.
 */
internal fun feeOnlySpendNote(status: Int): String = when (status) {
    1 -> "The amount is the fee, which is all that left the shielded pool."
    2 -> "It did not go through; the fee shown was not charged."
    else -> "Not confirmed yet: a shielded sync confirms it once it lands. The amount is the " +
        "fee, which is all that leaves the shielded pool."
}

/**
 * A row's supporting line: its status, and for a token kind or a fee-only spend what that
 * status means.
 */
internal fun supportingText(kindTag: Int, status: Int): String = when (kindTag) {
    KIND_TOKEN_PURCHASE -> "${statusLabel(status)}. ${tokenPurchaseNote(status)}"
    KIND_TOKEN_TRANSFER -> "${statusLabel(status)}. ${tokenTransferNote(status)}"
    KIND_TOKEN_UNSHIELD -> "${statusLabel(status)}. ${tokenUnshieldNote(status)}"
    KIND_PRIVATE_DOCUMENT_CREATE, KIND_PRIVATE_CONTACT_REQUEST ->
        "${statusLabel(status)}. ${feeOnlySpendNote(status)}"
    else -> statusLabel(status)
}

private fun statusLabel(status: Int): String = when (status) {
    0 -> "Pending"
    1 -> "Confirmed"
    2 -> "Failed"
    else -> "Unknown"
}

/**
 * 1 DASH = 1e11 credits; signed by direction (0 In +, 1 Out −, 2 Self),
 * exact down to one credit as Swift's `signedAmountText` is, so a small
 * amount is never rounded to 0. A token transfer or unshield shows its
 * token count in the token's smallest unit instead, as Rust records it.
 */
internal fun signedAmount(kindTag: Int, direction: Int, amount: Long): String {
    val sign = when (direction) {
        0 -> "+"
        1 -> "−"
        else -> ""
    }
    if (amountIsTokens(kindTag)) return "$sign${amount.toULong()} token units"
    return sign + formatCreditsExact(amount.toULong())
}

/**
 * "Fee 0.00144 DASH" for a row whose fee Rust recorded (always credits,
 * also for token kinds), or null when it has none (a restored spend).
 */
internal fun activityFeeText(hasFee: Boolean, fee: Long): String? =
    if (hasFee) "Fee ${formatCreditsExact(fee.toULong())}" else null

private fun signedAmount(row: ShieldedActivityEntity): String =
    signedAmount(row.kindTag, row.direction, row.amount)
