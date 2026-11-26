package org.dashfoundation.example.ui.shielded

import androidx.activity.compose.BackHandler
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
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.LifecycleResumeEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.dashsdk.wallet.ShieldedTokenPoolBalance
import org.dashfoundation.dashsdk.wallet.ShieldedTokenPoolSpendKind
import org.dashfoundation.dashsdk.wallet.ShieldedTokenSpendPreview
import org.dashfoundation.dashsdk.wallet.WalletSyncEvent
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.collectEachAsStateWithLifecycle
import org.dashfoundation.example.ui.tokens.displayLabel
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.DashAddress
import org.dashfoundation.example.util.formatCredits
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

// Spending a wallet's notes in a token's shielded pool: Send (type 26, SH-26)
// and Unshield to an identity (type 27, SH-27). Port of
// `ShieldedTokenSendSheet`, `ShieldedTokenUnshieldSheet`,
// `ShieldedTokenSpendFeeLine`, `ShieldedTokenAmountFeedback` and
// `ShieldedTokenSpendSourceSection` in
// `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Core/Views/ShieldedTokenPoolsView.swift`;
// the accessibility identifiers are reused verbatim as test tags.
//
// Rust decides everything: it checks the amount, the token and the network,
// picks the token notes and the credit pool notes that pay the fee, proves,
// broadcasts and records the activity. The forms only read the recipient and
// the amount (a whole number of the token's smallest unit, passed unchanged;
// cached decimals only give a hint), and show Rust's fee preview and answer.
// An accepted spend is reported as submitted, never as complete.

/**
 * Holds a spend form's draft, its call and its answer for the form's
 * back-stack entry, so a rotation or a tab switch while the proof runs
 * neither drops the answer nor lets a second spend start. ← the role of
 * Swift's `@State` plus `interactiveDismissDisabled`.
 *
 * The draft (recipient and amount) lives here and not in saved instance
 * state on purpose: after Android ends the process the ViewModel is gone,
 * and so must the draft be. A restored form must never offer a recipient
 * and amount that may already have been sent; it starts empty, with the
 * earlier spend's notes shown as Pending and its activity row pending, as a
 * Swift sheet does after a relaunch.
 */
class ShieldedTokenSpendViewModel : ViewModel() {

    /** The recipient as typed. */
    var recipientText by mutableStateOf("")

    /** The amount as typed, in the token's smallest unit. */
    var amountText by mutableStateOf("")

    var isSubmitting by mutableStateOf(false)
        private set

    /** Rust accepted the spend, or it may have gone through: either way it must not be sent again. */
    internal var result by mutableStateOf<ShieldedTokenSpendResult?>(null)
        private set

    /** A refusal: nothing was sent, so the form may be corrected and sent. */
    var error by mutableStateOf<String?>(null)
        private set

    /** Run [spend] once; ignored while one runs or after one was sent. */
    internal fun submit(kind: ShieldedTokenPoolSpendKind, spend: suspend () -> Unit) {
        if (isSubmitting || result != null) return
        isSubmitting = true
        error = null
        viewModelScope.launch {
            try {
                result = shieldedTokenSpendResult(kind, spend)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                error = e.message ?: e.toString()
            } finally {
                isSubmitting = false
            }
        }
    }
}

/**
 * Send tokens from one account's notes in a token's shielded pool to an
 * Orchard address in that pool (type 26) through
 * [PlatformWalletManager.shieldedTokenTransfer]. The recipient is a typed or
 * pasted bech32m Orchard address on the current network, or this account's
 * own one (**My Address**). ← Swift `ShieldedTokenSendSheet`.
 */
@Composable
fun ShieldedTokenSendScreen(
    walletIdHex: String,
    tokenIdHex: String,
    account: Int,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val viewModel: ShieldedTokenSpendViewModel = viewModel()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletId = remember(walletIdHex) { walletIdHex.hexToBytes() }
    val recipientText = viewModel.recipientText
    // The recipient is read against the network of the manager that spends
    // and broadcasts, never the app's selected network: during a network
    // switch the two can differ, and an address of the other network must
    // not pass.
    val spendingNetwork = manager?.network
    val recipientRaw = spendingNetwork?.let { shieldedTokenOrchardRecipient(recipientText, it) }
    // This account's own default Orchard address, once the account is bound,
    // encoded for that same manager's network.
    val ownAddress by produceState<String?>(null, manager, account) {
        value = manager?.let { m ->
            runCatching { m.shieldedDefaultAddress(walletId, account) }.getOrNull()
                ?.let { raw -> DashAddress.encodeOrchard(raw, m.network) }
        }
    }

    ShieldedTokenSpendForm(
        viewModel = viewModel,
        walletIdHex = walletIdHex,
        tokenIdHex = tokenIdHex,
        account = account,
        navController = navController,
        kind = ShieldedTokenPoolSpendKind.ShieldedTransfer,
        idPrefix = "shieldedTokenSend",
        title = "Send Shielded Tokens",
        submitLabel = "Send",
        recipientReady = recipientRaw != null,
        amountFooter = "A whole number of the token's smallest unit, sent exactly as typed.",
        submitFooter = "The fee is paid from this account's shielded credit balance.",
        recipientSection = { enabled ->
            FormSection(title = "Recipient") {
                OutlinedTextField(
                    value = recipientText,
                    onValueChange = { viewModel.recipientText = it },
                    placeholder = { Text("tdash1… or dash1…") },
                    enabled = enabled,
                    textStyle = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                    modifier = Modifier.fillMaxWidth().testTag("shieldedTokenSend.recipient"),
                )
                ownAddress?.let { address ->
                    TextButton(
                        onClick = { viewModel.recipientText = address },
                        enabled = enabled,
                        modifier = Modifier.testTag("shieldedTokenSend.ownAddressButton"),
                    ) { Text("My Address") }
                }
                if (recipientText.isNotEmpty() && recipientRaw == null) {
                    Text(
                        "Not a shielded (Orchard) address on this network.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
                Text(
                    "A shielded (Orchard) address. The tokens stay in the token's shielded pool.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        },
        spend = { activeManager, balance, amount ->
            val recipient = recipientRaw ?: return@ShieldedTokenSpendForm null
            suspend {
                activeManager.shieldedTokenTransfer(
                    walletId = walletId,
                    contractId = balance.dataContractId,
                    tokenPosition = balance.tokenContractPosition,
                    recipientRaw43 = recipient,
                    amount = amount,
                    account = balance.accountIndex,
                )
            }
        },
    )
}

/**
 * Move tokens from one account's notes in a token's shielded pool into an
 * existing identity's token balance (type 27) through
 * [PlatformWalletManager.shieldedTokenUnshield]. The recipient is one of this
 * wallet's identities (**My Identities**) or a typed Base58 identity id.
 * ← Swift `ShieldedTokenUnshieldSheet`.
 */
@Composable
fun ShieldedTokenUnshieldScreen(
    walletIdHex: String,
    tokenIdHex: String,
    account: Int,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val viewModel: ShieldedTokenSpendViewModel = viewModel()
    val walletId = remember(walletIdHex) { walletIdHex.hexToBytes() }
    // Projected to plain values: Room entities compare by id only.
    val identities by remember(walletIdHex) {
        container.database.identityDao().observeByWallet(walletId)
            .map { rows -> rows.map { Base58.encode(it.identityId) to it.displayLabel() } }
    }.collectAsStateWithLifecycle(initialValue = emptyList())
    val recipientText = viewModel.recipientText
    val recipientId = shieldedTokenIdentityRecipient(recipientText)
    val recipientName = recipientId?.let { id ->
        val idBase58 = Base58.encode(id)
        identities.firstOrNull { it.first == idBase58 }?.second
    }
    var pickerOpen by remember { mutableStateOf(false) }

    ShieldedTokenSpendForm(
        viewModel = viewModel,
        walletIdHex = walletIdHex,
        tokenIdHex = tokenIdHex,
        account = account,
        navController = navController,
        kind = ShieldedTokenPoolSpendKind.Unshield,
        idPrefix = "shieldedTokenUnshield",
        title = "Unshield Tokens",
        submitLabel = "Unshield",
        recipientReady = recipientId != null,
        amountFooter = "A whole number of the token's smallest unit, sent exactly as typed. " +
            "The fee is paid from this account's shielded credit balance.",
        submitFooter = null,
        recipientSection = { enabled ->
            FormSection(title = "Recipient Identity") {
                OutlinedTextField(
                    value = recipientText,
                    onValueChange = { viewModel.recipientText = it },
                    placeholder = { Text("Identity ID (Base58)") },
                    enabled = enabled,
                    textStyle = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                    modifier = Modifier.fillMaxWidth().testTag("shieldedTokenUnshield.recipient"),
                )
                if (identities.isNotEmpty()) {
                    Box {
                        TextButton(
                            onClick = { pickerOpen = true },
                            enabled = enabled,
                            modifier = Modifier.testTag("shieldedTokenUnshield.identityPicker"),
                        ) { Text("My Identities") }
                        DropdownMenu(expanded = pickerOpen, onDismissRequest = { pickerOpen = false }) {
                            identities.forEach { (idBase58, label) ->
                                DropdownMenuItem(
                                    text = { Text(label) },
                                    onClick = {
                                        pickerOpen = false
                                        viewModel.recipientText = idBase58
                                    },
                                    modifier = Modifier.testTag("shieldedTokenUnshield.identity.$idBase58"),
                                )
                            }
                        }
                    }
                }
                when {
                    recipientName != null -> Text(
                        recipientName,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.testTag("shieldedTokenUnshield.recipientName"),
                    )
                    recipientText.isNotEmpty() && recipientId == null -> Text(
                        "Not a Base58 identity id.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
                Text(
                    "The tokens leave the shielded pool and are added to this identity's token " +
                        "balance, where they are public.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        },
        spend = { activeManager, balance, amount ->
            val recipient = recipientId ?: return@ShieldedTokenSpendForm null
            suspend {
                activeManager.shieldedTokenUnshield(
                    walletId = walletId,
                    contractId = balance.dataContractId,
                    tokenPosition = balance.tokenContractPosition,
                    recipientIdentityId = recipient,
                    amount = amount,
                    account = balance.accountIndex,
                )
            }
        },
    )
}

/**
 * The parts Send and Unshield share: the row the form spends from, the
 * amount with its feedback and Rust's fee preview, the submit button, and the
 * result or refusal. [spend] builds the wallet call for the current inputs,
 * or returns null while the recipient does not parse.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
@Suppress("LongParameterList", "LongMethod")
private fun ShieldedTokenSpendForm(
    viewModel: ShieldedTokenSpendViewModel,
    walletIdHex: String,
    tokenIdHex: String,
    account: Int,
    navController: NavHostController,
    kind: ShieldedTokenPoolSpendKind,
    idPrefix: String,
    title: String,
    submitLabel: String,
    recipientReady: Boolean,
    amountFooter: String,
    submitFooter: String?,
    recipientSection: @Composable (enabled: Boolean) -> Unit,
    spend: (PlatformWalletManager, ShieldedTokenPoolBalance, ULong) -> (suspend () -> Unit)?,
) {
    val container = LocalAppContainer.current
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletId = remember(walletIdHex) { walletIdHex.hexToBytes() }
    // The token's name and decimals (display only) for the spending
    // manager's network.
    val spendingNetwork = manager?.network
    val tokens by remember(spendingNetwork) {
        spendingNetwork?.let { container.database.tokenDao().observeTokensByNetwork(it.ffiValue) }
            ?: flowOf(emptyList())
    }.collectEachAsStateWithLifecycle(initialValue = emptyList())

    // The row this form spends from, read again on every return and after
    // every shielded sync result for the wallet, so Available and Pending
    // and the fee follow the notes Rust prices from.
    var stateRevision by remember { mutableIntStateOf(0) }
    LifecycleResumeEffect(Unit) {
        stateRevision++
        onPauseOrDispose {}
    }
    LaunchedEffect(manager, walletIdHex) {
        manager?.syncEvents?.collect { event ->
            if (event is WalletSyncEvent.ShieldedResult && event.walletId.contentEquals(walletId)) {
                stateRevision++
            }
        }
    }
    val rows by produceState<SourceRows>(SourceRows.Loading, manager, walletIdHex, stateRevision) {
        val activeManager = manager ?: return@produceState
        value = try {
            SourceRows.Loaded(activeManager.shieldedTokenPoolBalances(walletId))
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            SourceRows.Failed(e.message ?: "unknown error")
        }
    }
    val tokenRows = (rows as? SourceRows.Loaded)?.balances.orEmpty().filter { it.tokenId.toHex() == tokenIdHex }
    val balance = tokenRows.firstOrNull { it.accountIndex == account }
    val token = balance?.let { knownShieldedToken(it, tokens) }

    val amountText = viewModel.amountText
    val trimmedAmount = amountText.trim()
    val amount = shieldedTokenSmallestUnitAmount(trimmedAmount)
    val amountRefused = trimmedAmount.isNotEmpty() && amount == null

    val result = viewModel.result
    val isSubmitting = viewModel.isSubmitting
    val canSubmit = !isSubmitting && result == null && recipientReady && amount != null &&
        balance != null && manager != null

    // The proof takes tens of seconds; leaving would drop its answer.
    BackHandler(enabled = isSubmitting) {}

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(title) },
                navigationIcon = {
                    TextButton(
                        onClick = { navController.popBackStack() },
                        enabled = !isSubmitting,
                        modifier = Modifier.testTag("$idPrefix.close"),
                    ) { Text(if (result == null) "Cancel" else "Done") }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState())
                .padding(16.dp).testTag("$idPrefix.form"),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            when (val current = rows) {
                SourceRows.Loading -> CircularProgressIndicator()
                is SourceRows.Failed -> Text(
                    "Could not read the token pool balances: ${current.message}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
                is SourceRows.Loaded -> if (balance == null) {
                    Text(
                        "This wallet no longer lists this token and account in its watched token pools.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                } else {
                    SpendSourceSection(balance, token, showsAccount = tokenRows.size > 1)
                }
            }

            recipientSection(!isSubmitting && result == null)

            FormSection(title = "Amount (smallest unit)") {
                OutlinedTextField(
                    value = amountText,
                    onValueChange = { viewModel.amountText = it },
                    placeholder = { Text("0") },
                    enabled = !isSubmitting && result == null,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().testTag("$idPrefix.amount"),
                )
                AmountFeedback(amountRefused, amount, token, idPrefix)
                val activeManager = manager
                if (balance != null && activeManager != null) {
                    SpendFeeLine(
                        manager = activeManager,
                        walletId = walletId,
                        balance = balance,
                        kind = kind,
                        amount = amount,
                        stateRevision = stateRevision,
                        testTag = "$idPrefix.fee",
                    )
                }
                Text(
                    amountFooter,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Button(
                    onClick = {
                        val activeManager = manager ?: return@Button
                        val source = balance ?: return@Button
                        val value = amount ?: return@Button
                        val call = spend(activeManager, source, value) ?: return@Button
                        viewModel.submit(kind, call)
                    },
                    enabled = canSubmit,
                    modifier = Modifier.fillMaxWidth().testTag("$idPrefix.submit"),
                ) {
                    Text(if (isSubmitting) "Proving and broadcasting…" else submitLabel)
                    if (isSubmitting) {
                        Spacer(Modifier.width(8.dp))
                        CircularProgressIndicator(
                            modifier = Modifier.size(16.dp),
                            strokeWidth = 2.dp,
                            color = MaterialTheme.colorScheme.onPrimary,
                        )
                    }
                }
                submitFooter?.let {
                    Text(
                        it,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            result?.let {
                Text(
                    it.message,
                    style = MaterialTheme.typography.bodyMedium,
                    color = if (it.mayHaveGoneThrough) {
                        MaterialTheme.colorScheme.tertiary
                    } else {
                        MaterialTheme.colorScheme.primary
                    },
                    modifier = Modifier.testTag("$idPrefix.result"),
                )
            }
            viewModel.error?.let { error ->
                Text(
                    error,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.testTag("$idPrefix.error"),
                )
            }
        }
    }
}

/** The rows a spend form reads to find the one it spends from. */
private sealed interface SourceRows {
    data object Loading : SourceRows
    data class Loaded(val balances: List<ShieldedTokenPoolBalance>) : SourceRows
    data class Failed(val message: String) : SourceRows
}

/** The token, account and balances at the top of a spend form. ← Swift `ShieldedTokenSpendSourceSection`. */
@Composable
private fun SpendSourceSection(balance: ShieldedTokenPoolBalance, token: TokenEntity?, showsAccount: Boolean) {
    FormSection {
        LabeledContent("Token", token?.name ?: Base58.encode(balance.tokenId))
        if (showsAccount) {
            LabeledContent("From account", "${balance.accountIndex}")
        }
        // In smallest units, the unit the amount is typed in.
        LabeledContent("Available", "${balance.balance} (smallest units)")
        if (balance.reservedBalance > 0uL) {
            LabeledContent("Pending", "${balance.reservedBalance} (smallest units)")
        }
    }
}

/**
 * An error when the amount text is not a whole number, otherwise, when the
 * app has the token cached, the approximate token amount at the cached
 * decimals (display only). ← Swift `ShieldedTokenAmountFeedback`.
 */
@Composable
private fun AmountFeedback(refused: Boolean, amount: ULong?, token: TokenEntity?, idPrefix: String) {
    if (refused) {
        Text(
            "Enter a whole number of the token's smallest unit: no decimal point or other characters.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.testTag("$idPrefix.precisionError"),
        )
    } else if (amount != null && token != null) {
        Text(
            shieldedTokenDecimalsHint(amount, token),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.testTag("$idPrefix.decimalsHint"),
        )
    }
}

private sealed interface FeeState {
    data object Idle : FeeState
    data object Loading : FeeState
    data class Quoted(val preview: ShieldedTokenSpendPreview) : FeeState
    data class Refused(val message: String) : FeeState
}

/** How long the amount must stay unchanged before the fee is asked for. */
private const val FEE_PREVIEW_DEBOUNCE_MS = 400L

/**
 * Rust's local fee preview for the spend: the fee in credits and the notes
 * it would take, or Rust's refusal when the spend would be refused on the
 * wallet's notes. Asks again a moment after the amount stops changing and
 * when [stateRevision] moves (a shielded sync of the wallet, a return to the
 * form); a newer ask cancels the pending one. Never computes a fee.
 * ← Swift `ShieldedTokenSpendFeeLine`.
 */
@Composable
@Suppress("LongParameterList")
private fun SpendFeeLine(
    manager: PlatformWalletManager,
    walletId: ByteArray,
    balance: ShieldedTokenPoolBalance,
    kind: ShieldedTokenPoolSpendKind,
    amount: ULong?,
    stateRevision: Int,
    testTag: String,
) {
    var state by remember { mutableStateOf<FeeState>(FeeState.Idle) }
    LaunchedEffect(manager, amount, stateRevision) {
        if (amount == null) {
            state = FeeState.Idle
            return@LaunchedEffect
        }
        state = FeeState.Loading
        delay(FEE_PREVIEW_DEBOUNCE_MS)
        state = try {
            FeeState.Quoted(
                manager.shieldedTokenSpendPreview(
                    walletId = walletId,
                    contractId = balance.dataContractId,
                    tokenPosition = balance.tokenContractPosition,
                    kind = kind,
                    amount = amount,
                    account = balance.accountIndex,
                ),
            )
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            FeeState.Refused(e.message ?: "unknown error")
        }
    }
    val modifier = Modifier.fillMaxWidth().testTag(testTag)
    when (val current = state) {
        FeeState.Idle -> Text(
            "Enter an amount to see the fee.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = modifier,
        )
        FeeState.Loading -> Row(modifier, verticalAlignment = Alignment.CenterVertically) {
            Text("Fee", modifier = Modifier.weight(1f))
            CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
        }
        is FeeState.Quoted -> Column(modifier) {
            LabeledContent("Fee", formatCredits(current.preview.fee))
            Text(
                shieldedTokenFeeNotesText(current.preview.tokenNotes, current.preview.feeNotes),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        is FeeState.Refused -> Text(
            current.message,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.tertiary,
            modifier = modifier,
        )
    }
}
