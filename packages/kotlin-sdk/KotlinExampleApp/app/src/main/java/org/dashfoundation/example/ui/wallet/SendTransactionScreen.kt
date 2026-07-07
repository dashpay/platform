package org.dashfoundation.example.ui.wallet

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
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.funding.ShieldedProver
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.QrScanner
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.DashAddress
import org.dashfoundation.example.util.DashAddressType
import org.dashfoundation.example.util.ScannedPayment
import org.dashfoundation.example.util.formatCredits
import org.dashfoundation.example.util.formatDuffs
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.parseDashToCredits
import org.dashfoundation.example.util.parseDashToDuffs

/**
 * The send flows this screen routes — the Android-bridged subset of iOS
 * `SendFlow` (SendViewModel.swift:6). The three shielded OUTFLOW spends
 * (types 16/17/19) ride the manager wrappers; the shield inflows
 * (`coreToShielded` / `platformToShielded`) live on their dedicated
 * screens (`ShieldedFundScreen`) / aren't bridged yet, and
 * `platformToPlatform` has the dedicated `TransferPlatformAddressScreen`.
 */
private enum class SendFlow(val displayName: String) {
    CORE_TO_CORE("Core Payment"),
    PLATFORM_TO_SHIELDED("Shield Credits"),
    SHIELDED_TO_SHIELDED("Shielded Transfer"),
    SHIELDED_TO_PLATFORM("Unshield"),
    SHIELDED_TO_CORE("Withdrawal to Core"),
}

/** Fund source for sending (← iOS `FundSource`, SendViewModel.swift:58). */
private enum class FundSource(val label: String) {
    CORE("Core"),
    PLATFORM("Platform"),
    SHIELDED("Shielded"),
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
 * Maximum UTF-8 byte length of a shielded memo (the 32-byte payload of the
 * 36-byte `DashMemo`) — mirrors `SendViewModel.memoByteLimit` /
 * `dpp::shielded::MEMO_PAYLOAD_SIZE`. Rust re-validates.
 */
private const val MEMO_BYTE_LIMIT = 32

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
 *
 * The shielded flows settle in credits (1 DASH = 1e11) and block through a
 * ~30s Halo 2 proof; their consensus-pinned fee comes from
 * [ShieldedProver.estimateFee]. A `ShieldedSpendUnconfirmed` outcome is
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
    var error by remember { mutableStateOf<String?>(null) }
    var isSending by remember { mutableStateOf(false) }
    var sentTxidHex by remember { mutableStateOf<String?>(null) }
    var successMessage by remember { mutableStateOf<String?>(null) }

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

    // Shielded credits — unspent-note sum from Room, only on a shielded
    // build (same source WalletDetailScreen's Shielded row reads).
    val hasShielded = remember { Sdk.hasShielded() }
    val shieldedBalance by remember(walletIdHex) {
        if (hasShielded) {
            container.database.shieldedDao().observeUnspentNotesByWallet(walletId)
                .map { notes -> notes.sumOf { it.value } }
        } else {
            MutableStateFlow(0L)
        }
    }.collectAsStateWithLifecycle(initialValue = 0L)

    // Transparent Platform-Payment balance (credits) — the fund source for a
    // shield (Type 15). Same DAO sum WalletDetailScreen's Platform row reads.
    val platformBalance by remember(walletIdHex) {
        container.database.platformAddressDao().observeByWallet(walletId)
            .map { rows -> rows.sumOf { it.balance } }
    }.collectAsStateWithLifecycle(initialValue = 0L)

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
    val availableSources = remember(addressType, hasShielded, shieldedBalance) {
        val shieldedSource =
            if (hasShielded && shieldedBalance > 0) listOf(FundSource.SHIELDED) else emptyList()
        // Shield-from-Platform (Type 15, SH-03) is offered for a shielded
        // recipient on any shielded build — it funds from the transparent
        // Platform-Payment balance, so it needs no existing pool notes
        // (← iOS availableSources offering .platform for an .orchard recipient).
        val platformShieldSource =
            if (hasShielded) listOf(FundSource.PLATFORM) else emptyList()
        when (addressType) {
            is DashAddressType.Core -> listOf(FundSource.CORE) + shieldedSource
            is DashAddressType.Orchard -> shieldedSource + platformShieldSource
            is DashAddressType.Platform -> shieldedSource
            DashAddressType.Unknown -> emptyList()
        }
    }
    // Auto-select the first available source when the recipient family
    // changes (← SendTransactionView.autoSelectSource).
    LaunchedEffect(availableSources) {
        if (selectedSource !in availableSources) {
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
    val shieldedFeeEstimate by produceState<Long?>(initialValue = null, flow) {
        value = when (flow) {
            // Type 15 Shield reserves the same compute_minimum_shielded_fee(2)
            // base as a shielded→shielded transfer (← iOS estimateFee: .transfer
            // for .platformToShielded), so they share the TransferOrShield kind.
            SendFlow.SHIELDED_TO_SHIELDED, SendFlow.PLATFORM_TO_SHIELDED ->
                runCatching {
                    ShieldedProver.estimateFee(ShieldedProver.FeeKind.TransferOrShield, 2)
                }.getOrNull()
            SendFlow.SHIELDED_TO_PLATFORM ->
                runCatching {
                    ShieldedProver.estimateFee(ShieldedProver.FeeKind.Unshield, 2)
                }.getOrNull()
            SendFlow.SHIELDED_TO_CORE ->
                runCatching {
                    ShieldedProver.estimateFee(ShieldedProver.FeeKind.Withdrawal, 2)
                }.getOrNull()
            SendFlow.CORE_TO_CORE, null -> null
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

    val canSend = when (flow) {
        SendFlow.CORE_TO_CORE -> coreRecipients != null
        SendFlow.SHIELDED_TO_SHIELDED -> (amountCredits ?: 0) > 0 && !memoOverLimit
        SendFlow.PLATFORM_TO_SHIELDED,
        SendFlow.SHIELDED_TO_PLATFORM, SendFlow.SHIELDED_TO_CORE -> (amountCredits ?: 0) > 0
        null -> false
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Send Dash") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
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
            if (availableSources.size > 1) {
                FormSection(title = "Send From") {
                    SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                        availableSources.forEachIndexed { index, source ->
                            SegmentedButton(
                                selected = selectedSource == source,
                                onClick = { selectedSource = source },
                                shape = SegmentedButtonDefaults.itemShape(
                                    index,
                                    availableSources.size,
                                ),
                                modifier = Modifier.testTag("send.source.${source.label}"),
                            ) { Text(source.label) }
                        }
                    }
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
                        FundSource.SHIELDED -> formatCredits(shieldedBalance)
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
                        "Estimated fee",
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
                Text(
                    when (addressType) {
                        is DashAddressType.Orchard, is DashAddressType.Platform ->
                            "No spendable shielded balance to fund this send — " +
                                "shield funds first (Wallet → Shielded Balance → Fund Shielded)."
                        else -> "No spendable balance to fund this send."
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            SubmitButton(
                text = "Send",
                isLoading = isSending,
                enabled = canSend && !isSending,
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("send.submitButton"),
            ) {
                val activeManager = manager
                if (activeManager == null || flow == null) {
                    error = "Wallet is not ready. Try again in a moment."
                    return@SubmitButton
                }
                isSending = true
                scope.launch {
                    try {
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
                                val recipientRaw =
                                    (addressType as? DashAddressType.Orchard)?.raw43
                                val credits = amountCredits
                                if (recipientRaw == null || credits == null) {
                                    error = "Invalid recipient or amount"
                                    return@launch
                                }
                                // Shield from Platform balance (Type 15, SH-03):
                                // Rust always shields into THIS wallet's own
                                // default Orchard pool, so the typed recipient is
                                // ignored on-chain. Constrain to self-shield so a
                                // different pasted Orchard address can't read as
                                // success while nothing reaches it (← iOS
                                // platformToShielded guard).
                                val ownShielded = runCatching {
                                    activeManager.shieldedDefaultAddress(walletId)
                                }.getOrNull()
                                if (ownShielded != null &&
                                    !ownShielded.contentEquals(recipientRaw)
                                ) {
                                    error = "Shield always sends to your own shielded " +
                                        "address — enter this wallet's own shielded " +
                                        "address as the recipient."
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
                                )
                                successMessage = "Withdrawal submitted"
                            }
                        }
                    } catch (e: DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed) {
                        // Broadcast accepted but its execution result couldn't
                        // be confirmed — the spend may already be on chain and
                        // the notes stay reserved Rust-side, so this must NOT
                        // read as a retryable failure (← SendViewModel.swift:790).
                        successMessage = "Transaction may have gone through — waiting for " +
                            "the next shielded sync to confirm. Do not retry."
                    } catch (t: Throwable) {
                        error = t.message ?: "Failed to send the transaction."
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

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}
