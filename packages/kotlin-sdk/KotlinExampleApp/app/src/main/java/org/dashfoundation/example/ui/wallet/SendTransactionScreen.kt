package org.dashfoundation.example.ui.wallet

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.LifecycleResumeEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import org.dashfoundation.example.services.shielded.ShieldedIdentityPaymentCoordinator
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.funding.ShieldedProver
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.QrScanner
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.shielded.IdentityShieldMessages
import org.dashfoundation.example.ui.shielded.ShieldedPaymentOutcome
import org.dashfoundation.example.ui.tokens.displayLabel
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.DashAddress
import org.dashfoundation.example.util.DashAddressType
import org.dashfoundation.example.util.ScannedPayment
import org.dashfoundation.example.util.formatCredits
import org.dashfoundation.example.util.formatDuffs
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.parseDashToCredits
import org.dashfoundation.example.util.parseDashToDuffs
import org.dashfoundation.example.util.toHex

/**
 * The send flows this screen routes — the Android-bridged subset of iOS
 * `SendFlow` (SendViewModel.swift:6). The three shielded OUTFLOW spends
 * (types 16/17/19) ride the manager wrappers, as do the shields from the
 * Platform balance (type 15) and from an identity's balance (type 21);
 * `coreToShielded` lives on its dedicated screen (`ShieldedFundScreen`), and
 * `platformToPlatform` has the dedicated `TransferPlatformAddressScreen`.
 */
private enum class SendFlow(val displayName: String) {
    CORE_TO_CORE("Core Payment"),
    PLATFORM_TO_SHIELDED("Shield Credits"),
    SHIELDED_TO_SHIELDED("Shielded Transfer"),
    SHIELDED_TO_PLATFORM("Unshield"),
    SHIELDED_TO_CORE("Withdrawal to Core"),
    IDENTITY_TO_SHIELDED("Shield from Identity"),
}

/** Fund source for sending (← iOS `FundSource`, SendViewModel.swift:58). */
private enum class FundSource(val label: String) {
    CORE("Core"),
    PLATFORM("Platform"),
    SHIELDED("Shielded"),
    IDENTITY("Identity"),
}

/**
 * An extra Core output beyond the primary recipient (← iOS `CoreRecipient`,
 * SendViewModel.swift:91). Only the CORE_TO_CORE flow appends these; each
 * field is observable so a row edit recomposes just that row. Held in a plain
 * `mutableStateListOf` (not `rememberSaveable`) — transient like the iOS
 * `@Published` array, and the primary row still round-trips a saved single
 * recipient.
 */
private class AdditionalRecipient {
    var address by mutableStateOf("")
    var amountText by mutableStateOf("")
}

/**
 * One of the wallet's identities as the Identity source shows it, copied out
 * of Room: entities compare by id only, so a balance update would not
 * recompose a list of them.
 */
private data class IdentitySourceOption(
    val idHex: String,
    val base58: String,
    val label: String,
    val balance: Long,
    val index: Int,
)

/**
 * Maximum UTF-8 byte length of a shielded memo (the 32-byte payload of the
 * 36-byte `DashMemo`) — mirrors `SendViewModel.memoByteLimit` /
 * `dpp::shielded::MEMO_PAYLOAD_SIZE`. Rust re-validates.
 */
private const val MEMO_BYTE_LIMIT = 32

/**
 * The shielded account this screen spends from. The balance it shows for the
 * shielded source is this account's, so the two always name the same money.
 */
private const val SHIELDED_SEND_ACCOUNT = 0

/**
 * Send form — port of `SendTransactionView.swift` + `SendViewModel.swift`:
 * recipient with address-family detection (Core / Platform / Shielded via
 * [DashAddress.parse]), a "Send From" source selector, and per-flow routing:
 *
 * - **Core → Core** — `ManagedPlatformWallet.sendToAddresses` (drives the
 *   `CoreTransactionBuilder` steps + broadcast; returns the txid).
 * - **Shielded → Shielded** (Type 16, SH-05) —
 *   `PlatformWalletManager.shieldedTransfer`, optional ≤32-byte UTF-8 memo.
 * - **Shielded → Platform** (Type 17, SH-06) —
 *   `PlatformWalletManager.shieldedUnshield` (bech32m string parsed Rust-side).
 * - **Shielded → Core** (Type 19, SH-08) —
 *   `PlatformWalletManager.shieldedWithdraw` (`coreFeePerByte = 1`).
 * - **Identity → Shielded** (Type 21, SH-21):
 *   `PlatformWalletManager.shieldedShieldFromIdentity`, signed with the
 *   identity's TRANSFER key, into this wallet's own shielded address. Rust
 *   reads the identity's proved balance and refuses an amount plus fee it
 *   does not cover before anything is saved; a call that returns is only
 *   "submitted", and one that may have gone through is listed under
 *   In-Progress Identity Shields in Shielded Activity (SH-23).
 *
 * The shielded flows settle in credits (1 DASH = 1e11) and block through a
 * ~30s Halo 2 proof; their consensus-pinned fee comes from
 * `PlatformWalletManager.estimateShieldedFee` (computed at the manager's
 * network-tracked platform version). A `ShieldedSpendUnconfirmed` outcome is
 * surfaced through the SUCCESS path ("may have gone through — do not
 * retry"), mirroring iOS SendViewModel.swift:790.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SendTransactionScreen(
    walletIdHex: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current

    val scope = rememberCoroutineScope()
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val walletId = remember(walletIdHex) { walletIdHex.hexToBytes() }

    var recipient by rememberSaveable { mutableStateOf("") }
    var amountText by rememberSaveable { mutableStateOf("") }
    var memoText by rememberSaveable { mutableStateOf("") }
    // Extra Core outputs beyond the primary recipient (CORE_TO_CORE only, ←
    // SendViewModel.additionalCoreRecipients). Empty until "Add recipient".
    val additionalRecipients = remember { mutableStateListOf<AdditionalRecipient>() }
    var selectedSource by rememberSaveable { mutableStateOf(FundSource.CORE) }
    // Whether [selectedSource] is the user's own pick, and the kind of the
    // last recipient that parsed; see the source selection below.
    var sourceChosenByUser by rememberSaveable { mutableStateOf(false) }
    var lastRecipientKind by rememberSaveable { mutableStateOf<String?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var isSending by remember { mutableStateOf(false) }
    var sentTxidHex by remember { mutableStateOf<String?>(null) }
    var successMessage by remember { mutableStateOf<String?>(null) }
    // The identity an identity shield debits (its id is public, not key
    // material) (← SendViewModel.selectedIdentityId).
    var selectedIdentityHex by rememberSaveable { mutableStateOf<String?>(null) }

    // Result hand-back from the QR scanner (← QRScannerView's onScan).
    val savedStateHandle = navController.currentBackStackEntry?.savedStateHandle
    LaunchedEffect(savedStateHandle) {
        savedStateHandle
            ?.getStateFlow<String?>(QrScanner.RESULT_KEY, null)
            ?.collect { raw ->
                if (raw != null) {
                    val parsed = ScannedPayment.parse(raw)
                    if (parsed == null) {
                        error = "The scanned code doesn't contain a Dash address."
                    } else {
                        recipient = parsed.address
                        parsed.amount?.let { amountText = it }
                    }
                    savedStateHandle[QrScanner.RESULT_KEY] = null
                }
            }
    }

    // Spendable Core balance from the live wallet handle.
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by remember(manager) {
        manager?.wallets ?: MutableStateFlow(emptyMap<String, ManagedPlatformWallet>())
    }.collectAsStateWithLifecycle()
    val managed = walletsMap[walletIdHex]
    var balance by remember { mutableStateOf<ManagedPlatformWallet.Balance?>(null) }
    LaunchedEffect(managed) {
        while (managed != null && !managed.isClosed) {
            balance = runCatching { managed.balance() }.getOrNull()
            delay(2_000)
        }
    }

    // Ordinary shielded sends spend account zero, so the shielded source is
    // what Rust says that account can spend now. Until a local snapshot is
    // known (null) the source is not offered.
    val hasShielded = remember { Sdk.hasShielded() }
    val shieldedSpendable by remember(walletIdHex) {
        container.shieldedService.accountBalance(walletId, SHIELDED_SEND_ACCOUNT)
    }.collectAsStateWithLifecycle(initialValue = null)
    val shieldedBalance = shieldedSpendable ?: 0L

    // Transparent Platform-Payment balance (credits) — the fund source for a
    // shield (Type 15). Same DAO sum WalletDetailScreen's Platform row reads.
    val platformBalance by remember(walletIdHex) {
        container.database.platformAddressDao().observeByWallet(walletId)
            .map { rows -> rows.sumOf { it.balance } }
    }.collectAsStateWithLifecycle(initialValue = 0L)

    // This wallet's identities with a balance: one of them funds an identity
    // shield (← SendTransactionView.eligibleIdentities).
    val walletIdentities by remember(walletIdHex) {
        container.database.identityDao().observeByWallet(walletId).map { rows ->
            rows.map {
                IdentitySourceOption(
                    idHex = it.identityId.toHex(),
                    base58 = Base58.encode(it.identityId),
                    label = it.displayLabel(),
                    balance = it.balance,
                    index = it.identityIndex,
                )
            }
        }
    }.collectAsStateWithLifecycle(initialValue = emptyList())
    val eligibleIdentities = remember(walletIdentities) {
        walletIdentities.filter { it.balance > 0 }.sortedBy { it.index }
    }
    val selectedIdentity = eligibleIdentities.firstOrNull { it.idHex == selectedIdentityHex }

    // An identity shield runs on the application scope, so a rotation or a
    // tab switch while it proves neither drops its outcome nor allows a
    // second one; this screen shows what the coordinator holds for the
    // wallet (← SendViewModel.isSending, which the view model outlives).
    val paymentCoordinator = container.shieldedIdentityPaymentCoordinator
    val paymentStates by paymentCoordinator.states.collectAsStateWithLifecycle()
    val walletKeyHex = remember(walletIdHex) { walletId.toHex() }
    val identityShieldStates = paymentStates.filterKeys {
        it.kind == ShieldedIdentityPaymentCoordinator.Kind.SHIELD_FROM_IDENTITY &&
            it.walletIdHex == walletKeyHex
    }
    val identityShieldRunning =
        identityShieldStates.values.any { it is ShieldedIdentityPaymentCoordinator.State.InFlight }
    val isBusy = isSending || identityShieldRunning

    // ── Address detection + flow routing (← SendViewModel.detectAddressType /
    //    updateFlow) ────────────────────────────────────────────────────
    val trimmedRecipient = recipient.trim()
    val addressType = remember(trimmedRecipient, network) {
        DashAddress.parse(trimmedRecipient, network)
    }

    // Which sources can fund a send to this recipient family — the
    // Android-bridged subset of iOS `availableSources` (SendViewModel
    // .swift:374): the Shielded source appears once the pool holds notes;
    // Core stays first (and always offered) for Core recipients so the
    // plain L1 flow is the default.
    val hasIdentitySource = hasShielded && eligibleIdentities.isNotEmpty()
    val availableSources = remember(
        addressType,
        hasShielded,
        shieldedBalance,
        platformBalance,
        hasIdentitySource,
    ) {
        val shieldedSource =
            if (hasShielded && shieldedBalance > 0) listOf(FundSource.SHIELDED) else emptyList()
        // Shield-from-Platform (Type 15, SH-03) funds a shielded recipient
        // from the transparent Platform-Payment balance — it needs no
        // existing pool notes, but is only offered when that balance is
        // non-zero, so an empty Platform balance can't auto-select a flow
        // that shows 0 spendable yet still enables submit
        // (← iOS SendViewModel.availableSources gates .platform on
        // platformBalance > 0 for an .orchard recipient).
        val platformShieldSource =
            if (hasShielded && platformBalance > 0) listOf(FundSource.PLATFORM) else emptyList()
        // Shield from an identity's balance (Type 21, SH-21), offered once an
        // identity of this wallet holds credits; last, so the private
        // shielded → shielded path stays the default (← SendViewModel
        // .availableSources).
        val identitySource = if (hasIdentitySource) listOf(FundSource.IDENTITY) else emptyList()
        when (addressType) {
            is DashAddressType.Core -> listOf(FundSource.CORE) + shieldedSource
            is DashAddressType.Orchard -> shieldedSource + platformShieldSource + identitySource
            is DashAddressType.Platform -> shieldedSource
            DashAddressType.Unknown -> emptyList()
        }
    }
    // A source the user picked is never replaced by another one: moving a
    // send the user set to Shielded onto Core would turn a private payment
    // into a public one. When that source stops being offered (its balance
    // went away, or is not known for a moment) it stays selected and Send
    // stays disabled until it is offered again or the user picks another.
    // The choice ends when the recipient changes to another kind.
    val recipientKind = when (addressType) {
        is DashAddressType.Core -> "core"
        is DashAddressType.Orchard -> "orchard"
        is DashAddressType.Platform -> "platform"
        DashAddressType.Unknown -> null
    }
    LaunchedEffect(recipientKind) {
        if (recipientKind != null) {
            if (lastRecipientKind != null && lastRecipientKind != recipientKind) {
                sourceChosenByUser = false
            }
            lastRecipientKind = recipientKind
        }
    }
    // Without a choice of the user's, follow the offered sources: the
    // first one is the default, also when it only appears later (Shielded
    // once its balance is known) while another one was the default until
    // then (← SendTransactionView's source reconciliation).
    LaunchedEffect(availableSources, sourceChosenByUser) {
        if (!sourceChosenByUser) {
            availableSources.firstOrNull()?.let { selectedSource = it }
        }
    }

    val flow: SendFlow? = when (addressType) {
        is DashAddressType.Core ->
            if (selectedSource == FundSource.SHIELDED) SendFlow.SHIELDED_TO_CORE
            else SendFlow.CORE_TO_CORE
        is DashAddressType.Orchard ->
            when (selectedSource) {
                FundSource.SHIELDED -> SendFlow.SHIELDED_TO_SHIELDED
                FundSource.PLATFORM -> SendFlow.PLATFORM_TO_SHIELDED
                FundSource.IDENTITY -> SendFlow.IDENTITY_TO_SHIELDED
                else -> null
            }
        is DashAddressType.Platform ->
            if (selectedSource == FundSource.SHIELDED) SendFlow.SHIELDED_TO_PLATFORM else null
        DashAddressType.Unknown -> null
    }
    val isShieldedFlow = flow != null && flow != SendFlow.CORE_TO_CORE

    // Amount in the flow's settlement unit: Core/L1 settles in duffs (1e8),
    // every shielded flow settles in credits (1e11) — mirroring
    // `SendViewModel.amount` vs `amountCredits`.
    val amountDuffs = parseDashToDuffs(amountText)
    val amountCredits = parseDashToCredits(amountText)

    // Memo (shielded → shielded only): UTF-8 BYTE length, like Rust.
    val trimmedMemo = memoText.trim()
    val memoByteCount = trimmedMemo.toByteArray(Charsets.UTF_8).size
    val memoOverLimit = memoByteCount > MEMO_BYTE_LIMIT

    // Halo 2 prover readiness + consensus-pinned fee for the active
    // shielded flow (← ShieldedFundScreen's prover panel; iOS resolves the
    // same estimator in SendViewModel.estimateFee(for:)). Warms the prover
    // on first entry to a shielded flow so the spend doesn't pay the ~30s
    // key build inline.
    val proverReady by produceState(initialValue = false, isShieldedFlow) {
        if (isShieldedFlow) {
            runCatching { ShieldedProver.warmUp() }
            while (value.not()) {
                value = runCatching { ShieldedProver.isReady() }.getOrDefault(false)
                if (!value) delay(2_000)
            }
        }
    }
    // Re-key on the published protocol version too: the SDK learns the
    // network's version on a background refresh after the manager exists,
    // so an estimate produced before the ratchet completed was computed at
    // the seed version (← iOS SendTransactionView re-resolves on
    // `platformState.platformProtocolVersion` the same way).
    val protocolVersion by appState.platformProtocolVersion.collectAsStateWithLifecycle()
    // ...and re-read on every resume, because the flow alone can go stale:
    // that refresh republishes the unchanged seed when its proven fetch
    // fails, and the SDK independently ratchets its version from ANY
    // proof-verified query's response metadata without publishing at all.
    // The estimate is a pure handle lookup, so re-reading is free.
    var feeReadEpoch by remember { mutableIntStateOf(0) }
    LifecycleResumeEffect(Unit) {
        feeReadEpoch++
        onPauseOrDispose {}
    }
    val shieldedFeeEstimate by produceState<Long?>(
        initialValue = null,
        flow,
        manager,
        protocolVersion,
        feeReadEpoch,
    ) {
        val activeManager = manager
        val kind = when (flow) {
            // Type 15 Shield reserves the same compute_minimum_shielded_fee(2)
            // base as a shielded→shielded transfer (← iOS estimateFee: .transfer
            // for .platformToShielded), so they share the TransferOrShield kind.
            SendFlow.SHIELDED_TO_SHIELDED, SendFlow.PLATFORM_TO_SHIELDED ->
                ShieldedProver.FeeKind.TransferOrShield
            SendFlow.SHIELDED_TO_PLATFORM -> ShieldedProver.FeeKind.Unshield
            SendFlow.SHIELDED_TO_CORE -> ShieldedProver.FeeKind.Withdrawal
            // The fee floor the identity must hold on top of the amount.
            SendFlow.IDENTITY_TO_SHIELDED -> ShieldedProver.FeeKind.ShieldFromIdentity
            SendFlow.CORE_TO_CORE, null -> null
        }
        value = if (activeManager != null && kind != null) {
            runCatching { activeManager.estimateShieldedFee(kind, 2) }.getOrNull()
        } else {
            null
        }
    }

    val recipientKnown = addressType != DashAddressType.Unknown

    // Validated Core batch for CORE_TO_CORE: the ordered output list (primary
    // row + each additional row, in display order) or null if ANY row is
    // invalid (← SendViewModel.coreRecipientPlan / coreRecipients). "Valid"
    // per row = the address parses as a Core address on this network AND its
    // duffs amount is > 0. Built atomically so a single bad extra row blocks
    // the whole send rather than silently dropping an output.
    val coreRecipients: List<Pair<String, Long>>? = run {
        if (flow != SendFlow.CORE_TO_CORE) return@run null
        val out = ArrayList<Pair<String, Long>>(1 + additionalRecipients.size)
        val primaryDuffs = amountDuffs
        if (addressType !is DashAddressType.Core || primaryDuffs == null || primaryDuffs <= 0L) {
            return@run null
        }
        out.add(trimmedRecipient to primaryDuffs)
        for (row in additionalRecipients) {
            val addr = row.address.trim()
            val duffs = parseDashToDuffs(row.amountText)
            if (DashAddress.parse(addr, network) !is DashAddressType.Core ||
                duffs == null || duffs <= 0L
            ) {
                return@run null
            }
            out.add(addr to duffs)
        }
        out
    }
    val coreSendTotalDuffs = coreRecipients?.sumOf { it.second } ?: 0L

    // A restored form can still name a source that is not offered now (the
    // shielded source before its balance is known, for one). The flow is
    // derived from the selection, so it must not be sendable until the
    // source is offered again.
    val sourceOffered = selectedSource in availableSources
    val canSend = sourceOffered && when (flow) {
        SendFlow.CORE_TO_CORE -> coreRecipients != null
        SendFlow.SHIELDED_TO_SHIELDED -> (amountCredits ?: 0) > 0 && !memoOverLimit
        SendFlow.PLATFORM_TO_SHIELDED,
        SendFlow.SHIELDED_TO_PLATFORM, SendFlow.SHIELDED_TO_CORE -> (amountCredits ?: 0) > 0
        // An identity is picked and the fee floor is known. Whether the
        // identity covers amount plus fee is Rust's check against its
        // proved balance, made before anything is saved.
        SendFlow.IDENTITY_TO_SHIELDED ->
            selectedIdentity != null && (amountCredits ?: 0) > 0 && shieldedFeeEstimate != null
        null -> false
    }

    // A shielded spend proves for tens of seconds; leaving the screen would
    // drop its result and allow a second one (← interactiveDismissDisabled).
    BackHandler(enabled = isBusy) {}

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Send Dash") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }, enabled = !isBusy) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .imePadding()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Recipient") {
                OutlinedTextField(
                    value = recipient,
                    onValueChange = { recipient = it },
                    label = { Text("Dash address") },
                    singleLine = true,
                    isError = trimmedRecipient.isNotEmpty() && !recipientKnown,
                    supportingText = {
                        // Address-family badge (← AddressTypeBadge).
                        if (trimmedRecipient.isNotEmpty()) {
                            Text(
                                when (addressType) {
                                    is DashAddressType.Core -> "Core Address"
                                    is DashAddressType.Platform -> "Platform Address"
                                    is DashAddressType.Orchard -> "Shielded Address"
                                    DashAddressType.Unknown -> "Not a valid Dash address"
                                },
                                modifier = Modifier.testTag("send.addressTypeBadge"),
                            )
                        }
                    },
                    trailingIcon = {
                        IconButton(
                            onClick = { navController.navigate(QrScanner) },
                            modifier = Modifier.testTag("send.scanQrButton"),
                        ) {
                            Icon(Icons.Default.QrCodeScanner, contentDescription = "Scan QR code")
                        }
                    },
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 8.dp)
                        .testTag("send.recipientField"),
                )
            }

            FormSection(title = "Amount") {
                OutlinedTextField(
                    value = amountText,
                    onValueChange = { amountText = it },
                    label = { Text("Amount (DASH)") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                    isError = amountText.isNotBlank() &&
                        (if (isShieldedFlow) amountCredits else amountDuffs) == null,
                    supportingText = {
                        if (amountText.isNotBlank()) {
                            if (isShieldedFlow && amountCredits == null) {
                                Text("Enter a positive amount with at most 11 decimals")
                            } else if (!isShieldedFlow && amountDuffs == null) {
                                Text("Enter a positive amount with at most 8 decimals")
                            }
                        }
                    },
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 8.dp)
                        .testTag("send.amountField"),
                )
                if (isShieldedFlow) {
                    amountCredits?.let { LabeledContent("Credits", it.toString()) }
                } else {
                    amountDuffs?.let { LabeledContent("Duffs", it.toString()) }
                }
            }

            // Additional recipients (CORE_TO_CORE only, ← the iOS
            // `additionalRecipientsSection`): extra address/amount rows the
            // batch appends as extra outputs of the same L1 tx (CORE-10). Each
            // row has its own Core-address + amount validation via
            // `coreRecipients`; the Rust coin-selector handles any output count.
            if (flow == SendFlow.CORE_TO_CORE) {
                FormSection(title = "Additional Recipients") {
                    additionalRecipients.forEachIndexed { index, row ->
                        Row(
                            modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                            verticalAlignment = Alignment.Top,
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                OutlinedTextField(
                                    value = row.address,
                                    onValueChange = { row.address = it },
                                    label = { Text("Dash address") },
                                    singleLine = true,
                                    isError = row.address.trim().isNotEmpty() &&
                                        DashAddress.parse(row.address.trim(), network)
                                        !is DashAddressType.Core,
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .testTag("send.extraRecipient.$index.address"),
                                )
                                OutlinedTextField(
                                    value = row.amountText,
                                    onValueChange = { row.amountText = it },
                                    label = { Text("Amount (DASH)") },
                                    singleLine = true,
                                    keyboardOptions = KeyboardOptions(
                                        keyboardType = KeyboardType.Decimal,
                                    ),
                                    isError = row.amountText.isNotBlank() &&
                                        (parseDashToDuffs(row.amountText)?.let { it <= 0L } ?: true),
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .padding(top = 4.dp)
                                        .testTag("send.extraRecipient.$index.amount"),
                                )
                            }
                            IconButton(
                                onClick = { additionalRecipients.removeAt(index) },
                                modifier = Modifier.testTag("send.extraRecipient.$index.remove"),
                            ) {
                                Icon(Icons.Default.Close, contentDescription = "Remove recipient")
                            }
                        }
                    }
                    TextButton(
                        onClick = { additionalRecipients.add(AdditionalRecipient()) },
                        modifier = Modifier.testTag("send.addRecipient"),
                    ) {
                        Icon(Icons.Default.Add, contentDescription = null)
                        Text("Add recipient", modifier = Modifier.padding(start = 8.dp))
                    }
                }
            }

            // Send From (← the iOS "Send From" section) — only shown once
            // the recipient resolves and more than one source can fund it.
            // Also shown when the user's own source is not offered now, so
            // another one can be picked.
            val chosenSourceNotOffered = sourceChosenByUser && !sourceOffered
            if (availableSources.size > 1 || (chosenSourceNotOffered && availableSources.isNotEmpty())) {
                FormSection(title = "Send From") {
                    if (chosenSourceNotOffered) {
                        Text(
                            "${selectedSource.label} is not available right now. Send is " +
                                "off until it is, or pick another source.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                        availableSources.forEachIndexed { index, source ->
                            SegmentedButton(
                                selected = selectedSource == source,
                                onClick = {
                                    selectedSource = source
                                    sourceChosenByUser = true
                                },
                                shape = SegmentedButtonDefaults.itemShape(
                                    index,
                                    availableSources.size,
                                ),
                                modifier = Modifier.testTag("send.source.${source.label.lowercase()}"),
                            ) { Text(source.label) }
                        }
                    }
                }
            }

            // Source identity (identity → shielded only, ← SendTransactionView
            // .identitySourceSection): this wallet's identities with a balance.
            if (flow == SendFlow.IDENTITY_TO_SHIELDED) {
                FormSection(title = "Source Identity") {
                    AccessiblePicker(
                        label = "Identity",
                        options = listOf<String?>(null) + eligibleIdentities.map { it.idHex },
                        selected = selectedIdentity?.idHex,
                        optionLabel = { hex ->
                            eligibleIdentities.firstOrNull { it.idHex == hex }
                                ?.let { "${it.label}: ${formatCreditsExact(it.balance)}" }
                                ?: "Select…"
                        },
                        modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
                        testTag = "send.identitySourcePicker",
                        // The identity that pays is captured at the tap.
                        enabled = !isBusy,
                        optionTestTag = { hex ->
                            eligibleIdentities.firstOrNull { it.idHex == hex }
                                ?.let { "send.identitySource.identity.${it.base58}" }
                                ?: "send.identitySource.none"
                        },
                    ) { selectedIdentityHex = it }
                    if (shieldedFeeEstimate == null) {
                        Text(
                            IdentityShieldMessages.FEE_FLOOR_UNAVAILABLE,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.tertiary,
                        )
                    }
                    Text(
                        IdentityShieldMessages.FOOTER,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            // Memo (shielded → shielded only, ← SendTransactionView's Memo
            // section): the on-chain note carries an optional 32-byte UTF-8
            // memo. Gated on the FLOW, not the recipient family.
            if (flow == SendFlow.SHIELDED_TO_SHIELDED) {
                FormSection(title = "Memo (optional)") {
                    OutlinedTextField(
                        value = memoText,
                        onValueChange = { memoText = it },
                        label = { Text("Note for the recipient") },
                        singleLine = true,
                        isError = memoOverLimit,
                        supportingText = {
                            Text(
                                "$memoByteCount/$MEMO_BYTE_LIMIT bytes",
                                color = if (memoOverLimit) {
                                    MaterialTheme.colorScheme.error
                                } else {
                                    MaterialTheme.colorScheme.onSurfaceVariant
                                },
                            )
                        },
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 8.dp)
                            .testTag("send.memoField"),
                    )
                }
            }

            FormSection(title = "Summary") {
                flow?.let {
                    LabeledContent("Transaction type", it.displayName)
                }
                LabeledContent(
                    "Spendable",
                    when (selectedSource) {
                        FundSource.CORE -> balance?.let { formatDuffs(it.confirmed) } ?: "—"
                        FundSource.PLATFORM -> formatCredits(platformBalance)
                        FundSource.SHIELDED ->
                            shieldedSpendable?.let { formatCredits(it) } ?: "—"
                        FundSource.IDENTITY ->
                            selectedIdentity?.let { formatCreditsExact(it.balance) } ?: "Unknown"
                    },
                )
                // Multi-output batch total (← the iOS "Outputs" summary's
                // Total row): the sum across every output of this one L1 tx.
                if (flow == SendFlow.CORE_TO_CORE && (coreRecipients?.size ?: 0) > 1) {
                    LabeledContent(
                        "Total to send (${coreRecipients?.size} outputs)",
                        formatDuffs(coreSendTotalDuffs),
                    )
                }
                if (isShieldedFlow) {
                    // Consensus-pinned flat shielded fee in credits (2-action
                    // single-note spend with change), from the Rust estimator.
                    LabeledContent(
                        if (flow == SendFlow.IDENTITY_TO_SHIELDED) "Fee (at least)" else "Estimated fee",
                        shieldedFeeEstimate?.let { formatCredits(it) } ?: "—",
                    )
                    LabeledContent(
                        "Halo 2 prover",
                        if (proverReady) "Ready" else "Preparing…",
                    )
                } else {
                    // Static Core fee estimate (← SendFlow.coreToCore.estimatedFee).
                    LabeledContent("Estimated fee", formatDuffs(500_000))
                }
            }

            if (trimmedRecipient.isNotEmpty() && recipientKnown && availableSources.isEmpty()) {
                // The funding advice is for a balance known to be zero. While
                // the shielded balance has not been read yet, say that.
                val shieldedNotKnown = hasShielded && shieldedSpendable == null
                Text(
                    when (addressType) {
                        is DashAddressType.Orchard, is DashAddressType.Platform ->
                            if (shieldedNotKnown) {
                                "The shielded balance is not known yet."
                            } else {
                                "No spendable shielded balance to fund this send — " +
                                    "shield funds first (Wallet → Shielded Balance → Fund Shielded)."
                            }
                        else -> "No spendable balance to fund this send."
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            SubmitButton(
                text = "Send",
                isLoading = isBusy,
                enabled = canSend && !isBusy,
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("send.submitButton"),
            ) {
                val activeManager = manager
                if (activeManager == null || flow == null || !sourceOffered) {
                    error = "Wallet is not ready. Try again in a moment."
                    return@SubmitButton
                }
                // The identity selected at the tap pays, and only while it is
                // still the selected one (← SendViewModel.identityToSend).
                val tappedIdentityHex = selectedIdentityHex
                isSending = true
                // Why the typed recipient is not this wallet's own default
                // shielded address, or null. Rust always shields into that
                // address, so another one would read as a payment to it while
                // nothing reaches it (← SendViewModel.selfShieldRecipientProblem).
                suspend fun selfShieldRecipientProblem(): String? {
                    val recipientRaw = (addressType as? DashAddressType.Orchard)?.raw43
                        ?: return "Invalid recipient"
                    val ownShielded = runCatching {
                        activeManager.shieldedDefaultAddress(walletId)
                    }.getOrNull()
                        // Unreadable is not "matches": without this wallet's
                        // own address the recipient cannot be checked.
                        ?: return "This wallet's shielded address could not be " +
                            "read, so the recipient could not be checked. " +
                            "Nothing was sent."
                    if (!ownShielded.contentEquals(recipientRaw)) {
                        return "Shield always sends to your own shielded " +
                            "address; enter this wallet's own shielded " +
                            "address as the recipient."
                    }
                    return null
                }
                scope.launch {
                    try {
                        // The values this handler captured are those of the
                        // last composition. A balance update can land after
                        // it and before this tap is handled, so a shielded
                        // spend reads the balance as it is now.
                        if (selectedSource == FundSource.SHIELDED) {
                            val spendableNow = container.shieldedService
                                .accountBalance(walletId, SHIELDED_SEND_ACCOUNT)
                                .first()
                            if (spendableNow == null || spendableNow <= 0L) {
                                error = "The shielded balance changed. Check the " +
                                    "source and tap Send again."
                                return@launch
                            }
                        }
                        when (flow) {
                            SendFlow.CORE_TO_CORE -> {
                                val wallet = managed
                                val outputs = coreRecipients
                                if (wallet == null || outputs == null) {
                                    error = "Wallet is not ready. Try again in a moment."
                                    return@launch
                                }
                                // One L1 tx, N outputs (primary + additionals) —
                                // the batch is already fully validated (CORE-10).
                                sentTxidHex = wallet.sendToAddresses(
                                    recipients = outputs,
                                    network = network,
                                    coreSignerHandle = activeManager.mnemonicResolverHandle,
                                )
                            }

                            SendFlow.PLATFORM_TO_SHIELDED -> {
                                val credits = amountCredits
                                if (credits == null) {
                                    error = "Invalid recipient or amount"
                                    return@launch
                                }
                                // Shield from Platform balance (Type 15, SH-03):
                                // Rust always shields into THIS wallet's own
                                // default Orchard pool, so constrain to
                                // self-shield (← iOS platformToShielded guard).
                                selfShieldRecipientProblem()?.let {
                                    error = it
                                    return@launch
                                }
                                activeManager.shieldedShield(
                                    walletId = walletId,
                                    amount = credits,
                                )
                                successMessage = "Shielding complete"
                            }

                            SendFlow.SHIELDED_TO_SHIELDED -> {
                                val recipientRaw =
                                    (addressType as? DashAddressType.Orchard)?.raw43
                                val credits = amountCredits
                                if (recipientRaw == null || credits == null) {
                                    error = "Invalid recipient or amount"
                                    return@launch
                                }
                                activeManager.shieldedTransfer(
                                    walletId = walletId,
                                    recipientRaw43 = recipientRaw,
                                    amount = credits,
                                    account = SHIELDED_SEND_ACCOUNT,
                                    memo = trimmedMemo.ifEmpty { null },
                                )
                                successMessage = "Shielded transfer complete"
                            }

                            SendFlow.SHIELDED_TO_PLATFORM -> {
                                val credits = amountCredits
                                if (credits == null) {
                                    error = "Invalid amount"
                                    return@launch
                                }
                                // The bech32m string is forwarded as-is —
                                // Rust parses + network-checks it.
                                activeManager.shieldedUnshield(
                                    walletId = walletId,
                                    toPlatformAddress = trimmedRecipient,
                                    amount = credits,
                                    account = SHIELDED_SEND_ACCOUNT,
                                )
                                successMessage = "Unshield complete"
                            }

                            SendFlow.SHIELDED_TO_CORE -> {
                                val credits = amountCredits
                                if (credits == null) {
                                    error = "Invalid amount"
                                    return@launch
                                }
                                activeManager.shieldedWithdraw(
                                    walletId = walletId,
                                    toCoreAddress = trimmedRecipient,
                                    amount = credits,
                                    coreFeePerByte = 1,
                                    account = SHIELDED_SEND_ACCOUNT,
                                )
                                successMessage = "Withdrawal submitted"
                            }

                            SendFlow.IDENTITY_TO_SHIELDED -> {
                                val credits = amountCredits
                                if (credits == null) {
                                    error = "Invalid amount"
                                    return@launch
                                }
                                selfShieldRecipientProblem()?.let {
                                    error = it
                                    return@launch
                                }
                                val identityHex = tappedIdentityHex
                                if (identityHex == null || identityHex != selectedIdentityHex) {
                                    error = IdentityShieldMessages.IDENTITY_CHANGED
                                    return@launch
                                }
                                // Signed with the identity's TRANSFER key by
                                // the manager's identity signer. The call and
                                // its outcome belong to the coordinator, not
                                // to this screen.
                                val key = ShieldedIdentityPaymentCoordinator.Key(
                                    ShieldedIdentityPaymentCoordinator.Kind.SHIELD_FROM_IDENTITY,
                                    walletKeyHex,
                                    identityHex,
                                )
                                val started = paymentCoordinator.start(key, credits) {
                                    try {
                                        activeManager.shieldedShieldFromIdentity(
                                            walletId = walletId,
                                            identityId = identityHex.hexToBytes(),
                                            amount = credits,
                                        )
                                        IdentityShieldMessages.outcome(null)
                                    } catch (e: Exception) {
                                        IdentityShieldMessages.outcome(e)
                                    }
                                }
                                if (!started) error = IdentityShieldMessages.ALREADY_RUNNING
                            }
                        }
                    } catch (t: Throwable) {
                        if (shieldedOperationMayHaveGoneThrough(t)) {
                            // It may have reached a node and its result is not
                            // proved (a node's refusal or verdict included,
                            // code 18), or the SDK could not confirm a
                            // transition it may have accepted (code 20), so it
                            // may already be on chain and a retry signs a
                            // second transition: this must NOT read as a
                            // retryable failure (← SendViewModel.swift).
                            successMessage = "Transaction may have gone through — waiting for " +
                                "the next shielded sync to confirm. Do not retry."
                        } else {
                            error = t.message ?: "Failed to send the transaction."
                        }
                    } finally {
                        isSending = false
                    }
                }
            }
        }
    }

    sentTxidHex?.let { txid ->
        androidx.compose.material3.AlertDialog(
            onDismissRequest = {
                sentTxidHex = null
                navController.popBackStack()
            },
            title = { Text("Payment Sent") },
            text = {
                Text(
                    "Your transaction was signed and broadcast.\n\nTxid:\n$txid",
                    modifier = Modifier.testTag("send.successTxid"),
                )
            },
            confirmButton = {
                androidx.compose.material3.TextButton(
                    onClick = {
                        sentTxidHex = null
                        navController.popBackStack()
                    },
                ) { Text("Done") }
            },
        )
    }

    // Shielded-flow outcome (no txid to show — the transition settles on
    // Platform; the note/balance change arrives on the next shielded sync).
    successMessage?.let { message ->
        androidx.compose.material3.AlertDialog(
            onDismissRequest = {
                successMessage = null
                navController.popBackStack()
            },
            title = { Text("Success") },
            text = {
                Text(message, modifier = Modifier.testTag("send.successMessage"))
            },
            confirmButton = {
                androidx.compose.material3.TextButton(
                    onClick = {
                        successMessage = null
                        navController.popBackStack()
                    },
                ) { Text("Done") }
            },
        )
    }

    // An identity shield is never reported as a success: a returned call is
    // submitted, and one that may have gone through is not confirmed yet
    // (← SendTransactionView's "Submitted" / "Not Confirmed Yet" alerts).
    identityShieldStates.entries
        .firstOrNull { it.value is ShieldedIdentityPaymentCoordinator.State.Done }
        ?.let { (key, state) ->
            val outcome = (state as ShieldedIdentityPaymentCoordinator.State.Done).outcome
            if (outcome is ShieldedPaymentOutcome.Failed) {
                ErrorAlertDialog(
                    message = outcome.message,
                    onDismiss = { paymentCoordinator.acknowledge(key) },
                )
            } else {
                val submitted = outcome == ShieldedPaymentOutcome.Submitted
                val close = {
                    paymentCoordinator.acknowledge(key)
                    navController.popBackStack()
                    Unit
                }
                androidx.compose.material3.AlertDialog(
                    onDismissRequest = close,
                    title = { Text(if (submitted) "Submitted" else "Not Confirmed Yet") },
                    text = {
                        Text(
                            if (submitted) {
                                IdentityShieldMessages.SUBMITTED
                            } else {
                                IdentityShieldMessages.MAY_HAVE_GONE_THROUGH
                            },
                            modifier = Modifier.testTag(
                                if (submitted) "send.submittedMessage" else "send.unconfirmedMessage",
                            ),
                        )
                    },
                    confirmButton = { TextButton(onClick = close) { Text("Done") } },
                )
            }
        }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

/**
 * Whether a shielded operation's error means it may already be on chain, so
 * no retry is offered: a spend whose result is not proved (18) or a state
 * transition the SDK could not confirm after it may have been accepted (20).
 * ← Swift `SendViewModel.mayHaveGoneThrough`.
 */
internal fun shieldedOperationMayHaveGoneThrough(error: Throwable): Boolean =
    error is DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed ||
        error is DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed ||
        error is DashSdkError.StateTransitionResultUnconfirmed
